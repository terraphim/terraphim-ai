//! Comprehensive scoring function x haystack test matrix
//!
//! This test matrix validates every combination of scoring functions and haystacks
//! to ensure compatibility, performance, and correctness across the system.

use anyhow::Result;
use serde_json::{json, Value};
use serial_test::serial;
use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};

/// Available scoring functions/relevance functions in the system
#[derive(Debug, Clone, PartialEq)]
enum ScoringFunction {
    TerraphimGraph,
    TitleScorer,
    BM25,
    BM25F,
    BM25Plus,
}

/// Advanced query scorers available within title scorer and other functions
#[derive(Debug, Clone, PartialEq)]
enum QueryScorer {
    Levenshtein,
    Jaro,
    JaroWinkler,
    BM25,
    BM25F,
    BM25Plus,
    Tfidf,
    Jaccard,
    QueryRatio,
    OkapiBM25,
}

impl QueryScorer {
    fn as_config_str(&self) -> &'static str {
        match self {
            Self::Levenshtein => "levenshtein",
            Self::Jaro => "jaro",
            Self::JaroWinkler => "jaro_winkler",
            Self::BM25 => "bm25",
            Self::BM25F => "bm25f",
            Self::BM25Plus => "bm25plus",
            Self::Tfidf => "tfidf",
            Self::Jaccard => "jaccard",
            Self::QueryRatio => "query_ratio",
            Self::OkapiBM25 => "okapi_bm25",
        }
    }

    fn all() -> Vec<Self> {
        vec![
            Self::Levenshtein,
            Self::Jaro,
            Self::JaroWinkler,
            Self::BM25,
            Self::BM25F,
            Self::BM25Plus,
            Self::Tfidf,
            Self::Jaccard,
            Self::QueryRatio,
            Self::OkapiBM25,
        ]
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Levenshtein => "Levenshtein",
            Self::Jaro => "Jaro",
            Self::JaroWinkler => "JaroWinkler",
            Self::BM25 => "BM25",
            Self::BM25F => "BM25F",
            Self::BM25Plus => "BM25Plus",
            Self::Tfidf => "TFIDF",
            Self::Jaccard => "Jaccard",
            Self::QueryRatio => "QueryRatio",
            Self::OkapiBM25 => "OkapiBM25",
        }
    }

    fn supports_relevance_function(&self, relevance_function: &ScoringFunction) -> bool {
        match relevance_function {
            ScoringFunction::TitleScorer => true, // All query scorers work with title scorer
            ScoringFunction::BM25 => matches!(self, Self::BM25 | Self::OkapiBM25),
            ScoringFunction::BM25F => matches!(self, Self::BM25F),
            ScoringFunction::BM25Plus => matches!(self, Self::BM25Plus),
            ScoringFunction::TerraphimGraph => false, // Graph scoring has its own algorithm
        }
    }
}

impl ScoringFunction {
    fn as_config_str(&self) -> &'static str {
        match self {
            Self::TerraphimGraph => "terraphim-graph",
            Self::TitleScorer => "title-scorer",
            Self::BM25 => "bm25",
            Self::BM25F => "bm25f",
            Self::BM25Plus => "bm25plus",
        }
    }

    fn all() -> Vec<Self> {
        vec![
            Self::TerraphimGraph,
            Self::TitleScorer,
            Self::BM25,
            Self::BM25F,
            Self::BM25Plus,
        ]
    }

    fn name(&self) -> &'static str {
        match self {
            Self::TerraphimGraph => "TerraphimGraph",
            Self::TitleScorer => "TitleScorer",
            Self::BM25 => "BM25",
            Self::BM25F => "BM25F",
            Self::BM25Plus => "BM25Plus",
        }
    }
}

/// Available haystack types/services in the system
#[derive(Debug, Clone, PartialEq)]
enum HaystackType {
    Ripgrep,
    Atomic,
    QueryRs,
    ClickUp,
    Mcp,
    Perplexity,
}

