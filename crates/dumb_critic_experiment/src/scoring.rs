use crate::models::{ModelReview, ParsedDefect};
use crate::types::{Defect, PlanGroundTruth};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Scoring result for a single model on a single plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanScore {
    /// Model tier
    pub model_tier: String,
    /// Plan ID
    pub plan_id: String,
    /// True positives (correctly identified defects)
    pub true_positives: Vec<MatchedDefect>,
    /// False positives (reported defects that don't exist in ground truth)
    pub false_positives: Vec<ParsedDefect>,
    /// False negatives (ground truth defects not found)
    pub false_negatives: Vec<Defect>,
    /// Defects with unclear matches (need human review)
    pub unclear_matches: Vec<ParsedDefect>,
    /// Recall: TP / (TP + FN)
    pub recall: f64,
    /// Precision: TP / (TP + FP)
    pub precision: f64,
    /// F1 score: 2 * (precision * recall) / (precision + recall)
    pub f1_score: f64,
    /// Actionability score: fraction of TPs with clear fixes
    pub actionability: f64,
    /// Whether the model praised despite instructions
    pub praise_contamination: bool,
    /// Cost in USD (estimated)
    pub cost_usd: f64,
    /// Latency in milliseconds
    pub latency_ms: u64,
}

/// A matched defect (TP)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedDefect {
    /// The defect from ground truth
    pub ground_truth_defect: Defect,
    /// The defect found by the model
    pub model_defect: ParsedDefect,
    /// Match confidence score (0.0-1.0)
    pub match_score: f64,
    /// Whether the fix suggestion is actionable
    pub is_actionable: bool,
}

/// Complete scoring results for the experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResults {
    /// Timestamp of scoring
    pub scored_at: String,
    /// Individual plan scores
    pub plan_scores: Vec<PlanScore>,
    /// Aggregated scores by model tier
    pub tier_aggregates: HashMap<String, TierAggregate>,
    /// Defect type performance by tier
    pub defect_type_performance: HashMap<String, HashMap<String, DefectTypeMetrics>>,
    /// Overall conclusion
    pub conclusion: ExperimentConclusion,
}

/// Aggregated metrics for a model tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierAggregate {
    pub tier: String,
    pub total_reviews: usize,
    pub avg_recall: f64,
    pub avg_precision: f64,
    pub avg_f1: f64,
    pub avg_actionability: f64,
    pub total_cost_usd: f64,
    pub avg_latency_ms: f64,
    pub cost_effectiveness: f64, // defects found per dollar
    pub praise_contamination_rate: f64,
}

/// Metrics for a specific defect type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefectTypeMetrics {
    pub defect_type: String,
    pub recall: f64,
    pub count: usize,
}

/// Experiment conclusion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExperimentConclusion {
    /// Confirmed: Smaller model(s) outperform larger ones
    #[serde(rename = "confirmed")]
    Confirmed {
        winning_tier: String,
        evidence: String,
    },
    /// Refuted: Largest model strictly outperforms
    #[serde(rename = "refuted")]
    Refuted { evidence: String },
    /// Nuanced: Different models excel at different things
    #[serde(rename = "nuanced")]
    Nuanced { summary: String, notes: Vec<String> },
}

/// Scores model reviews against ground truth
pub struct Scorer;

impl Scorer {
    /// Score all reviews for a plan
    pub fn score_plan(ground_truth: &PlanGroundTruth, reviews: &[&ModelReview]) -> Vec<PlanScore> {
        reviews
            .iter()
            .map(|review| Self::score_single_review(ground_truth, review))
            .collect()
    }

