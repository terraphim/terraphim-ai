#!/usr/bin/env rust-script

//! Performance monitoring utility for Terraphim test matrix
//!
//! This script analyzes test matrix results and generates performance reports
//! with optimization recommendations.

use std::collections::HashMap;
use std::fs;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct PerformanceMetric {
    combination: String,
    scoring_function: String,
    haystack_type: String,
    query_scorer: Option<String>,
    response_time_ms: u64,
    results_per_second: f64,
    result_count: usize,
    success: bool,
    timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PerformanceReport {
    total_combinations: usize,
    success_rate: f64,
    average_response_time_ms: f64,
    top_performers: Vec<PerformanceMetric>,
    slow_performers: Vec<PerformanceMetric>,
    recommendations: Vec<String>,
}

impl PerformanceReport {
    fn new() -> Self {
        Self {
            total_combinations: 0,
            success_rate: 0.0,
            average_response_time_ms: 0.0,
            top_performers: Vec::new(),
            slow_performers: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    fn analyze_metrics(&mut self, metrics: Vec<PerformanceMetric>) {
        self.total_combinations = metrics.len();

        if metrics.is_empty() {
            return;
        }

        // Calculate success rate
        let successful = metrics.iter().filter(|m| m.success).count();
        self.success_rate = (successful as f64 / metrics.len() as f64) * 100.0;

        // Calculate average response time
        let total_time: u64 = metrics.iter()
            .filter(|m| m.success)
            .map(|m| m.response_time_ms)
            .sum();
        self.average_response_time_ms = total_time as f64 / successful as f64;

        // Find top performers (fastest 5)
        let mut sorted_by_speed = metrics.clone();
        sorted_by_speed.sort_by(|a, b| b.results_per_second.partial_cmp(&a.results_per_second).unwrap());
        self.top_performers = sorted_by_speed.into_iter().take(5).collect();

        // Find slow performers (slowest 5)
        let mut sorted_by_time = metrics.clone();
        sorted_by_time.sort_by(|a, b| b.response_time_ms.cmp(&a.response_time_ms));
        self.slow_performers = sorted_by_time.into_iter()
            .filter(|m| m.success && m.response_time_ms > 5000) // > 5 seconds
            .take(5)
            .collect();

        // Generate recommendations
        self.generate_recommendations(&metrics);
    }

    fn generate_recommendations(&mut self, metrics: &[PerformanceMetric]) {
        let mut recommendations = Vec::new();

        // Analyze haystack performance
        let mut haystack_performance: HashMap<String, Vec<f64>> = HashMap::new();
        for metric in metrics.iter().filter(|m| m.success) {
            haystack_performance
                .entry(metric.haystack_type.clone())
                .or_default()
                .push(metric.results_per_second);
        }

        // Find best performing haystack
        if let Some((best_haystack, _)) = haystack_performance
            .iter()
            .max_by(|(_, a), (_, b)| {
                let avg_a = a.iter().sum::<f64>() / a.len() as f64;
                let avg_b = b.iter().sum::<f64>() / b.len() as f64;
                avg_a.partial_cmp(&avg_b).unwrap()
            })
        {
            recommendations.push(format!(
                "🚀 Use {} haystack for best performance (highest average results/sec)",
                best_haystack
            ));
        }

        // Identify slow combinations
        let slow_threshold = 10000; // 10 seconds
        let slow_combinations: Vec<_> = metrics
            .iter()
            .filter(|m| m.success && m.response_time_ms > slow_threshold)
            .collect();

        if !slow_combinations.is_empty() {
            recommendations.push(format!(
                "⚠️ {} combinations are slower than {}s - consider optimization",
                slow_combinations.len(),
                slow_threshold / 1000
            ));

            // Group slow combinations by haystack type
            let mut slow_by_haystack: HashMap<String, usize> = HashMap::new();
            for combo in &slow_combinations {
                *slow_by_haystack.entry(combo.haystack_type.clone()).or_insert(0) += 1;
            }

            for (haystack, count) in slow_by_haystack {
                recommendations.push(format!(
                    "🔧 Optimize {} haystack ({} slow combinations detected)",
                    haystack, count
                ));
            }
        }

        // Analyze query scorer performance for TitleScorer
        let title_scorer_metrics: Vec<_> = metrics
            .iter()
            .filter(|m| m.scoring_function == "TitleScorer" && m.query_scorer.is_some())
            .collect();

        if !title_scorer_metrics.is_empty() {
            let mut scorer_performance: HashMap<String, Vec<f64>> = HashMap::new();
            for metric in &title_scorer_metrics {
                if let Some(scorer) = &metric.query_scorer {
                    scorer_performance
                        .entry(scorer.clone())
                        .or_default()
                        .push(metric.results_per_second);
                }
            }

            if let Some((best_scorer, _)) = scorer_performance
                .iter()
                .max_by(|(_, a), (_, b)| {
                    let avg_a = a.iter().sum::<f64>() / a.len() as f64;
                    let avg_b = b.iter().sum::<f64>() / b.len() as f64;
                    avg_a.partial_cmp(&avg_b).unwrap()
                })
            {
                recommendations.push(format!(
                    "🎯 Use {} query scorer with TitleScorer for optimal performance",
                    best_scorer
                ));
            }
        }

        // Memory and efficiency recommendations
        recommendations.push("💾 Consider implementing connection pooling for remote haystacks".to_string());
        recommendations.push("🔄 Add result caching for frequently accessed queries".to_string());
        recommendations.push("⚡ Implement parallel processing for multi-haystack searches".to_string());

        self.recommendations = recommendations;
    }

    fn generate_json_report(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "Error generating JSON report".to_string())
    }

    fn generate_markdown_report(&self) -> String {
        let mut report = String::new();

        report.push_str("# 📊 Performance Analysis Report\n\n");
        report.push_str(&format!("**Generated**: {}\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        report.push_str(&format!("**Total Combinations**: {}\n", self.total_combinations));
        report.push_str(&format!("**Success Rate**: {:.1}%\n", self.success_rate));
        report.push_str(&format!("**Average Response Time**: {:.1}ms\n\n", self.average_response_time_ms));

        // Top performers
        report.push_str("## 🏆 Top Performers\n\n");
        report.push_str("| Rank | Combination | Results/Sec | Response Time |\n");
        report.push_str("|------|-------------|-------------|---------------|\n");
        for (i, metric) in self.top_performers.iter().enumerate() {
            let scorer_part = metric.query_scorer.as_ref()
                .map(|s| format!(" ({})", s))
                .unwrap_or_default();
            report.push_str(&format!(
                "| {} | {} + {}{} | {:.2} | {}ms |\n",
                i + 1,
                metric.scoring_function,
                metric.haystack_type,
                scorer_part,
                metric.results_per_second,
                metric.response_time_ms
            ));
        }

        // Slow performers
        if !self.slow_performers.is_empty() {
            report.push_str("\n## ⚠️ Slow Performers\n\n");
            report.push_str("| Combination | Response Time | Results/Sec |\n");
            report.push_str("|-------------|---------------|-------------|\n");
            for metric in &self.slow_performers {
                let scorer_part = metric.query_scorer.as_ref()
                    .map(|s| format!(" ({})", s))
                    .unwrap_or_default();
                report.push_str(&format!(
                    "| {} + {}{} | {}ms | {:.2} |\n",
                    metric.scoring_function,
                    metric.haystack_type,
                    scorer_part,
                    metric.response_time_ms,
                    metric.results_per_second
                ));
            }
        }

        // Recommendations
        report.push_str("\n## 🎯 Optimization Recommendations\n\n");
        for recommendation in &self.recommendations {
            report.push_str(&format!("- {}\n", recommendation));
        }

        report
    }
}

fn main() {
    println!("🔍 Terraphim Performance Monitor");
    println!("==================================");

    // This is a template - in practice, metrics would be collected from test runs
    let sample_metrics = vec![
        PerformanceMetric {
            combination: "TitleScorer + QueryRs (JaroWinkler)".to_string(),
            scoring_function: "TitleScorer".to_string(),
            haystack_type: "QueryRs".to_string(),
            query_scorer: Some("JaroWinkler".to_string()),
            response_time_ms: 131,
            results_per_second: 7.64,
            result_count: 26,
            success: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
        PerformanceMetric {
            combination: "BM25 + ClickUp".to_string(),
            scoring_function: "BM25".to_string(),
            haystack_type: "ClickUp".to_string(),
            query_scorer: None,
            response_time_ms: 39941,
            results_per_second: 0.025,
            result_count: 0,
            success: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    ];

    let mut report = PerformanceReport::new();
    report.analyze_metrics(sample_metrics);

    println!("📊 Performance Report Generated:");
    println!("{}", report.generate_markdown_report());

    // Save JSON report
    if let Ok(_) = fs::write("performance_report.json", report.generate_json_report()) {
        println!("\n💾 JSON report saved to: performance_report.json");
    }

    // Save Markdown report
    if let Ok(_) = fs::write("performance_report.md", report.generate_markdown_report()) {
        println!("📝 Markdown report saved to: performance_report.md");
    }
}

// Add dependencies for a real implementation:
// [dependencies]
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// chrono = { version = "0.4", features = ["serde"] }
