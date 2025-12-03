/// Performance Optimization Demo
///
/// This example demonstrates the advanced performance optimization features
/// implemented in the Terraphim GPUI component architecture.
///
/// Run with: cargo run --example performance_optimization_demo

use std::sync::Arc;
use std::time::Duration;

use gpui::*;
use anyhow::Result;

// Import optimization modules
use terraphim_desktop_gpui::components::{
    PerformanceManager, PerformanceMode,
    AdvancedVirtualizationState, AdvancedVirtualizationConfig,
    MemoryOptimizer, MemoryOptimizerConfig,
    RenderOptimizer, RenderOptimizerConfig,
    AsyncOptimizer, AsyncOptimizerConfig,
    PerformanceBenchmark, BenchmarkConfig,
};

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Run the demo
    App::new().run(|cx| {
        // Initialize performance manager
        cx.spawn(async move {
            let manager = PerformanceManager::new();

            // Initialize all optimization systems
            manager.initialize().await.expect("Failed to initialize performance manager");

            // Demonstrate different performance modes
            demonstrate_performance_modes(&manager).await;

            // Run benchmarks
            run_performance_benchmarks(&manager).await;

            // Show optimization recommendations
            show_recommendations(&manager).await;

            // Generate comprehensive report
            generate_performance_report(&manager).await;
        });

        // Simple UI to show the demo is running
        cx.new_view(|cx| PerformanceDemoView::new(cx))
    })
}

/// Demonstrate different performance modes
async fn demonstrate_performance_modes(manager: &PerformanceManager) {
    println!("\n=== Performance Modes Demonstration ===\n");

    // Get available modes
    let modes = PerformanceManager::get_available_modes();

    for mode in modes {
        println!("Switching to: {}", mode.name);
        println!("Description: {}", mode.description);

        // Switch to this mode
        manager.set_mode(mode).await.expect("Failed to set performance mode");

        // Get current metrics
        let metrics = manager.get_integrated_metrics().await;
        println!("Current FPS: {:.2}", metrics.current_fps);
        println!("Memory Usage: {:.2} MB", metrics.memory_usage_mb);
        println!("Performance Score: {:.2}", metrics.performance_score);
        println!("---");

        // Simulate some work
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Run performance benchmarks
async fn run_performance_benchmarks(manager: &PerformanceManager) {
    println!("\n=== Performance Benchmarks ===\n");

    // Run all benchmarks
    let results = manager.run_benchmarks().await.expect("Failed to run benchmarks");

    println!("Completed {} benchmarks", results.len());

    // Show benchmark results
    for result in results {
        println!("Benchmark: {} ({})", result.name, result.category);
        println!("  Mean: {:?}", result.statistics.mean);
        println!("  P95: {:?}", result.statistics.p95);
        println!("  Std Dev: {:?}", result.statistics.std_dev);

        if let Some(comparison) = result.baseline_comparison {
            println!("  Change: {:.1}% ({:?})", comparison.change_percent, comparison.direction);
        }
        println!();
    }
}

/// Show optimization recommendations
async fn show_recommendations(manager: &PerformanceManager) {
    println!("\n=== Optimization Recommendations ===\n");

    let recommendations = manager.get_recommendations().await;

    if recommendations.is_empty() {
        println!("No optimizations needed at this time.");
    } else {
        for (i, rec) in recommendations.iter().enumerate() {
            println!("{}. {}", i + 1, rec);
        }
    }
}

/// Generate comprehensive performance report
async fn generate_performance_report(manager: &PerformanceManager) {
    println!("\n=== Performance Report ===\n");

    let report = manager.generate_report().await;

    println!("Generated at: {:?}", report.generated_at);
    println!("Performance Mode: {}", report.current_mode.name);

    println!("\n--- Metrics ---");
    println!("Performance Score: {:.2}/100", report.metrics.performance_score);
    println!("Current FPS: {:.2}", report.metrics.current_fps);
    println!("Memory Usage: {:.2} MB", report.metrics.memory_usage_mb);
    println!("Memory Efficiency: {:.2}%", report.metrics.memory_efficiency);
    println!("Render Efficiency: {:.2}%", report.metrics.render_efficiency);

    println!("\n--- Summary ---");
    println!("Total Benchmarks: {}", report.benchmarks.len());

    let regressions = report.benchmarks.iter().filter(|b| b.regression).count();
    if regressions > 0 {
        println!("⚠️  Regressions detected: {}", regressions);
    } else {
        println!("✅ No performance regressions detected");
    }

    println!("\n--- Recommendations ---");
    for rec in report.recommendations {
        println!("• {}", rec);
    }
}

/// Simple demo view to show the application is running
struct PerformanceDemoView {
    manager: Arc<PerformanceManager>,
    last_metrics: IntegratedMetrics,
}

impl PerformanceDemoView {
    fn new(cx: &mut ViewContext<Self>) -> Self {
        // Get global performance manager
        let manager = Arc::new(unsafe { std::mem::transmute(global_performance_manager()) });

        Self {
            manager,
            last_metrics: IntegratedMetrics::default(),
        }
    }
}

impl Render for PerformanceDemoView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .p_4()
            .gap_4()
            .bg(rgb(0xf5f5f5))
            .child(
                div()
                    .text_2xl()
                    .font_semibold()
                    .child("Performance Optimization Demo")
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        self.metric_card("Performance Score",
                            format!("{:.1}%", self.last_metrics.performance_score),
                            rgb(0x10b981)
                        )
                    )
                    .child(
                        self.metric_card("FPS",
                            format!("{:.1}", self.last_metrics.current_fps),
                            if self.last_metrics.current_fps >= 60.0 { rgb(0x10b981) } else { rgb(0xf59e0b) }
                        )
                    )
                    .child(
                        self.metric_card("Memory",
                            format!("{:.1} MB", self.last_metrics.memory_usage_mb),
                            rgb(0x3b82f6)
                        )
                    )
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x6b7280))
                    .child("Check the console output for detailed performance analysis and recommendations.")
            )
    }
}

impl PerformanceDemoView {
    fn metric_card(&self, title: &str, value: String, color: Rgba) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .p_4()
            .bg(rgb(0xffffff))
            .rounded_lg()
            .border_1()
            .border_color(rgb(0xe5e7eb))
            .shadow_sm()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x6b7280))
                    .child(title)
            )
            .child(
                div()
                    .text_xl()
                    .font_semibold()
                    .text_color(color)
                    .child(value)
            )
    }
}

// Re-export for the demo
use terraphim_desktop_gpui::components::optimization_integration::{
    global_performance_manager, init_performance_manager, IntegratedMetrics
};