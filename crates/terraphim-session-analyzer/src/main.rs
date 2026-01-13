mod analyzer;
mod models;
mod parser;
mod patterns;
mod reporter;
mod tool_analyzer;

use models::SessionAnalysis;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use std::path::PathBuf;
use tracing::{info, warn};

use analyzer::Analyzer;
use parser::SessionParser;
use patterns::{load_all_patterns, AhoCorasickMatcher, PatternMatcher};
use reporter::Reporter;

#[derive(Parser)]
#[command(name = "claude-log-analyzer")]
#[command(
    version,
    about = "Analyze Claude session logs to identify AI agent usage"
)]
#[command(long_about = r#"
Claude Log Analyzer (cla) - Analyze Claude session logs to identify AI agent usage

This tool parses Claude Code session logs from $HOME/.claude/projects/ (by default)
and identifies which AI agents were used to build specific documents.

Examples:
  cla analyze                                          # Analyze all sessions
  cla analyze --target "STATUS_IMPLEMENTATION.md"     # Find specific file
  cla list                                            # List available sessions
  cla analyze --format json --output report.json     # Export to JSON
"#)]
struct Cli {
    /// Use verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Session directory (defaults to $HOME/.claude/projects)
    #[arg(short = 'd', long, env = "CLAUDE_SESSION_DIR", global = true)]
    session_dir: Option<PathBuf>,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze sessions to identify agent usage
    Analyze {
        /// Session file or directory to analyze (defaults to $HOME/.claude/projects)
        path: Option<String>,

        /// Target file to track (e.g., "STATUS_IMPLEMENTATION.md")
        #[arg(short, long)]
        target: Option<String>,

        /// Output format
        #[arg(short = 'f', long, default_value = "terminal")]
        format: OutputFormat,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show only sessions that modified files
        #[arg(long)]
        files_only: bool,
    },

    /// List available sessions
    List {
        /// Show detailed information about each session
        #[arg(long)]
        detailed: bool,

        /// Filter by project directory
        #[arg(short, long)]
        project: Option<String>,
    },

    /// Show summary statistics across all sessions
    Summary {
        /// Number of top agents to show
        #[arg(short, long, default_value = "10")]
        top: usize,
    },

    /// Generate timeline visualization (HTML)
    Timeline {
        /// Session file to visualize
        session: PathBuf,

        /// Output HTML file
        #[arg(short, long, default_value = "timeline.html")]
        output: PathBuf,
    },

    /// Watch for new sessions in real-time
    Watch {
        /// Directory to watch (defaults to $HOME/.claude/projects)
        path: Option<String>,

        /// Refresh interval in seconds
        #[arg(short, long, default_value = "5")]
        interval: u64,
    },

    /// Analyze tool usage patterns across sessions
    Tools {
        /// Session file or directory to analyze
        path: Option<String>,

        /// Output format
        #[arg(short = 'f', long, default_value = "terminal")]
        format: OutputFormat,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Filter by tool name
        #[arg(short, long)]
        tool: Option<String>,

        /// Filter by agent type
        #[arg(short, long)]
        agent: Option<String>,

        /// Show tool chains (sequences of tools used together)
        #[arg(long)]
        show_chains: bool,

        /// Show correlation matrix between agents and tools
        #[arg(long)]
        show_correlation: bool,

        /// Minimum usage count to display
        #[arg(long, default_value = "1")]
        min_usage: u32,

        /// Sort by: frequency, recent, alphabetical
        #[arg(long, default_value = "frequency")]
        sort_by: SortBy,

        /// Knowledge graph search query (e.g., "deploy OR publish OR release")
        #[arg(long)]
        #[cfg(feature = "terraphim")]
        kg_search: Option<String>,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Terminal,
    Json,
    Csv,
    Markdown,
    Html,
}

#[derive(Debug, Clone, ValueEnum)]
enum SortBy {
    Frequency,
    Recent,
    Alphabetical,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose { "debug" } else { "info" };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(false)
        .init();

    // Disable colors if requested or if not in terminal
    if cli.no_color || !atty::is(atty::Stream::Stdout) {
        colored::control::set_override(false);
    }

