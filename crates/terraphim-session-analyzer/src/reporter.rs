use crate::models::{
    AgentAttribution, AgentStatistics, AgentToolCorrelation, CollaborationPattern, SessionAnalysis,
    ToolAnalysis,
};
use anyhow::Result;
use colored::Colorize;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;
use tabled::{
    settings::{object::Columns, Modify, Style, Width},
    Table, Tabled,
};

pub struct Reporter {
    show_colors: bool,
}

impl Reporter {
    #[must_use]
    pub fn new() -> Self {
        Self { show_colors: true }
    }

    #[must_use]
    pub fn with_colors(mut self, show_colors: bool) -> Self {
        self.show_colors = show_colors;
        self
    }

    /// Print analysis results to terminal with rich formatting
    pub fn print_terminal(&self, analyses: &[SessionAnalysis]) {
        if analyses.is_empty() {
            println!("{}", "No sessions found to analyze".yellow());
            return;
        }

        // Print header
        self.print_header(analyses);

        // Print each session analysis
        for (i, analysis) in analyses.iter().enumerate() {
            if i > 0 {
                println!();
            }
            self.print_session_analysis(analysis);
        }

        // Print summary if multiple sessions
        if analyses.len() > 1 {
            println!();
            self.print_summary(analyses);
        }
    }

    fn print_header(&self, analyses: &[SessionAnalysis]) {
        let title = if analyses.len() == 1 {
            "Claude Session Analysis"
        } else {
            "Claude Sessions Analysis"
        };

        println!("{}", format!("═══ {} ═══", title).bold().cyan());

        if analyses.len() > 1 {
            println!(
                "{} {}",
                "Sessions analyzed:".bold(),
                analyses.len().to_string().yellow()
            );
        }
        println!();
    }

    fn print_session_analysis(&self, analysis: &SessionAnalysis) {
        // Session info
        println!("{} {}", "Session:".bold(), analysis.session_id.yellow());
        println!("{} {}", "Project:".bold(), analysis.project_path.green());
        println!("{} {}ms", "Duration:".bold(), analysis.duration_ms);

        if !analysis.agents.is_empty() {
            println!("{} {}", "Agents used:".bold(), analysis.agents.len());
        }

        // File attributions table
        if !analysis.file_to_agents.is_empty() {
            println!("\n{}", "📊 File Contributions:".bold());
            self.print_file_attributions(&analysis.file_to_agents);
        }

        // Agent statistics
        if !analysis.agent_stats.is_empty() {
            println!("\n{}", "👥 Agent Statistics:".bold());
            self.print_agent_statistics(&analysis.agent_stats);
        }

        // Timeline
        if !analysis.agents.is_empty() {
            println!("\n{}", "⏱️ Timeline:".bold());
            self.print_timeline(analysis);
        }

        // Collaboration patterns
        if !analysis.collaboration_patterns.is_empty() {
            println!("\n{}", "🔗 Collaboration Patterns:".bold());
            self.print_collaboration_patterns(&analysis.collaboration_patterns);
        }
    }

    fn print_file_attributions(&self, file_to_agents: &IndexMap<String, Vec<AgentAttribution>>) {
        let mut table_data = Vec::new();

        for (file_path, attributions) in file_to_agents {
            let file_display = self.truncate_path(file_path, 40);

            for (i, attr) in attributions.iter().enumerate() {
                let file_col = if i == 0 {
                    file_display.clone()
                } else {
                    String::new()
                };

                table_data.push(FileRow {
                    file: file_col,
                    agent: self.format_agent_display(&attr.agent_type),
                    contribution: format!("{:.1}%", attr.contribution_percent),
                    confidence: format!("{:.0}%", attr.confidence_score * 100.0),
                    operations: attr.operations.len().to_string(),
                });
            }
        }

        if !table_data.is_empty() {
            let table = Table::new(table_data)
                .with(Style::modern())
                .with(Modify::new(Columns::new(..1)).with(Width::wrap(40)))
                .to_string();
            println!("{}", table);
        }
    }

    fn print_agent_statistics(&self, agent_stats: &IndexMap<String, AgentStatistics>) {
        let mut table_data = Vec::new();

        for stats in agent_stats.values() {
            table_data.push(AgentRow {
                agent: self.format_agent_display(&stats.agent_type),
                invocations: stats.total_invocations.to_string(),
                duration: self.format_duration(stats.total_duration_ms),
                files: stats.files_touched.to_string(),
                tools: stats.tools_used.len().to_string(),
            });
        }

        if !table_data.is_empty() {
            let table = Table::new(table_data).with(Style::modern()).to_string();
            println!("{}", table);
        }
    }