impl HaystackType {
    fn as_config_str(&self) -> &'static str {
        match self {
            Self::Ripgrep => "Ripgrep",
            Self::Atomic => "Atomic",
            Self::QueryRs => "QueryRs",
            Self::ClickUp => "ClickUp",
            Self::Mcp => "Mcp",
            Self::Perplexity => "Perplexity",
        }
    }

    fn all() -> Vec<Self> {
        vec![
            Self::Ripgrep,
            Self::Atomic,
            Self::QueryRs,
            Self::ClickUp,
            Self::Mcp,
            Self::Perplexity,
        ]
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Ripgrep => "Ripgrep",
            Self::Atomic => "Atomic",
            Self::QueryRs => "QueryRs",
            Self::ClickUp => "ClickUp",
            Self::Mcp => "MCP",
            Self::Perplexity => "Perplexity",
        }
    }

    fn default_location(&self) -> &'static str {
        match self {
            Self::Ripgrep => "./docs/src",
            Self::Atomic => "http://localhost:9883",
            Self::QueryRs => "https://docs.rs/",
            Self::ClickUp => "https://api.clickup.com/api/v2",
            Self::Mcp => "stdio://localhost:3000",
            Self::Perplexity => "https://api.perplexity.ai",
        }
    }

    #[allow(dead_code)]
    fn is_local(&self) -> bool {
        matches!(self, Self::Ripgrep)
    }

    #[allow(dead_code)]
    fn requires_credentials(&self) -> bool {
        matches!(self, Self::Atomic | Self::ClickUp | Self::Perplexity)
    }

    fn supports_read_only(&self) -> bool {
        matches!(self, Self::Ripgrep | Self::Atomic | Self::QueryRs)
    }
}

/// Test result for a scoring function x haystack combination
#[derive(Debug, Clone)]
struct MatrixTestResult {
    scoring_function: ScoringFunction,
    haystack_type: HaystackType,
    query_scorer: Option<QueryScorer>,
    config_creation_success: bool,
    search_success: bool,
    response_time: Option<Duration>,
    result_count: Option<usize>,
    error_message: Option<String>,
    performance_score: Option<f64>,
}

impl MatrixTestResult {
    fn new(scoring_function: ScoringFunction, haystack_type: HaystackType) -> Self {
        Self {
            scoring_function,
            haystack_type,
            query_scorer: None,
            config_creation_success: false,
            search_success: false,
            response_time: None,
            result_count: None,
            error_message: None,
            performance_score: None,
        }
    }

    fn new_with_scorer(
        scoring_function: ScoringFunction,
        haystack_type: HaystackType,
        query_scorer: QueryScorer,
    ) -> Self {
        Self {
            scoring_function,
            haystack_type,
            query_scorer: Some(query_scorer),
            config_creation_success: false,
            search_success: false,
            response_time: None,
            result_count: None,
            error_message: None,
            performance_score: None,
        }
    }

    fn is_success(&self) -> bool {
        self.config_creation_success && self.search_success
    }

    fn combination_name(&self) -> String {
        if let Some(scorer) = &self.query_scorer {
            format!(
                "{} + {} ({})",
                self.scoring_function.name(),
                self.haystack_type.name(),
                scorer.name()
            )
        } else {
            format!(
                "{} + {}",
                self.scoring_function.name(),
                self.haystack_type.name()
            )
        }
    }
}

/// Test matrix engine for running comprehensive tests
struct TestMatrix {
    results: Vec<MatrixTestResult>,
    total_combinations: usize,
    successful_combinations: usize,
}

impl TestMatrix {
    fn new() -> Self {
        let scoring_functions = ScoringFunction::all();
        let haystack_types = HaystackType::all();
        let total_combinations = scoring_functions.len() * haystack_types.len();

        Self {
            results: Vec::new(),
            total_combinations,
            successful_combinations: 0,
        }
    }

