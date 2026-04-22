//! Mention chain tracking for depth limiting and cycle detection.
//!
//! When agents mention other agents via `@adf:agent-name` in their Gitea
//! comments, the orchestrator tracks the chain depth to prevent infinite
//! loops and control blast radius. This module provides:
//!
//! - **Depth limiting**: Reject dispatches that exceed `max_mention_depth`
//! - **Cycle detection**: Prevent direct A->B->A loops
//! - **Context building**: Structured markdown for inter-agent handoff

/// Default maximum mention chain nesting depth.
pub const DEFAULT_MAX_MENTION_DEPTH: u32 = 3;

/// Errors from mention chain validation.
#[derive(Debug, thiserror::Error)]
pub enum MentionChainError {
    #[error("agent '{agent}' cannot mention itself")]
    SelfMention { agent: String },

    #[error("mention chain depth {depth} exceeds max {max_depth} for agent '{agent}'")]
    DepthExceeded {
        depth: u32,
        max_depth: u32,
        agent: String,
    },

    #[error("cycle detected: {from} -> {to} would create a loop")]
    CycleDetected { from: String, to: String },
}

/// Stateless mention chain validation.
///
/// All chain metadata lives in [`crate::dispatcher::DispatchTask::MentionDriven`]
/// fields. This tracker provides pure check functions with no mutable state.
pub struct MentionChainTracker;

impl MentionChainTracker {
    /// Check if a mention dispatch should proceed.
    ///
    /// Returns `Ok(())` if the dispatch is safe, `Err(MentionChainError)` if blocked.
    ///
    /// # Arguments
    /// * `depth` - Current depth in the mention chain (0 = initial human mention)
    /// * `parent_agent` - Name of the agent that triggered this mention (empty for human)
    /// * `target_agent` - Name of the agent being mentioned
    /// * `max_depth` - Maximum allowed depth from config
    pub fn check(
        depth: u32,
        parent_agent: &str,
        target_agent: &str,
        max_depth: u32,
    ) -> Result<(), MentionChainError> {
        if parent_agent == target_agent && !parent_agent.is_empty() {
            return Err(MentionChainError::SelfMention {
                agent: target_agent.to_string(),
            });
        }

        if depth >= max_depth {
            return Err(MentionChainError::DepthExceeded {
                depth,
                max_depth,
                agent: target_agent.to_string(),
            });
        }

        Ok(())
    }

    /// Build structured mention context for the spawned agent's task.
    ///
    /// Produces a markdown block that the agent can use to understand
    /// why it was mentioned and what context to carry forward.
    pub fn build_context(args: &MentionContextArgs, max_depth: u32) -> String {
        let remaining = max_depth.saturating_sub(args.depth + 1);
        let parent_line = if args.parent_agent.is_empty() {
            "Triggered by: human mention".to_string()
        } else {
            format!(
                "Triggered by: `@adf:{}` on issue #{}",
                args.parent_agent, args.issue_number
            )
        };

        let body_excerpt = if args.comment_body.len() > 2000 {
            format!("{}\n...[truncated]", &args.comment_body[..2000])
        } else {
            args.comment_body.clone()
        };

        let agents_section = if args.available_agents.is_empty() {
            String::new()
        } else {
            let list: String = args
                .available_agents
                .iter()
                .map(|a| format!("- `@adf:{}`", a))
                .collect::<Vec<_>>()
                .join("\n");
            format!("\nAvailable agents to mention:\n{}\n", list)
        };

        format!(
            "---\n\
             **Mention Context** (chain: `{}`, depth: {})\n\
             {}\n\
             ---\n\
             \n{}\n\
             \n---\n\
             When your work is complete, you may mention another agent using \
             `@adf:agent-name` in your output.{}\n\
             Maximum mention chain depth remaining: {}\n\
             ---",
            args.chain_id, args.depth, parent_line, body_excerpt, agents_section, remaining
        )
    }
}

/// Arguments for building mention context.
#[derive(Debug, Clone, Default)]
pub struct MentionContextArgs {
    /// Name of the agent that triggered this mention (empty for human).
    pub parent_agent: String,
    /// Issue number where the mention appeared.
    pub issue_number: u64,
    /// Body of the comment containing the mention.
    pub comment_body: String,
    /// Current depth in the mention chain.
    pub depth: u32,
    /// ULID identifying this mention chain.
    pub chain_id: String,
    /// Names of other agents available for downstream mentions.
    pub available_agents: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_mention_rejected() {
        let result = MentionChainTracker::check(0, "agent-a", "agent-a", 3);
        assert!(matches!(result, Err(MentionChainError::SelfMention { .. })));
    }

