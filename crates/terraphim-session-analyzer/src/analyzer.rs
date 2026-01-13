use crate::models::{
    AgentAttribution, AgentInvocation, AgentStatistics, AgentToolCorrelation, AnalyzerConfig,
    CollaborationPattern, FileOperation, SessionAnalysis, ToolCategory, ToolInvocation,
    ToolStatistics,
};
use crate::parser::SessionParser;
use anyhow::Result;
use indexmap::IndexMap;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, info};

pub struct Analyzer {
    parsers: Vec<SessionParser>,
    config: AnalyzerConfig,
}

impl Analyzer {
    /// Create analyzer from a specific path (file or directory)
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exist or cannot be read
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let parsers = if path.is_file() {
            vec![SessionParser::from_file(path)?]
        } else if path.is_dir() {
            SessionParser::from_directory(path)?
        } else {
            return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
        };

        Ok(Self {
            parsers,
            config: AnalyzerConfig::default(),
        })
    }

    /// Create analyzer from default Claude session location
    ///
    /// # Errors
    ///
    /// Returns an error if the default Claude directory doesn't exist
    pub fn from_default_location() -> Result<Self> {
        let parsers = SessionParser::from_default_location()?;
        Ok(Self {
            parsers,
            config: AnalyzerConfig::default(),
        })
    }

    /// Set custom configuration
    /// Used in integration tests
    #[allow(dead_code)]
    #[must_use]
    pub fn with_config(mut self, config: AnalyzerConfig) -> Self {
        self.config = config;
        self
    }

    /// Analyze all sessions or filter by target file
    ///
    /// # Errors
    ///
    /// Returns an error if session parsing or analysis fails
    pub fn analyze(&self, target_file: Option<&str>) -> Result<Vec<SessionAnalysis>> {
        info!("Analyzing {} session(s)", self.parsers.len());

        let analyses: Result<Vec<_>> = self
            .parsers
            .par_iter()
            .filter_map(|parser| {
                match self.analyze_session(parser, target_file) {
                    Ok(analysis) => {
                        // If target file specified, only include sessions with relevant operations
                        if let Some(_target) = target_file {
                            if analysis.file_operations.is_empty() {
                                return None; // Skip sessions without target file operations
                            }
                        }
                        Some(Ok(analysis))
                    }
                    Err(e) => Some(Err(e)),
                }
            })
            .collect();

        analyses
    }

    /// Analyze a single session
    fn analyze_session(
        &self,
        parser: &SessionParser,
        target_file: Option<&str>,
    ) -> Result<SessionAnalysis> {
        let (session_id, project_path, start_time, end_time) = parser.get_session_info();

        debug!("Analyzing session: {}", session_id);

        // Extract raw data
        let mut agents = parser.extract_agent_invocations();
        let mut file_operations = parser.extract_file_operations();

        // Sort agents by timestamp for efficient binary search in set_agent_context
        agents.sort_by_key(|a| a.timestamp);

        // Set agent context for file operations
        self.set_agent_context(&mut file_operations, &agents, parser);

        // Calculate agent durations
        Self::calculate_agent_durations(&mut agents);

        // Filter by target file if specified
        if let Some(target) = target_file {
            file_operations.retain(|op| op.file_path.contains(target));

            // Also filter agents to only those that worked on the target file
            let relevant_agent_contexts: HashSet<&str> = file_operations
                .iter()
                .filter_map(|op| op.agent_context.as_deref())
                .collect();

            agents.retain(|agent| relevant_agent_contexts.contains(agent.agent_type.as_str()));
        }

        // Build file-to-agent attributions
        let file_to_agents = self.build_file_attributions(&file_operations, &agents);

        // Calculate agent statistics
        let agent_stats = Self::calculate_agent_statistics(&agents, &file_operations);

        // Detect collaboration patterns
        let collaboration_patterns = self.detect_collaboration_patterns(&agents);

        let duration_ms = if let (Some(start), Some(end)) = (start_time, end_time) {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                (end - start).total(jiff::Unit::Millisecond)? as u64
            }
        } else {
            0
        };

        Ok(SessionAnalysis {
            session_id,
            project_path,
            start_time: start_time.unwrap_or_else(jiff::Timestamp::now),
            end_time: end_time.unwrap_or_else(jiff::Timestamp::now),
            duration_ms,
            agents,
            file_operations,
            file_to_agents,
            agent_stats,
            collaboration_patterns,
        })
    }

    /// Set agent context for file operations based on temporal proximity
    /// Uses binary search for O(log n) lookup instead of O(n) linear search
    fn set_agent_context(
        &self,
        file_operations: &mut [FileOperation],
        agents: &[AgentInvocation],
        parser: &SessionParser,
    ) {
        // Ensure agents are sorted by timestamp for binary search
        // (they should already be sorted from extraction, but let's verify)
        debug_assert!(agents.windows(2).all(|w| w[0].timestamp <= w[1].timestamp));

        for file_op in file_operations.iter_mut() {
            // First try to find agent from parser's context lookup
            if let Some(agent) = parser.find_active_agent(&file_op.message_id) {
                file_op.agent_context = Some(agent);
                continue;
            }

            // Use binary search to find the most recent agent before this file operation
            // We're looking for the rightmost agent with timestamp <= file_op.timestamp
            let agent_idx = match agents.binary_search_by_key(&file_op.timestamp, |a| a.timestamp) {
                Ok(idx) => Some(idx), // Exact match
                Err(idx) => {
                    if idx > 0 {
                        Some(idx - 1) // Previous agent is the most recent before this operation
                    } else {
                        None // No agent before this operation
                    }
                }
            };

            if let Some(idx) = agent_idx {
                let agent = &agents[idx];
                let time_diff = file_op.timestamp - agent.timestamp;
                let time_diff_ms = time_diff.total(jiff::Unit::Millisecond).unwrap_or(0.0);
                let window_ms = self.config.file_attribution_window_ms;

                #[allow(clippy::cast_precision_loss)]
                if time_diff_ms <= (window_ms as f64) {
                    file_op.agent_context = Some(agent.agent_type.clone());
                }
            }
        }
    }

    /// Calculate durations for agent invocations
    fn calculate_agent_durations(agents: &mut [AgentInvocation]) {
        // Sort by timestamp for duration calculation
        agents.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        for i in 0..agents.len() {
            if i + 1 < agents.len() {
                let duration = agents[i + 1].timestamp - agents[i].timestamp;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    agents[i].duration_ms =
                        Some(duration.total(jiff::Unit::Millisecond).unwrap_or(0.0) as u64);
                }
            }
        }
    }

    /// Build file-to-agent attribution mapping
    fn build_file_attributions(
        &self,
        file_operations: &[FileOperation],
        _agents: &[AgentInvocation],
    ) -> IndexMap<String, Vec<AgentAttribution>> {
        let mut file_to_agents: IndexMap<String, Vec<AgentAttribution>> = IndexMap::new();

        // Group file operations by file path
        let mut file_groups: HashMap<String, Vec<&FileOperation>> = HashMap::new();
        for op in file_operations {
            // Skip files matching exclude patterns
            if self.should_exclude_file(&op.file_path) {
                continue;
            }

            file_groups
                .entry(op.file_path.clone())
                .or_default()
                .push(op);
        }

        // Calculate attributions for each file
        for (file_path, ops) in file_groups {
            let mut agent_contributions: HashMap<String, AgentContribution> = HashMap::new();

            for op in ops {
                if let Some(agent_type) = &op.agent_context {
                    let contribution = agent_contributions
                        .entry(agent_type.clone())
                        .or_insert_with(|| AgentContribution {
                            operations: Vec::new(),
                            first_interaction: op.timestamp,
                            last_interaction: op.timestamp,
                        });

                    contribution.operations.push(format!("{:?}", op.operation));
                    if op.timestamp < contribution.first_interaction {
                        contribution.first_interaction = op.timestamp;
                    }
                    if op.timestamp > contribution.last_interaction {
                        contribution.last_interaction = op.timestamp;
                    }
                }
            }

            // Convert to attributions with percentages
            #[allow(clippy::cast_precision_loss)]
            let total_ops = agent_contributions
                .values()
                .map(|c| c.operations.len())
                .sum::<usize>() as f32;

            if total_ops > 0.0 {
                let attributions: Vec<AgentAttribution> = agent_contributions
                    .into_iter()
                    .map(|(agent_type, contribution)| {
                        #[allow(clippy::cast_precision_loss)]
                        let contribution_percent =
                            (contribution.operations.len() as f32 / total_ops) * 100.0;
                        let confidence_score = Self::calculate_confidence_score(
                            &contribution.operations,
                            contribution_percent,
                        );

                        AgentAttribution {
                            agent_type,
                            contribution_percent,
                            confidence_score,
                            operations: contribution.operations,
                            first_interaction: contribution.first_interaction,
                            last_interaction: contribution.last_interaction,
                        }
                    })
                    .collect();

                file_to_agents.insert(file_path, attributions);
            }
        }

        file_to_agents
    }

    /// Calculate agent statistics
    fn calculate_agent_statistics(
        agents: &[AgentInvocation],
        file_operations: &[FileOperation],
    ) -> IndexMap<String, AgentStatistics> {
        let mut stats: IndexMap<String, AgentStatistics> = IndexMap::new();

        // Group agents by type
        let mut agent_groups: HashMap<String, Vec<&AgentInvocation>> = HashMap::new();
        for agent in agents {
            agent_groups
                .entry(agent.agent_type.clone())
                .or_default()
                .push(agent);
        }

        // Calculate statistics for each agent type
        for (agent_type, agent_list) in agent_groups {
            #[allow(clippy::cast_possible_truncation)]
            let total_invocations = agent_list.len() as u32;
            let total_duration_ms = agent_list.iter().filter_map(|a| a.duration_ms).sum::<u64>();

            #[allow(clippy::cast_possible_truncation)]
            let files_touched = file_operations
                .iter()
                .filter(|op| op.agent_context.as_ref() == Some(&agent_type))
                .map(|op| &op.file_path)
                .collect::<HashSet<_>>()
                .len() as u32;

            let tools_used = file_operations
                .iter()
                .filter(|op| op.agent_context.as_ref() == Some(&agent_type))
                .map(|op| format!("{:?}", op.operation))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();

            let first_seen = agent_list
                .iter()
                .map(|a| a.timestamp)
                .min()
                .unwrap_or_else(jiff::Timestamp::now);
            let last_seen = agent_list
                .iter()
                .map(|a| a.timestamp)
                .max()
                .unwrap_or_else(jiff::Timestamp::now);

            stats.insert(
                agent_type.clone(),
                AgentStatistics {
                    agent_type,
                    total_invocations,
                    total_duration_ms,
                    files_touched,
                    tools_used,
                    first_seen,
                    last_seen,
                },
            );
        }

        stats
    }

    /// Detect collaboration patterns between agents
    fn detect_collaboration_patterns(
        &self,
        agents: &[AgentInvocation],
    ) -> Vec<CollaborationPattern> {
        let mut patterns = Vec::new();

        // Sequential pattern: architect -> developer -> test-writer
        let sequential_pattern = Self::detect_sequential_pattern(agents);
        if let Some(pattern) = sequential_pattern {
            patterns.push(pattern);
        }

        // Parallel pattern: multiple agents working simultaneously
        let parallel_pattern = Self::detect_parallel_pattern(agents);
        if let Some(pattern) = parallel_pattern {
            patterns.push(pattern);
        }

        patterns
    }

    /// Detect sequential collaboration patterns
    fn detect_sequential_pattern(agents: &[AgentInvocation]) -> Option<CollaborationPattern> {
        let common_sequences = vec![
            vec!["architect", "developer", "test-writer-fixer"],
            vec!["architect", "backend-architect", "developer"],
            vec!["rapid-prototyper", "developer", "technical-writer"],
        ];

        for sequence in common_sequences {
            if Self::matches_sequence(agents, &sequence) {
                return Some(CollaborationPattern {
                    pattern_type: "Sequential".to_string(),
                    agents: sequence.iter().map(|s| (*s).to_string()).collect(),
                    description: format!("Sequential workflow: {}", sequence.join(" → ")),
                    frequency: 1,
                    confidence: 0.8,
                });
            }
        }

        None
    }

    /// Detect parallel collaboration patterns
    fn detect_parallel_pattern(agents: &[AgentInvocation]) -> Option<CollaborationPattern> {
        // Group agents by time windows
        let window_ms = 300_000; // 5 minutes
        let mut time_groups: Vec<Vec<&AgentInvocation>> = Vec::new();

        for agent in agents {
            let mut found_group = false;
            for group in &mut time_groups {
                if let Some(first) = group.first() {
                    let time_diff = (agent.timestamp - first.timestamp)
                        .total(jiff::Unit::Millisecond)
                        .unwrap_or(0.0)
                        .abs();
                    if time_diff <= f64::from(window_ms) {
                        group.push(agent);
                        found_group = true;
                        break;
                    }
                }
            }
            if !found_group {
                time_groups.push(vec![agent]);
            }
        }

        // Find groups with multiple different agents
        for group in time_groups {
            let unique_agents: HashSet<&str> =
                group.iter().map(|a| a.agent_type.as_str()).collect();

            if unique_agents.len() >= 2 {
                return Some(CollaborationPattern {
                    pattern_type: "Parallel".to_string(),
                    agents: unique_agents.iter().map(|s| (*s).to_string()).collect(),
                    description: format!(
                        "Parallel collaboration: {}",
                        unique_agents
                            .iter()
                            .map(|s| (*s).to_string())
                            .collect::<Vec<_>>()
                            .join(" + ")
                    ),
                    frequency: 1,
                    confidence: 0.7,
                });
            }
        }

        None
    }

    /// Check if agents match a specific sequence
    fn matches_sequence(agents: &[AgentInvocation], sequence: &[&str]) -> bool {
        let agent_types: Vec<&str> = agents.iter().map(|a| a.agent_type.as_str()).collect();

        // Simple substring matching for now
        for window in agent_types.windows(sequence.len()) {
            if window == sequence {
                return true;
            }
        }

        false
    }

    /// Calculate confidence score for agent attribution
    fn calculate_confidence_score(operations: &[String], contribution_percent: f32) -> f32 {
        let mut confidence = 0.5; // Base confidence

        // Higher confidence for more operations
        #[allow(clippy::cast_precision_loss)]
        {
            confidence += (operations.len() as f32 * 0.1).min(0.3);
        }

        // Higher confidence for higher contribution percentage
        confidence += (contribution_percent / 100.0) * 0.4;

        // Boost confidence for write operations vs read operations
        #[allow(clippy::cast_precision_loss)]
        let write_ops = operations
            .iter()
            .filter(|op| matches!(op.as_str(), "Write" | "Edit" | "MultiEdit"))
            .count() as f32;
        #[allow(clippy::cast_precision_loss)]
        let total_ops = operations.len() as f32;

        if total_ops > 0.0 {
            let write_ratio = write_ops / total_ops;
            confidence += write_ratio * 0.2;
        }

        confidence.min(1.0)
    }

    /// Check if file should be excluded based on patterns
    fn should_exclude_file(&self, file_path: &str) -> bool {
        for pattern in &self.config.exclude_patterns {
            if file_path.contains(pattern) {
                return true;
            }
        }
        false
    }

    /// Get summary statistics across all sessions
    ///
    /// # Errors
    ///
    /// Returns an error if analysis fails
    pub fn get_summary_stats(&self) -> Result<SummaryStats> {
        let analyses = self.analyze(None)?;

        let total_sessions = analyses.len();
        let total_agents = analyses.iter().map(|a| a.agents.len()).sum::<usize>();
        let total_files = analyses
            .iter()
            .map(|a| a.file_to_agents.len())
            .sum::<usize>();

        let agent_types: HashSet<String> = analyses
            .iter()
            .flat_map(|a| a.agents.iter().map(|ag| ag.agent_type.clone()))
            .collect();

        Ok(SummaryStats {
            total_sessions,
            total_agents,
            total_files,
            unique_agent_types: agent_types.len(),
            most_active_agents: Self::get_most_active_agents(&analyses),
        })
    }

    /// Get most active agent types across all sessions
    fn get_most_active_agents(analyses: &[SessionAnalysis]) -> Vec<(String, u32)> {
        let mut agent_counts: HashMap<String, u32> = HashMap::new();

        for analysis in analyses {
            for agent in &analysis.agents {
                *agent_counts.entry(agent.agent_type.clone()).or_insert(0) += 1;
            }
        }

        let mut sorted: Vec<_> = agent_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.into_iter().take(10).collect()
    }

    /// Calculate which agents use which tools
    /// Returns correlations sorted by usage count (descending)
    #[must_use]
    pub fn calculate_agent_tool_correlations(
        &self,
        tool_invocations: &[ToolInvocation],
    ) -> Vec<AgentToolCorrelation> {
        // Group tools by agent and tool name
        let mut agent_tool_map: HashMap<(String, String), AgentToolData> = HashMap::new();

        for invocation in tool_invocations {
            if let Some(agent) = &invocation.agent_context {
                let key = (agent.clone(), invocation.tool_name.clone());
                let data = agent_tool_map.entry(key).or_insert_with(|| AgentToolData {
                    usage_count: 0,
                    success_count: 0,
                    failure_count: 0,
                    session_count: HashSet::new(),
                });

                data.usage_count += 1;
                data.session_count.insert(invocation.session_id.clone());

                if let Some(exit_code) = invocation.exit_code {
                    if exit_code == 0 {
                        data.success_count += 1;
                    } else {
                        data.failure_count += 1;
                    }
                }
            }
        }

        // Calculate total tool usage per agent
        let mut agent_totals: HashMap<String, u32> = HashMap::new();
        for ((agent, _), data) in &agent_tool_map {
            *agent_totals.entry(agent.clone()).or_insert(0) += data.usage_count;
        }

        // Convert to correlations and calculate percentages
        let mut correlations: Vec<AgentToolCorrelation> = agent_tool_map
            .into_iter()
            .map(|((agent_type, tool_name), data)| {
                let total_attempts = data.success_count + data.failure_count;
                #[allow(clippy::cast_precision_loss)]
                let success_rate = if total_attempts > 0 {
                    (data.success_count as f32) / (total_attempts as f32)
                } else {
                    0.0
                };

                #[allow(clippy::cast_precision_loss)]
                let average_invocations_per_session = if !data.session_count.is_empty() {
                    (data.usage_count as f32) / (data.session_count.len() as f32)
                } else {
                    0.0
                };

                AgentToolCorrelation {
                    agent_type,
                    tool_name,
                    usage_count: data.usage_count,
                    success_rate,
                    average_invocations_per_session,
                }
            })
            .collect();

        // Sort by usage count descending
        correlations.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));

        correlations
    }

    /// Calculate comprehensive tool statistics
    #[must_use]
    pub fn calculate_tool_statistics(
        &self,
        tool_invocations: &[ToolInvocation],
    ) -> IndexMap<String, ToolStatistics> {
        let mut tool_map: HashMap<String, ToolStatsData> = HashMap::new();

        for invocation in tool_invocations {
            let data = tool_map
                .entry(invocation.tool_name.clone())
                .or_insert_with(|| ToolStatsData {
                    category: invocation.tool_category.clone(),
                    total_invocations: 0,
                    agents_using: HashSet::new(),
                    success_count: 0,
                    failure_count: 0,
                    first_seen: invocation.timestamp,
                    last_seen: invocation.timestamp,
                    command_patterns: HashSet::new(),
                    sessions: HashSet::new(),
                });

            data.total_invocations += 1;
            data.sessions.insert(invocation.session_id.clone());

            if let Some(agent) = &invocation.agent_context {
                data.agents_using.insert(agent.clone());
            }

            if let Some(exit_code) = invocation.exit_code {
                if exit_code == 0 {
                    data.success_count += 1;
                } else {
                    data.failure_count += 1;
                }
            }

            if invocation.timestamp < data.first_seen {
                data.first_seen = invocation.timestamp;
            }
            if invocation.timestamp > data.last_seen {
                data.last_seen = invocation.timestamp;
            }

            // Extract common command patterns (first 100 chars of command)
            let pattern = if invocation.command_line.len() > 100 {
                invocation.command_line[..100].to_string()
            } else {
                invocation.command_line.clone()
            };
            data.command_patterns.insert(pattern);
        }

        // Convert to IndexMap for ordered results
        let mut stats: IndexMap<String, ToolStatistics> = tool_map
            .into_iter()
            .map(|(tool_name, data)| {
                #[allow(clippy::cast_possible_truncation)]
                let stats = ToolStatistics {
                    tool_name: tool_name.clone(),
                    category: data.category,
                    total_invocations: data.total_invocations,
                    agents_using: data.agents_using.into_iter().collect(),
                    success_count: data.success_count,
                    failure_count: data.failure_count,
                    first_seen: data.first_seen,
                    last_seen: data.last_seen,
                    command_patterns: data.command_patterns.into_iter().take(10).collect(),
                    sessions: data.sessions.into_iter().collect(),
                };
                (tool_name, stats)
            })
            .collect();

        // Sort by total invocations descending
        stats.sort_by(|_, v1, _, v2| v2.total_invocations.cmp(&v1.total_invocations));

        stats
    }

    /// Calculate category breakdown
    #[must_use]
    pub fn calculate_category_breakdown(
        &self,
        tool_invocations: &[ToolInvocation],
    ) -> IndexMap<ToolCategory, u32> {
        let mut category_counts: HashMap<ToolCategory, u32> = HashMap::new();

        for invocation in tool_invocations {
            *category_counts
                .entry(invocation.tool_category.clone())
                .or_insert(0) += 1;
        }

        // Convert to IndexMap and sort by count descending
        let mut breakdown: IndexMap<ToolCategory, u32> = category_counts.into_iter().collect();
        breakdown.sort_by(|_, v1, _, v2| v2.cmp(v1));

        breakdown
    }

    /// Detect sequential tool usage patterns (tool chains)
    ///
    /// # Arguments
    /// * `tool_invocations` - List of tool invocations to analyze
    ///
    /// # Returns
    /// A vector of `ToolChain` instances representing detected patterns
    ///
    /// # Algorithm
    /// 1. Group tools by session
    /// 2. Sort by timestamp within each session
    /// 3. Use sliding windows (2-5 tools) to find sequences
    /// 4. Group identical sequences across sessions
    /// 5. Calculate frequency, timing, and success rate
    /// 6. Filter chains that appear at least twice
    #[must_use]
    #[allow(dead_code)] // Will be used when tool chain analysis is exposed in CLI
    pub fn detect_tool_chains(
        &self,
        tool_invocations: &[ToolInvocation],
    ) -> Vec<crate::models::ToolChain> {
        use crate::models::ToolChain;

        // Group tools by session
        let mut session_tools: HashMap<String, Vec<&ToolInvocation>> = HashMap::new();
        for invocation in tool_invocations {
            session_tools
                .entry(invocation.session_id.clone())
                .or_default()
                .push(invocation);
        }

        // Track sequences: (tool_names) -> SequenceData
        let mut sequence_map: HashMap<Vec<String>, SequenceData> = HashMap::new();

        // Maximum time window between tools in a chain (1 hour = 3,600,000 ms)
        const MAX_TIME_BETWEEN_TOOLS_MS: u64 = 3_600_000;

        // Process each session
        for (_session_id, mut tools) in session_tools {
            // Sort by timestamp
            tools.sort_by_key(|t| t.timestamp);

            // Try different window sizes (2 to 5 tools)
            for window_size in 2..=5.min(tools.len()) {
                // Extract all consecutive sequences of this size
                for window in tools.windows(window_size) {
                    // Check if tools are within time window
                    let first_time = window[0].timestamp;
                    let last_time = window[window_size - 1].timestamp;

                    let time_diff = last_time - first_time;
                    #[allow(clippy::cast_sign_loss)]
                    let time_diff_ms = time_diff
                        .total(jiff::Unit::Millisecond)
                        .unwrap_or(0.0)
                        .abs() as u64;

                    if time_diff_ms > MAX_TIME_BETWEEN_TOOLS_MS {
                        continue; // Tools too far apart, skip
                    }

                    // Extract tool names
                    let tool_names: Vec<String> =
                        window.iter().map(|t| t.tool_name.clone()).collect();

                    // Get agent context (use first tool's agent)
                    let agent = window[0].agent_context.clone();

                    // Calculate time between tools
                    let mut time_diffs = Vec::new();
                    for i in 0..window.len() - 1 {
                        let diff = window[i + 1].timestamp - window[i].timestamp;
                        #[allow(clippy::cast_sign_loss)]
                        let diff_ms =
                            diff.total(jiff::Unit::Millisecond).unwrap_or(0.0).abs() as u64;
                        time_diffs.push(diff_ms);
                    }

                    // Calculate success rate
                    let total_with_exit_code =
                        window.iter().filter(|t| t.exit_code.is_some()).count();
                    let successful = window.iter().filter(|t| t.exit_code == Some(0)).count();

                    let data = sequence_map
                        .entry(tool_names)
                        .or_insert_with(SequenceData::new);
                    data.frequency += 1;
                    data.time_diffs.extend(time_diffs);
                    data.total_with_exit_code += total_with_exit_code;
                    data.successful += successful;

                    if let Some(agent) = agent {
                        *data.agent_counts.entry(agent).or_insert(0) += 1;
                    }
                }
            }
        }

        // Convert to ToolChain instances
        let mut chains: Vec<ToolChain> = sequence_map
            .into_iter()
            .filter(|(_, data)| data.frequency >= 2) // Must appear at least twice
            .map(|(tools, data)| {
                #[allow(clippy::cast_precision_loss)]
                let average_time_between_ms = if data.time_diffs.is_empty() {
                    0
                } else {
                    data.time_diffs.iter().sum::<u64>() / (data.time_diffs.len() as u64)
                };

                // Find most common agent
                let typical_agent = data
                    .agent_counts
                    .into_iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(agent, _)| agent);

                #[allow(clippy::cast_precision_loss)]
                let success_rate = if data.total_with_exit_code > 0 {
                    (data.successful as f32) / (data.total_with_exit_code as f32)
                } else {
                    0.0
                };

                ToolChain {
                    tools,
                    frequency: data.frequency,
                    average_time_between_ms,
                    typical_agent,
                    success_rate,
                }
            })
            .collect();

        // Sort by frequency descending
        chains.sort_by(|a, b| b.frequency.cmp(&a.frequency));

        chains
    }
}