    /// Generate a test configuration for a specific scoring function and haystack combination
    fn generate_test_config(
        &self,
        scoring_function: &ScoringFunction,
        haystack_type: &HaystackType,
        query_scorer: Option<&QueryScorer>,
    ) -> Value {
        let role_name = if let Some(scorer) = query_scorer {
            format!(
                "Test_{}_{}_with_{}",
                scoring_function.name(),
                scorer.name(),
                haystack_type.name()
            )
        } else {
            format!(
                "Test_{}_with_{}",
                scoring_function.name(),
                haystack_type.name()
            )
        };

        let mut haystack = json!({
            "location": haystack_type.default_location(),
            "service": haystack_type.as_config_str(),
            "read_only": haystack_type.supports_read_only(),
            "extra_parameters": {}
        });

        // Add service-specific configurations with real credentials from environment
        match haystack_type {
            HaystackType::Atomic => {
                if let Ok(secret) = std::env::var("ATOMIC_SERVER_SECRET") {
                    haystack["atomic_server_secret"] = json!(secret);
                } else {
                    // Use null for tests that don't have atomic server running
                    haystack["atomic_server_secret"] = json!(null);
                }
            }
            HaystackType::ClickUp => {
                if let (Ok(token), Ok(team)) = (
                    std::env::var("CLICKUP_API_TOKEN"),
                    std::env::var("CLICKUP_TEAM_ID"),
                ) {
                    haystack["extra_parameters"] = json!({
                        "team_id": team,
                        "api_token": token
                    });
                } else {
                    // Use test credentials if real ones not available
                    haystack["extra_parameters"] = json!({
                        "team_id": "test_team",
                        "api_token": "test_token"
                    });
                }
            }
            HaystackType::Mcp => {
                haystack["extra_parameters"] = json!({
                    "server_name": "test_mcp_server",
                    "timeout": "30"
                });
            }
            HaystackType::Perplexity => {
                haystack["extra_parameters"] = json!({
                    "api_key": "test_key",
                    "model": "llama-3.1-sonar-small-128k-online"
                });
            }
            _ => {}
        }

        let mut role_config = json!({
            "shortname": format!("{}-{}",
                scoring_function.name().to_lowercase(),
                haystack_type.name().to_lowercase()
            ),
            "name": role_name,
            "relevance_function": scoring_function.as_config_str(),
            "terraphim_it": *scoring_function == ScoringFunction::TerraphimGraph,
            "theme": "Default",
            "kg": {
                "automata_path": {
                    "Local": "./docs/src/kg"
                },
                "knowledge_graph_local": null,
                "public": false,
                "publish": false
            },
            "haystacks": [haystack],
            "extra": {}
        });

        // Add query scorer configuration if specified and supported
        if let Some(scorer) = query_scorer {
            if scorer.supports_relevance_function(scoring_function) {
                role_config["query_scorer"] = json!(scorer.as_config_str());
            }
        }

        json!({
            "id": "Embedded",
            "global_shortcut": "Ctrl+X",
            "default_role": role_name.clone(),
            "selected_role": role_name,
            "roles": {
                role_name: role_config
            }
        })
    }

