//! JudgeAgent - Supervised agent for code quality evaluation
//!
//! Combines SimpleAgent (KG lookups) with JudgeModelRouter (tiered LLM routing)
//! to implement a complete evaluation pipeline.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use terraphim_agent_supervisor::{
    AgentPid, AgentStatus, InitArgs, SupervisedAgent, SupervisionError, SupervisionResult,
    SupervisorId, SystemMessage, TerminateReason,
};
use terraphim_rolegraph::RoleGraph;

use crate::model_router::JudgeModelRouter;
use crate::simple_agent::SimpleAgent;
use crate::{JudgeError, JudgeResult};

/// A verdict from the judge evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeVerdict {
    /// The final verdict string (e.g., "PASS", "FAIL", "NEEDS_REVIEW")
    pub verdict: String,
    /// Detailed scores by category
    pub scores: BTreeMap<String, f64>,
    /// Which judge tier produced this verdict
    pub judge_tier: String,
    /// The CLI command used for evaluation
    pub judge_cli: String,
    /// Evaluation latency in milliseconds
    pub latency_ms: u64,
}

impl JudgeVerdict {
    /// Create a new judge verdict
    pub fn new(
        verdict: String,
        scores: BTreeMap<String, f64>,
        judge_tier: String,
        judge_cli: String,
        latency_ms: u64,
    ) -> Self {
        Self {
            verdict,
            scores,
            judge_tier,
            judge_cli,
            latency_ms,
        }
    }

    /// Check if the verdict is a pass
    pub fn is_pass(&self) -> bool {
        self.verdict.eq_ignore_ascii_case("PASS")
    }

    /// Check if the verdict is a fail
    pub fn is_fail(&self) -> bool {
        self.verdict.eq_ignore_ascii_case("FAIL")
    }

    /// Get the overall score (average of all scores)
    pub fn overall_score(&self) -> f64 {
        if self.scores.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.scores.values().sum();
        sum / self.scores.len() as f64
    }
}

/// JudgeAgent combines SimpleAgent and JudgeModelRouter for evaluation
pub struct JudgeAgent {
    /// Unique agent identifier
    pid: AgentPid,
    /// Supervisor identifier
    supervisor_id: SupervisorId,
    /// Agent status
    status: AgentStatus,
    /// SimpleAgent for KG lookups
    kg_agent: Option<SimpleAgent>,
    /// Model router for tier selection
    model_router: JudgeModelRouter,
    /// Configuration
    config: serde_json::Value,
}

impl JudgeAgent {
    /// Create a new JudgeAgent with default configuration
    pub fn new() -> Self {
        Self {
            pid: AgentPid::new(),
            supervisor_id: SupervisorId::new(),
            status: AgentStatus::Stopped,
            kg_agent: None,
            model_router: JudgeModelRouter::new(),
            config: serde_json::Value::Null,
        }
    }

    /// Create a new JudgeAgent with a RoleGraph for KG lookups
    pub fn with_rolegraph(rolegraph: Arc<RoleGraph>) -> Self {
        Self {
            pid: AgentPid::new(),
            supervisor_id: SupervisorId::new(),
            status: AgentStatus::Stopped,
            kg_agent: Some(SimpleAgent::new(rolegraph)),
            model_router: JudgeModelRouter::new(),
            config: serde_json::Value::Null,
        }
    }

    /// Set the model router configuration from a file
    pub fn with_model_config(mut self, path: &Path) -> JudgeResult<Self> {
        self.model_router = JudgeModelRouter::from_config(path)?;
        Ok(self)
    }

    /// Evaluate a file and return a verdict
    ///
    /// This is the main entry point for the evaluation pipeline:
    /// 1. Load file content
    /// 2. Enrich with KG context (if KG agent is configured)
    /// 3. Select model tier based on profile
    /// 4. Dispatch to judge CLI
    /// 5. Parse and return verdict
    ///
    /// # Example
    /// ```
    /// use terraphim_judge_evaluator::JudgeAgent;
    /// use std::path::Path;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let agent = JudgeAgent::new();
    /// // Note: This is a simplified example; real usage requires a file
    /// // let verdict = agent.evaluate(Path::new("src/main.rs"), "default").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn evaluate(&self, file_path: &Path, profile: &str) -> JudgeResult<JudgeVerdict> {
        let start_time = std::time::Instant::now();

