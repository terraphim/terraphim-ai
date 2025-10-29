// Repository security model using knowledge graphs
//
// Implements intelligent command validation using:
// - terraphim_rolegraph for allowed/blocked command storage
// - terraphim_automata for fast command matching (Aho-Corasick)
// - Fuzzy matching for command variations (Jaro-Winkler, Levenshtein)
// - Thesaurus for synonym resolution
// - Learning system from user decisions

use ahash::AHashMap;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use terraphim_automata::autocomplete::{fuzzy_autocomplete_search, AutocompleteConfig};
use terraphim_automata::{find_matches, AutocompleteIndex};
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};
use tracing::{debug, info, warn};

#[cfg(feature = "typescript")]
use tsify::Tsify;

/// Permission decision for a command
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum CommandPermission {
    /// Command is allowed to run without asking
    Allow,
    /// Command is blocked and will never run
    Block,
    /// Command requires user confirmation
    Ask(String),
}

/// Repository security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub repository: String,
    pub security_level: SecurityLevel,
    pub allowed_commands: HashMap<String, Vec<String>>,
    pub blocked_commands: HashMap<String, Vec<String>>,
    pub ask_commands: HashMap<String, Vec<String>>,
    pub command_synonyms: HashMap<String, String>,
}

impl SecurityConfig {
    /// Create default security config for a repository
    pub fn default_for_repo(repo_path: &Path) -> Self {
        let mut config = Self {
            repository: repo_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            security_level: SecurityLevel::Development,
            allowed_commands: HashMap::new(),
            blocked_commands: HashMap::new(),
            ask_commands: HashMap::new(),
            command_synonyms: HashMap::new(),
        };

        // Always safe file operations
        config
            .allowed_commands
            .insert("cat".to_string(), vec!["*".to_string()]);
        config
            .allowed_commands
            .insert("ls".to_string(), vec!["*".to_string()]);
        config
            .allowed_commands
            .insert("grep".to_string(), vec!["*".to_string()]);
        config
            .allowed_commands
            .insert("find".to_string(), vec!["*".to_string()]);
        config.allowed_commands.insert(
            "git".to_string(),
            vec!["status".to_string(), "diff".to_string(), "log".to_string()],
        );

        // Always dangerous operations
        config.blocked_commands.insert(
            "rm".to_string(),
            vec!["-rf /".to_string(), "-rf /*".to_string()],
        );
        config
            .blocked_commands
            .insert("sudo".to_string(), vec!["*".to_string()]);

        // Common synonyms
        config
            .command_synonyms
            .insert("show file".to_string(), "cat".to_string());
        config
            .command_synonyms
            .insert("list files".to_string(), "ls".to_string());
        config
            .command_synonyms
            .insert("search".to_string(), "grep".to_string());

        config
    }

    /// Save security config to repository
    pub async fn save(&self, repo_path: &Path) -> Result<()> {
        let config_dir = repo_path.join(".terraphim");
        tokio::fs::create_dir_all(&config_dir).await?;

        let config_path = config_dir.join("security.json");
        let json = serde_json::to_string_pretty(&self)?;
        tokio::fs::write(config_path, json).await?;

        Ok(())
    }

    /// Load security config from repository
    pub async fn load(repo_path: &Path) -> Result<Self> {
        let config_path = repo_path.join(".terraphim/security.json");

        if !config_path.exists() {
            // Generate default
            let config = Self::default_for_repo(repo_path);
            config.save(repo_path).await?;
            return Ok(config);
        }

        let json = tokio::fs::read_to_string(config_path).await?;
        let config: SecurityConfig = serde_json::from_str(&json)?;

        Ok(config)
    }
}

/// Security level affects default permissions
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum SecurityLevel {
    Development,
    Staging,
    Production,
}

/// Repository security graph using terraphim infrastructure
pub struct RepositorySecurityGraph {
    config: SecurityConfig,
    command_index: Option<AutocompleteIndex>,
    command_thesaurus: Option<Thesaurus>,
}

