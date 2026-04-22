//! Unified dispatch queue for both time-driven and issue-driven modes.
//!
//! Provides a priority queue with fairness between different dispatch sources,
//! including per-project round-robin within the same priority score so that
//! one noisy project cannot starve the others.

use std::collections::{HashMap, VecDeque};

/// Synthetic project id used for legacy single-project mode.
pub const LEGACY_PROJECT_ID: &str = "__global__";

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
        /// Project id this agent belongs to.
        project: String,
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
        /// Project id that owns the tracker producing this issue.
        project: String,
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
        /// Project id this mention was detected in.
        project: String,
        /// ULID identifying this mention chain (same across nested mentions).
        chain_id: String,
        /// Current depth in the mention chain (0 = initial human mention).
        depth: u32,
        /// Name of the agent that triggered this mention (empty for human).
        parent_agent: String,
    },
    /// PR review dispatch — triggers the automated review pipeline.
    ReviewPr {
        /// Pull request number.
        pr_number: u64,
        /// Project id owning this PR.
        project: String,
        /// HEAD commit SHA at dispatch time.
        head_sha: String,
        /// GitHub/Gitea login of the PR author.
        author_login: String,
        /// PR title.
        title: String,
        /// Lines of code changed (used for routing / cost gating).
        diff_loc: u32,
    },
    /// Auto-merge dispatch — merges a PR after all required checks pass.
    AutoMerge {
        /// Pull request number.
        pr_number: u64,
        /// Project id owning this PR.
        project: String,
        /// HEAD commit SHA that passed checks.
        head_sha: String,
    },
    /// Post-merge test-gate dispatch — runs regression gates after a merge.
    PostMergeTestGate {
        /// Pull request number that was merged.
        pr_number: u64,
        /// Project id owning this PR.
        project: String,
        /// Merge commit SHA.
        merge_sha: String,
        /// PR title (for issue/comment attribution).
        title: String,
    },
}

impl DispatchTask {
    /// Project id this task is associated with.
    pub fn project(&self) -> &str {
        match self {
            DispatchTask::TimeDriven { project, .. } => project,
            DispatchTask::IssueDriven { project, .. } => project,
            DispatchTask::MentionDriven { project, .. } => project,
            DispatchTask::ReviewPr { project, .. } => project,
            DispatchTask::AutoMerge { project, .. } => project,
            DispatchTask::PostMergeTestGate { project, .. } => project,
        }
    }
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
    /// Internal priority queue (sorted ascending by score, then seq).
    queue: VecDeque<PrioritizedTask>,
    /// Sequence counter for FIFO ordering within same priority.
    seq_counter: u64,
    /// Monotonic dequeue counter used to timestamp per-project service order.
    dequeue_counter: u64,
    /// Last dequeue counter value recorded for each project. Projects that
    /// have never been dequeued are treated as "least recently served" and
    /// are preferred for round-robin fairness.
    last_dequeue_seq: HashMap<String, u64>,
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
    /// Current in-queue task counts by source.
    pub by_source: HashMap<String, u64>,
    /// Current in-queue task counts by project.
    pub by_project: HashMap<String, u64>,
}