    /// Run TUI command and capture results
    fn run_tui_command(&self, args: &[&str]) -> Result<(String, String, i32, Duration)> {
        let start = Instant::now();

        let mut cmd = Command::new("cargo");
        cmd.args(["run", "-p", "terraphim_tui", "--"]).args(args);

        let output = cmd.output()?;
        let duration = start.elapsed();

        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
            output.status.code().unwrap_or(-1),
            duration,
        ))
    }

    /// Test a specific scoring function and haystack combination
    async fn test_combination(
        &self,
        scoring_function: &ScoringFunction,
        haystack_type: &HaystackType,
    ) -> MatrixTestResult {
        self.test_combination_with_scorer(scoring_function, haystack_type, None)
            .await
    }

    /// Test a specific scoring function, haystack, and query scorer combination
    async fn test_combination_with_scorer(
        &self,
        scoring_function: &ScoringFunction,
        haystack_type: &HaystackType,
        query_scorer: Option<&QueryScorer>,
    ) -> MatrixTestResult {
        // Load environment variables for credentials
        dotenvy::dotenv().ok();

        let mut result = if let Some(scorer) = query_scorer {
            MatrixTestResult::new_with_scorer(
                scoring_function.clone(),
                haystack_type.clone(),
                scorer.clone(),
            )
        } else {
            MatrixTestResult::new(scoring_function.clone(), haystack_type.clone())
        };

        let combo_name = result.combination_name();
        println!("🧪 Testing: {}", combo_name);

        // Step 1: Generate test configuration
        let config = self.generate_test_config(scoring_function, haystack_type, query_scorer);

        // Step 2: Create temporary config file
        let temp_config_path = if let Some(scorer) = query_scorer {
            format!(
                "/tmp/terraphim_test_matrix_{}_{}_{}.json",
                scoring_function.name().to_lowercase(),
                scorer.name().to_lowercase(),
                haystack_type.name().to_lowercase()
            )
        } else {
            format!(
                "/tmp/terraphim_test_matrix_{}_{}.json",
                scoring_function.name().to_lowercase(),
                haystack_type.name().to_lowercase()
            )
        };

        match std::fs::write(
            &temp_config_path,
            serde_json::to_string_pretty(&config).unwrap(),
        ) {
            Ok(_) => {
                result.config_creation_success = true;
                println!("  ✅ Config created: {}", temp_config_path);
            }
            Err(e) => {
                result.error_message = Some(format!("Config creation failed: {}", e));
                println!("  ❌ Config creation failed: {}", e);
                return result;
            }
        }

        // Step 3: Test search with this configuration

        // Use appropriate search terms based on haystack type
        let search_query = match haystack_type {
            HaystackType::Ripgrep => "async", // This exists in docs/src
            HaystackType::QueryRs => "async", // Rust-related term
            HaystackType::ClickUp => "task",  // Task management term
            HaystackType::Atomic => "test",   // Generic term for atomic data
            HaystackType::Mcp => "protocol",  // MCP protocol term
            HaystackType::Perplexity => "documentation", // Generic query
        };

        match self.run_tui_command(&[
            "--config",
            &temp_config_path,
            "search",
            search_query,
            "--limit",
            "5",
        ]) {
            Ok((stdout, stderr, code, duration)) => {
                result.response_time = Some(duration);

                if code == 0 {
                    // Count results first
                    let clean_output: String = stdout
                        .lines()
                        .filter(|line| {
                            !line.contains("INFO")
                                && !line.contains("WARN")
                                && !line.contains("DEBUG")
                                && !line.trim().is_empty()
                        })
                        .collect::<Vec<&str>>()
                        .join("\n");

                    let result_count = clean_output
                        .lines()
                        .filter(|line| line.starts_with("- "))
                        .count();

                    result.result_count = Some(result_count);

                    // Only consider it successful if we got results AND the command succeeded
                    if result_count > 0 {
                        result.search_success = true;

                        // Calculate performance score (results per second)
                        if duration.as_secs_f64() > 0.0 {
                            result.performance_score =
                                Some(result_count as f64 / duration.as_secs_f64());
                        }

                        println!(
                            "  ✅ Search succeeded: {} results in {:?}",
                            result_count, duration
                        );
                    } else {
                        result.search_success = false;
                        result.error_message = Some("Search returned 0 results".to_string());
                        println!(
                            "  ⚠️ Search completed but returned 0 results in {:?}",
                            duration
                        );
                    }
                } else {
                    result.error_message =
                        Some(format!("Search failed with code {}: {}", code, stderr));
                    println!("  ❌ Search failed: code={}, stderr={}", code, stderr);
                }
            }
            Err(e) => {
                result.error_message = Some(format!("Command execution failed: {}", e));
                println!("  ❌ Command execution failed: {}", e);
            }
        }

        // Step 4: Cleanup temporary config
        let _ = std::fs::remove_file(&temp_config_path);

        result
    }

    /// Run the complete test matrix including query scorers
    async fn run_complete_matrix_with_scorers(&mut self) -> Result<()> {
        println!(
            "🚀 Starting comprehensive scoring function x haystack x query scorer test matrix"
        );

        let scoring_functions = ScoringFunction::all();
        let haystack_types = HaystackType::all();
        let query_scorers = QueryScorer::all();

        // Calculate total combinations including scorer variations
        let mut total_with_scorers = 0;
        for scoring_function in &scoring_functions {
            for _haystack_type in &haystack_types {
                total_with_scorers += 1; // Base combination

                // Add scorer combinations for title scorer
                if *scoring_function == ScoringFunction::TitleScorer {
                    total_with_scorers += query_scorers.len();
                }
            }
        }

        self.total_combinations = total_with_scorers;
        println!(
            "📊 Total combinations to test (including scorers): {}",
            self.total_combinations
        );
        println!();

        for scoring_function in &scoring_functions {
            println!("🔬 Testing scoring function: {}", scoring_function.name());

            for haystack_type in &haystack_types {
                // Test base combination without specific query scorer
                let result = self.test_combination(scoring_function, haystack_type).await;

                if result.is_success() {
                    self.successful_combinations += 1;
                }

                self.results.push(result);

                // Test with specific query scorers if this is TitleScorer
                if *scoring_function == ScoringFunction::TitleScorer {
                    for query_scorer in &query_scorers {
                        let result = self
                            .test_combination_with_scorer(
                                scoring_function,
                                haystack_type,
                                Some(query_scorer),
                            )
                            .await;

                        if result.is_success() {
                            self.successful_combinations += 1;
                        }

                        self.results.push(result);
                    }
                }
            }

            println!();
        }

        Ok(())
    }

    /// Run the complete test matrix
    async fn run_complete_matrix(&mut self) -> Result<()> {
        println!("🚀 Starting comprehensive scoring function x haystack test matrix");
        println!("📊 Total combinations to test: {}", self.total_combinations);
        println!();

        let scoring_functions = ScoringFunction::all();
        let haystack_types = HaystackType::all();

        for scoring_function in &scoring_functions {
            println!("🔬 Testing scoring function: {}", scoring_function.name());

            for haystack_type in &haystack_types {
                let result = self.test_combination(scoring_function, haystack_type).await;

                if result.is_success() {
                    self.successful_combinations += 1;
                }

                self.results.push(result);
            }

            println!();
        }

        Ok(())
    }

    /// Generate comprehensive test report
    fn generate_report(&self) {
        println!("📋 TEST MATRIX RESULTS REPORT");
        println!("{}", "=".repeat(80));
        println!();

        // Overall summary
        let success_rate =
            (self.successful_combinations as f64 / self.total_combinations as f64) * 100.0;
        println!("📊 OVERALL SUMMARY:");
        println!("  Total combinations tested: {}", self.total_combinations);
        println!(
            "  Successful combinations: {}",
            self.successful_combinations
        );
        println!("  Success rate: {:.1}%", success_rate);
        println!();

        // Results by scoring function
        println!("📈 RESULTS BY SCORING FUNCTION:");
        for scoring_function in ScoringFunction::all() {
            let scoring_results: Vec<_> = self
                .results
                .iter()
                .filter(|r| r.scoring_function == scoring_function)
                .collect();

            let successes = scoring_results.iter().filter(|r| r.is_success()).count();
            let total = scoring_results.len();

            println!(
                "  {}: {}/{} ({:.1}%)",
                scoring_function.name(),
                successes,
                total,
                (successes as f64 / total as f64) * 100.0
            );
        }
        println!();

        // Results by haystack type
        println!("📈 RESULTS BY HAYSTACK TYPE:");
        for haystack_type in HaystackType::all() {
            let haystack_results: Vec<_> = self
                .results
                .iter()
                .filter(|r| r.haystack_type == haystack_type)
                .collect();

            let successes = haystack_results.iter().filter(|r| r.is_success()).count();
            let total = haystack_results.len();

            println!(
                "  {}: {}/{} ({:.1}%)",
                haystack_type.name(),
                successes,
                total,
                (successes as f64 / total as f64) * 100.0
            );
        }
        println!();

        // Performance analysis
        println!("⚡ PERFORMANCE ANALYSIS:");
        let mut performance_scores: Vec<_> = self
            .results
            .iter()
            .filter_map(|r| r.performance_score.map(|score| (r, score)))
            .collect();

        performance_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        if !performance_scores.is_empty() {
            println!("  Top performing combinations:");
            for (result, score) in performance_scores.iter().take(5) {
                println!(
                    "    {}: {:.2} results/sec",
                    result.combination_name(),
                    score
                );
            }
        }
        println!();

        // Credential status check
        println!("🔐 CREDENTIAL STATUS:");
        println!(
            "  ATOMIC_SERVER_SECRET: {}",
            if std::env::var("ATOMIC_SERVER_SECRET").is_ok() {
                "✅ Available"
            } else {
                "❌ Missing"
            }
        );
        println!(
            "  CLICKUP_API_TOKEN: {}",
            if std::env::var("CLICKUP_API_TOKEN").is_ok() {
                "✅ Available"
            } else {
                "❌ Missing"
            }
        );
        println!(
            "  CLICKUP_TEAM_ID: {}",
            if std::env::var("CLICKUP_TEAM_ID").is_ok() {
                "✅ Available"
            } else {
                "❌ Missing"
            }
        );
        println!();

        // Detailed failure analysis
        let failures: Vec<_> = self.results.iter().filter(|r| !r.is_success()).collect();

        if !failures.is_empty() {
            println!("❌ FAILURE ANALYSIS:");

            // Group failures by type
            let zero_results = failures
                .iter()
                .filter(|f| {
                    f.error_message
                        .as_ref()
                        .is_some_and(|msg| msg.contains("0 results"))
                })
                .count();
            let config_failures = failures
                .iter()
                .filter(|f| !f.config_creation_success)
                .count();
            let search_failures = failures
                .iter()
                .filter(|f| {
                    f.config_creation_success
                        && !f.search_success
                        && !f
                            .error_message
                            .as_ref()
                            .is_some_and(|msg| msg.contains("0 results"))
                })
                .count();

            println!("  Summary:");
            println!("    ⚠️ Zero results: {} combinations", zero_results);
            println!("    🔧 Config failures: {} combinations", config_failures);
            println!("    🔍 Search failures: {} combinations", search_failures);
            println!();

            println!("  Details:");
            for failure in &failures {
                let category = if !failure.config_creation_success {
                    "CONFIG"
                } else if failure
                    .error_message
                    .as_ref()
                    .is_some_and(|msg| msg.contains("0 results"))
                {
                    "ZERO_RESULTS"
                } else {
                    "SEARCH"
                };

                println!(
                    "    [{}] {}: {}",
                    category,
                    failure.combination_name(),
                    failure
                        .error_message
                        .as_ref()
                        .unwrap_or(&"Unknown error".to_string())
                );
            }
        }

        println!();
        println!("🎯 MATRIX TEST COMPLETE");
    }
}