impl RepositorySecurityGraph {
    /// Create new security graph from config
    pub async fn new(config: SecurityConfig) -> Result<Self> {
        let mut graph = Self {
            config,
            command_index: None,
            command_thesaurus: None,
        };

        // Build autocomplete index from allowed commands for fast matching
        graph.build_command_index().await?;

        Ok(graph)
    }

    /// Load from repository path
    pub async fn load(repo_path: &Path) -> Result<Self> {
        let config = SecurityConfig::load(repo_path).await?;
        Self::new(config).await
    }

    /// Build autocomplete index from commands for fast matching
    async fn build_command_index(&mut self) -> Result<()> {
        // Build thesaurus from all commands and synonyms
        let mut thesaurus = Thesaurus::new("command_security".to_string());

        let mut id_counter = 0u64;

        // Add allowed commands
        for (cmd, args) in &self.config.allowed_commands {
            for arg in args {
                let full_cmd = format!("{} {}", cmd, arg);
                thesaurus.insert(
                    NormalizedTermValue::from(full_cmd.clone()),
                    NormalizedTerm {
                        id: id_counter,
                        value: NormalizedTermValue::from(full_cmd.clone()),
                        url: Some(format!("allowed:{}", full_cmd)),
                    },
                );
                id_counter += 1;
            }
        }

        // Add blocked commands
        for (cmd, args) in &self.config.blocked_commands {
            for arg in args {
                let full_cmd = format!("{} {}", cmd, arg);
                thesaurus.insert(
                    NormalizedTermValue::from(full_cmd.clone()),
                    NormalizedTerm {
                        id: id_counter,
                        value: NormalizedTermValue::from(full_cmd.clone()),
                        url: Some(format!("blocked:{}", full_cmd)),
                    },
                );
                id_counter += 1;
            }
        }

        // Build autocomplete index
        let index = terraphim_automata::build_autocomplete_index(thesaurus.clone(), None)?;

        self.command_thesaurus = Some(thesaurus);
        self.command_index = Some(index);

        Ok(())
    }

    /// Validate a command using multi-strategy matching
    pub async fn validate_command(&self, command: &str) -> Result<CommandPermission> {
        debug!("Validating command: {}", command);

        // Strategy 1: Check exact match in allowed commands
        if let Some(permission) = self.check_exact_match(command) {
            debug!("Exact match found: {:?}", permission);
            return Ok(permission);
        }

        // Strategy 2: Check synonym resolution
        if let Some(resolved) = self.config.command_synonyms.get(command) {
            info!("Resolved synonym: '{}' → '{}'", command, resolved);
            // Box the recursive call to avoid infinite type
            return Box::pin(self.validate_command(resolved)).await;
        }

        // Strategy 3: Fuzzy match using autocomplete
        if let Some(index) = &self.command_index {
            let results = fuzzy_autocomplete_search(index, command, 0.85, Some(5))?;

            if !results.is_empty() {
                let best_match = &results[0];
                info!(
                    "Fuzzy match: '{}' → '{}' (score: {:.2})",
                    command, best_match.term, best_match.score
                );

                // Check if fuzzy match is in allowed/blocked
                if let Some(permission) = self.check_exact_match(&best_match.term) {
                    return Ok(permission);
                }
            }
        }

        // Strategy 4: Pattern matching for command families
        if let Some(permission) = self.check_pattern_match(command) {
            return Ok(permission);
        }

        // Default: Unknown command - ask user for safety
        warn!("Unknown command, defaulting to Ask: {}", command);
        Ok(CommandPermission::Ask(command.to_string()))
    }

    /// Check exact match in allowed/blocked lists
    fn check_exact_match(&self, command: &str) -> Option<CommandPermission> {
        // Check blocked first (security priority)
        for (cmd, patterns) in &self.config.blocked_commands {
            for pattern in patterns {
                let full_cmd = format!("{} {}", cmd, pattern);
                if command.contains(&full_cmd) || (pattern == "*" && command.starts_with(cmd)) {
                    return Some(CommandPermission::Block);
                }
            }
        }

        // Check allowed
        for (cmd, patterns) in &self.config.allowed_commands {
            for pattern in patterns {
                let full_cmd = format!("{} {}", cmd, pattern);
                if command == full_cmd || (pattern == "*" && command.starts_with(cmd)) {
                    return Some(CommandPermission::Allow);
                }
            }
        }

        // Check ask
        for (cmd, patterns) in &self.config.ask_commands {
            for pattern in patterns {
                let full_cmd = format!("{} {}", cmd, pattern);
                if command == full_cmd || (pattern == "*" && command.starts_with(cmd)) {
                    return Some(CommandPermission::Ask(command.to_string()));
                }
            }
        }

        None
    }

