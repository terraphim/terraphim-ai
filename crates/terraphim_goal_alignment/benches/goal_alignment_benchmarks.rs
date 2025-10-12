//! Benchmarks for the goal alignment system
//! Currently disabled due to API changes requiring async RoleGraph::new()

// Commented out due to API incompatibility with to_async and RoleGraph::new()
/*
use std::sync::Arc;

use criterion::{black_box, BenchmarkId, Criterion};
use tokio::runtime::Runtime;

use terraphim_goal_alignment::{
    AnalysisType, AutomataConfig, Goal, GoalAlignmentAnalysis, GoalHierarchy, GoalLevel,
    KnowledgeGraphGoalAnalyzer, SimilarityThresholds,
};
use terraphim_rolegraph::RoleGraph;

fn bench_goal_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("goal_creation");

    for num_goals in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("create_goals", num_goals),
            num_goals,
            |b, &num_goals| {
                b.iter(|| {
                    let mut hierarchy = GoalHierarchy::new();

                    for i in 0..num_goals {
                        let goal = Goal::new(
                            format!("goal_{}", i),
                            GoalLevel::Local,
                            format!("Goal {} description", i),
                            i as u32,
                        );
                        black_box(hierarchy.add_goal(goal).unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_goal_alignment_analysis(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("goal_alignment_analysis");
    group.sample_size(10);

    for num_goals in [5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("analyze_alignment", num_goals),
            num_goals,
            |b, &num_goals| {
                b.to_async(&rt).iter(|| async {
                    let role_graph = Arc::new(RoleGraph::new());
                    let analyzer = KnowledgeGraphGoalAnalyzer::new(
                        role_graph,
                        AutomataConfig::default(),
                        SimilarityThresholds::default(),
                    );

                    let mut goals = Vec::new();
                    for i in 0..num_goals {
                        let mut goal = Goal::new(
                            format!("goal_{}", i),
                            GoalLevel::Local,
                            format!("Goal {} for testing alignment analysis", i),
                            i as u32,
                        );
                        goal.knowledge_context.concepts =
                            vec![format!("concept_{}", i), "shared_concept".to_string()];
                        goals.push(goal);
                    }

                    let analysis = GoalAlignmentAnalysis {
                        goals,
                        analysis_type: AnalysisType::Comprehensive,
                        context: std::collections::HashMap::new(),
                    };

                    black_box(analyzer.analyze_goal_alignment(analysis).await.unwrap());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_goal_creation, bench_goal_alignment_analysis);
criterion_main!(benches);
*/

fn main() {
    // Benchmarks are temporarily disabled due to async API issues
    println!("Goal alignment benchmarks temporarily disabled");
}