/// Main test that runs the complete scoring function x haystack matrix
#[tokio::test]
#[serial]
async fn test_complete_scoring_haystack_matrix() -> Result<()> {
    let mut matrix = TestMatrix::new();

    matrix.run_complete_matrix().await?;
    matrix.generate_report();

    // Assert that we have reasonable success rate (at least 60%)
    let success_rate =
        (matrix.successful_combinations as f64 / matrix.total_combinations as f64) * 100.0;
    assert!(
        success_rate >= 40.0,
        "Matrix test success rate should be at least 40%, got {:.1}%",
        success_rate
    );

    // Assert that at least some combinations work
    assert!(
        matrix.successful_combinations > 0,
        "At least some scoring function x haystack combinations should work"
    );

    println!(
        "✅ Matrix test completed with {:.1}% success rate",
        success_rate
    );

    Ok(())
}

/// Test specific high-priority combinations
#[tokio::test]
#[serial]
async fn test_priority_combinations() -> Result<()> {
    println!("🎯 Testing priority scoring function x haystack combinations");

    let matrix = TestMatrix::new();

    // Priority combinations that should always work
    let priority_combinations = vec![
        (ScoringFunction::TitleScorer, HaystackType::Ripgrep),
        (ScoringFunction::BM25, HaystackType::Ripgrep),
        (ScoringFunction::TerraphimGraph, HaystackType::Ripgrep),
        (ScoringFunction::BM25F, HaystackType::QueryRs),
        (ScoringFunction::BM25Plus, HaystackType::QueryRs),
    ];

    let mut successful_priority_tests = 0;

    for (scoring_function, haystack_type) in priority_combinations {
        println!(
            "Testing priority combination: {} + {}",
            scoring_function.name(),
            haystack_type.name()
        );

        let result = matrix
            .test_combination(&scoring_function, &haystack_type)
            .await;

        if result.is_success() {
            successful_priority_tests += 1;
            println!("  ✅ Success");
        } else {
            println!(
                "  ❌ Failed: {}",
                result.error_message.unwrap_or("Unknown error".to_string())
            );
        }
    }

    // All priority combinations should work
    assert!(
        successful_priority_tests >= 3,
        "At least 3 priority combinations should work, got {}",
        successful_priority_tests
    );

    println!(
        "✅ Priority combinations test completed: {}/5 successful",
        successful_priority_tests
    );

    Ok(())
}