    /// Check pattern-based matching
    fn check_pattern_match(&self, command: &str) -> Option<CommandPermission> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let cmd = parts[0];

        // Check if command family is blocked
        if self.config.blocked_commands.contains_key(cmd) {
            return Some(CommandPermission::Block);
        }

        // Check if command family is allowed
        if self.config.allowed_commands.contains_key(cmd) {
            return Some(CommandPermission::Allow);
        }

        None
    }
}

/// Security event for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: String,
    pub command: String,
    pub matched_as: String,
    pub permission: String,
    pub executed: bool,
    pub similarity_score: f64,
}

/// Security audit log
pub struct SecurityAuditLog {
    log_file: PathBuf,
    events: Vec<SecurityEvent>,
}

impl SecurityAuditLog {
    pub fn new(log_file: PathBuf) -> Self {
        Self {
            log_file,
            events: Vec::new(),
        }
    }

    pub async fn log_event(&mut self, event: SecurityEvent) -> Result<()> {
        self.events.push(event.clone());

        // Write to file
        let entry = serde_json::to_string(&event)? + "\n";
        tokio::fs::write(&self.log_file, entry).await?;

        Ok(())
    }
}

/// User decision on a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDecision {
    pub command: String,
    pub allowed: bool,
    pub timestamp: String,
    pub context: HashMap<String, String>,
}

/// Learning system that adapts permissions based on user decisions
pub struct SecurityLearner {
    decisions: Vec<UserDecision>,
    learning_threshold: usize,
}

impl SecurityLearner {
    pub fn new(learning_threshold: usize) -> Self {
        Self {
            decisions: Vec::new(),
            learning_threshold,
        }
    }

    /// Record a user's decision on a command
    pub async fn record_decision(
        &mut self,
        command: &str,
        allowed: bool,
    ) -> Option<LearningAction> {
        let decision = UserDecision {
            command: command.to_string(),
            allowed,
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: HashMap::new(),
        };

        self.decisions.push(decision);

        // Check if we have enough data to learn
        if self.decisions.len() >= self.learning_threshold {
            return self.analyze_patterns(command).await;
        }

        None
    }

    /// Analyze patterns to determine if command should be auto-allowed or auto-blocked
    async fn analyze_patterns(&self, command: &str) -> Option<LearningAction> {
        // Find all similar decisions for this command
        let similar_decisions: Vec<&UserDecision> = self
            .decisions
            .iter()
            .filter(|d| self.is_similar_command(&d.command, command))
            .collect();

        if similar_decisions.len() < 3 {
            return None; // Not enough data
        }

        let allowed_count = similar_decisions.iter().filter(|d| d.allowed).count();
        let denied_count = similar_decisions.len() - allowed_count;

        // Consistent approval pattern
        if allowed_count >= 5 && denied_count == 0 {
            info!(
                "📝 Learning: Command '{}' consistently allowed ({} times)",
                command, allowed_count
            );
            return Some(LearningAction::AddToAllowed(command.to_string()));
        }

        // Consistent denial pattern
        if denied_count >= 3 && allowed_count == 0 {
            warn!(
                "🚫 Learning: Command '{}' consistently blocked ({} times)",
                command, denied_count
            );
            return Some(LearningAction::AddToBlocked(command.to_string()));
        }

        None
    }

    /// Check if two commands are similar (simple string matching for now)
    fn is_similar_command(&self, cmd1: &str, cmd2: &str) -> bool {
        // Extract base command (first word)
        let base1 = cmd1.split_whitespace().next().unwrap_or("");
        let base2 = cmd2.split_whitespace().next().unwrap_or("");

        base1 == base2 || cmd1.contains(cmd2) || cmd2.contains(cmd1)
    }