    match cli.command {
        Commands::Analyze {
            ref path,
            ref target,
            ref format,
            ref output,
            files_only,
        } => {
            let analyzer = create_analyzer(path.clone(), &cli)?;
            let mut analyses = analyzer.analyze(target.as_deref())?;

            if files_only {
                analyses.retain(|a| !a.file_to_agents.is_empty());
            }

            if analyses.is_empty() {
                println!("{}", "No matching sessions found".yellow());
                return Ok(());
            }

            let reporter = Reporter::new().with_colors(!cli.no_color);

            match &format {
                OutputFormat::Terminal => {
                    reporter.print_terminal(&analyses);
                }
                OutputFormat::Json => {
                    let json = reporter.to_json(&analyses)?;
                    write_output(&json, output.clone())?;
                }
                OutputFormat::Csv => {
                    let csv = reporter.to_csv(&analyses)?;
                    write_output(&csv, output.clone())?;
                }
                OutputFormat::Markdown => {
                    let markdown = reporter.to_markdown(&analyses)?;
                    write_output(&markdown, output.clone())?;
                }
                OutputFormat::Html => {
                    println!("{}", "HTML format not yet implemented".yellow());
                }
            }
        }

        Commands::List {
            detailed,
            ref project,
        } => {
            list_sessions(&cli, detailed, project.as_deref())?;
        }

        Commands::Summary { top } => {
            show_summary(&cli, top)?;
        }

        Commands::Timeline { session, output } => {
            generate_timeline(session, output)?;
        }

        Commands::Watch { ref path, interval } => {
            watch_sessions(path.as_deref(), &cli, interval)?;
        }

        Commands::Tools {
            ref path,
            ref format,
            ref output,
            ref tool,
            ref agent,
            show_chains,
            show_correlation,
            min_usage,
            ref sort_by,
            #[cfg(feature = "terraphim")]
            ref kg_search,
        } => {
            analyze_tools(
                path.as_deref(),
                &cli,
                format,
                output.clone(),
                tool.as_deref(),
                agent.as_deref(),
                show_chains,
                show_correlation,
                min_usage,
                sort_by,
                #[cfg(feature = "terraphim")]
                kg_search.as_deref(),
            )?;
        }
    }