    fn print_timeline(&self, analysis: &SessionAnalysis) {
        let mut events: Vec<_> = analysis
            .agents
            .iter()
            .map(|a| (a.timestamp, &a.agent_type, &a.task_description))
            .collect();

        events.sort_by(|a, b| a.0.cmp(&b.0));

        for (timestamp, agent_type, description) in events.iter().take(10) {
            let time_str = self.format_timestamp(*timestamp);
            let agent_display = self.format_agent_display(agent_type);
            let desc = self.truncate_text(description, 60);

            println!(
                "  {} {} - {}",
                time_str.dimmed(),
                agent_display,
                desc.dimmed()
            );
        }

        if events.len() > 10 {
            println!(
                "  {} {} more events...",
                "...".dimmed(),
                (events.len() - 10).to_string().dimmed()
            );
        }
    }

    fn print_collaboration_patterns(&self, patterns: &[CollaborationPattern]) {
        for pattern in patterns {
            let agents_display = pattern
                .agents
                .iter()
                .map(|a| self.format_agent_icon(a))
                .collect::<Vec<_>>()
                .join(" → ");

            println!(
                "  {} {} ({}% confidence)",
                agents_display,
                pattern.description.dimmed(),
                (pattern.confidence * 100.0) as u32
            );
        }
    }

    fn print_summary(&self, analyses: &[SessionAnalysis]) {
        println!("{}", "📈 Summary Statistics:".bold());

        // Calculate totals
        let total_agents: usize = analyses.iter().map(|a| a.agents.len()).sum();
        let total_files: usize = analyses.iter().map(|a| a.file_to_agents.len()).sum();
        let total_duration: u64 = analyses.iter().map(|a| a.duration_ms).sum();

        // Most active agents across all sessions
        let mut agent_counts: IndexMap<String, u32> = IndexMap::new();
        for analysis in analyses {
            for agent in &analysis.agents {
                *agent_counts.entry(agent.agent_type.clone()).or_insert(0) += 1;
            }
        }

        let mut sorted_agents: Vec<_> = agent_counts.into_iter().collect();
        sorted_agents.sort_by(|a, b| b.1.cmp(&a.1));

        println!("  {} {}", "Total agent invocations:".bold(), total_agents);
        println!("  {} {}", "Total files modified:".bold(), total_files);
        println!(
            "  {} {}",
            "Total session time:".bold(),
            self.format_duration(total_duration)
        );

        println!("\n{}", "🏆 Most Active Agents:".bold());
        for (agent, count) in sorted_agents.iter().take(5) {
            println!(
                "  {} {} ({}x)",
                self.format_agent_icon(agent),
                agent.cyan(),
                count.to_string().yellow()
            );
        }
    }

    /// Generate markdown report
    pub fn to_markdown(&self, analyses: &[SessionAnalysis]) -> Result<String> {
        let mut md = String::new();

        writeln!(md, "# Claude Session Analysis Report\n")?;

        if analyses.len() > 1 {
            writeln!(md, "**Sessions Analyzed**: {}\n", analyses.len())?;
        }

        for (i, analysis) in analyses.iter().enumerate() {
            if analyses.len() > 1 {
                writeln!(md, "## Session {} - {}\n", i + 1, analysis.session_id)?;
            } else {
                writeln!(md, "## Session Analysis\n")?;
            }

            writeln!(md, "- **Session ID**: `{}`", analysis.session_id)?;
            writeln!(md, "- **Project**: `{}`", analysis.project_path)?;
            writeln!(md, "- **Duration**: {} ms", analysis.duration_ms)?;
            writeln!(md, "- **Agents Used**: {}", analysis.agents.len())?;
            writeln!(
                md,
                "- **Files Modified**: {}\n",
                analysis.file_to_agents.len()
            )?;

            if !analysis.file_to_agents.is_empty() {
                writeln!(md, "### Files Created/Modified\n")?;

                for (file_path, attributions) in &analysis.file_to_agents {
                    writeln!(md, "#### `{}`\n", file_path)?;
                    writeln!(md, "| Agent | Contribution | Confidence | Operations |")?;
                    writeln!(md, "|-------|-------------|------------|------------|")?;

                    for attr in attributions {
                        writeln!(
                            md,
                            "| {} | {:.1}% | {:.0}% | {} |",
                            attr.agent_type,
                            attr.contribution_percent,
                            attr.confidence_score * 100.0,
                            attr.operations.len()
                        )?;
                    }
                    writeln!(md)?;
                }
            }

            if !analysis.collaboration_patterns.is_empty() {
                writeln!(md, "### Collaboration Patterns\n")?;
                for pattern in &analysis.collaboration_patterns {
                    writeln!(
                        md,
                        "- **{}**: {} ({:.0}% confidence)",
                        pattern.pattern_type,
                        pattern.description,
                        pattern.confidence * 100.0
                    )?;
                }
                writeln!(md)?;
            }
        }

        Ok(md)
    }