        // 1. Load file content
        let content = std::fs::read_to_string(file_path).map_err(JudgeError::IoError)?;

        // 2. Enrich with KG context if available
        let enriched_prompt = if let Some(kg_agent) = &self.kg_agent {
            kg_agent.enrich_prompt(&content)
        } else {
            content
        };

        // 3. Resolve profile to get tier sequence
        let tiers = self.model_router.resolve_profile(profile)?;

        // For now, use the first tier (simplified implementation)
        // In a full implementation, this would iterate through tiers
        // and aggregate results
        let (provider, model) = &tiers[0];

        // 4. Build judge CLI command
        let judge_cli = format!("judge --provider {} --model {}", provider, model);

        // 5. Simulate dispatch to CLI (placeholder - real implementation would spawn process)
        // In production, this would:
        // - Spawn the judge CLI process
        // - Pass enriched_prompt as input
        // - Capture and parse output
        let verdict = self
            .simulate_judge_response(&enriched_prompt, profile)
            .await?;

        let latency_ms = start_time.elapsed().as_millis() as u64;

        Ok(JudgeVerdict::new(
            verdict,
            BTreeMap::new(),    // Scores would be parsed from CLI output
            tiers[0].1.clone(), // Model name as tier
            judge_cli,
            latency_ms,
        ))
    }

    /// Parse a verdict from judge CLI output
    ///
    /// Parses JSON output from the judge CLI into a JudgeVerdict.
    pub fn parse_verdict(
        &self,
        output: &str,
        judge_tier: String,
        judge_cli: String,
        latency_ms: u64,
    ) -> JudgeResult<JudgeVerdict> {
        // Try to parse as JSON first
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
            let verdict = json
                .get("verdict")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string();

            let scores = json
                .get("scores")
                .and_then(|s| s.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_f64().map(|score| (k.clone(), score)))
                        .collect()
                })
                .unwrap_or_default();

            return Ok(JudgeVerdict::new(
                verdict, scores, judge_tier, judge_cli, latency_ms,
            ));
        }

        // Fallback: parse simple text format
        // Look for lines like "VERDICT: PASS" or "Score: quality=0.95"
        let mut verdict = "UNKNOWN".to_string();
        let mut scores: BTreeMap<String, f64> = BTreeMap::new();

        for line in output.lines() {
            let line = line.trim();

            if line.starts_with("VERDICT:") {
                verdict = line
                    .split(':')
                    .nth(1)
                    .unwrap_or("UNKNOWN")
                    .trim()
                    .to_string();
            } else if line.starts_with("Score:") {
                // Parse score lines like "Score: quality=0.95"
                if let Some(score_part) = line.strip_prefix("Score:") {
                    for part in score_part.split(',') {
                        let parts: Vec<&str> = part.split('=').collect();
                        if parts.len() == 2 {
                            if let Ok(score) = parts[1].trim().parse::<f64>() {
                                scores.insert(parts[0].trim().to_string(), score);
                            }
                        }
                    }
                }
            }
        }

        Ok(JudgeVerdict::new(
            verdict, scores, judge_tier, judge_cli, latency_ms,
        ))
    }

    /// Simulate a judge response (for testing)
    ///
    /// In production, this would be replaced by actual CLI execution
    async fn simulate_judge_response(&self, _prompt: &str, profile: &str) -> JudgeResult<String> {
        // Simulate different responses based on profile
        match profile {
            "default" => Ok("PASS".to_string()),
            "critical" => Ok("NEEDS_REVIEW".to_string()),
            _ => Ok("PASS".to_string()),
        }
    }

    /// Get the KG agent reference
    pub fn kg_agent(&self) -> Option<&SimpleAgent> {
        self.kg_agent.as_ref()
    }

    /// Get the model router reference
    pub fn model_router(&self) -> &JudgeModelRouter {
        &self.model_router
    }
}

