//! Unified dispatch queue for both time-driven and issue-driven modes.
//!
//! Provides a priority queue with fairness between different dispatch sources.

use std::collections::VecDeque;

/// A dispatch task from any source.
#[derive(Debug, Clone)]
pub enum DispatchTask {
    /// Time-driven agent dispatch.
    TimeDriven {
        /// Agent name.
        name: String,
        /// Task to execute.
        task: String,
        /// Layer (Safety, Core, Growth).
        layer: crate::AgentLayer,
    },
    /// Issue-driven agent dispatch.
    IssueDriven {
        /// Issue identifier.
        identifier: String,
        /// Issue title.
        title: String,
        /// Priority (lower = higher).
        priority: Option<i32>,
        /// PageRank score (higher = more important).
        pagerank_score: Option<f64>,
    },
    /// Mention-driven agent dispatch (from @adf: comment mentions).
    MentionDriven {
        /// Name of the agent to dispatch.
        agent_name: String,
        /// Issue number where the mention was detected.
        issue_number: u64,
        /// Comment ID that contained the mention.
        comment_id: u64,
        /// Full comment body for context.
        context: String,
    },
}

/// Priority wrapper for dispatch tasks.
#[derive(Debug)]
struct PrioritizedTask {
    /// Sequential insertion order for tie-breaking.
    seq: u64,
    /// Computed priority score (lower = higher priority).
    score: i64,
    /// The actual task.
    task: DispatchTask,
}

/// Unified dispatcher queue.
pub struct Dispatcher {
    /// Internal priority queue.
    queue: VecDeque<PrioritizedTask>,
    /// Sequence counter for FIFO ordering within same priority.
    seq_counter: u64,
    /// Statistics.
    stats: DispatcherStats,
}

/// Dispatcher statistics.
#[derive(Debug, Default, Clone)]
pub struct DispatcherStats {
    /// Total tasks enqueued.
    pub total_enqueued: u64,
    /// Total tasks dequeued.
    pub total_dequeued: u64,
    /// Current queue depth.
    pub current_depth: usize,
    /// Tasks by source.
    pub by_source: std::collections::HashMap<String, u64>,
}