    /// Generate JSON report
    pub fn to_json(&self, analyses: &[SessionAnalysis]) -> Result<String> {
        let json = if analyses.len() == 1 {
            serde_json::to_string_pretty(&analyses[0])?
        } else {
            serde_json::to_string_pretty(analyses)?
        };
        Ok(json)
    }

    /// Generate CSV report
    pub fn to_csv(&self, analyses: &[SessionAnalysis]) -> Result<String> {
        let mut csv_data = Vec::new();

        // Add header
        csv_data.push(vec![
            "session_id".to_string(),
            "file_path".to_string(),
            "agent_type".to_string(),
            "contribution_percent".to_string(),
            "confidence_score".to_string(),
            "operations_count".to_string(),
        ]);

        // Add data rows
        for analysis in analyses {
            for (file_path, attributions) in &analysis.file_to_agents {
                for attr in attributions {
                    csv_data.push(vec![
                        analysis.session_id.clone(),
                        file_path.clone(),
                        attr.agent_type.clone(),
                        attr.contribution_percent.to_string(),
                        attr.confidence_score.to_string(),
                        attr.operations.len().to_string(),
                    ]);
                }
            }
        }

        let mut csv_output = String::new();
        for row in csv_data {
            writeln!(csv_output, "{}", row.join(","))?;
        }

        Ok(csv_output)
    }

    // Helper formatting functions

    fn format_agent_display(&self, agent_type: &str) -> String {
        if self.show_colors {
            format!(
                "{} {}",
                self.format_agent_icon(agent_type),
                agent_type.cyan()
            )
        } else {
            format!("{} {}", self.format_agent_icon(agent_type), agent_type)
        }
    }

    pub fn format_agent_icon(&self, agent_type: &str) -> String {
        match agent_type {
            "architect" => "🏗️".to_string(),
            "developer" => "💻".to_string(),
            "backend-architect" => "🔧".to_string(),
            "frontend-developer" => "🎨".to_string(),
            "rust-performance-expert" => "🦀".to_string(),
            "rust-code-reviewer" => "🔍".to_string(),
            "debugger" => "🐛".to_string(),
            "technical-writer" => "📝".to_string(),
            "test-writer-fixer" => "🧪".to_string(),
            "rapid-prototyper" => "⚡".to_string(),
            "devops-automator" => "🚀".to_string(),
            "overseer" => "👁️".to_string(),
            "ai-engineer" => "🤖".to_string(),
            "general-purpose" => "🎯".to_string(),
            _ => "🔧".to_string(),
        }
    }

    fn format_timestamp(&self, timestamp: jiff::Timestamp) -> String {
        timestamp.strftime("%H:%M:%S").to_string()
    }

    fn format_duration(&self, duration_ms: u64) -> String {
        if duration_ms < 1000 {
            format!("{}ms", duration_ms)
        } else if duration_ms < 60_000 {
            format!("{:.1}s", duration_ms as f64 / 1000.0)
        } else if duration_ms < 3_600_000 {
            format!("{:.1}m", duration_ms as f64 / 60_000.0)
        } else {
            format!("{:.1}h", duration_ms as f64 / 3_600_000.0)
        }
    }

    fn truncate_path(&self, path: &str, max_len: usize) -> String {
        if path.len() <= max_len {
            path.to_string()
        } else {
            let start_len = max_len / 3;
            let end_len = max_len - start_len - 3;
            format!("{}...{}", &path[..start_len], &path[path.len() - end_len..])
        }
    }