    /// Score a single model review
    fn score_single_review(ground_truth: &PlanGroundTruth, review: &ModelReview) -> PlanScore {
        let mut true_positives = Vec::new();
        let mut false_positives = Vec::new();
        let mut false_negatives = Vec::new();
        let mut unclear_matches = Vec::new();

        // Track which GT defects have been matched
        let mut gt_matched: Vec<bool> = vec![false; ground_truth.defects.len()];

        // Try to match each model-reported defect to ground truth
        for model_defect in &review.parsed_defects {
            let match_result = Self::find_best_match(model_defect, ground_truth, &gt_matched);

            match match_result {
                MatchResult::TruePositive { gt_index, score } => {
                    gt_matched[gt_index] = true;
                    let is_actionable = Self::is_actionable(model_defect);
                    true_positives.push(MatchedDefect {
                        ground_truth_defect: ground_truth.defects[gt_index].clone(),
                        model_defect: model_defect.clone(),
                        match_score: score,
                        is_actionable,
                    });
                }
                MatchResult::FalsePositive => {
                    false_positives.push(model_defect.clone());
                }
                MatchResult::Unclear => {
                    unclear_matches.push(model_defect.clone());
                }
            }
        }

        // Find false negatives (GT defects not matched)
        for (idx, matched) in gt_matched.iter().enumerate() {
            if !matched {
                false_negatives.push(ground_truth.defects[idx].clone());
            }
        }

        // Calculate metrics
        let tp_count = true_positives.len() as f64;
        let fp_count = false_positives.len() as f64;
        let fn_count = false_negatives.len() as f64;

        let recall = if tp_count + fn_count > 0.0 {
            tp_count / (tp_count + fn_count)
        } else {
            0.0
        };

        let precision = if tp_count + fp_count > 0.0 {
            tp_count / (tp_count + fp_count)
        } else {
            0.0
        };

        let f1_score = if recall + precision > 0.0 {
            2.0 * (recall * precision) / (recall + precision)
        } else {
            0.0
        };

        let actionable_count = true_positives.iter().filter(|m| m.is_actionable).count() as f64;
        let actionability = if tp_count > 0.0 {
            actionable_count / tp_count
        } else {
            0.0
        };

        let cost_usd = review
            .token_usage
            .as_ref()
            .map(|u| u.estimated_cost(&review.model_tier))
            .unwrap_or(0.0);

        let praise_contamination = Self::detect_praise(&review.raw_output);

        PlanScore {
            model_tier: review.model_tier.as_str().to_string(),
            plan_id: review.plan_id.clone(),
            true_positives,
            false_positives,
            false_negatives,
            unclear_matches,
            recall,
            precision,
            f1_score,
            actionability,
            praise_contamination,
            cost_usd,
            latency_ms: review.latency_ms,
        }
    }

    /// Find best match for a model defect in ground truth
    fn find_best_match(
        model_defect: &ParsedDefect,
        ground_truth: &PlanGroundTruth,
        already_matched: &[bool],
    ) -> MatchResult {
        let mut best_match: Option<(usize, f64)> = None;

        for (idx, gt_defect) in ground_truth.defects.iter().enumerate() {
            if already_matched[idx] {
                continue;
            }

            let score = Self::calculate_match_score(model_defect, gt_defect);

            if score >= 0.6 {
                // Good match threshold
                if best_match.map(|(_, s)| score > s).unwrap_or(true) {
                    best_match = Some((idx, score));
                }
            }
        }

        match best_match {
            Some((idx, score)) if score >= 0.8 => MatchResult::TruePositive {
                gt_index: idx,
                score,
            },
            Some((idx, score)) => MatchResult::TruePositive {
                gt_index: idx,
                score,
            },
            None => {
                // Check if it might be a valid finding not in ground truth
                let model_desc = model_defect.description.to_ascii_lowercase();
                let is_likely_valid = model_desc.len() > 20
                    && !model_desc.contains("no defects")
                    && !model_desc.contains("none found");

                if is_likely_valid {
                    MatchResult::Unclear
                } else {
                    MatchResult::FalsePositive
                }
            }
        }
    }