    /// Get learning statistics
    pub fn stats(&self) -> LearningStats {
        let total = self.decisions.len();
        let allowed = self.decisions.iter().filter(|d| d.allowed).count();
        let denied = total - allowed;

        LearningStats {
            total_decisions: total,
            allowed_count: allowed,
            denied_count: denied,
        }
    }
}

/// Action recommended by learning system
#[derive(Debug, Clone, PartialEq)]
pub enum LearningAction {
    AddToAllowed(String),
    AddToBlocked(String),
}

/// Learning statistics
#[derive(Debug, Clone)]
pub struct LearningStats {
    pub total_decisions: usize,
    pub allowed_count: usize,
    pub denied_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_config_default() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config = SecurityConfig::default_for_repo(temp_dir.path());

        // Check safe operations are allowed
        assert!(config.allowed_commands.contains_key("cat"));
        assert!(config.allowed_commands.contains_key("ls"));
        assert!(config.allowed_commands.contains_key("git"));

        // Check dangerous operations are blocked
        assert!(config.blocked_commands.contains_key("rm"));
        assert!(config.blocked_commands.contains_key("sudo"));

        // Check synonyms exist
        assert!(config.command_synonyms.contains_key("show file"));
    }

    #[tokio::test]
    async fn test_security_graph_validate_allowed() {
        let config = SecurityConfig::default_for_repo(Path::new("/test"));
        let graph = RepositorySecurityGraph::new(config).await.unwrap();

        let permission = graph.validate_command("git status").await.unwrap();
        assert_eq!(permission, CommandPermission::Allow);
    }

    #[tokio::test]
    async fn test_security_graph_validate_blocked() {
        let config = SecurityConfig::default_for_repo(Path::new("/test"));
        let graph = RepositorySecurityGraph::new(config).await.unwrap();

        let permission = graph.validate_command("sudo rm -rf /").await.unwrap();
        assert_eq!(permission, CommandPermission::Block);
    }

    #[tokio::test]
    async fn test_security_graph_synonym_resolution() {
        let config = SecurityConfig::default_for_repo(Path::new("/test"));
        let graph = RepositorySecurityGraph::new(config).await.unwrap();

        // "show file" should resolve to "cat" which is allowed
        let permission = graph.validate_command("show file").await.unwrap();
        assert_eq!(permission, CommandPermission::Allow);
    }

    #[tokio::test]
    async fn test_security_config_save_and_load() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config = SecurityConfig::default_for_repo(temp_dir.path());

        // Save
        config.save(temp_dir.path()).await.unwrap();

        // Load
        let loaded = SecurityConfig::load(temp_dir.path()).await.unwrap();

        assert_eq!(config.repository, loaded.repository);
        assert_eq!(config.allowed_commands.len(), loaded.allowed_commands.len());
    }

    #[tokio::test]
    async fn test_security_learner_consistent_allow() {
        let mut learner = SecurityLearner::new(3);

        // Record 5 consistent approvals
        for _ in 0..5 {
            learner.record_decision("git push", true).await;
        }

        // Should recommend adding to allowed list
        let action = learner.record_decision("git push", true).await;
        assert_eq!(
            action,
            Some(LearningAction::AddToAllowed("git push".to_string()))
        );
    }

    #[tokio::test]
    async fn test_security_learner_consistent_deny() {
        let mut learner = SecurityLearner::new(3);

        // Record 3 consistent denials
        for _ in 0..3 {
            learner.record_decision("rm -rf /", false).await;
        }

        // Should recommend adding to blocked list
        let action = learner.record_decision("rm -rf /", false).await;
        assert_eq!(
            action,
            Some(LearningAction::AddToBlocked("rm -rf /".to_string()))
        );
    }

    #[tokio::test]
    async fn test_security_learner_stats() {
        let mut learner = SecurityLearner::new(10);

        learner.record_decision("git status", true).await;
        learner.record_decision("git diff", true).await;
        learner.record_decision("rm file.txt", false).await;

        let stats = learner.stats();
        assert_eq!(stats.total_decisions, 3);
        assert_eq!(stats.allowed_count, 2);
        assert_eq!(stats.denied_count, 1);
    }
}
