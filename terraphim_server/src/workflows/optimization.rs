use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::time::{sleep, Duration};

use super::{
    complete_workflow_session, create_workflow_session, fail_workflow_session,
    generate_workflow_id, multi_agent_handlers::MultiAgentWorkflowExecutor, update_workflow_status,
    ExecutionStatus, WorkflowMetadata, WorkflowRequest, WorkflowResponse,
};
use crate::AppState;

#[derive(Debug, Clone, Serialize)]
struct EvaluatorAgent {
    id: String,
    name: String,
    evaluation_criteria: Vec<String>,
    scoring_methodology: String,
    expertise_domain: String,
}

#[derive(Debug, Clone, Serialize)]
struct OptimizerAgent {
    id: String,
    name: String,
    optimization_strategies: Vec<String>,
    improvement_focus: String,
    adaptation_capability: String,
}

#[derive(Debug, Serialize)]
struct ContentVariant {
    id: String,
    variant_name: String,
    content: String,
    generation_approach: String,
    metadata: VariantMetadata,
}

#[derive(Debug, Serialize)]
struct VariantMetadata {
    generation_time_ms: u64,
    complexity_score: f64,
    novelty_index: f64,
    estimated_effectiveness: f64,
}

#[derive(Debug, Serialize)]
struct EvaluationResult {
    variant_id: String,
    evaluator_id: String,
    overall_score: f64,
    criterion_scores: Vec<CriterionScore>,
    strengths: Vec<String>,
    weaknesses: Vec<String>,
    improvement_suggestions: Vec<String>,
    evaluation_confidence: f64,
}

#[derive(Debug, Serialize)]
struct CriterionScore {
    criterion: String,
    score: f64,
    weight: f64,
    rationale: String,
}

#[derive(Debug, Serialize)]
struct OptimizationIteration {
    iteration_number: usize,
    generated_variants: Vec<ContentVariant>,
    evaluation_results: Vec<EvaluationResult>,
    best_variant_id: String,
    improvement_delta: f64,
    optimization_decisions: Vec<OptimizationDecision>,
}

#[derive(Debug, Serialize)]
struct OptimizationDecision {
    decision_type: String,
    rationale: String,
    parameters_adjusted: Vec<String>,
    expected_impact: String,
}

#[derive(Debug, Serialize)]
struct OptimizationResult {
    optimization_summary: OptimizationSummary,
    iteration_history: Vec<OptimizationIteration>,
    final_optimized_content: FinalOptimizedContent,
    performance_analytics: PerformanceAnalytics,
    convergence_analysis: ConvergenceAnalysis,
}

#[derive(Debug, Serialize)]
struct OptimizationSummary {
    total_iterations: usize,
    variants_generated: usize,
    evaluations_performed: usize,
    final_quality_score: f64,
    total_improvement: f64,
    convergence_achieved: bool,
}

#[derive(Debug, Serialize)]
struct FinalOptimizedContent {
    content: String,
    quality_metrics: QualityMetrics,
    optimization_path: Vec<String>,
    key_improvements: Vec<String>,
}

#[derive(Debug, Serialize)]
struct QualityMetrics {
    overall_quality: f64,
    clarity_score: f64,
    relevance_score: f64,
    engagement_score: f64,
    technical_accuracy: f64,
    creativity_index: f64,
}

#[derive(Debug, Serialize)]
struct PerformanceAnalytics {
    optimization_velocity: f64,
    evaluation_consistency: f64,
    improvement_trajectory: Vec<f64>,
    efficiency_metrics: EfficiencyMetrics,
}

#[derive(Debug, Serialize)]
struct EfficiencyMetrics {
    time_per_iteration_ms: f64,
    variants_per_second: f64,
    evaluations_per_second: f64,
    resource_utilization: ResourceUtilization,
}

#[derive(Debug, Serialize)]
struct ResourceUtilization {
    cpu_efficiency: f64,
    memory_efficiency: f64,
    network_efficiency: f64,
    overall_efficiency: f64,
}

#[derive(Debug, Serialize)]
struct ConvergenceAnalysis {
    convergence_rate: f64,
    stability_window: usize,
    plateau_detection: PlateauAnalysis,
    termination_reason: String,
}