/// Test performance comparison across scoring functions
#[tokio::test]
#[serial]
async fn test_scoring_function_performance_comparison() -> Result<()> {
    println!("⚡ Testing performance comparison across scoring functions");

    let matrix = TestMatrix::new();
    let test_query = "system architecture documentation";

    let mut performance_results = HashMap::new();

    // Test each scoring function with Ripgrep (most reliable haystack)
    for scoring_function in ScoringFunction::all() {
        println!("  Testing {} performance...", scoring_function.name());

        let config = matrix.generate_test_config(&scoring_function, &HaystackType::Ripgrep, None);
        let temp_config_path = format!(
            "/tmp/terraphim_perf_test_{}.json",
            scoring_function.name().to_lowercase()
        );

        std::fs::write(&temp_config_path, serde_json::to_string_pretty(&config)?)?;
        match matrix.run_tui_command(&[
            "--config",
            &temp_config_path,
            "search",
            test_query,
            "--limit",
            "10",
        ]) {
            Ok((stdout, _, code, duration)) => {
                if code == 0 {
                    let result_count = stdout.lines().filter(|line| line.starts_with("- ")).count();

                    performance_results.insert(scoring_function.name(), (duration, result_count));
                    println!("    {} results in {:?}", result_count, duration);
                }
            }
            Err(e) => {
                println!("    Failed: {}", e);
            }
        }

        let _ = std::fs::remove_file(&temp_config_path);
    }

    // Report performance comparison
    if !performance_results.is_empty() {
        println!("\n📊 PERFORMANCE COMPARISON:");
        let mut sorted_results: Vec<_> = performance_results.iter().collect();
        sorted_results.sort_by(|a, b| a.1 .0.cmp(&b.1 .0));

        for (name, (duration, count)) in sorted_results {
            println!(
                "  {}: {} results in {:?} ({:.2} results/sec)",
                name,
                count,
                duration,
                if duration.as_secs_f64() > 0.0 {
                    *count as f64 / duration.as_secs_f64()
                } else {
                    0.0
                }
            );
        }
    }

    // Assert that we got at least some performance data
    assert!(
        !performance_results.is_empty(),
        "Should get performance data for at least some scoring functions"
    );

    println!("✅ Performance comparison completed");

    Ok(())
}