    /// Calculate similarity score between model defect and ground truth
    fn calculate_match_score(model: &ParsedDefect, ground_truth: &Defect) -> f64 {
        let mut score = 0.0;

        // Type matching (40% weight)
        let model_type = model.defect_type.to_ascii_lowercase();
        let gt_type = ground_truth.defect_type.as_str().to_ascii_lowercase();
        if model_type == gt_type || model_type.contains(&gt_type) || gt_type.contains(&model_type) {
            score += 0.4;
        }

        // Location matching (20% weight)
        if model.location == ground_truth.location.as_deref().unwrap_or("") {
            score += 0.2;
        }

        // Description similarity (40% weight) - simple keyword overlap
        let model_words: std::collections::HashSet<String> = model
            .description
            .to_ascii_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let gt_words: std::collections::HashSet<String> = ground_truth
            .description
            .to_ascii_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if !model_words.is_empty() && !gt_words.is_empty() {
            let intersection: std::collections::HashSet<_> =
                model_words.intersection(&gt_words).collect();
            let union_size = model_words.union(&gt_words).count() as f64;
            if union_size > 0.0 {
                let jaccard = intersection.len() as f64 / union_size;
                score += 0.4 * jaccard;
            }
        }

        score
    }

    /// Check if a fix suggestion is actionable
    fn is_actionable(defect: &ParsedDefect) -> bool {
        let fix = defect.suggested_fix.to_ascii_lowercase();
        fix.len() > 10
            && !fix.contains("review")
            && !fix.contains("consider")
            && !fix.contains("maybe")
            && fix.contains(char::is_alphabetic)
    }

    /// Detect praise contamination (model complimenting despite instructions)
    fn detect_praise(output: &str) -> bool {
        let output_lower = output.to_ascii_lowercase();
        let praise_phrases = [
            "well written",
            "good job",
            "excellent",
            "great work",
            "nicely done",
            "impressive",
            "well structured",
            "well organized",
            "clear and concise",
            "comprehensive",
            "thorough",
        ];

        praise_phrases
            .iter()
            .any(|phrase| output_lower.contains(phrase))
    }

    /// Aggregate scores by tier
    pub fn aggregate_by_tier(plan_scores: &[PlanScore]) -> HashMap<String, TierAggregate> {
        let mut by_tier: HashMap<String, Vec<&PlanScore>> = HashMap::new();

        for score in plan_scores {
            by_tier
                .entry(score.model_tier.clone())
                .or_default()
                .push(score);
        }

        by_tier
            .iter()
            .map(|(tier, scores)| {
                let total_reviews = scores.len();
                let avg_recall =
                    scores.iter().map(|s| s.recall).sum::<f64>() / total_reviews as f64;
                let avg_precision =
                    scores.iter().map(|s| s.precision).sum::<f64>() / total_reviews as f64;
                let avg_f1 = scores.iter().map(|s| s.f1_score).sum::<f64>() / total_reviews as f64;
                let avg_actionability =
                    scores.iter().map(|s| s.actionability).sum::<f64>() / total_reviews as f64;
                let total_cost = scores.iter().map(|s| s.cost_usd).sum::<f64>();
                let avg_latency =
                    scores.iter().map(|s| s.latency_ms).sum::<u64>() as f64 / total_reviews as f64;
                let total_tps: usize = scores.iter().map(|s| s.true_positives.len()).sum();
                let cost_effectiveness = if total_cost > 0.0 {
                    total_tps as f64 / total_cost
                } else {
                    0.0
                };
                let praise_count = scores.iter().filter(|s| s.praise_contamination).count();
                let praise_rate = praise_count as f64 / total_reviews as f64;

                (
                    tier.clone(),
                    TierAggregate {
                        tier: tier.clone(),
                        total_reviews,
                        avg_recall,
                        avg_precision,
                        avg_f1,
                        avg_actionability,
                        total_cost_usd: total_cost,
                        avg_latency_ms: avg_latency,
                        cost_effectiveness,
                        praise_contamination_rate: praise_rate,
                    },
                )
            })
            .collect()
    }