    #[test]
    fn test_depth_limit_enforced() {
        let result = MentionChainTracker::check(3, "agent-a", "agent-b", 3);
        assert!(matches!(
            result,
            Err(MentionChainError::DepthExceeded { depth: 3, .. })
        ));
    }

    #[test]
    fn test_depth_zero_allowed() {
        let result = MentionChainTracker::check(0, "", "agent-a", 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_depth_one_allowed() {
        let result = MentionChainTracker::check(1, "agent-a", "agent-b", 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_depth_two_allowed() {
        let result = MentionChainTracker::check(2, "agent-a", "agent-b", 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_depth_three_blocked() {
        let result = MentionChainTracker::check(3, "agent-a", "agent-b", 3);
        assert!(matches!(
            result,
            Err(MentionChainError::DepthExceeded { .. })
        ));
    }

    #[test]
    fn test_cycle_detection_ab_a() {
        let result = MentionChainTracker::check(1, "agent-b", "agent-a", 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_different_agents_allowed() {
        let result = MentionChainTracker::check(2, "agent-a", "agent-c", 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_default_mention_depth() {
        assert_eq!(DEFAULT_MAX_MENTION_DEPTH, 3);
    }

    #[test]
    fn test_build_context_includes_chain_id() {
        let args = MentionContextArgs {
            parent_agent: "agent-a".to_string(),
            issue_number: 42,
            comment_body: "please review".to_string(),
            depth: 1,
            chain_id: "01HZTEST123".to_string(),
            available_agents: vec![],
        };
        let ctx = MentionChainTracker::build_context(&args, 3);
        assert!(ctx.contains("01HZTEST123"));
        assert!(ctx.contains("depth: 1"));
        assert!(ctx.contains("@adf:agent-a"));
        assert!(ctx.contains("#42"));
    }

    #[test]
    fn test_build_context_includes_remaining_depth() {
        let args = MentionContextArgs {
            parent_agent: "agent-a".to_string(),
            issue_number: 1,
            comment_body: "do thing".to_string(),
            depth: 1,
            chain_id: "chain-1".to_string(),
            available_agents: vec![],
        };
        let ctx = MentionChainTracker::build_context(&args, 3);
        assert!(ctx.contains("remaining: 1"));
    }

    #[test]
    fn test_build_context_human_mention() {
        let args = MentionContextArgs {
            parent_agent: String::new(),
            issue_number: 5,
            comment_body: "please check".to_string(),
            depth: 0,
            chain_id: "chain-human".to_string(),
            available_agents: vec![],
        };
        let ctx = MentionChainTracker::build_context(&args, 3);
        assert!(ctx.contains("human mention"));
        assert!(ctx.contains("remaining: 2"));
    }

    #[test]
    fn test_build_context_truncates_long_body() {
        let long_body = "x".repeat(3000);
        let args = MentionContextArgs {
            parent_agent: String::new(),
            issue_number: 1,
            comment_body: long_body,
            depth: 0,
            chain_id: "chain-1".to_string(),
            available_agents: vec![],
        };
        let ctx = MentionChainTracker::build_context(&args, 3);
        assert!(ctx.contains("[truncated]"));
    }

    #[test]
    fn test_empty_parent_not_self_mention() {
        let result = MentionChainTracker::check(0, "", "agent-a", 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_depth_zero_at_zero_max() {
        let result = MentionChainTracker::check(0, "", "agent-a", 0);
        assert!(matches!(
            result,
            Err(MentionChainError::DepthExceeded { .. })
        ));
    }

    #[test]
    fn test_build_context_includes_available_agents() {
        let args = MentionContextArgs {
            parent_agent: String::new(),
            issue_number: 1,
            comment_body: "please review".to_string(),
            depth: 0,
            chain_id: "chain-1".to_string(),
            available_agents: vec!["reviewer".to_string(), "coder".to_string()],
        };
        let ctx = MentionChainTracker::build_context(&args, 3);
        assert!(ctx.contains("Available agents to mention"));
        assert!(ctx.contains("`@adf:reviewer`"));
        assert!(ctx.contains("`@adf:coder`"));
    }

    #[test]
    fn test_build_context_empty_agents_no_section() {
        let args = MentionContextArgs {
            parent_agent: String::new(),
            issue_number: 1,
            comment_body: "please review".to_string(),
            depth: 0,
            chain_id: "chain-1".to_string(),
            available_agents: vec![],
        };
        let ctx = MentionChainTracker::build_context(&args, 3);
        assert!(!ctx.contains("Available agents to mention"));
    }
}