#[derive(Debug, Serialize)]
struct PlateauAnalysis {
    plateau_detected: bool,
    plateau_duration: usize,
    plateau_threshold: f64,
    break_strategy: String,
}

pub async fn execute_optimization(
    State(state): State<AppState>,
    Json(request): Json<WorkflowRequest>,
) -> Result<Json<WorkflowResponse>, StatusCode> {
    let workflow_id = generate_workflow_id();
    let start_time = Instant::now();

    // Create workflow session
    create_workflow_session(
        &state.workflow_sessions,
        &state.websocket_broadcaster,
        workflow_id.clone(),
        "Optimization".to_string(),
    )
    .await;

    let role = request
        .role
        .unwrap_or_else(|| "Optimization Specialist".to_string());
    let overall_role = request
        .overall_role
        .unwrap_or_else(|| "Quality Optimizer".to_string());

    // Use real multi-agent execution instead of simulation
    let result = match MultiAgentWorkflowExecutor::new().await {
        Ok(executor) => {
            match executor
                .execute_optimization(
                    &workflow_id,
                    &request.prompt,
                    &role,
                    &overall_role,
                    &state.workflow_sessions,
                    &state.websocket_broadcaster,
                )
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    fail_workflow_session(
                        &state.workflow_sessions,
                        &state.websocket_broadcaster,
                        workflow_id.clone(),
                        e.to_string(),
                    )
                    .await;
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
        Err(e) => {
            log::error!("Failed to create multi-agent executor: {:?}", e);
            fail_workflow_session(
                &state.workflow_sessions,
                &state.websocket_broadcaster,
                workflow_id.clone(),
                format!("Failed to initialize multi-agent system: {}", e),
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let execution_time = start_time.elapsed().as_millis() as u64;

    // Complete workflow
    complete_workflow_session(
        &state.workflow_sessions,
        &state.websocket_broadcaster,
        workflow_id.clone(),
        result.clone(),
    )
    .await;

    let response = WorkflowResponse {
        workflow_id,
        success: true,
        result: Some(result.clone()),
        error: None,
        metadata: WorkflowMetadata {
            execution_time_ms: execution_time,
            pattern: "Optimization".to_string(),
            steps: result["execution_summary"]["iterations_completed"]
                .as_u64()
                .unwrap_or(3) as usize,
            role,
            overall_role,
        },
    };

    Ok(Json(response))
}

async fn create_evaluator_agent(prompt: &str) -> EvaluatorAgent {
    let content_type = analyze_content_type(prompt);

    let (name, criteria, methodology, domain) = match content_type.as_str() {
        "technical_content" => (
            "Technical Content Evaluator".to_string(),
            vec![
                "Technical accuracy".to_string(),
                "Clarity and comprehensiveness".to_string(),
                "Practical applicability".to_string(),
                "Code quality and best practices".to_string(),
                "Documentation completeness".to_string(),
            ],
            "Multi-criteria weighted scoring with technical validation".to_string(),
            "Software Engineering & Technical Documentation".to_string(),
        ),
        "creative_content" => (
            "Creative Content Evaluator".to_string(),
            vec![
                "Originality and creativity".to_string(),
                "Engagement and appeal".to_string(),
                "Narrative coherence".to_string(),
                "Emotional impact".to_string(),
                "Aesthetic quality".to_string(),
            ],
            "Subjective quality assessment with creativity metrics".to_string(),
            "Creative Writing & Content Production".to_string(),
        ),
        "business_content" => (
            "Business Content Evaluator".to_string(),
            vec![
                "Strategic alignment".to_string(),
                "Market relevance".to_string(),
                "Actionability".to_string(),
                "ROI potential".to_string(),
                "Risk assessment".to_string(),
            ],
            "Business impact analysis with quantitative metrics".to_string(),
            "Business Strategy & Market Analysis".to_string(),
        ),
        _ => (
            "General Content Evaluator".to_string(),
            vec![
                "Content quality".to_string(),
                "Clarity and readability".to_string(),
                "Relevance to requirements".to_string(),
                "Completeness".to_string(),
                "Overall effectiveness".to_string(),
            ],
            "Holistic quality assessment framework".to_string(),
            "General Content Analysis".to_string(),
        ),
    };

    EvaluatorAgent {
        id: "evaluator_001".to_string(),
        name,
        evaluation_criteria: criteria,
        scoring_methodology: methodology,
        expertise_domain: domain,
    }
}

async fn create_optimizer_agent(prompt: &str) -> OptimizerAgent {
    let optimization_focus = analyze_optimization_focus(prompt);

    let (name, strategies, focus, capability) = match optimization_focus.as_str() {
        "performance_optimization" => (
            "Performance Optimization Specialist".to_string(),
            vec![
                "Algorithm efficiency improvement".to_string(),
                "Resource utilization optimization".to_string(),
                "Bottleneck elimination".to_string(),
                "Scalability enhancement".to_string(),
            ],
            "System performance and efficiency".to_string(),
            "Adaptive performance tuning with real-time feedback".to_string(),
        ),
        "content_optimization" => (
            "Content Quality Optimizer".to_string(),
            vec![
                "Iterative refinement".to_string(),
                "Multi-objective optimization".to_string(),
                "Feedback-driven improvement".to_string(),
                "Quality convergence strategies".to_string(),
            ],
            "Content quality and engagement".to_string(),
            "Dynamic content adaptation with quality metrics".to_string(),
        ),
        "user_experience_optimization" => (
            "User Experience Optimizer".to_string(),
            vec![
                "Usability enhancement".to_string(),
                "Accessibility optimization".to_string(),
                "Interaction flow improvement".to_string(),
                "User satisfaction maximization".to_string(),
            ],
            "User experience and satisfaction".to_string(),
            "User-centric optimization with behavioral analytics".to_string(),
        ),
        _ => (
            "General Process Optimizer".to_string(),
            vec![
                "Iterative improvement".to_string(),
                "Quality enhancement".to_string(),
                "Efficiency optimization".to_string(),
                "Adaptive refinement".to_string(),
            ],
            "Overall process and outcome quality".to_string(),
            "Multi-dimensional optimization with adaptive strategies".to_string(),
        ),
    };

    OptimizerAgent {
        id: "optimizer_001".to_string(),
        name,
        optimization_strategies: strategies,
        improvement_focus: focus,
        adaptation_capability: capability,
    }
}

async fn execute_iterative_optimization(
    evaluator: &EvaluatorAgent,
    optimizer: &OptimizerAgent,
    prompt: &str,
    state: &AppState,
    workflow_id: &str,
) -> OptimizationResult {
    let mut iterations = Vec::new();
    let mut current_best_score = 0.0;
    let mut improvement_history = Vec::new();
    let max_iterations = 5; // Configurable
    let convergence_threshold = 0.05; // 5% improvement threshold

    for iteration in 0..max_iterations {
        let iteration_start = Instant::now();

        // Generate content variants
        let variants = generate_content_variants(prompt, iteration, &current_best_score).await;

        // Evaluate all variants
        let mut evaluation_results = Vec::new();
        for variant in &variants {
            let evaluation = evaluate_variant(evaluator, variant).await;
            evaluation_results.push(evaluation);
        }

        // Find best variant in this iteration
        let best_score = evaluation_results
            .iter()
            .map(|r| r.overall_score)
            .fold(0.0, f64::max);

        let best_variant_id = evaluation_results
            .iter()
            .find(|r| r.overall_score == best_score)
            .map(|r| r.variant_id.clone())
            .unwrap_or_default();

        let improvement_delta = best_score - current_best_score;

        // Generate optimization decisions
        let optimization_decisions =
            generate_optimization_decisions(optimizer, &evaluation_results, improvement_delta)
                .await;

        // Update progress
        let progress = 20.0 + (iteration as f64 / max_iterations as f64) * 60.0;
        update_workflow_status(
            &state.workflow_sessions,
            &state.websocket_broadcaster,
            workflow_id,
            ExecutionStatus::Running,
            progress,
            Some(format!(
                "Iteration {}/{}: Generated {} variants, best score: {:.3} (+{:.3})",
                iteration + 1,
                max_iterations,
                variants.len(),
                best_score,
                improvement_delta
            )),
        )
        .await;

        // Create iteration record
        let optimization_iteration = OptimizationIteration {
            iteration_number: iteration,
            generated_variants: variants,
            evaluation_results,
            best_variant_id,
            improvement_delta,
            optimization_decisions,
        };

        iterations.push(optimization_iteration);
        current_best_score = best_score;
        improvement_history.push(current_best_score);

        // Check for convergence
        if improvement_delta < convergence_threshold && iteration > 1 {
            break;
        }

        // Simulate optimization time
        sleep(Duration::from_millis(800)).await;
    }

    // Generate final optimized content
    let final_content = generate_final_optimized_content(&iterations, prompt).await;

    // Calculate performance analytics
    let performance_analytics = calculate_performance_analytics(&iterations, &improvement_history);

    // Perform convergence analysis
    let convergence_analysis = analyze_convergence(&improvement_history);

    OptimizationResult {
        optimization_summary: OptimizationSummary {
            total_iterations: iterations.len(),
            variants_generated: iterations.iter().map(|i| i.generated_variants.len()).sum(),
            evaluations_performed: iterations.iter().map(|i| i.evaluation_results.len()).sum(),
            final_quality_score: current_best_score,
            total_improvement: current_best_score - improvement_history[0],
            convergence_achieved: convergence_analysis.convergence_rate > 0.8,
        },
        iteration_history: iterations,
        final_optimized_content: final_content,
        performance_analytics,
        convergence_analysis,
    }
}

async fn generate_content_variants(
    prompt: &str,
    iteration: usize,
    current_best: &f64,
) -> Vec<ContentVariant> {
    let mut variants = Vec::new();
    let variant_count = if iteration == 0 { 4 } else { 3 }; // More variants in first iteration

    for i in 0..variant_count {
        let generation_start = Instant::now();

        let (variant_name, content, approach) = match i {
            0 => (
                "Conservative Enhancement".to_string(),
                generate_conservative_variant(prompt, current_best).await,
                "Incremental improvement with low risk".to_string(),
            ),
            1 => (
                "Aggressive Optimization".to_string(),
                generate_aggressive_variant(prompt, current_best).await,
                "Bold changes targeting major improvements".to_string(),
            ),
            2 => (
                "Balanced Approach".to_string(),
                generate_balanced_variant(prompt, current_best).await,
                "Moderate changes balancing risk and reward".to_string(),
            ),
            3 => (
                "Creative Alternative".to_string(),
                generate_creative_variant(prompt, current_best).await,
                "Novel approach with creative elements".to_string(),
            ),
            _ => (
                "Adaptive Variant".to_string(),
                generate_adaptive_variant(prompt, current_best, iteration).await,
                "Data-driven adaptation based on previous results".to_string(),
            ),
        };

        let generation_time = generation_start.elapsed().as_millis() as u64;

        variants.push(ContentVariant {
            id: format!("variant_{}_{}", iteration, i),
            variant_name,
            content: content.clone(),
            generation_approach: approach,
            metadata: VariantMetadata {
                generation_time_ms: generation_time,
                complexity_score: calculate_complexity_score(&content),
                novelty_index: calculate_novelty_index(&content, iteration),
                estimated_effectiveness: estimate_effectiveness(&content, current_best),
            },
        });
    }

    variants
}

async fn generate_conservative_variant(prompt: &str, current_best: &f64) -> String {
    format!(
        "Conservative approach to '{}': Focus on proven methodologies with incremental improvements. \
        Building upon established best practices while maintaining stability and reliability. \
        Key enhancement: refined execution with attention to detail and risk mitigation. \
        Quality score target: {:.3} → {:.3}",
        prompt.chars().take(50).collect::<String>(),
        current_best,
        current_best + 0.05
    )
}

async fn generate_aggressive_variant(prompt: &str, current_best: &f64) -> String {
    format!(
        "Aggressive optimization for '{}': Revolutionary approach challenging conventional limits. \
        Implementing cutting-edge techniques with bold architectural changes. \
        Risk/reward profile: high potential gains with calculated risks. \
        Breakthrough target: {:.3} → {:.3}",
        prompt.chars().take(50).collect::<String>(),
        current_best,
        current_best + 0.15
    )
}

async fn generate_balanced_variant(prompt: &str, current_best: &f64) -> String {
    format!(
        "Balanced optimization for '{}': Strategic improvements balancing innovation with stability. \
        Incorporating modern techniques while maintaining proven foundations. \
        Optimized risk profile with sustainable improvements. \
        Balanced target: {:.3} → {:.3}",
        prompt.chars().take(50).collect::<String>(),
        current_best,
        current_best + 0.08
    )
}

async fn generate_creative_variant(prompt: &str, current_best: &f64) -> String {
    format!(
        "Creative solution for '{}': Innovative approach introducing novel perspectives and methodologies. \
        Leveraging creative problem-solving with unconventional techniques. \
        Emphasis on originality while maintaining practical applicability. \
        Innovation target: {:.3} → {:.3}",
        prompt.chars().take(50).collect::<String>(),
        current_best,
        current_best + 0.12
    )
}

async fn generate_adaptive_variant(prompt: &str, current_best: &f64, iteration: usize) -> String {
    format!(
        "Adaptive enhancement for '{}' (iteration {}): Data-driven improvements based on previous optimization cycles. \
        Incorporating lessons learned with targeted refinements. \
        Evidence-based optimization with continuous learning integration. \
        Adaptive target: {:.3} → {:.3}",
        prompt.chars().take(50).collect::<String>(),
        iteration + 1,
        current_best,
        current_best + 0.06
    )
}

async fn evaluate_variant(
    evaluator: &EvaluatorAgent,
    variant: &ContentVariant,
) -> EvaluationResult {
    let mut criterion_scores = Vec::new();
    let mut total_weighted_score = 0.0;
    let mut total_weight = 0.0;

    // Evaluate against each criterion
    for (i, criterion) in evaluator.evaluation_criteria.iter().enumerate() {
        let weight = match i {
            0 => 0.25, // First criterion gets highest weight
            1 => 0.20,
            2 => 0.20,
            3 => 0.15,
            _ => 0.10,
        };

        let score = calculate_criterion_score(criterion, variant);
        let rationale = generate_evaluation_rationale(criterion, score);

        criterion_scores.push(CriterionScore {
            criterion: criterion.clone(),
            score,
            weight,
            rationale,
        });

        total_weighted_score += score * weight;
        total_weight += weight;
    }

    let overall_score = total_weighted_score / total_weight;

    let strengths = identify_strengths(variant, &criterion_scores);
    let weaknesses = identify_weaknesses(variant, &criterion_scores);
    let improvement_suggestions = generate_improvement_suggestions(variant, &criterion_scores);
    let evaluation_confidence = calculate_evaluation_confidence(&criterion_scores);

    EvaluationResult {
        variant_id: variant.id.clone(),
        evaluator_id: evaluator.id.clone(),
        overall_score,
        criterion_scores,
        strengths,
        weaknesses,
        improvement_suggestions,
        evaluation_confidence,
    }
}

async fn generate_optimization_decisions(
    optimizer: &OptimizerAgent,
    evaluation_results: &[EvaluationResult],
    improvement_delta: f64,
) -> Vec<OptimizationDecision> {
    let mut decisions = Vec::new();

    // Decision based on improvement delta
    if improvement_delta > 0.1 {
        decisions.push(OptimizationDecision {
            decision_type: "Continue Aggressive Strategy".to_string(),
            rationale: "Significant improvement observed, maintain current optimization direction"
                .to_string(),
            parameters_adjusted: vec![
                "learning_rate".to_string(),
                "exploration_factor".to_string(),
            ],
            expected_impact: "Sustained high-quality improvements".to_string(),
        });
    } else if improvement_delta < 0.03 {
        decisions.push(OptimizationDecision {
            decision_type: "Increase Exploration".to_string(),
            rationale: "Marginal improvements suggest need for more diverse approaches".to_string(),
            parameters_adjusted: vec![
                "variant_diversity".to_string(),
                "creativity_weight".to_string(),
            ],
            expected_impact: "Enhanced solution space exploration".to_string(),
        });
    }

    // Decision based on evaluation consistency
    let score_variance = calculate_score_variance(evaluation_results);
    if score_variance > 0.05 {
        decisions.push(OptimizationDecision {
            decision_type: "Stabilize Quality".to_string(),
            rationale: "High variance in results indicates need for more consistent approaches"
                .to_string(),
            parameters_adjusted: vec![
                "consistency_weight".to_string(),
                "validation_threshold".to_string(),
            ],
            expected_impact: "More predictable and stable optimization outcomes".to_string(),
        });
    }

    decisions
}

async fn generate_final_optimized_content(
    iterations: &[OptimizationIteration],
    prompt: &str,
) -> FinalOptimizedContent {
    // Find the best variant across all iterations
    let mut best_score = 0.0;
    let mut best_variant: Option<&ContentVariant> = None;

    for iteration in iterations {
        for result in &iteration.evaluation_results {
            if result.overall_score > best_score {
                best_score = result.overall_score;
                // Find corresponding variant
                best_variant = iteration
                    .generated_variants
                    .iter()
                    .find(|v| v.id == result.variant_id);
            }
        }
    }

    let optimized_content = if let Some(variant) = best_variant {
        format!(
            "OPTIMIZED SOLUTION for '{}':\n\n{}\n\n--- OPTIMIZATION DETAILS ---\n\
            Final Quality Score: {:.3}\n\
            Optimization Approach: {}\n\
            Iterations Required: {}\n\
            Key Success Factors: Advanced iterative refinement with multi-criteria evaluation",
            prompt,
            variant.content,
            best_score,
            variant.generation_approach,
            iterations.len()
        )
    } else {
        format!(
            "Optimization completed for '{}' with {} iterations",
            prompt,
            iterations.len()
        )
    };

    let optimization_path: Vec<String> = iterations
        .iter()
        .enumerate()
        .map(|(i, iteration)| {
            format!(
                "Iteration {}: {} variants, best score {:.3} (Δ{:+.3})",
                i + 1,
                iteration.generated_variants.len(),
                iteration
                    .evaluation_results
                    .iter()
                    .map(|r| r.overall_score)
                    .fold(0.0, f64::max),
                iteration.improvement_delta
            )
        })
        .collect();

    FinalOptimizedContent {
        content: optimized_content,
        quality_metrics: QualityMetrics {
            overall_quality: best_score,
            clarity_score: best_score * 0.95,
            relevance_score: best_score * 0.98,
            engagement_score: best_score * 0.92,
            technical_accuracy: best_score * 0.97,
            creativity_index: best_score * 0.88,
        },
        optimization_path,
        key_improvements: vec![
            "Iterative refinement with multi-criteria evaluation".to_string(),
            "Adaptive optimization strategy based on performance feedback".to_string(),
            "Quality convergence through systematic enhancement".to_string(),
        ],
    }
}

// Helper functions for calculations

fn analyze_content_type(prompt: &str) -> String {
    let prompt_lower = prompt.to_lowercase();
    if prompt_lower.contains("code")
        || prompt_lower.contains("technical")
        || prompt_lower.contains("software")
    {
        "technical_content".to_string()
    } else if prompt_lower.contains("creative")
        || prompt_lower.contains("story")
        || prompt_lower.contains("content")
    {
        "creative_content".to_string()
    } else if prompt_lower.contains("business")
        || prompt_lower.contains("strategy")
        || prompt_lower.contains("market")
    {
        "business_content".to_string()
    } else {
        "general_content".to_string()
    }
}

fn analyze_optimization_focus(prompt: &str) -> String {
    let prompt_lower = prompt.to_lowercase();
    if prompt_lower.contains("performance")
        || prompt_lower.contains("speed")
        || prompt_lower.contains("efficiency")
    {
        "performance_optimization".to_string()
    } else if prompt_lower.contains("user")
        || prompt_lower.contains("experience")
        || prompt_lower.contains("usability")
    {
        "user_experience_optimization".to_string()
    } else {
        "content_optimization".to_string()
    }
}

fn calculate_complexity_score(content: &str) -> f64 {
    let word_count = content.split_whitespace().count();
    let sentence_count = content.matches('.').count() + 1;
    let avg_sentence_length = word_count as f64 / sentence_count as f64;

    // Normalized complexity based on sentence length and vocabulary diversity
    (avg_sentence_length / 15.0).min(1.0) * 0.7 + 0.3
}

fn calculate_novelty_index(content: &str, iteration: usize) -> f64 {
    // Simplified novelty calculation
    let base_novelty = if iteration == 0 { 0.5 } else { 0.3 };
    let content_uniqueness = (content.len() % 100) as f64 / 100.0;
    (base_novelty + content_uniqueness * 0.5).min(1.0)
}

fn estimate_effectiveness(content: &str, current_best: &f64) -> f64 {
    let content_quality_indicators = content.matches("optimization").count()
        + content.matches("improvement").count()
        + content.matches("enhancement").count();

    let estimated_boost = (content_quality_indicators as f64 * 0.02).min(0.1);
    current_best + estimated_boost
}

fn calculate_criterion_score(criterion: &str, variant: &ContentVariant) -> f64 {
    // Simplified scoring based on criterion type and content characteristics
    let base_score = 0.7;
    let content_alignment = match criterion {
        s if s.contains("quality") => variant.metadata.estimated_effectiveness * 0.3,
        s if s.contains("clarity") => (variant.content.len() as f64 / 500.0).min(0.2),
        s if s.contains("creativity") => variant.metadata.novelty_index * 0.25,
        s if s.contains("accuracy") => variant.metadata.complexity_score * 0.2,
        _ => 0.1,
    };

    (base_score + content_alignment).min(0.98).max(0.5)
}

fn generate_evaluation_rationale(criterion: &str, score: f64) -> String {
    let quality_descriptor = if score > 0.8 {
        "Excellent"
    } else if score > 0.6 {
        "Good"
    } else {
        "Adequate"
    };
    format!(
        "{} performance on {} with score {:.2}",
        quality_descriptor,
        criterion.to_lowercase(),
        score
    )
}

fn identify_strengths(variant: &ContentVariant, scores: &[CriterionScore]) -> Vec<String> {
    scores
        .iter()
        .filter(|s| s.score > 0.75)
        .map(|s| format!("Strong {}", s.criterion.to_lowercase()))
        .collect()
}

fn identify_weaknesses(variant: &ContentVariant, scores: &[CriterionScore]) -> Vec<String> {
    scores
        .iter()
        .filter(|s| s.score < 0.6)
        .map(|s| format!("Could improve {}", s.criterion.to_lowercase()))
        .collect()
}

fn generate_improvement_suggestions(
    variant: &ContentVariant,
    scores: &[CriterionScore],
) -> Vec<String> {
    let mut suggestions = Vec::new();

    for score in scores {
        if score.score < 0.7 {
            suggestions.push(format!(
                "Enhance {} through targeted refinement",
                score.criterion.to_lowercase()
            ));
        }
    }

    if suggestions.is_empty() {
        suggestions.push("Continue current optimization direction".to_string());
    }

    suggestions
}

fn calculate_evaluation_confidence(scores: &[CriterionScore]) -> f64 {
    let avg_score: f64 = scores.iter().map(|s| s.score).sum::<f64>() / scores.len() as f64;
    let score_variance: f64 = scores
        .iter()
        .map(|s| (s.score - avg_score).powi(2))
        .sum::<f64>()
        / scores.len() as f64;

    // Lower variance = higher confidence
    (1.0 - score_variance.sqrt()).max(0.6).min(0.95)
}

fn calculate_score_variance(results: &[EvaluationResult]) -> f64 {
    if results.is_empty() {
        return 0.0;
    }

    let avg_score: f64 =
        results.iter().map(|r| r.overall_score).sum::<f64>() / results.len() as f64;
    let variance: f64 = results
        .iter()
        .map(|r| (r.overall_score - avg_score).powi(2))
        .sum::<f64>()
        / results.len() as f64;

    variance.sqrt()
}

fn calculate_performance_analytics(
    iterations: &[OptimizationIteration],
    improvement_history: &[f64],
) -> PerformanceAnalytics {
    let total_variants: usize = iterations.iter().map(|i| i.generated_variants.len()).sum();
    let total_evaluations: usize = iterations.iter().map(|i| i.evaluation_results.len()).sum();
    let total_time_estimate = iterations.len() as f64 * 1000.0; // Estimated 1s per iteration

    PerformanceAnalytics {
        optimization_velocity: improvement_history.last().unwrap_or(&0.0)
            - improvement_history.first().unwrap_or(&0.0),
        evaluation_consistency: calculate_evaluation_consistency(iterations),
        improvement_trajectory: improvement_history.to_vec(),
        efficiency_metrics: EfficiencyMetrics {
            time_per_iteration_ms: total_time_estimate / iterations.len() as f64,
            variants_per_second: total_variants as f64 / (total_time_estimate / 1000.0),
            evaluations_per_second: total_evaluations as f64 / (total_time_estimate / 1000.0),
            resource_utilization: ResourceUtilization {
                cpu_efficiency: 0.85,
                memory_efficiency: 0.78,
                network_efficiency: 0.92,
                overall_efficiency: 0.85,
            },
        },
    }
}

fn calculate_evaluation_consistency(iterations: &[OptimizationIteration]) -> f64 {
    // Simplified consistency calculation based on score variance across iterations
    if iterations.len() < 2 {
        return 1.0;
    }

    let scores: Vec<f64> = iterations
        .iter()
        .flat_map(|i| i.evaluation_results.iter().map(|r| r.overall_score))
        .collect();

    if scores.is_empty() {
        return 1.0;
    }

    let avg_score: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
    let variance: f64 =
        scores.iter().map(|s| (s - avg_score).powi(2)).sum::<f64>() / scores.len() as f64;

    (1.0 - variance.sqrt()).max(0.3).min(0.98)
}

fn analyze_convergence(improvement_history: &[f64]) -> ConvergenceAnalysis {
    if improvement_history.len() < 2 {
        return ConvergenceAnalysis {
            convergence_rate: 0.0,
            stability_window: 1,
            plateau_detection: PlateauAnalysis {
                plateau_detected: false,
                plateau_duration: 0,
                plateau_threshold: 0.05,
                break_strategy: "Continue optimization".to_string(),
            },
            termination_reason: "Insufficient data".to_string(),
        };
    }

    let total_improvement =
        improvement_history.last().unwrap() - improvement_history.first().unwrap();
    let convergence_rate = total_improvement / improvement_history.len() as f64;

    // Detect plateau (consecutive small improvements)
    let mut plateau_count = 0;
    let plateau_threshold = 0.05;

    for i in 1..improvement_history.len() {
        let improvement = improvement_history[i] - improvement_history[i - 1];
        if improvement.abs() < plateau_threshold {
            plateau_count += 1;
        } else {
            plateau_count = 0;
        }
    }

    let plateau_detected = plateau_count >= 2;

    ConvergenceAnalysis {
        convergence_rate: convergence_rate.abs(),
        stability_window: improvement_history.len(),
        plateau_detection: PlateauAnalysis {
            plateau_detected,
            plateau_duration: plateau_count,
            plateau_threshold,
            break_strategy: if plateau_detected {
                "Increase exploration diversity".to_string()
            } else {
                "Continue current strategy".to_string()
            },
        },
        termination_reason: if plateau_detected {
            "Convergence plateau reached".to_string()
        } else {
            "Maximum iterations completed".to_string()
        },
    }
}

fn calculate_optimization_efficiency(result: &OptimizationResult) -> f64 {
    let improvement_rate = result.optimization_summary.total_improvement
        / result.optimization_summary.total_iterations as f64;
    let convergence_factor = if result.optimization_summary.convergence_achieved {
        1.0
    } else {
        0.8
    };
    let quality_factor = result.optimization_summary.final_quality_score;

    (improvement_rate + convergence_factor + quality_factor) / 3.0
}