    fn truncate_text(&self, text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}...", &text[..max_len - 3])
        }
    }

    /// Print tool usage analysis to terminal
    #[allow(dead_code)] // Replaced by print_tool_analysis_detailed
    pub fn print_tool_analysis(
        &self,
        stats: &std::collections::HashMap<String, crate::models::ToolStatistics>,
    ) {
        if stats.is_empty() {
            println!("{}", "No tool usage found".yellow());
            return;
        }

        println!("{}", "Tool Usage Analysis".bold().cyan());
        println!();

        // Convert to sorted vector
        let mut tool_stats: Vec<_> = stats.iter().collect();
        tool_stats.sort_by(|a, b| b.1.total_invocations.cmp(&a.1.total_invocations));

        // Create table rows
        let mut rows = Vec::new();
        for (tool_name, stat) in tool_stats {
            let agents_str = if stat.agents_using.is_empty() {
                "-".to_string()
            } else {
                stat.agents_using.join(", ")
            };

            let sessions_str = format!("{} sessions", stat.sessions.len());
            let category_str = format!("{:?}", stat.category);

            rows.push(ToolRow {
                tool: tool_name.clone(),
                count: stat.total_invocations.to_string(),
                category: category_str,
                agents: self.truncate_text(&agents_str, 40),
                sessions: sessions_str,
            });
        }

        let table = Table::new(rows)
            .with(Style::modern())
            .with(Modify::new(Columns::new(0..1)).with(Width::wrap(20)))
            .with(Modify::new(Columns::new(3..4)).with(Width::wrap(40)))
            .to_string();

        println!("{table}");
        println!();
        println!(
            "{} {} unique tools found",
            "Total:".bold(),
            stats.len().to_string().yellow()
        );
    }

    /// Print detailed tool analysis with correlation matrix
    pub fn print_tool_analysis_detailed(
        &self,
        analysis: &ToolAnalysis,
        show_correlation: bool,
    ) -> Result<()> {
        if analysis.tool_statistics.is_empty() {
            println!("{}", "No tool usage found".yellow());
            return Ok(());
        }

        // Header
        println!("{}", "═══ Tool Analysis ═══".bold().cyan());
        println!();

        // Summary statistics
        println!("{}", "📊 Summary:".bold());
        println!(
            "  {} {}",
            "Total Tool Invocations:".bold(),
            analysis.total_tool_invocations.to_string().yellow()
        );
        println!(
            "  {} {}",
            "Unique Tools:".bold(),
            analysis.tool_statistics.len().to_string().yellow()
        );
        println!(
            "  {} {}",
            "Tool Categories:".bold(),
            analysis.category_breakdown.len().to_string().yellow()
        );
        println!();

        // Tool frequency table
        println!("{}", "🔧 Tool Frequency:".bold());
        let mut tool_rows = Vec::new();
        for (tool_name, stat) in &analysis.tool_statistics {
            let agents_str = if stat.agents_using.is_empty() {
                "-".to_string()
            } else {
                stat.agents_using.join(", ")
            };

            let success_rate = if stat.total_invocations > 0 {
                #[allow(clippy::cast_precision_loss)]
                let rate = (stat.success_count as f32 / stat.total_invocations as f32) * 100.0;
                format!("{:.1}%", rate)
            } else {
                "-".to_string()
            };

            tool_rows.push(DetailedToolRow {
                tool: tool_name.clone(),
                count: stat.total_invocations.to_string(),
                category: format!("{:?}", stat.category),
                agents: self.truncate_text(&agents_str, 30),
                success_rate,
                sessions: stat.sessions.len().to_string(),
            });
        }

        // Sort by invocation count
        tool_rows.sort_by(|a, b| {
            b.count
                .parse::<u32>()
                .unwrap_or(0)
                .cmp(&a.count.parse::<u32>().unwrap_or(0))
        });

        let table = Table::new(tool_rows)
            .with(Style::modern())
            .with(Modify::new(Columns::new(0..1)).with(Width::wrap(20)))
            .with(Modify::new(Columns::new(3..4)).with(Width::wrap(30)))
            .to_string();
        println!("{}", table);
        println!();

        // Category breakdown
        println!("{}", "📂 Category Breakdown:".bold());
        let mut category_rows: Vec<_> = analysis
            .category_breakdown
            .iter()
            .map(|(cat, count)| (format!("{:?}", cat), *count))
            .collect();
        category_rows.sort_by(|a, b| b.1.cmp(&a.1));

        for (category, count) in category_rows {
            #[allow(clippy::cast_precision_loss)]
            let percentage = (count as f32 / analysis.total_tool_invocations as f32) * 100.0;
            println!(
                "  {} {} ({:.1}%)",
                category.cyan(),
                count.to_string().yellow(),
                percentage
            );
        }
        println!();

        // Correlation matrix if requested
        if show_correlation && !analysis.agent_tool_correlations.is_empty() {
            self.print_correlation_matrix(&analysis.agent_tool_correlations);
        }

        Ok(())
    }

    /// Print agent-tool correlation matrix using Unicode blocks
    pub fn print_correlation_matrix(&self, correlations: &[AgentToolCorrelation]) {
        println!("{}", "🔗 Agent-Tool Correlation Matrix:".bold());
        println!();

        // Build matrix structure
        let mut agents: Vec<String> = correlations
            .iter()
            .map(|c| c.agent_type.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        agents.sort();

        let mut tools: Vec<String> = correlations
            .iter()
            .map(|c| c.tool_name.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        tools.sort();

        // Build lookup map
        let mut correlation_map: HashMap<(String, String), &AgentToolCorrelation> = HashMap::new();
        for corr in correlations {
            correlation_map.insert((corr.agent_type.clone(), corr.tool_name.clone()), corr);
        }

        // Print header row
        print!("{:15}", "");
        for tool in &tools {
            print!("{:12}", self.truncate_text(tool, 10));
        }
        println!();

        // Print separator
        print!("{:15}", "");
        for _ in &tools {
            print!("{:12}", "─".repeat(10));
        }
        println!();

        // Print each agent row
        for agent in &agents {
            print!("{:15}", self.truncate_text(agent, 13));

            for tool in &tools {
                let block = if let Some(corr) = correlation_map.get(&(agent.clone(), tool.clone()))
                {
                    self.get_correlation_block(corr.average_invocations_per_session)
                } else {
                    "-".to_string()
                };

                if self.show_colors {
                    print!("{:12}", block.cyan());
                } else {
                    print!("{:12}", block);
                }
            }
            println!();
        }
        println!();

        // Legend
        println!("{}", "Legend:".dimmed());
        println!("{}", "  █████ = High usage (8+ per session)".dimmed());
        println!("{}", "  ████  = Medium-high (6-8 per session)".dimmed());
        println!("{}", "  ███   = Medium (4-6 per session)".dimmed());
        println!("{}", "  ██    = Low-medium (2-4 per session)".dimmed());
        println!("{}", "  █     = Low (1-2 per session)".dimmed());
        println!("{}", "  -     = None".dimmed());
        println!();
    }

    /// Get Unicode block representation for correlation strength
    fn get_correlation_block(&self, avg_invocations: f32) -> String {
        if avg_invocations >= 8.0 {
            "█████".to_string()
        } else if avg_invocations >= 6.0 {
            "████".to_string()
        } else if avg_invocations >= 4.0 {
            "███".to_string()
        } else if avg_invocations >= 2.0 {
            "██".to_string()
        } else if avg_invocations >= 1.0 {
            "█".to_string()
        } else if avg_invocations > 0.0 {
            "▒".to_string()
        } else {
            "-".to_string()
        }
    }

    /// Export tool analysis to JSON
    pub fn tool_analysis_to_json(&self, analysis: &ToolAnalysis) -> Result<String> {
        let json = serde_json::to_string_pretty(analysis)?;
        Ok(json)
    }

    /// Export tool analysis to CSV
    pub fn tool_analysis_to_csv(&self, analysis: &ToolAnalysis) -> Result<String> {
        let mut csv_data = Vec::new();

        // Add header
        csv_data.push(vec![
            "tool_name".to_string(),
            "category".to_string(),
            "count".to_string(),
            "agents_using".to_string(),
            "success_rate".to_string(),
            "sessions".to_string(),
        ]);

        // Add data rows
        for (tool_name, stat) in &analysis.tool_statistics {
            let agents_str = stat.agents_using.join(";");

            let success_rate = if stat.total_invocations > 0 {
                #[allow(clippy::cast_precision_loss)]
                let rate = (stat.success_count as f32 / stat.total_invocations as f32) * 100.0;
                format!("{:.2}", rate)
            } else {
                "0".to_string()
            };

            csv_data.push(vec![
                tool_name.clone(),
                format!("{:?}", stat.category),
                stat.total_invocations.to_string(),
                agents_str,
                success_rate,
                stat.sessions.len().to_string(),
            ]);
        }

        let mut csv_output = String::new();
        for row in csv_data {
            writeln!(csv_output, "{}", row.join(","))?;
        }

        Ok(csv_output)
    }

    /// Export tool analysis to Markdown
    pub fn tool_analysis_to_markdown(&self, analysis: &ToolAnalysis) -> Result<String> {
        let mut md = String::new();

        writeln!(md, "# Tool Usage Analysis Report\n")?;

        // Summary
        writeln!(md, "## Summary\n")?;
        writeln!(
            md,
            "- **Total Tool Invocations**: {}",
            analysis.total_tool_invocations
        )?;
        writeln!(md, "- **Unique Tools**: {}", analysis.tool_statistics.len())?;
        writeln!(
            md,
            "- **Tool Categories**: {}\n",
            analysis.category_breakdown.len()
        )?;

        // Category breakdown
        writeln!(md, "## Category Breakdown\n")?;
        let mut category_rows: Vec<_> = analysis
            .category_breakdown
            .iter()
            .map(|(cat, count)| (format!("{:?}", cat), *count))
            .collect();
        category_rows.sort_by(|a, b| b.1.cmp(&a.1));

        for (category, count) in category_rows {
            #[allow(clippy::cast_precision_loss)]
            let percentage = (count as f32 / analysis.total_tool_invocations as f32) * 100.0;
            writeln!(md, "- **{}**: {} ({:.1}%)", category, count, percentage)?;
        }
        writeln!(md)?;

        // Tool frequency table
        writeln!(md, "## Tool Frequency\n")?;
        writeln!(
            md,
            "| Tool | Category | Count | Agents | Success Rate | Sessions |"
        )?;
        writeln!(
            md,
            "|------|----------|-------|--------|--------------|----------|"
        )?;

        let mut tool_list: Vec<_> = analysis.tool_statistics.iter().collect();
        tool_list.sort_by(|a, b| b.1.total_invocations.cmp(&a.1.total_invocations));

        for (tool_name, stat) in tool_list {
            let agents_str = if stat.agents_using.is_empty() {
                "-".to_string()
            } else {
                stat.agents_using.join(", ")
            };

            let success_rate = if stat.total_invocations > 0 {
                #[allow(clippy::cast_precision_loss)]
                let rate = (stat.success_count as f32 / stat.total_invocations as f32) * 100.0;
                format!("{:.1}%", rate)
            } else {
                "-".to_string()
            };

            writeln!(
                md,
                "| {} | {:?} | {} | {} | {} | {} |",
                tool_name,
                stat.category,
                stat.total_invocations,
                agents_str,
                success_rate,
                stat.sessions.len()
            )?;
        }
        writeln!(md)?;

        // Agent-tool correlations
        if !analysis.agent_tool_correlations.is_empty() {
            writeln!(md, "## Agent-Tool Correlations\n")?;
            writeln!(
                md,
                "| Agent | Tool | Usage Count | Success Rate | Avg/Session |"
            )?;
            writeln!(
                md,
                "|-------|------|-------------|--------------|-------------|"
            )?;

            for corr in &analysis.agent_tool_correlations {
                writeln!(
                    md,
                    "| {} | {} | {} | {:.1}% | {:.2} |",
                    corr.agent_type,
                    corr.tool_name,
                    corr.usage_count,
                    corr.success_rate * 100.0,
                    corr.average_invocations_per_session
                )?;
            }
            writeln!(md)?;
        }

        // Tool chains
        if !analysis.tool_chains.is_empty() {
            writeln!(md, "## Common Tool Chains\n")?;
            writeln!(md, "| Tools | Frequency | Success Rate | Typical Agent |")?;
            writeln!(md, "|-------|-----------|--------------|---------------|")?;

            for chain in &analysis.tool_chains {
                let agent_str = chain.typical_agent.as_ref().map_or("-", |a| a.as_str());
                writeln!(
                    md,
                    "| {} | {} | {:.1}% | {} |",
                    chain.tools.join(" → "),
                    chain.frequency,
                    chain.success_rate * 100.0,
                    agent_str
                )?;
            }
            writeln!(md)?;
        }

        Ok(md)
    }
}

impl Default for Reporter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Tabled)]
struct FileRow {
    #[tabled(rename = "File")]
    file: String,
    #[tabled(rename = "Agent")]
    agent: String,
    #[tabled(rename = "Contribution")]
    contribution: String,
    #[tabled(rename = "Confidence")]
    confidence: String,
    #[tabled(rename = "Ops")]
    operations: String,
}