    /// Generate experiment conclusion from aggregated results
    pub fn generate_conclusion(
        tier_aggregates: &HashMap<String, TierAggregate>,
    ) -> ExperimentConclusion {
        // Find best model by recall
        let best_recall = tier_aggregates
            .values()
            .max_by(|a, b| a.avg_recall.partial_cmp(&b.avg_recall).unwrap());

        // Find best model by cost-effectiveness
        let best_cost_eff = tier_aggregates.values().max_by(|a, b| {
            a.cost_effectiveness
                .partial_cmp(&b.cost_effectiveness)
                .unwrap()
        });

        // Find oracle (largest model)
        let oracle = tier_aggregates.get("oracle");

        match (best_recall, best_cost_eff, oracle) {
            (Some(best), _, Some(oracle))
                if best.avg_recall > oracle.avg_recall && best.tier != "oracle" =>
            {
                let cost_eff_ratio =
                    best.cost_effectiveness / oracle.cost_effectiveness.max(0.0001);
                if cost_eff_ratio > 3.0 {
                    ExperimentConclusion::Confirmed {
                        winning_tier: best.tier.clone(),
                        evidence: format!(
                            "{} achieved {:.1}% recall vs oracle's {:.1}%, with {:.1}x cost-effectiveness",
                            best.tier,
                            best.avg_recall * 100.0,
                            oracle.avg_recall * 100.0,
                            cost_eff_ratio
                        ),
                    }
                } else {
                    ExperimentConclusion::Nuanced {
                        summary: "Mixed results across metrics".to_string(),
                        notes: vec![
                            format!(
                                "{} has highest recall ({:.1}%)",
                                best.tier,
                                best.avg_recall * 100.0
                            ),
                            format!("Oracle recall: {:.1}%", oracle.avg_recall * 100.0),
                            format!("Cost-effectiveness ratio: {:.1}x", cost_eff_ratio),
                        ],
                    }
                }
            }
            (_, _, Some(oracle)) => {
                // Check if oracle strictly outperforms all others
                let oracle_strictly_best = tier_aggregates
                    .values()
                    .filter(|t| t.tier != "oracle")
                    .all(|t| {
                        oracle.avg_recall >= t.avg_recall && oracle.avg_precision >= t.avg_precision
                    });

                if oracle_strictly_best {
                    ExperimentConclusion::Refuted {
                        evidence: format!(
                            "Oracle achieved highest recall ({:.1}%) and precision ({:.1}%)",
                            oracle.avg_recall * 100.0,
                            oracle.avg_precision * 100.0
                        ),
                    }
                } else {
                    ExperimentConclusion::Nuanced {
                        summary: "Different models excel at different metrics".to_string(),
                        notes: tier_aggregates
                            .values()
                            .map(|t| {
                                format!(
                                    "{}: recall={:.1}%, precision={:.1}%",
                                    t.tier,
                                    t.avg_recall * 100.0,
                                    t.avg_precision * 100.0
                                )
                            })
                            .collect(),
                    }
                }
            }
            _ => ExperimentConclusion::Nuanced {
                summary: "Results inconclusive".to_string(),
                notes: vec!["Insufficient data to determine conclusion".to_string()],
            },
        }
    }
}

enum MatchResult {
    TruePositive { gt_index: usize, score: f64 },
    FalsePositive,
    Unclear,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_match_score() {
        let model_defect = ParsedDefect {
            defect_type: "missing_prerequisite".to_string(),
            location: "Step 3".to_string(),
            description: "Missing database setup step".to_string(),
            suggested_fix: "Add setup step".to_string(),
            matches_ground_truth: None,
            matched_gt_id: None,
        };

        let gt_defect = Defect {
            id: "test-1".to_string(),
            defect_type: crate::types::DefectType::MissingPrerequisite,
            location: Some("Step 3".to_string()),
            description: "Missing database setup step".to_string(),
            is_seeded: true,
            expected_fix: "Add setup".to_string(),
        };

        let score = Scorer::calculate_match_score(&model_defect, &gt_defect);
        assert!(score > 0.8, "Expected high match score, got {}", score);
    }

    #[test]
    fn test_detect_praise() {
        assert!(Scorer::detect_praise(
            "This is well written and comprehensive"
        ));
        assert!(!Scorer::detect_praise("Several issues found in the code"));
    }
}