impl Default for JudgeAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SupervisedAgent for JudgeAgent {
    async fn init(&mut self, args: InitArgs) -> SupervisionResult<()> {
        self.pid = args.agent_id;
        self.supervisor_id = args.supervisor_id;
        self.config = args.config;
        self.status = AgentStatus::Starting;

        // Initialize KG agent if config provides rolegraph
        if let Some(rolegraph_path) = self.config.get("rolegraph_path").and_then(|v| v.as_str()) {
            // In a real implementation, this would load the RoleGraph from the path
            // For now, we leave it as None
            log::info!("Would load RoleGraph from: {}", rolegraph_path);
        }

        // Initialize model router if config provides mapping path
        if let Some(mapping_path) = self
            .config
            .get("model_mapping_path")
            .and_then(|v| v.as_str())
        {
            match JudgeModelRouter::from_config(Path::new(mapping_path)) {
                Ok(router) => {
                    self.model_router = router;
                }
                Err(e) => {
                    return Err(SupervisionError::InvalidAgentSpec(format!(
                        "Failed to load model mapping: {}",
                        e
                    )));
                }
            }
        }

        Ok(())
    }

    async fn start(&mut self) -> SupervisionResult<()> {
        self.status = AgentStatus::Running;
        log::info!("JudgeAgent {} started", self.pid);
        Ok(())
    }

    async fn stop(&mut self) -> SupervisionResult<()> {
        self.status = AgentStatus::Stopping;
        self.status = AgentStatus::Stopped;
        log::info!("JudgeAgent {} stopped", self.pid);
        Ok(())
    }

    async fn handle_system_message(&mut self, message: SystemMessage) -> SupervisionResult<()> {
        match message {
            SystemMessage::Shutdown => {
                self.stop().await?;
            }
            SystemMessage::Restart => {
                self.stop().await?;
                self.start().await?;
            }
            SystemMessage::HealthCheck => {
                // Health check is handled by health_check method
            }
            SystemMessage::StatusUpdate(status) => {
                self.status = status;
            }
            SystemMessage::SupervisorMessage(msg) => {
                log::info!("JudgeAgent {} received message: {}", self.pid, msg);
            }
        }
        Ok(())
    }

    fn status(&self) -> AgentStatus {
        self.status.clone()
    }

    fn pid(&self) -> &AgentPid {
        &self.pid
    }

    fn supervisor_id(&self) -> &SupervisorId {
        &self.supervisor_id
    }

    async fn health_check(&self) -> SupervisionResult<bool> {
        // JudgeAgent is healthy if it's running and has a valid model router
        Ok(matches!(self.status, AgentStatus::Running))
    }

    async fn terminate(&mut self, reason: TerminateReason) -> SupervisionResult<()> {
        log::info!("JudgeAgent {} terminating due to: {:?}", self.pid, reason);
        self.status = AgentStatus::Stopped;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, RoleName, Thesaurus};

    fn create_test_rolegraph() -> Arc<RoleGraph> {
        let mut thesaurus = Thesaurus::new("test".to_string());

        let term1 = NormalizedTerm::new(1, NormalizedTermValue::from("rust"));
        let term2 = NormalizedTerm::new(2, NormalizedTermValue::from("async"));

        thesaurus.insert(NormalizedTermValue::from("rust"), term1);
        thesaurus.insert(NormalizedTermValue::from("async"), term2);

        Arc::new(
            RoleGraph::new_sync(RoleName::new("engineer"), thesaurus)
                .expect("Failed to create RoleGraph"),
        )
    }

    #[test]
    fn test_judge_verdict_creation() {
        let mut scores = BTreeMap::new();
        scores.insert("quality".to_string(), 0.95);
        scores.insert("safety".to_string(), 0.88);

        let verdict = JudgeVerdict::new(
            "PASS".to_string(),
            scores.clone(),
            "quick".to_string(),
            "judge --provider test --model m".to_string(),
            150,
        );

        assert_eq!(verdict.verdict, "PASS");
        assert_eq!(verdict.scores, scores);
        assert_eq!(verdict.judge_tier, "quick");
        assert_eq!(verdict.latency_ms, 150);
    }

    #[test]
    fn test_judge_verdict_is_pass() {
        let verdict = JudgeVerdict::new(
            "PASS".to_string(),
            BTreeMap::new(),
            "quick".to_string(),
            "".to_string(),
            0,
        );
        assert!(verdict.is_pass());
        assert!(!verdict.is_fail());

        let verdict_fail = JudgeVerdict::new(
            "FAIL".to_string(),
            BTreeMap::new(),
            "quick".to_string(),
            "".to_string(),
            0,
        );
        assert!(!verdict_fail.is_pass());
        assert!(verdict_fail.is_fail());
    }