#[derive(Tabled)]
#[allow(dead_code)] // Replaced by DetailedToolRow
struct ToolRow {
    #[tabled(rename = "Tool")]
    tool: String,
    #[tabled(rename = "Count")]
    count: String,
    #[tabled(rename = "Category")]
    category: String,
    #[tabled(rename = "Agents")]
    agents: String,
    #[tabled(rename = "Sessions")]
    sessions: String,
}

#[derive(Tabled)]
struct DetailedToolRow {
    #[tabled(rename = "Tool")]
    tool: String,
    #[tabled(rename = "Count")]
    count: String,
    #[tabled(rename = "Category")]
    category: String,
    #[tabled(rename = "Agents")]
    agents: String,
    #[tabled(rename = "Success Rate")]
    success_rate: String,
    #[tabled(rename = "Sessions")]
    sessions: String,
}

#[derive(Tabled)]
struct AgentRow {
    #[tabled(rename = "Agent")]
    agent: String,
    #[tabled(rename = "Invocations")]
    invocations: String,
    #[tabled(rename = "Duration")]
    duration: String,
    #[tabled(rename = "Files")]
    files: String,
    #[tabled(rename = "Tools")]
    tools: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AgentInvocation, ToolCategory, ToolStatistics};

    fn create_test_analysis() -> SessionAnalysis {
        let timestamp = jiff::Timestamp::now();

        SessionAnalysis {
            session_id: "test-session".to_string(),
            project_path: "/test/project".to_string(),
            start_time: timestamp,
            end_time: timestamp,
            duration_ms: 5000,
            agents: vec![AgentInvocation {
                timestamp,
                agent_type: "architect".to_string(),
                task_description: "Design system".to_string(),
                prompt: "Design the architecture".to_string(),
                files_modified: vec![],
                tools_used: vec![],
                duration_ms: Some(2000),
                parent_message_id: "msg-1".to_string(),
                session_id: "test-session".to_string(),
            }],
            file_operations: vec![],
            file_to_agents: IndexMap::new(),
            agent_stats: IndexMap::new(),
            collaboration_patterns: vec![],
        }
    }

    fn create_test_tool_analysis() -> ToolAnalysis {
        let timestamp = jiff::Timestamp::now();
        let mut tool_statistics = IndexMap::new();

        tool_statistics.insert(
            "npm".to_string(),
            ToolStatistics {
                tool_name: "npm".to_string(),
                category: ToolCategory::PackageManager,
                total_invocations: 10,
                agents_using: vec!["developer".to_string()],
                success_count: 9,
                failure_count: 1,
                first_seen: timestamp,
                last_seen: timestamp,
                command_patterns: vec!["npm install".to_string()],
                sessions: vec!["session-1".to_string()],
            },
        );

        tool_statistics.insert(
            "cargo".to_string(),
            ToolStatistics {
                tool_name: "cargo".to_string(),
                category: ToolCategory::BuildTool,
                total_invocations: 5,
                agents_using: vec!["developer".to_string()],
                success_count: 5,
                failure_count: 0,
                first_seen: timestamp,
                last_seen: timestamp,
                command_patterns: vec!["cargo build".to_string()],
                sessions: vec!["session-1".to_string()],
            },
        );

        let mut category_breakdown = IndexMap::new();
        category_breakdown.insert(ToolCategory::PackageManager, 10);
        category_breakdown.insert(ToolCategory::BuildTool, 5);

        ToolAnalysis {
            session_id: "test-session".to_string(),
            total_tool_invocations: 15,
            tool_statistics,
            agent_tool_correlations: vec![
                AgentToolCorrelation {
                    agent_type: "developer".to_string(),
                    tool_name: "npm".to_string(),
                    usage_count: 10,
                    success_rate: 0.9,
                    average_invocations_per_session: 5.0,
                },
                AgentToolCorrelation {
                    agent_type: "developer".to_string(),
                    tool_name: "cargo".to_string(),
                    usage_count: 5,
                    success_rate: 1.0,
                    average_invocations_per_session: 2.5,
                },
            ],
            tool_chains: vec![],
            category_breakdown,
        }
    }

    #[test]
    fn test_format_agent_icon() {
        let reporter = Reporter::new();
        assert_eq!(reporter.format_agent_icon("architect"), "🏗️");
        assert_eq!(reporter.format_agent_icon("developer"), "💻");
        assert_eq!(reporter.format_agent_icon("unknown"), "🔧");
    }

    #[test]
    fn test_format_duration() {
        let reporter = Reporter::new();
        assert_eq!(reporter.format_duration(500), "500ms");
        assert_eq!(reporter.format_duration(1500), "1.5s");
        assert_eq!(reporter.format_duration(65000), "1.1m");
    }

    #[test]
    fn test_truncate_path() {
        let reporter = Reporter::new();
        let long_path = "/very/long/path/to/some/file/deep/in/directory/structure/file.rs";
        let truncated = reporter.truncate_path(long_path, 20);
        assert!(truncated.len() <= 20);
        assert!(truncated.contains("..."));
    }

    #[test]
    fn test_to_markdown() {
        let reporter = Reporter::new();
        let analysis = create_test_analysis();
        let result = reporter.to_markdown(&[analysis]);
        assert!(result.is_ok());

        let markdown = result.unwrap();
        assert!(markdown.contains("# Claude Session Analysis Report"));
        assert!(markdown.contains("test-session"));
    }

    #[test]
    fn test_to_json() {
        let reporter = Reporter::new();
        let analysis = create_test_analysis();
        let result = reporter.to_json(&[analysis]);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert!(json.contains("test-session"));
        assert!(json.contains("architect"));
    }

    #[test]
    fn test_get_correlation_block() {
        let reporter = Reporter::new();
        assert_eq!(reporter.get_correlation_block(10.0), "█████");
        assert_eq!(reporter.get_correlation_block(7.0), "████");
        assert_eq!(reporter.get_correlation_block(5.0), "███");
        assert_eq!(reporter.get_correlation_block(3.0), "██");
        assert_eq!(reporter.get_correlation_block(1.5), "█");
        assert_eq!(reporter.get_correlation_block(0.5), "▒");
        assert_eq!(reporter.get_correlation_block(0.0), "-");
    }

    #[test]
    fn test_tool_analysis_to_json() {
        let reporter = Reporter::new();
        let analysis = create_test_tool_analysis();
        let result = reporter.tool_analysis_to_json(&analysis);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert!(json.contains("npm"));
        assert!(json.contains("cargo"));
        assert!(json.contains("PackageManager"));
        assert!(json.contains("developer"));
    }

    #[test]
    fn test_tool_analysis_to_csv() {
        let reporter = Reporter::new();
        let analysis = create_test_tool_analysis();
        let result = reporter.tool_analysis_to_csv(&analysis);
        assert!(result.is_ok());

        let csv = result.unwrap();
        assert!(csv.contains("tool_name,category,count,agents_using,success_rate,sessions"));
        assert!(csv.contains("npm"));
        assert!(csv.contains("cargo"));
        assert!(csv.contains("PackageManager"));
    }

    #[test]
    fn test_tool_analysis_to_markdown() {
        let reporter = Reporter::new();
        let analysis = create_test_tool_analysis();
        let result = reporter.tool_analysis_to_markdown(&analysis);
        assert!(result.is_ok());

        let markdown = result.unwrap();
        assert!(markdown.contains("# Tool Usage Analysis Report"));
        assert!(markdown.contains("## Summary"));
        assert!(markdown.contains("## Category Breakdown"));
        assert!(markdown.contains("## Tool Frequency"));
        assert!(markdown.contains("npm"));
        assert!(markdown.contains("cargo"));
    }

    #[test]
    fn test_print_tool_analysis_detailed() {
        let reporter = Reporter::new();
        let analysis = create_test_tool_analysis();
        let result = reporter.print_tool_analysis_detailed(&analysis, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_tool_analysis_detailed_with_correlation() {
        let reporter = Reporter::new();
        let analysis = create_test_tool_analysis();
        let result = reporter.print_tool_analysis_detailed(&analysis, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_correlation_matrix() {
        let reporter = Reporter::new();
        let correlations = vec![
            AgentToolCorrelation {
                agent_type: "developer".to_string(),
                tool_name: "npm".to_string(),
                usage_count: 10,
                success_rate: 0.9,
                average_invocations_per_session: 5.0,
            },
            AgentToolCorrelation {
                agent_type: "architect".to_string(),
                tool_name: "git".to_string(),
                usage_count: 3,
                success_rate: 1.0,
                average_invocations_per_session: 1.5,
            },
        ];

        reporter.print_correlation_matrix(&correlations);
    }

    #[test]
    fn test_print_tool_analysis_detailed_empty() {
        let reporter = Reporter::new();
        let analysis = ToolAnalysis {
            session_id: "test".to_string(),
            total_tool_invocations: 0,
            tool_statistics: IndexMap::new(),
            agent_tool_correlations: vec![],
            tool_chains: vec![],
            category_breakdown: IndexMap::new(),
        };

        let result = reporter.print_tool_analysis_detailed(&analysis, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_analysis_csv_with_semicolons() {
        let reporter = Reporter::new();
        let timestamp = jiff::Timestamp::now();
        let mut tool_statistics = IndexMap::new();

        tool_statistics.insert(
            "npm".to_string(),
            ToolStatistics {
                tool_name: "npm".to_string(),
                category: ToolCategory::PackageManager,
                total_invocations: 10,
                agents_using: vec!["developer".to_string(), "architect".to_string()],
                success_count: 9,
                failure_count: 1,
                first_seen: timestamp,
                last_seen: timestamp,
                command_patterns: vec![],
                sessions: vec!["session-1".to_string()],
            },
        );

        let analysis = ToolAnalysis {
            session_id: "test".to_string(),
            total_tool_invocations: 10,
            tool_statistics,
            agent_tool_correlations: vec![],
            tool_chains: vec![],
            category_breakdown: IndexMap::new(),
        };

        let result = reporter.tool_analysis_to_csv(&analysis);
        assert!(result.is_ok());

        let csv = result.unwrap();
        assert!(csv.contains("developer;architect"));
    }
}