    Ok(())
}

fn create_analyzer(path: Option<String>, cli: &Cli) -> Result<Analyzer> {
    if let Some(path_str) = path {
        let path = expand_home_dir(&path_str)?;
        Analyzer::new(path)
    } else if let Some(session_dir) = &cli.session_dir {
        Analyzer::new(session_dir)
    } else {
        Analyzer::from_default_location()
    }
}

fn expand_home_dir(path: &str) -> Result<PathBuf> {
    if path.starts_with("$HOME") || path.starts_with("~") {
        let home = home::home_dir().context("Could not find home directory")?;
        let relative = path
            .trim_start_matches("$HOME")
            .trim_start_matches("~")
            .trim_start_matches('/');
        Ok(home.join(relative))
    } else {
        Ok(PathBuf::from(path))
    }
}

fn write_output(content: &str, output: Option<PathBuf>) -> Result<()> {
    match output {
        Some(path) => {
            std::fs::write(&path, content)
                .with_context(|| format!("Failed to write to {}", path.display()))?;
            info!("Output written to: {}", path.display());
        }
        None => {
            print!("{}", content);
        }
    }
    Ok(())
}

fn list_sessions(cli: &Cli, detailed: bool, project_filter: Option<&str>) -> Result<()> {
    let analyzer = create_analyzer(None, cli)?;
    let analyses = analyzer.analyze(None)?;

    if analyses.is_empty() {
        println!("{}", "No sessions found".yellow());
        return Ok(());
    }

    println!("{}", "📋 Available Sessions:".bold().cyan());
    println!();

    for analysis in &analyses {
        // Apply project filter if specified
        if let Some(filter) = &project_filter {
            if !analysis.project_path.contains(filter) {
                continue;
            }
        }

        println!("{} {}", "Session:".bold(), analysis.session_id.yellow());
        println!(
            "  {} {}",
            "Project:".dimmed(),
            analysis.project_path.green()
        );
        println!("  {} {}ms", "Duration:".dimmed(), analysis.duration_ms);

        if detailed {
            println!("  {} {}", "Agents:".dimmed(), analysis.agents.len());
            println!("  {} {}", "Files:".dimmed(), analysis.file_to_agents.len());
            println!(
                "  {} {}",
                "Start:".dimmed(),
                analysis.start_time.strftime("%Y-%m-%d %H:%M:%S")
            );

            if !analysis.agents.is_empty() {
                let agent_types: Vec<_> = analysis
                    .agents
                    .iter()
                    .map(|a| &a.agent_type)
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                let agent_types_str: Vec<String> =
                    agent_types.iter().map(|s| s.to_string()).collect();
                println!(
                    "  {} {}",
                    "Agent types:".dimmed(),
                    agent_types_str.join(", ").cyan()
                );
            }
        }

        println!();
    }

    let filtered_count = if project_filter.is_some() {
        analyses
            .iter()
            .filter(|a| {
                project_filter
                    .as_ref()
                    .map_or(true, |f| a.project_path.contains(f))
            })
            .count()
    } else {
        analyses.len()
    };

    println!(
        "{} {} sessions",
        "Total:".bold(),
        filtered_count.to_string().yellow()
    );

    Ok(())
}

fn show_summary(cli: &Cli, top_count: usize) -> Result<()> {
    let analyzer = create_analyzer(None, cli)?;
    let summary = analyzer.get_summary_stats()?;

    println!("{}", "📈 Summary Statistics:".bold().cyan());
    println!();

    println!(
        "{} {}",
        "Total sessions:".bold(),
        summary.total_sessions.to_string().yellow()
    );
    println!(
        "{} {}",
        "Total agent invocations:".bold(),
        summary.total_agents.to_string().yellow()
    );
    println!(
        "{} {}",
        "Total files modified:".bold(),
        summary.total_files.to_string().yellow()
    );
    println!(
        "{} {}",
        "Unique agent types:".bold(),
        summary.unique_agent_types.to_string().yellow()
    );

    println!("\n{}", "🏆 Most Active Agents:".bold());
    for (i, (agent, count)) in summary
        .most_active_agents
        .iter()
        .take(top_count)
        .enumerate()
    {
        let reporter = Reporter::new();
        println!(
            "  {}. {} {} ({}x)",
            (i + 1).to_string().dimmed(),
            reporter.format_agent_icon(agent),
            agent.cyan(),
            count.to_string().yellow()
        );
    }

    Ok(())
}

fn generate_timeline(session_path: PathBuf, output_path: PathBuf) -> Result<()> {
    info!(
        "Generating timeline for session: {}",
        session_path.display()
    );

    let analyzer = Analyzer::new(&session_path)?;
    let analyses = analyzer.analyze(None)?;

    if analyses.is_empty() {
        return Err(anyhow::anyhow!(
            "No valid sessions found in {}",
            session_path.display()
        ));
    }

    let analysis = &analyses[0];

    // Generate simple HTML timeline
    let html = generate_timeline_html(analysis)?;

    std::fs::write(&output_path, html)
        .with_context(|| format!("Failed to write timeline to {}", output_path.display()))?;

    println!(
        "{} Timeline generated: {}",
        "✅".green(),
        output_path.display().to_string().yellow()
    );

    Ok(())
}

fn generate_timeline_html(analysis: &SessionAnalysis) -> Result<String> {
    let mut html = String::new();

    html.push_str(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Claude Session Timeline</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .timeline { border-left: 3px solid #ccc; padding-left: 20px; margin: 20px 0; }
        .event { margin-bottom: 20px; position: relative; }
        .event::before {
            content: '';
            position: absolute;
            left: -26px;
            top: 5px;
            width: 12px;
            height: 12px;
            border-radius: 50%;
            background: #007acc;
        }
        .time { color: #666; font-size: 0.9em; }
        .agent { font-weight: bold; color: #007acc; }
        .description { margin-top: 5px; color: #333; }
    </style>
</head>
<body>
    <h1>Claude Session Timeline</h1>
    <p><strong>Session:</strong> "#,
    );

    html.push_str(&analysis.session_id);
    html.push_str("</p>\n    <p><strong>Project:</strong> ");
    html.push_str(&analysis.project_path);
    html.push_str("</p>\n\n    <div class=\"timeline\">\n");

    for agent in &analysis.agents {
        html.push_str("        <div class=\"event\">\n");
        html.push_str(&format!(
            "            <div class=\"time\">{}</div>\n",
            agent.timestamp.strftime("%H:%M:%S")
        ));
        html.push_str(&format!(
            "            <div class=\"agent\">{}</div>\n",
            agent.agent_type
        ));
        html.push_str(&format!(
            "            <div class=\"description\">{}</div>\n",
            agent.task_description
        ));
        html.push_str("        </div>\n");
    }

    html.push_str("    </div>\n</body>\n</html>");

    Ok(html)
}

fn watch_sessions(path: Option<&str>, cli: &Cli, interval: u64) -> Result<()> {
    let watch_path = if let Some(p) = path {
        expand_home_dir(p)?
    } else if let Some(session_dir) = &cli.session_dir {
        session_dir.clone()
    } else {
        let home = home::home_dir().context("Could not find home directory")?;
        home.join(".claude").join("projects")
    };

    println!(
        "{} Watching for new sessions in: {}",
        "👀".cyan(),
        watch_path.display().to_string().green()
    );
    println!("Press Ctrl+C to stop...\n");

    let mut last_count = 0;

    loop {
        match Analyzer::new(&watch_path) {
            Ok(analyzer) => {
                match analyzer.analyze(None) {
                    Ok(analyses) => {
                        let current_count = analyses.len();
                        if current_count > last_count {
                            let new_sessions = current_count - last_count;
                            println!(
                                "{} {} new session(s) detected",
                                "🆕".green(),
                                new_sessions.to_string().yellow()
                            );

                            // Show details of new sessions
                            for analysis in analyses.iter().skip(last_count) {
                                println!(
                                    "  {} {} - {} agents, {} files",
                                    "Session:".dimmed(),
                                    analysis.session_id.yellow(),
                                    analysis.agents.len(),
                                    analysis.file_to_agents.len()
                                );
                            }
                        }
                        last_count = current_count;
                    }
                    Err(e) => {
                        warn!("Failed to analyze sessions: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to create analyzer: {}", e);
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(interval));
    }
}

/// Convert local ToolCategory to library ToolCategory
#[cfg(feature = "terraphim")]
fn convert_tool_category(cat: &models::ToolCategory) -> terraphim_session_analyzer::ToolCategory {
    use models::ToolCategory as Local;
    use terraphim_session_analyzer::ToolCategory as Lib;
    match cat {
        Local::PackageManager => Lib::PackageManager,
        Local::BuildTool => Lib::BuildTool,
        Local::Testing => Lib::Testing,
        Local::Linting => Lib::Linting,
        Local::Git => Lib::Git,
        Local::CloudDeploy => Lib::CloudDeploy,
        Local::Database => Lib::Database,
        Local::Other(s) => Lib::Other(s.clone()),
    }
}

/// Convert local ToolInvocation to library ToolInvocation for KG module
#[cfg(feature = "terraphim")]
fn convert_to_lib_invocation(
    inv: &models::ToolInvocation,
) -> terraphim_session_analyzer::ToolInvocation {
    terraphim_session_analyzer::ToolInvocation {
        timestamp: inv.timestamp,
        tool_name: inv.tool_name.clone(),
        tool_category: convert_tool_category(&inv.tool_category),
        command_line: inv.command_line.clone(),
        arguments: inv.arguments.clone(),
        flags: inv.flags.clone(),
        exit_code: inv.exit_code,
        agent_context: inv.agent_context.clone(),
        session_id: inv.session_id.clone(),
        message_id: inv.message_id.clone(),
    }
}

/// Calculate tool chains from invocations
fn calculate_tool_chains(invocations: &[models::ToolInvocation]) -> Vec<models::ToolChain> {
    use std::collections::HashMap;

    // Group invocations by session
    let mut session_tools: HashMap<String, Vec<&models::ToolInvocation>> = HashMap::new();
    for inv in invocations {
        session_tools
            .entry(inv.session_id.clone())
            .or_default()
            .push(inv);
    }

    // Find common sequences (2-tool chains)
    let mut chain_freq: HashMap<(String, String), ChainData> = HashMap::new();

    for tools in session_tools.values() {
        let mut sorted_tools = tools.clone();
        sorted_tools.sort_by_key(|t| t.timestamp);

        for window in sorted_tools.windows(2) {
            let key = (window[0].tool_name.clone(), window[1].tool_name.clone());

            let time_diff = window[1].timestamp - window[0].timestamp;
            let time_diff_ms = time_diff
                .total(jiff::Unit::Millisecond)
                .unwrap_or(0.0)
                .abs();

            // Only consider tools within 5 minutes of each other
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            if time_diff_ms <= 300_000.0 {
                let entry = chain_freq.entry(key).or_insert_with(|| ChainData {
                    frequency: 0,
                    total_time_ms: 0,
                    success_count: 0,
                    total_count: 0,
                    agents: std::collections::HashSet::new(),
                });

                entry.frequency += 1;
                entry.total_time_ms += time_diff_ms as u64;
                entry.total_count += 1;

                if window[1].exit_code == Some(0) {
                    entry.success_count += 1;
                }

                if let Some(ref agent) = window[1].agent_context {
                    entry.agents.insert(agent.clone());
                }
            }
        }
    }

    // Convert to ToolChain structs, filter by frequency >= 2
    let mut chains: Vec<models::ToolChain> = chain_freq
        .into_iter()
        .filter(|(_, data)| data.frequency >= 2)
        .map(|((tool1, tool2), data)| {
            #[allow(clippy::cast_precision_loss)]
            let avg_time = data.total_time_ms / u64::from(data.total_count.max(1));
            #[allow(clippy::cast_precision_loss)]
            let success_rate = if data.total_count > 0 {
                data.success_count as f32 / data.total_count as f32
            } else {
                0.0
            };

            models::ToolChain {
                tools: vec![tool1, tool2],
                frequency: data.frequency,
                average_time_between_ms: avg_time,
                typical_agent: data.agents.iter().next().cloned(),
                success_rate,
            }
        })
        .collect();

    // Sort by frequency
    chains.sort_by(|a, b| b.frequency.cmp(&a.frequency));
    chains.truncate(10); // Top 10 chains

    chains
}

struct ChainData {
    frequency: u32,
    total_time_ms: u64,
    success_count: u32,
    total_count: u32,
    agents: std::collections::HashSet<String>,
}

/// Calculate agent-tool correlations
/// TODO: Remove in Phase 2 Part 2 - now handled by Analyzer::calculate_agent_tool_correlations
#[allow(dead_code)]
fn calculate_agent_tool_correlations(
    invocations: &[models::ToolInvocation],
) -> Vec<models::AgentToolCorrelation> {
    use std::collections::HashMap;

    // Group by (agent, tool)
    let mut correlation_data: HashMap<(String, String), CorrelationData> = HashMap::new();

    for inv in invocations {
        if let Some(ref agent) = inv.agent_context {
            let key = (agent.clone(), inv.tool_name.clone());
            let entry = correlation_data
                .entry(key)
                .or_insert_with(|| CorrelationData {
                    usage_count: 0,
                    success_count: 0,
                    sessions: std::collections::HashSet::new(),
                });

            entry.usage_count += 1;
            entry.sessions.insert(inv.session_id.clone());

            if inv.exit_code == Some(0) {
                entry.success_count += 1;
            }
        }
    }

    // Convert to correlation structs
    let mut correlations: Vec<models::AgentToolCorrelation> = correlation_data
        .into_iter()
        .map(|((agent, tool), data)| {
            #[allow(clippy::cast_precision_loss)]
            let success_rate = if data.usage_count > 0 {
                data.success_count as f32 / data.usage_count as f32
            } else {
                0.0
            };

            #[allow(clippy::cast_precision_loss)]
            let avg_per_session = if !data.sessions.is_empty() {
                data.usage_count as f32 / data.sessions.len() as f32
            } else {
                0.0
            };

            models::AgentToolCorrelation {
                agent_type: agent,
                tool_name: tool,
                usage_count: data.usage_count,
                success_rate,
                average_invocations_per_session: avg_per_session,
            }
        })
        .collect();

    // Sort by usage count
    correlations.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
    correlations.truncate(20); // Top 20 correlations

    correlations
}

#[allow(dead_code)]
struct CorrelationData {
    usage_count: u32,
    success_count: u32,
    sessions: std::collections::HashSet<String>,
}

#[allow(clippy::too_many_arguments)]
fn analyze_tools(
    path: Option<&str>,
    cli: &Cli,
    format: &OutputFormat,
    output: Option<PathBuf>,
    tool_filter: Option<&str>,
    agent_filter: Option<&str>,
    show_chains: bool,
    show_correlation: bool,
    min_usage: u32,
    sort_by: &SortBy,
    #[cfg(feature = "terraphim")] kg_search_query: Option<&str>,
) -> Result<()> {
    let analyzer = create_analyzer(path.map(String::from), cli)?;
    let analyses = analyzer.analyze(None)?;

    if analyses.is_empty() {
        println!("{}", "No sessions found".yellow());
        return Ok(());
    }

    // Show progress for large session sets
    if analyses.len() > 10 {
        info!("Analyzing tool usage across {} sessions...", analyses.len());
    }

    // Initialize pattern matcher with built-in and user patterns
    let mut matcher = AhoCorasickMatcher::new();
    let patterns = load_all_patterns().context("Failed to load patterns")?;
    matcher
        .initialize(&patterns)
        .context("Failed to initialize pattern matcher")?;

    // Extract tool invocations from all sessions
    let mut all_invocations = Vec::new();

    for (i, analysis) in analyses.iter().enumerate() {
        if analyses.len() > 10 && i % 10 == 0 {
            info!("Processing session {}/{}", i + 1, analyses.len());
        }

        // Find the session file
        let session_path = find_session_path(&analysis.session_id, cli)?;

        // Parse the session
        if let Ok(parser) = SessionParser::from_file(&session_path) {
            // Extract tool invocations from Bash commands
            if let Ok(mut invocations) = extract_tool_invocations_from_session(&parser, &matcher) {
                // Link tool invocations to agents from the analysis
                for invocation in &mut invocations {
                    // Find the agent that was active at this timestamp
                    let active_agent = analysis
                        .agents
                        .iter()
                        .filter(|a| a.timestamp <= invocation.timestamp)
                        .max_by_key(|a| a.timestamp);

                    if let Some(agent) = active_agent {
                        invocation.agent_context = Some(agent.agent_type.clone());
                    }
                }

                all_invocations.extend(invocations);
            }
        }
    }

    if all_invocations.is_empty() {
        println!("{}", "No tool invocations found".yellow());
        return Ok(());
    }

    // Handle KG search if provided
    #[cfg(feature = "terraphim")]
    if let Some(query_str) = kg_search_query {
        use terraphim_session_analyzer::kg::{
            KnowledgeGraphBuilder, KnowledgeGraphSearch, QueryParser,
        };

        // Parse the query
        let query_ast = QueryParser::parse(query_str)
            .with_context(|| format!("Failed to parse query: {query_str}"))?;

        // Convert to library types for KG module
        let lib_invocations: Vec<terraphim_session_analyzer::ToolInvocation> = all_invocations
            .iter()
            .map(convert_to_lib_invocation)
            .collect();

        // Build knowledge graph from tool invocations
        let builder = KnowledgeGraphBuilder::from_tool_invocations(&lib_invocations);
        let kg_search = KnowledgeGraphSearch::new(builder);

        // Search through invocations and collect results
        let mut matching_invocations = Vec::new();

        for invocation in &all_invocations {
            match kg_search.search(&invocation.command_line, &query_ast) {
                Ok(results) if !results.is_empty() => {
                    // Calculate total relevance for this invocation
                    let total_relevance: f32 = results.iter().map(|r| r.relevance_score).sum();

                    // Collect all matched concepts
                    let mut matched_concepts: Vec<String> = results
                        .iter()
                        .flat_map(|r| r.concepts_matched.clone())
                        .collect();
                    matched_concepts.sort();
                    matched_concepts.dedup();

                    matching_invocations.push((invocation, total_relevance, matched_concepts));
                }
                _ => {}
            }
        }

        // Sort by relevance score
        matching_invocations
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Display results
        println!(
            "\n{} Knowledge Graph Search Results for: {}",
            "🔍".cyan(),
            query_str.yellow().bold()
        );
        println!("{}", "=".repeat(80).dimmed());
        println!(
            "\n{} {} matching commands found\n",
            "Found:".bold(),
            matching_invocations.len().to_string().yellow()
        );

        for (i, (invocation, relevance, matched_concepts)) in
            matching_invocations.iter().enumerate().take(50)
        {
            // Show top 50 results
            println!(
                "{}. {} {}",
                (i + 1).to_string().dimmed(),
                "Command:".bold(),
                invocation.command_line.green()
            );
            println!("   {} {}", "Tool:".dimmed(), invocation.tool_name.cyan());
            println!(
                "   {} {}",
                "Session:".dimmed(),
                invocation.session_id.dimmed()
            );
            if let Some(ref agent) = invocation.agent_context {
                let agent_str = agent.as_str();
                println!("   {} {}", "Agent:".dimmed(), agent_str.yellow());
            }
            println!("   {} {:.2}", "Relevance:".dimmed(), relevance);
            println!(
                "   {} {}",
                "Matched:".dimmed(),
                matched_concepts.join(", ").cyan()
            );
            println!();
        }

        if matching_invocations.len() > 50 {
            println!(
                "{} Showing top 50 of {} results",
                "Note:".yellow(),
                matching_invocations.len()
            );
        }

        return Ok(());
    }

    // Calculate comprehensive statistics using the new Analyzer methods
    let tool_stats = analyzer.calculate_tool_statistics(&all_invocations);
    let category_breakdown = analyzer.calculate_category_breakdown(&all_invocations);

    // Apply filters to the IndexMap
    let filtered_stats: Vec<(String, models::ToolStatistics)> = tool_stats
        .into_iter()
        .filter(|(name, stats)| {
            // Tool name filter
            if let Some(tool_filter_str) = tool_filter {
                if !name
                    .to_lowercase()
                    .contains(&tool_filter_str.to_lowercase())
                {
                    return false;
                }
            }

            // Agent filter
            if let Some(agent_filter_str) = agent_filter {
                if !stats
                    .agents_using
                    .iter()
                    .any(|a| a.to_lowercase().contains(&agent_filter_str.to_lowercase()))
                {
                    return false;
                }
            }

            // Minimum usage filter
            if stats.total_invocations < min_usage {
                return false;
            }

            true
        })
        .collect();

    if filtered_stats.is_empty() {
        println!("{}", "No tools match the specified criteria".yellow());
        return Ok(());
    }

    // Sort the results
    let sorted_stats: Vec<(String, models::ToolStatistics)> = {
        let mut stats = filtered_stats;
        match sort_by {
            SortBy::Frequency => {
                stats.sort_by(|a, b| b.1.total_invocations.cmp(&a.1.total_invocations))
            }
            SortBy::Alphabetical => stats.sort_by(|a, b| a.0.cmp(&b.0)),
            SortBy::Recent => stats.sort_by(|a, b| b.1.last_seen.cmp(&a.1.last_seen)),
        }
        stats
    };

    // Convert sorted_stats back to IndexMap
    let mut tool_statistics = indexmap::IndexMap::new();
    for (name, stat) in sorted_stats {
        tool_statistics.insert(name, stat);
    }

    // Calculate correlations if requested
    let correlations = if show_correlation {
        analyzer.calculate_agent_tool_correlations(&all_invocations)
    } else {
        Vec::new()
    };

    // Calculate tool chains if requested
    let tool_chains = if show_chains {
        calculate_tool_chains(&all_invocations)
    } else {
        Vec::new()
    };

    // Build ToolAnalysis struct
    #[allow(clippy::cast_possible_truncation)]
    let tool_analysis = models::ToolAnalysis {
        session_id: "aggregated".to_string(), // This is across all sessions
        total_tool_invocations: all_invocations.len() as u32,
        tool_statistics,
        agent_tool_correlations: correlations,
        tool_chains,
        category_breakdown,
    };

    // Create reporter
    let reporter = Reporter::new().with_colors(!cli.no_color);

    // Display results based on format
    match format {
        OutputFormat::Terminal => {
            reporter.print_tool_analysis_detailed(&tool_analysis, show_correlation)?;
        }
        OutputFormat::Json => {
            let json = reporter.tool_analysis_to_json(&tool_analysis)?;
            write_output(&json, output)?;
        }
        OutputFormat::Csv => {
            let csv = reporter.tool_analysis_to_csv(&tool_analysis)?;
            write_output(&csv, output)?;
        }
        OutputFormat::Markdown => {
            let md = reporter.tool_analysis_to_markdown(&tool_analysis)?;
            write_output(&md, output)?;
        }
        OutputFormat::Html => {
            println!(
                "{}",
                "HTML format not yet implemented for tool analysis".yellow()
            );
        }
    }

    Ok(())
}

fn find_session_path(session_id: &str, cli: &Cli) -> Result<PathBuf> {
    let base_dir = if let Some(ref session_dir) = cli.session_dir {
        session_dir.clone()
    } else {
        let home = home::home_dir().context("Could not find home directory")?;
        home.join(".claude").join("projects")
    };

    // Look for the session file in all subdirectories
    for entry in walkdir::WalkDir::new(&base_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".jsonl") && name.contains(session_id) {
                    return Ok(entry.path().to_path_buf());
                }
            }
        }
    }

    Err(anyhow::anyhow!("Session file not found: {session_id}"))
}

fn extract_tool_invocations_from_session(
    parser: &SessionParser,
    matcher: &dyn PatternMatcher,
) -> Result<Vec<models::ToolInvocation>> {
    use models::{ContentBlock, Message, ToolCategory, ToolInvocation};

    let mut invocations = Vec::new();

    for entry in parser.entries() {
        if let Message::Assistant { content, .. } = &entry.message {
            for block in content {
                if let ContentBlock::ToolUse { name, input, .. } = block {
                    if name == "Bash" {
                        if let Some(command) = input.get("command").and_then(|v| v.as_str()) {
                            let matches = matcher.find_matches(command);

                            for tool_match in matches {
                                // Parse the command context
                                if let Some((full_cmd, args, flags)) =
                                    tool_analyzer::parse_command_context(command, tool_match.start)
                                {
                                    if let Ok(timestamp) = models::parse_timestamp(&entry.timestamp)
                                    {
                                        // Map category string to ToolCategory enum
                                        let category = match tool_match.category.as_str() {
                                            "package-manager" => ToolCategory::PackageManager,
                                            "version-control" => ToolCategory::Git,
                                            "testing" => ToolCategory::Testing,
                                            "linting" => ToolCategory::Linting,
                                            "cloudflare" => ToolCategory::CloudDeploy,
                                            _ => ToolCategory::Other(tool_match.category.clone()),
                                        };

                                        invocations.push(ToolInvocation {
                                            timestamp,
                                            tool_name: tool_match.tool_name.clone(),
                                            tool_category: category,
                                            command_line: full_cmd,
                                            arguments: args,
                                            flags,
                                            exit_code: None,
                                            agent_context: None,
                                            session_id: entry.session_id.clone(),
                                            message_id: entry.uuid.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(invocations)
}

// Check if running in terminal
#[cfg(unix)]
mod atty {
    pub enum Stream {
        Stdout,
    }

    pub fn is(stream: Stream) -> bool {
        match stream {
            Stream::Stdout => unsafe { libc::isatty(libc::STDOUT_FILENO) != 0 },
        }
    }
}

#[cfg(not(unix))]
mod atty {
    pub enum Stream {
        Stdout,
    }

    pub fn is(_stream: Stream) -> bool {
        true // Assume terminal on non-Unix systems
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_home_dir() {
        let result = expand_home_dir("~/.claude/projects");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn test_expand_home_dir_absolute() {
        let result = expand_home_dir("/absolute/path");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert_eq!(path, PathBuf::from("/absolute/path"));
    }
}