impl Dispatcher {
    /// Create a new dispatcher.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            seq_counter: 0,
            dequeue_counter: 0,
            last_dequeue_seq: HashMap::new(),
            stats: DispatcherStats::default(),
        }
    }

    /// Enqueue a task with automatic priority computation.
    pub fn enqueue(&mut self, task: DispatchTask) {
        let seq = self.seq_counter;
        self.seq_counter += 1;

        let score = self.compute_priority(&task);
        let source = self.task_source(&task);
        let project = task.project().to_string();

        let pt = PrioritizedTask { seq, score, task };

        // Maintain insertion order by (score, seq) for deterministic FIFO
        // within equal scores. Round-robin fairness is applied in dequeue().
        let insert_pos = self
            .queue
            .iter()
            .position(|existing| {
                existing.score > score || (existing.score == score && existing.seq > seq)
            })
            .unwrap_or(self.queue.len());

        self.queue.insert(insert_pos, pt);

        self.stats.total_enqueued += 1;
        self.stats.current_depth = self.queue.len();
        *self.stats.by_source.entry(source).or_insert(0) += 1;
        *self.stats.by_project.entry(project).or_insert(0) += 1;

        tracing::debug!(score, depth = self.stats.current_depth, "task enqueued");
    }

    /// Dequeue the highest priority task, applying per-project round-robin
    /// among tasks that share the lowest priority score.
    pub fn dequeue(&mut self) -> Option<DispatchTask> {
        if self.queue.is_empty() {
            return None;
        }

        let min_score = self.queue.front().map(|pt| pt.score)?;

        // Find the tied-score prefix and select the entry whose project was
        // least recently dequeued. Break ties by seq (FIFO).
        let mut best_idx: usize = 0;
        let mut best_key: Option<(u64, u64)> = None;
        for (i, pt) in self.queue.iter().enumerate() {
            if pt.score != min_score {
                break;
            }
            let project = pt.task.project();
            let last = self.last_dequeue_seq.get(project).copied().unwrap_or(0);
            let key = (last, pt.seq);
            if best_key.map_or(true, |b| key < b) {
                best_key = Some(key);
                best_idx = i;
            }
        }

        let pt = self.queue.remove(best_idx)?;
        let source = self.task_source(&pt.task);
        let project = pt.task.project().to_string();

        self.dequeue_counter += 1;
        self.last_dequeue_seq
            .insert(project.clone(), self.dequeue_counter);

        self.stats.total_dequeued += 1;
        self.stats.current_depth = self.queue.len();
        if let Some(count) = self.stats.by_source.get_mut(&source) {
            *count = count.saturating_sub(1);
        }
        if let Some(count) = self.stats.by_project.get_mut(&project) {
            *count = count.saturating_sub(1);
        }

        tracing::debug!(depth = self.stats.current_depth, "task dequeued");
        Some(pt.task)
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
            DispatchTask::TimeDriven { layer, .. } => match layer {
                crate::AgentLayer::Safety => 0,
                crate::AgentLayer::Core => 1000,
                crate::AgentLayer::Growth => 2000,
            },
            DispatchTask::IssueDriven {
                priority,
                pagerank_score,
                ..
            } => {
                let base = priority.map(|p| p as i64 * 100).unwrap_or(500);
                let pagerank_bonus = pagerank_score.map(|pr| -(pr * 100.0) as i64).unwrap_or(0);
                base + pagerank_bonus + 3000
            }
            DispatchTask::MentionDriven { .. } => 200,
            DispatchTask::ReviewPr { .. } => 400,
            DispatchTask::AutoMerge { .. } => 500,
            DispatchTask::PostMergeTestGate { .. } => 600,
        }
    }

    /// Get source identifier for a task.
    fn task_source(&self, task: &DispatchTask) -> String {
        match task {
            DispatchTask::TimeDriven { .. } => "time_driven".into(),
            DispatchTask::IssueDriven { .. } => "issue_driven".into(),
            DispatchTask::MentionDriven { .. } => "mention_driven".into(),
            DispatchTask::ReviewPr { .. } => "review_pr".into(),
            DispatchTask::AutoMerge { .. } => "auto_merge".into(),
            DispatchTask::PostMergeTestGate { .. } => "post_merge_gate".into(),
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

    fn time_task(name: &str, project: &str, layer: crate::AgentLayer) -> DispatchTask {
        DispatchTask::TimeDriven {
            name: name.into(),
            task: "task".into(),
            layer,
            project: project.into(),
        }
    }

    #[test]
    fn test_enqueue_dequeue() {
        let mut dispatcher = Dispatcher::new();

        dispatcher.enqueue(time_task(
            "test",
            LEGACY_PROJECT_ID,
            crate::AgentLayer::Safety,
        ));

        assert_eq!(dispatcher.depth(), 1);

        let task = dispatcher.dequeue();
        assert!(task.is_some());
        assert_eq!(dispatcher.depth(), 0);
    }

    #[test]
    fn test_priority_ordering() {
        let mut dispatcher = Dispatcher::new();

        dispatcher.enqueue(time_task(
            "growth",
            LEGACY_PROJECT_ID,
            crate::AgentLayer::Growth,
        ));
        dispatcher.enqueue(time_task(
            "core",
            LEGACY_PROJECT_ID,
            crate::AgentLayer::Core,
        ));
        dispatcher.enqueue(time_task(
            "safety",
            LEGACY_PROJECT_ID,
            crate::AgentLayer::Safety,
        ));

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
    fn test_fifo_within_same_priority_and_project() {
        let mut dispatcher = Dispatcher::new();

        dispatcher.enqueue(time_task("first", "alpha", crate::AgentLayer::Safety));
        dispatcher.enqueue(time_task("second", "alpha", crate::AgentLayer::Safety));

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
            project: "alpha".into(),
        });

        dispatcher.enqueue(DispatchTask::IssueDriven {
            identifier: "high-pr".into(),
            title: "High PageRank".into(),
            priority: Some(1),
            pagerank_score: Some(2.5),
            project: "alpha".into(),
        });

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

        dispatcher.enqueue(time_task("safety", "alpha", crate::AgentLayer::Safety));
        dispatcher.enqueue(DispatchTask::IssueDriven {
            identifier: "issue-1".into(),
            title: "Issue".into(),
            priority: Some(1),
            pagerank_score: None,
            project: "beta".into(),
        });

        let stats = dispatcher.stats();
        assert_eq!(stats.total_enqueued, 2);
        assert_eq!(stats.current_depth, 2);
        assert_eq!(stats.by_source.get("time_driven"), Some(&1));
        assert_eq!(stats.by_source.get("issue_driven"), Some(&1));
        assert_eq!(stats.by_project.get("alpha"), Some(&1));
        assert_eq!(stats.by_project.get("beta"), Some(&1));

        dispatcher.dequeue();

        let stats = dispatcher.stats();
        assert_eq!(stats.total_dequeued, 1);
        assert_eq!(stats.current_depth, 1);
    }

    #[test]
    fn test_round_robin_across_projects_within_same_score() {
        let mut dispatcher = Dispatcher::new();

        // Three tasks for alpha enqueued before beta's single task.
        dispatcher.enqueue(time_task("a1", "alpha", crate::AgentLayer::Core));
        dispatcher.enqueue(time_task("a2", "alpha", crate::AgentLayer::Core));
        dispatcher.enqueue(time_task("a3", "alpha", crate::AgentLayer::Core));
        dispatcher.enqueue(time_task("b1", "beta", crate::AgentLayer::Core));

        // First dequeue: alpha is earliest by seq, and neither project has
        // been served -> alpha wins on seq tie-break.
        let t1 = dispatcher.dequeue().unwrap();
        assert_eq!(t1.project(), "alpha");

        // Second dequeue: beta has never been served, alpha now has a
        // recent dequeue seq -> beta should jump ahead of alpha's backlog.
        let t2 = dispatcher.dequeue().unwrap();
        assert_eq!(t2.project(), "beta");

        // Remaining two tasks are alpha only; FIFO among them.
        let t3 = dispatcher.dequeue().unwrap();
        assert_eq!(t3.project(), "alpha");
        let t4 = dispatcher.dequeue().unwrap();
        assert_eq!(t4.project(), "alpha");

        assert!(dispatcher.dequeue().is_none());
    }

    #[test]
    fn dispatch_task_review_pr_project() {
        let t = DispatchTask::ReviewPr {
            pr_number: 1,
            project: "odilo".to_string(),
            head_sha: "abc".to_string(),
            author_login: "claude-code".to_string(),
            title: "fix".to_string(),
            diff_loc: 10,
        };
        assert_eq!(t.project(), "odilo");
    }

    #[test]
    fn dispatch_task_auto_merge_project() {
        let t = DispatchTask::AutoMerge {
            pr_number: 1,
            project: "terraphim".to_string(),
            head_sha: "x".to_string(),
        };
        assert_eq!(t.project(), "terraphim");
    }

    #[test]
    fn dispatch_task_post_merge_project() {
        let t = DispatchTask::PostMergeTestGate {
            pr_number: 1,
            project: "digital-twins".to_string(),
            merge_sha: "x".to_string(),
            title: "t".to_string(),
        };
        assert_eq!(t.project(), "digital-twins");
    }

    #[test]
    fn test_round_robin_does_not_override_priority() {
        let mut dispatcher = Dispatcher::new();

        // High-priority alpha task should dequeue before low-priority beta
        // even though beta has never been served.
        dispatcher.enqueue(time_task("a-growth", "alpha", crate::AgentLayer::Growth));
        dispatcher.enqueue(time_task("a-safety", "alpha", crate::AgentLayer::Safety));
        dispatcher.enqueue(time_task("b-growth", "beta", crate::AgentLayer::Growth));

        let t1 = dispatcher.dequeue().unwrap();
        match t1 {
            DispatchTask::TimeDriven { name, .. } => assert_eq!(name, "a-safety"),
            _ => panic!("expected TimeDriven"),
        }

        // Now alpha has been served; beta's Growth should win round-robin
        // over alpha's Growth backlog.
        let t2 = dispatcher.dequeue().unwrap();
        assert_eq!(t2.project(), "beta");

        let t3 = dispatcher.dequeue().unwrap();
        assert_eq!(t3.project(), "alpha");
    }
}