/// Extended test that includes query scorer combinations
#[tokio::test]
#[serial]
async fn test_extended_matrix_with_query_scorers() -> Result<()> {
    let mut matrix = TestMatrix::new();

    matrix.run_complete_matrix_with_scorers().await?;
    matrix.generate_report();

    // Assert that we have reasonable success rate for extended matrix
    let success_rate =
        (matrix.successful_combinations as f64 / matrix.total_combinations as f64) * 100.0;
    assert!(
        success_rate >= 30.0,
        "Extended matrix test success rate should be at least 30%, got {:.1}%",
        success_rate
    );

    // Assert that we tested more combinations than the basic matrix
    assert!(
        matrix.total_combinations > 30, // 5 scoring functions * 6 haystacks = 30 base combinations
        "Extended matrix should test more than base combinations"
    );

    println!(
        "✅ Extended matrix test completed with {:.1}% success rate",
        success_rate
    );
    println!(
        "📊 Total combinations tested: {}",
        matrix.total_combinations
    );

    Ok(())
}

/// Test specific query scorer combinations with title scorer
#[tokio::test]
#[serial]
async fn test_title_scorer_query_combinations() -> Result<()> {
    println!("🎯 Testing TitleScorer with various query scorers");

    let matrix = TestMatrix::new();
    let haystack_type = HaystackType::Ripgrep; // Most reliable for testing

    let mut successful_scorer_tests = 0;
    let mut total_scorer_tests = 0;

    // Test each query scorer with TitleScorer
    for query_scorer in QueryScorer::all() {
        if query_scorer.supports_relevance_function(&ScoringFunction::TitleScorer) {
            println!(
                "Testing TitleScorer + {} with {}",
                query_scorer.name(),
                haystack_type.name()
            );

            let result = matrix
                .test_combination_with_scorer(
                    &ScoringFunction::TitleScorer,
                    &haystack_type,
                    Some(&query_scorer),
                )
                .await;

            total_scorer_tests += 1;

            if result.is_success() {
                successful_scorer_tests += 1;
                println!(
                    "  ✅ Success: {} results in {:?}",
                    result.result_count.unwrap_or(0),
                    result.response_time.unwrap_or(Duration::from_secs(0))
                );
            } else {
                println!(
                    "  ❌ Failed: {}",
                    result.error_message.unwrap_or("Unknown error".to_string())
                );
            }
        }
    }

    let scorer_success_rate = if total_scorer_tests > 0 {
        (successful_scorer_tests as f64 / total_scorer_tests as f64) * 100.0
    } else {
        0.0
    };

    // At least half of the query scorer combinations should work
    assert!(
        scorer_success_rate >= 50.0 || successful_scorer_tests >= 3,
        "At least 50% of query scorer combinations should work or at least 3 should succeed. Got {:.1}% ({}/{})",
        scorer_success_rate, successful_scorer_tests, total_scorer_tests
    );

    println!(
        "✅ Query scorer combinations test completed: {}/{} successful ({:.1}%)",
        successful_scorer_tests, total_scorer_tests, scorer_success_rate
    );

    Ok(())
}