impl Dispatcher {
    /// Create a new dispatcher.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            seq_counter: 0,
            stats: DispatcherStats::default(),
        }
    }

    /// Enqueue a task with automatic priority computation.
    pub fn enqueue(&mut self, task: DispatchTask) {
        let seq = self.seq_counter;
        self.seq_counter += 1;

        let score = self.compute_priority(&task);
        let source = self.task_source(&task);

        let pt = PrioritizedTask { seq, score, task };

        // Insert maintaining priority order (lower score = higher priority)
        let insert_pos = self
            .queue
            .iter()
            .position(|existing| {
                // Higher priority (lower score) comes first
                // For same score, earlier sequence comes first (FIFO)
                existing.score > score || (existing.score == score && existing.seq > seq)
            })
            .unwrap_or(self.queue.len());

        self.queue.insert(insert_pos, pt);

        // Update stats
        self.stats.total_enqueued += 1;
        self.stats.current_depth = self.queue.len();
        *self.stats.by_source.entry(source).or_insert(0) += 1;

        tracing::debug!(score, depth = self.stats.current_depth, "task enqueued");
    }

    /// Dequeue the highest priority task.
    pub fn dequeue(&mut self) -> Option<DispatchTask> {
        let task = self.queue.pop_front().map(|pt| {
            let source = self.task_source(&pt.task);

            // Update stats
            self.stats.total_dequeued += 1;
            self.stats.current_depth = self.queue.len();
            if let Some(count) = self.stats.by_source.get_mut(&source) {
                *count -= 1;
            }

            pt.task
        });

        if task.is_some() {
            tracing::debug!(depth = self.stats.current_depth, "task dequeued");
        }

        task
    }

    /// Peek at the next task without removing it.
    pub fn peek(&self) -> Option<&DispatchTask> {
        self.queue.front().map(|pt| &pt.task)
    }

    /// Get current queue depth.
    pub fn depth(&self) -> usize {
        self.queue.len()
    }

    /// Get dispatcher statistics.
    pub fn stats(&self) -> &DispatcherStats {
        &self.stats
    }

    /// Compute priority score for a task (lower = higher priority).
    fn compute_priority(&self, task: &DispatchTask) -> i64 {
        match task {
            DispatchTask::TimeDriven { layer, .. } => {
                // Safety agents have highest priority (lowest score)
                // Core = medium, Growth = lowest
                match layer {
                    crate::AgentLayer::Safety => 0,
                    crate::AgentLayer::Core => 1000,
                    crate::AgentLayer::Growth => 2000,
                }
            }
            DispatchTask::IssueDriven {
                priority,
                pagerank_score,
                ..
            } => {
                // Base priority from issue priority (lower = more urgent)
                let base = priority.map(|p| p as i64 * 100).unwrap_or(500);

                // Adjust by PageRank (higher PageRank = more important = lower score)
                let pagerank_bonus = pagerank_score.map(|pr| -(pr * 100.0) as i64).unwrap_or(0);

                // Time-driven gets slight priority over issue-driven at same urgency
                base + pagerank_bonus + 3000
            }
            DispatchTask::MentionDriven { .. } => {
                // Mention-driven tasks sit between Safety (0) and Core (1000).
                // Priority 200 ensures mentions are handled promptly but Safety
                // agent restarts remain the highest priority.
                200
            }
        }
    }

    /// Get source identifier for a task.
    fn task_source(&self, task: &DispatchTask) -> String {
        match task {
            DispatchTask::TimeDriven { .. } => "time_driven".into(),
            DispatchTask::IssueDriven { .. } => "issue_driven".into(),
            DispatchTask::MentionDriven { .. } => "mention_driven".into(),
        }
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_dequeue() {
        let mut dispatcher = Dispatcher::new();

        dispatcher.enqueue(DispatchTask::TimeDriven {
            name: "test".into(),
            task: "do something".into(),
            layer: crate::AgentLayer::Safety,
        });

        assert_eq!(dispatcher.depth(), 1);

        let task = dispatcher.dequeue();
        assert!(task.is_some());
        assert_eq!(dispatcher.depth(), 0);
    }

    #[test]
    fn test_priority_ordering() {
        let mut dispatcher = Dispatcher::new();

        // Enqueue in reverse priority order
        dispatcher.enqueue(DispatchTask::TimeDriven {
            name: "growth".into(),
            task: "task".into(),
            layer: crate::AgentLayer::Growth,
        });

        dispatcher.enqueue(DispatchTask::TimeDriven {
            name: "core".into(),
            task: "task".into(),
            layer: crate::AgentLayer::Core,
        });

        dispatcher.enqueue(DispatchTask::TimeDriven {
            name: "safety".into(),
            task: "task".into(),
            layer: crate::AgentLayer::Safety,
        });

        // Should dequeue in priority order: Safety, Core, Growth
        if let Some(DispatchTask::TimeDriven { name, .. }) = dispatcher.dequeue() {
            assert_eq!(name, "safety");
        }
        if let Some(DispatchTask::TimeDriven { name, .. }) = dispatcher.dequeue() {
            assert_eq!(name, "core");
        }
        if let Some(DispatchTask::TimeDriven { name, .. }) = dispatcher.dequeue() {
            assert_eq!(name, "growth");
        }
    }

    #[test]
    fn test_fifo_within_same_priority() {
        let mut dispatcher = Dispatcher::new();

        dispatcher.enqueue(DispatchTask::TimeDriven {
            name: "first".into(),
            task: "task".into(),
            layer: crate::AgentLayer::Safety,
        });

        dispatcher.enqueue(DispatchTask::TimeDriven {
            name: "second".into(),
            task: "task".into(),
            layer: crate::AgentLayer::Safety,
        });

        if let Some(DispatchTask::TimeDriven { name, .. }) = dispatcher.dequeue() {
            assert_eq!(name, "first");
        }
        if let Some(DispatchTask::TimeDriven { name, .. }) = dispatcher.dequeue() {
            assert_eq!(name, "second");
        }
    }

    #[test]
    fn test_pagerank_priority() {
        let mut dispatcher = Dispatcher::new();

        dispatcher.enqueue(DispatchTask::IssueDriven {
            identifier: "low-pr".into(),
            title: "Low PageRank".into(),
            priority: Some(1),
            pagerank_score: Some(0.15),
        });

        dispatcher.enqueue(DispatchTask::IssueDriven {
            identifier: "high-pr".into(),
            title: "High PageRank".into(),
            priority: Some(1),
            pagerank_score: Some(2.5),
        });

        // Higher PageRank should come first
        if let Some(DispatchTask::IssueDriven { identifier, .. }) = dispatcher.dequeue() {
            assert_eq!(identifier, "high-pr");
        }
        if let Some(DispatchTask::IssueDriven { identifier, .. }) = dispatcher.dequeue() {
            assert_eq!(identifier, "low-pr");
        }
    }

    #[test]
    fn test_stats_tracking() {
        let mut dispatcher = Dispatcher::new();

        dispatcher.enqueue(DispatchTask::TimeDriven {
            name: "safety".into(),
            task: "task".into(),
            layer: crate::AgentLayer::Safety,
        });

        dispatcher.enqueue(DispatchTask::IssueDriven {
            identifier: "issue-1".into(),
            title: "Issue".into(),
            priority: Some(1),
            pagerank_score: None,
        });

        let stats = dispatcher.stats();
        assert_eq!(stats.total_enqueued, 2);
        assert_eq!(stats.current_depth, 2);
        assert_eq!(stats.by_source.get("time_driven"), Some(&1));
        assert_eq!(stats.by_source.get("issue_driven"), Some(&1));

        dispatcher.dequeue();

        let stats = dispatcher.stats();
        assert_eq!(stats.total_dequeued, 1);
        assert_eq!(stats.current_depth, 1);
    }
}