#[derive(Debug)]
struct AgentContribution {
    operations: Vec<String>,
    first_interaction: jiff::Timestamp,
    last_interaction: jiff::Timestamp,
}

/// Helper struct for tracking agent-tool usage data
struct AgentToolData {
    usage_count: u32,
    success_count: u32,
    failure_count: u32,
    session_count: HashSet<String>,
}

/// Helper struct for tracking tool statistics data
struct ToolStatsData {
    category: ToolCategory,
    total_invocations: u32,
    agents_using: HashSet<String>,
    success_count: u32,
    failure_count: u32,
    first_seen: jiff::Timestamp,
    last_seen: jiff::Timestamp,
    command_patterns: HashSet<String>,
    sessions: HashSet<String>,
}

/// Helper struct for tracking tool chain sequence data
#[allow(dead_code)] // Used in tool chain detection
struct SequenceData {
    frequency: u32,
    time_diffs: Vec<u64>,
    agent_counts: HashMap<String, u32>,
    total_with_exit_code: usize,
    successful: usize,
}

#[allow(dead_code)] // Used in tool chain detection
impl SequenceData {
    fn new() -> Self {
        Self {
            frequency: 0,
            time_diffs: Vec::new(),
            agent_counts: HashMap::new(),
            total_with_exit_code: 0,
            successful: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SummaryStats {
    pub total_sessions: usize,
    pub total_agents: usize,
    pub total_files: usize,
    pub unique_agent_types: usize,
    pub most_active_agents: Vec<(String, u32)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_confidence_score() {
        let _analyzer = Analyzer {
            parsers: vec![],
            config: AnalyzerConfig::default(),
        };

        let operations = vec!["Write".to_string(), "Edit".to_string()];
        let confidence = Analyzer::calculate_confidence_score(&operations, 75.0);

        assert!(confidence > 0.5);
        assert!(confidence <= 1.0);
    }

    #[test]
    fn test_should_exclude_file() {
        let config = AnalyzerConfig {
            exclude_patterns: vec!["node_modules/".to_string(), "target/".to_string()],
            ..Default::default()
        };

        let analyzer = Analyzer {
            parsers: vec![],
            config,
        };

        assert!(analyzer.should_exclude_file("node_modules/package.json"));
        assert!(analyzer.should_exclude_file("target/debug/main"));
        assert!(!analyzer.should_exclude_file("src/main.rs"));
    }

    #[test]
    fn test_calculate_agent_tool_correlations() {
        use crate::models::{ToolCategory, ToolInvocation};
        use jiff::Timestamp;

        let analyzer = Analyzer {
            parsers: vec![],
            config: AnalyzerConfig::default(),
        };

        let now = Timestamp::now();
        let tool_invocations = vec![
            ToolInvocation {
                timestamp: now,
                tool_name: "npm".to_string(),
                tool_category: ToolCategory::PackageManager,
                command_line: "npm install".to_string(),
                arguments: vec!["install".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-1".to_string(),
            },
            ToolInvocation {
                timestamp: now,
                tool_name: "npm".to_string(),
                tool_category: ToolCategory::PackageManager,
                command_line: "npm test".to_string(),
                arguments: vec!["test".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-2".to_string(),
            },
            ToolInvocation {
                timestamp: now,
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::BuildTool,
                command_line: "cargo build".to_string(),
                arguments: vec!["build".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("rust-expert".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-3".to_string(),
            },
        ];

        let correlations = analyzer.calculate_agent_tool_correlations(&tool_invocations);

        assert_eq!(correlations.len(), 2); // 2 unique agent-tool pairs
        assert_eq!(correlations[0].usage_count, 2); // developer-npm should be first (highest usage)
        assert_eq!(correlations[0].agent_type, "developer");
        assert_eq!(correlations[0].tool_name, "npm");
        assert_eq!(correlations[0].success_rate, 1.0); // Both npm calls succeeded
        assert_eq!(correlations[1].usage_count, 1);
        assert_eq!(correlations[1].agent_type, "rust-expert");
    }

    #[test]
    fn test_calculate_tool_statistics() {
        use crate::models::{ToolCategory, ToolInvocation};
        use jiff::Timestamp;

        let analyzer = Analyzer {
            parsers: vec![],
            config: AnalyzerConfig::default(),
        };

        let now = Timestamp::now();
        let tool_invocations = vec![
            ToolInvocation {
                timestamp: now,
                tool_name: "npm".to_string(),
                tool_category: ToolCategory::PackageManager,
                command_line: "npm install".to_string(),
                arguments: vec!["install".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-1".to_string(),
            },
            ToolInvocation {
                timestamp: now,
                tool_name: "npm".to_string(),
                tool_category: ToolCategory::PackageManager,
                command_line: "npm test".to_string(),
                arguments: vec!["test".to_string()],
                flags: HashMap::new(),
                exit_code: Some(1),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-2".to_string(),
            },
        ];

        let stats = analyzer.calculate_tool_statistics(&tool_invocations);

        assert_eq!(stats.len(), 1); // Only npm
        let npm_stats = stats.get("npm").unwrap();
        assert_eq!(npm_stats.total_invocations, 2);
        assert_eq!(npm_stats.success_count, 1);
        assert_eq!(npm_stats.failure_count, 1);
        assert!(npm_stats.agents_using.contains(&"developer".to_string()));
        assert!(matches!(npm_stats.category, ToolCategory::PackageManager));
    }

    #[test]
    fn test_calculate_category_breakdown() {
        use crate::models::{ToolCategory, ToolInvocation};
        use jiff::Timestamp;

        let analyzer = Analyzer {
            parsers: vec![],
            config: AnalyzerConfig::default(),
        };

        let now = Timestamp::now();
        let tool_invocations = vec![
            ToolInvocation {
                timestamp: now,
                tool_name: "npm".to_string(),
                tool_category: ToolCategory::PackageManager,
                command_line: "npm install".to_string(),
                arguments: vec![],
                flags: HashMap::new(),
                exit_code: None,
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-1".to_string(),
            },
            ToolInvocation {
                timestamp: now,
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::BuildTool,
                command_line: "cargo build".to_string(),
                arguments: vec![],
                flags: HashMap::new(),
                exit_code: None,
                agent_context: Some("rust-expert".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-2".to_string(),
            },
            ToolInvocation {
                timestamp: now,
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::BuildTool,
                command_line: "cargo test".to_string(),
                arguments: vec![],
                flags: HashMap::new(),
                exit_code: None,
                agent_context: Some("rust-expert".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-3".to_string(),
            },
        ];

        let breakdown = analyzer.calculate_category_breakdown(&tool_invocations);

        assert_eq!(breakdown.len(), 2);
        // BuildTool should be first (2 invocations)
        let categories: Vec<_> = breakdown.keys().collect();
        assert!(matches!(categories[0], ToolCategory::BuildTool));
        assert_eq!(breakdown[categories[0]], 2);
        assert_eq!(breakdown[categories[1]], 1);
    }

    #[test]
    fn test_detect_tool_chains_basic() {
        use crate::models::{ToolCategory, ToolInvocation};
        use jiff::Timestamp;

        let analyzer = Analyzer {
            parsers: vec![],
            config: AnalyzerConfig::default(),
        };

        let now = Timestamp::now();
        let one_sec = jiff::Span::new().seconds(1);

        // Create a chain that appears twice: cargo build -> cargo test
        let tool_invocations = vec![
            // First occurrence
            ToolInvocation {
                timestamp: now,
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::BuildTool,
                command_line: "cargo build".to_string(),
                arguments: vec!["build".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-1".to_string(),
            },
            ToolInvocation {
                timestamp: now.checked_add(one_sec).unwrap(),
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::Testing,
                command_line: "cargo test".to_string(),
                arguments: vec!["test".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-2".to_string(),
            },
            // Second occurrence
            ToolInvocation {
                timestamp: now.checked_add(jiff::Span::new().seconds(10)).unwrap(),
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::BuildTool,
                command_line: "cargo build".to_string(),
                arguments: vec!["build".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-3".to_string(),
            },
            ToolInvocation {
                timestamp: now.checked_add(jiff::Span::new().seconds(11)).unwrap(),
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::Testing,
                command_line: "cargo test".to_string(),
                arguments: vec!["test".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-4".to_string(),
            },
        ];

        let chains = analyzer.detect_tool_chains(&tool_invocations);

        assert!(!chains.is_empty(), "Should detect at least one chain");

        // Find the chain with the exact pattern we're looking for
        let cargo_chain = chains
            .iter()
            .find(|c| c.tools == vec!["cargo".to_string(), "cargo".to_string()]);
        assert!(cargo_chain.is_some(), "Should find cargo->cargo chain");

        let chain = cargo_chain.unwrap();
        // With 4 tools, we get 3 overlapping windows: [0,1], [1,2], [2,3]
        assert!(chain.frequency >= 2, "Frequency should be at least 2");
        assert_eq!(chain.typical_agent, Some("developer".to_string()));
        assert_eq!(chain.success_rate, 1.0);
    }

    #[test]
    fn test_detect_tool_chains_deployment_pipeline() {
        use crate::models::{ToolCategory, ToolInvocation};
        use jiff::Timestamp;

        let analyzer = Analyzer {
            parsers: vec![],
            config: AnalyzerConfig::default(),
        };

        let now = Timestamp::now();
        let one_sec = jiff::Span::new().seconds(1);

        // Create deployment pipeline: npm install -> npm build -> wrangler deploy
        // Appears twice across different sessions
        let tool_invocations = vec![
            // Session 1 - First occurrence
            ToolInvocation {
                timestamp: now,
                tool_name: "npm".to_string(),
                tool_category: ToolCategory::PackageManager,
                command_line: "npm install".to_string(),
                arguments: vec!["install".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("devops".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-1".to_string(),
            },
            ToolInvocation {
                timestamp: now.checked_add(one_sec).unwrap(),
                tool_name: "npm".to_string(),
                tool_category: ToolCategory::BuildTool,
                command_line: "npm build".to_string(),
                arguments: vec!["build".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("devops".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-2".to_string(),
            },
            ToolInvocation {
                timestamp: now.checked_add(jiff::Span::new().seconds(2)).unwrap(),
                tool_name: "wrangler".to_string(),
                tool_category: ToolCategory::CloudDeploy,
                command_line: "wrangler deploy".to_string(),
                arguments: vec!["deploy".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("devops".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-3".to_string(),
            },
            // Session 2 - Second occurrence
            ToolInvocation {
                timestamp: now.checked_add(jiff::Span::new().minutes(10)).unwrap(),
                tool_name: "npm".to_string(),
                tool_category: ToolCategory::PackageManager,
                command_line: "npm install".to_string(),
                arguments: vec!["install".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("devops".to_string()),
                session_id: "session-2".to_string(),
                message_id: "msg-4".to_string(),
            },
            ToolInvocation {
                timestamp: now
                    .checked_add(jiff::Span::new().minutes(10).seconds(1))
                    .unwrap(),
                tool_name: "npm".to_string(),
                tool_category: ToolCategory::BuildTool,
                command_line: "npm build".to_string(),
                arguments: vec!["build".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("devops".to_string()),
                session_id: "session-2".to_string(),
                message_id: "msg-5".to_string(),
            },
            ToolInvocation {
                timestamp: now
                    .checked_add(jiff::Span::new().minutes(10).seconds(2))
                    .unwrap(),
                tool_name: "wrangler".to_string(),
                tool_category: ToolCategory::CloudDeploy,
                command_line: "wrangler deploy".to_string(),
                arguments: vec!["deploy".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("devops".to_string()),
                session_id: "session-2".to_string(),
                message_id: "msg-6".to_string(),
            },
        ];

        let chains = analyzer.detect_tool_chains(&tool_invocations);

        assert!(!chains.is_empty(), "Should detect deployment chain");

        // Find the 3-tool chain (npm -> npm -> wrangler)
        let three_tool_chain = chains.iter().find(|c| c.tools.len() == 3);
        assert!(three_tool_chain.is_some(), "Should find 3-tool chain");

        let chain = three_tool_chain.unwrap();
        assert_eq!(
            chain.tools,
            vec!["npm".to_string(), "npm".to_string(), "wrangler".to_string()]
        );
        assert_eq!(chain.frequency, 2);
        assert_eq!(chain.typical_agent, Some("devops".to_string()));
        assert_eq!(chain.success_rate, 1.0);
    }

    #[test]
    fn test_detect_tool_chains_ignores_single_occurrence() {
        use crate::models::{ToolCategory, ToolInvocation};
        use jiff::Timestamp;

        let analyzer = Analyzer {
            parsers: vec![],
            config: AnalyzerConfig::default(),
        };

        let now = Timestamp::now();
        let one_sec = jiff::Span::new().seconds(1);

        // Create a chain that appears only once
        let tool_invocations = vec![
            ToolInvocation {
                timestamp: now,
                tool_name: "npm".to_string(),
                tool_category: ToolCategory::PackageManager,
                command_line: "npm install".to_string(),
                arguments: vec!["install".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-1".to_string(),
            },
            ToolInvocation {
                timestamp: now.checked_add(one_sec).unwrap(),
                tool_name: "npm".to_string(),
                tool_category: ToolCategory::Testing,
                command_line: "npm test".to_string(),
                arguments: vec!["test".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-2".to_string(),
            },
        ];

        let chains = analyzer.detect_tool_chains(&tool_invocations);

        // Should be empty because chain appears only once
        assert!(
            chains.is_empty(),
            "Should not detect chains that appear only once"
        );
    }

    #[test]
    fn test_detect_tool_chains_time_window() {
        use crate::models::{ToolCategory, ToolInvocation};
        use jiff::Timestamp;

        let analyzer = Analyzer {
            parsers: vec![],
            config: AnalyzerConfig::default(),
        };

        let now = Timestamp::now();

        // Create tools that are too far apart (> 1 hour)
        let tool_invocations = vec![
            ToolInvocation {
                timestamp: now,
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::BuildTool,
                command_line: "cargo build".to_string(),
                arguments: vec!["build".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-1".to_string(),
            },
            ToolInvocation {
                timestamp: now.checked_add(jiff::Span::new().hours(2)).unwrap(),
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::Testing,
                command_line: "cargo test".to_string(),
                arguments: vec!["test".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-2".to_string(),
            },
        ];

        let chains = analyzer.detect_tool_chains(&tool_invocations);

        // Should be empty because tools are too far apart
        assert!(
            chains.is_empty(),
            "Should not detect chains with tools too far apart"
        );
    }

    #[test]
    fn test_detect_tool_chains_success_rate() {
        use crate::models::{ToolCategory, ToolInvocation};
        use jiff::Timestamp;

        let analyzer = Analyzer {
            parsers: vec![],
            config: AnalyzerConfig::default(),
        };

        let now = Timestamp::now();
        let one_sec = jiff::Span::new().seconds(1);

        // Create a chain with mixed success: 2 occurrences, 1 success, 1 failure
        let tool_invocations = vec![
            // First occurrence - success
            ToolInvocation {
                timestamp: now,
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::BuildTool,
                command_line: "cargo build".to_string(),
                arguments: vec!["build".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-1".to_string(),
            },
            ToolInvocation {
                timestamp: now.checked_add(one_sec).unwrap(),
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::Testing,
                command_line: "cargo test".to_string(),
                arguments: vec!["test".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-2".to_string(),
            },
            // Second occurrence - failure on test
            ToolInvocation {
                timestamp: now.checked_add(jiff::Span::new().seconds(10)).unwrap(),
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::BuildTool,
                command_line: "cargo build".to_string(),
                arguments: vec!["build".to_string()],
                flags: HashMap::new(),
                exit_code: Some(0),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-3".to_string(),
            },
            ToolInvocation {
                timestamp: now.checked_add(jiff::Span::new().seconds(11)).unwrap(),
                tool_name: "cargo".to_string(),
                tool_category: ToolCategory::Testing,
                command_line: "cargo test".to_string(),
                arguments: vec!["test".to_string()],
                flags: HashMap::new(),
                exit_code: Some(1),
                agent_context: Some("developer".to_string()),
                session_id: "session-1".to_string(),
                message_id: "msg-4".to_string(),
            },
        ];

        let chains = analyzer.detect_tool_chains(&tool_invocations);

        assert!(!chains.is_empty(), "Should detect chain");

        // Find the cargo->cargo chain
        let cargo_chain = chains
            .iter()
            .find(|c| c.tools == vec!["cargo".to_string(), "cargo".to_string()]);
        assert!(cargo_chain.is_some(), "Should find cargo->cargo chain");

        let chain = cargo_chain.unwrap();
        assert!(chain.frequency >= 2, "Frequency should be at least 2");
        // With overlapping windows, we have:
        // Window [0,1]: cargo(exit 0) + cargo(exit 0) = 2 success, 0 failure
        // Window [1,2]: cargo(exit 0) + cargo(exit 0) = 2 success, 0 failure
        // Window [2,3]: cargo(exit 0) + cargo(exit 1) = 1 success, 1 failure
        // Total: 5 successes out of 6 tools = 0.833...
        assert!(
            chain.success_rate >= 0.82 && chain.success_rate <= 0.84,
            "Success rate should be around 0.83, got {}",
            chain.success_rate
        );
    }
}