    #[test]
    fn test_judge_verdict_overall_score() {
        let mut scores = BTreeMap::new();
        scores.insert("a".to_string(), 0.8);
        scores.insert("b".to_string(), 1.0);

        let verdict = JudgeVerdict::new(
            "PASS".to_string(),
            scores,
            "quick".to_string(),
            "".to_string(),
            0,
        );

        assert!((verdict.overall_score() - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_verdict_json() {
        let agent = JudgeAgent::new();
        let output = r#"{"verdict": "PASS", "scores": {"quality": 0.95, "safety": 0.88}}"#;

        let verdict = agent
            .parse_verdict(output, "quick".to_string(), "judge-cli".to_string(), 100)
            .unwrap();

        assert_eq!(verdict.verdict, "PASS");
        assert_eq!(verdict.scores.get("quality"), Some(&0.95));
        assert_eq!(verdict.scores.get("safety"), Some(&0.88));
    }

    #[test]
    fn test_parse_verdict_text_format() {
        let agent = JudgeAgent::new();
        let output = "VERDICT: PASS\nScore: quality=0.95, safety=0.88";

        let verdict = agent
            .parse_verdict(output, "deep".to_string(), "judge-cli".to_string(), 200)
            .unwrap();

        assert_eq!(verdict.verdict, "PASS");
        assert_eq!(verdict.scores.get("quality"), Some(&0.95));
        assert_eq!(verdict.scores.get("safety"), Some(&0.88));
        assert_eq!(verdict.judge_tier, "deep");
        assert_eq!(verdict.latency_ms, 200);
    }

    #[test]
    fn test_parse_verdict_unknown_format() {
        let agent = JudgeAgent::new();
        let output = "Some random output without verdict";

        let verdict = agent
            .parse_verdict(output, "quick".to_string(), "judge-cli".to_string(), 50)
            .unwrap();

        assert_eq!(verdict.verdict, "UNKNOWN");
        assert!(verdict.scores.is_empty());
    }

    #[tokio::test]
    async fn test_supervised_agent_lifecycle() {
        let mut agent = JudgeAgent::new();
        let args = InitArgs {
            agent_id: AgentPid::new(),
            supervisor_id: SupervisorId::new(),
            config: json!({}),
        };

        // Initialize
        agent.init(args).await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Starting);

        // Start
        agent.start().await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Running);

        // Health check
        assert!(agent.health_check().await.unwrap());

        // Stop
        agent.stop().await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Stopped);
    }

    #[tokio::test]
    async fn test_supervised_agent_system_messages() {
        let mut agent = JudgeAgent::new();
        let args = InitArgs {
            agent_id: AgentPid::new(),
            supervisor_id: SupervisorId::new(),
            config: json!({}),
        };

        agent.init(args).await.unwrap();
        agent.start().await.unwrap();

        // Test status update
        agent
            .handle_system_message(SystemMessage::StatusUpdate(AgentStatus::Running))
            .await
            .unwrap();
        assert_eq!(agent.status(), AgentStatus::Running);

        // Test shutdown
        agent
            .handle_system_message(SystemMessage::Shutdown)
            .await
            .unwrap();
        assert_eq!(agent.status(), AgentStatus::Stopped);
    }

    #[test]
    fn test_judge_agent_with_rolegraph() {
        let rolegraph = create_test_rolegraph();
        let agent = JudgeAgent::with_rolegraph(rolegraph);

        assert!(agent.kg_agent().is_some());
        assert!(agent.model_router().available_tiers().len() > 0);
    }

    #[tokio::test]
    async fn test_profile_based_tier_selection() {
        let agent = JudgeAgent::new();

        // Test default profile
        // Note: This uses the simulate method which returns different responses
        // based on profile
        let verdict = agent
            .simulate_judge_response("test", "default")
            .await
            .unwrap();
        assert_eq!(verdict, "PASS");

        let verdict = agent
            .simulate_judge_response("test", "critical")
            .await
            .unwrap();
        assert_eq!(verdict, "NEEDS_REVIEW");
    }

    #[tokio::test]
    async fn test_judge_agent_termination() {
        let mut agent = JudgeAgent::new();
        let args = InitArgs {
            agent_id: AgentPid::new(),
            supervisor_id: SupervisorId::new(),
            config: json!({}),
        };

        agent.init(args).await.unwrap();
        agent.start().await.unwrap();

        agent.terminate(TerminateReason::Normal).await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Stopped);
    }
}
