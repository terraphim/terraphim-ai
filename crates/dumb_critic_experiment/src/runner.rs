use crate::ground_truth::GroundTruthGenerator;
use crate::llm_client::LlmClient;
use crate::models::{ModelReview, ModelTier, ReviewConfig};
use crate::scoring::{ExperimentResults, Scorer};
use crate::types::GroundTruthManifest;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// Main experiment runner
pub struct ExperimentRunner {
    llm_client: LlmClient,
    config: ReviewConfig,
    results_dir: String,
}

impl ExperimentRunner {
    pub fn new(llm_client: LlmClient, results_dir: String) -> Self {
        Self {
            llm_client,
            config: ReviewConfig::default(),
            results_dir,
        }
    }

    /// Generate ground truth from plans directory
    pub fn generate_ground_truth(
        &self,
        plans_dir: &Path,
        output_path: &Path,
    ) -> Result<GroundTruthManifest> {
        info!("Generating ground truth from {:?}", plans_dir);

        let manifest = GroundTruthGenerator::generate_from_plans(plans_dir)?;
        GroundTruthGenerator::save_manifest(&manifest, output_path)?;

        info!(
            "Generated ground truth with {} plans, {} total defects",
            manifest.plans.len(),
            manifest.total_defects()
        );

        Ok(manifest)
    }

    /// Load ground truth from file
    pub fn load_ground_truth(&self, path: &Path) -> Result<GroundTruthManifest> {
        GroundTruthGenerator::load_manifest(path)
    }

    /// Run the experiment: review all plans with all models
    pub async fn run_experiment(
        &self,
        manifest: &GroundTruthManifest,
        _plans_dir: &Path,
        models: Option<Vec<ModelTier>>,
    ) -> Result<Vec<ModelReview>> {
        let models_to_test = models.unwrap_or_else(ModelTier::all);
        let total_reviews = manifest.plans.len() * models_to_test.len();

        info!(
            "Running experiment: {} plans x {} models = {} reviews",
            manifest.plans.len(),
            models_to_test.len(),
            total_reviews
        );

        let mut all_reviews = Vec::new();
        let mut completed = 0;

        for plan in &manifest.plans {
            let plan_content = fs::read_to_string(&plan.plan_path)?;

            for model in &models_to_test {
                completed += 1;
                info!(
                    "[{} / {}] Reviewing {} with {}",
                    completed,
                    total_reviews,
                    plan.plan_id,
                    model.display_name()
                );

                match self
                    .llm_client
                    .review_plan(*model, &plan.plan_id, &plan_content, &self.config)
                    .await
                {
                    Ok(review) => {
                        // Save individual review
                        self.save_review(&review)?;
                        all_reviews.push(review);
                    }
                    Err(e) => {
                        warn!(
                            "Failed to review {} with {}: {}",
                            plan.plan_id,
                            model.display_name(),
                            e
                        );
                    }
                }

                // Small delay to avoid rate limits
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }

        info!(
            "Experiment complete: {} reviews collected",
            all_reviews.len()
        );
        Ok(all_reviews)
    }

    /// Resume experiment from saved reviews
    pub async fn resume_experiment(
        &self,
        manifest: &GroundTruthManifest,
        _plans_dir: &Path,
        models: Option<Vec<ModelTier>>,
    ) -> Result<Vec<ModelReview>> {
        let models_to_test = models.unwrap_or_else(ModelTier::all);

        // Load existing reviews
        let existing_reviews = self.load_all_reviews()?;
        let mut completed_pairs: HashMap<(String, String), bool> = HashMap::new();

        for review in &existing_reviews {
            completed_pairs.insert(
                (
                    review.plan_id.clone(),
                    review.model_tier.as_str().to_string(),
                ),
                true,
            );
        }

        info!(
            "Resuming experiment: {} reviews already collected",
            existing_reviews.len()
        );

        let mut all_reviews = existing_reviews;

        for plan in &manifest.plans {
            let plan_content = fs::read_to_string(&plan.plan_path)?;

            for model in &models_to_test {
                let key = (plan.plan_id.clone(), model.as_str().to_string());

                if completed_pairs.contains_key(&key) {
                    info!(
                        "Skipping {} / {} (already completed)",
                        plan.plan_id,
                        model.display_name()
                    );
                    continue;
                }

                info!("Reviewing {} with {}", plan.plan_id, model.display_name());

                match self
                    .llm_client
                    .review_plan(*model, &plan.plan_id, &plan_content, &self.config)
                    .await
                {
                    Ok(review) => {
                        self.save_review(&review)?;
                        all_reviews.push(review);
                    }
                    Err(e) => {
                        warn!(
                            "Failed to review {} with {}: {}",
                            plan.plan_id,
                            model.display_name(),
                            e
                        );
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }

        info!("Experiment complete: {} total reviews", all_reviews.len());
        Ok(all_reviews)
    }

    /// Score all reviews and generate results
    pub fn score_experiment(
        &self,
        manifest: &GroundTruthManifest,
        reviews: &[ModelReview],
    ) -> Result<ExperimentResults> {
        info!("Scoring {} reviews against ground truth", reviews.len());

        let mut all_scores = Vec::new();

        for plan in &manifest.plans {
            let plan_reviews: Vec<&ModelReview> = reviews
                .iter()
                .filter(|r| r.plan_id == plan.plan_id)
                .collect();

            let scores = Scorer::score_plan(plan, &plan_reviews);
            all_scores.extend(scores);
        }

        // Aggregate by tier
        let tier_aggregates = Scorer::aggregate_by_tier(&all_scores);

        // Calculate defect type performance
        let defect_type_performance = self.calculate_defect_type_performance(manifest, &all_scores);

        // Generate conclusion
        let conclusion = Scorer::generate_conclusion(&tier_aggregates);

        let results = ExperimentResults {
            scored_at: chrono::Utc::now().to_rfc3339(),
            plan_scores: all_scores,
            tier_aggregates,
            defect_type_performance,
            conclusion,
        };

        // Save results
        self.save_results(&results)?;

        info!("Scoring complete");
        Ok(results)
    }

    fn calculate_defect_type_performance(
        &self,
        _manifest: &GroundTruthManifest,
        scores: &[crate::scoring::PlanScore],
    ) -> HashMap<String, HashMap<String, crate::scoring::DefectTypeMetrics>> {
        use crate::scoring::DefectTypeMetrics;

        let mut by_tier_and_type: HashMap<String, HashMap<String, (usize, usize)>> = HashMap::new();

        for score in scores {
            let tier_entry = by_tier_and_type
                .entry(score.model_tier.clone())
                .or_default();

            for matched in &score.true_positives {
                let defect_type = matched.ground_truth_defect.defect_type.as_str().to_string();
                let entry = tier_entry.entry(defect_type).or_insert((0, 0));
                entry.0 += 1; // Found
            }
        }

        // Convert to metrics format
        by_tier_and_type
            .into_iter()
            .map(|(tier, type_map)| {
                let metrics_map: HashMap<String, DefectTypeMetrics> = type_map
                    .into_iter()
                    .map(|(defect_type, (found, total))| {
                        let recall = if total > 0 {
                            found as f64 / total as f64
                        } else {
                            0.0
                        };
                        (
                            defect_type.clone(),
                            DefectTypeMetrics {
                                defect_type,
                                recall,
                                count: total,
                            },
                        )
                    })
                    .collect();
                (tier, metrics_map)
            })
            .collect()
    }

    fn save_review(&self, review: &ModelReview) -> Result<()> {
        let filename = format!("{}_{}.json", review.plan_id, review.model_tier.as_str());
        let path = Path::new(&self.results_dir).join("reviews").join(filename);

        fs::create_dir_all(path.parent().unwrap())?;

        let json = serde_json::to_string_pretty(review)?;
        fs::write(path, json)?;

        Ok(())
    }

    fn load_all_reviews(&self) -> Result<Vec<ModelReview>> {
        let reviews_dir = Path::new(&self.results_dir).join("reviews");

        if !reviews_dir.exists() {
            return Ok(Vec::new());
        }

        let mut reviews = Vec::new();

        for entry in fs::read_dir(reviews_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = fs::read_to_string(&path)?;
                let review: ModelReview = serde_json::from_str(&content)?;
                reviews.push(review);
            }
        }

        Ok(reviews)
    }

    fn save_results(&self, results: &ExperimentResults) -> Result<()> {
        let path = Path::new(&self.results_dir).join("experiment_results.json");
        let json = serde_json::to_string_pretty(results)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Generate markdown report
    pub fn generate_report(&self, results: &ExperimentResults, output_path: &Path) -> Result<()> {
        let mut report = String::new();

        report.push_str("# Dumb Critic Experiment Results\n\n");
        report.push_str(&format!("**Scored at:** {}\n\n", results.scored_at));

        // Summary table
        report.push_str("## Summary by Model Tier\n\n");
        report.push_str("| Tier | Avg Recall | Avg Precision | F1 Score | Actionability | Cost (USD) | Cost-Effectiveness |\n");
        report.push_str("|------|------------|---------------|----------|---------------|------------|-------------------|\n");

        let mut tiers: Vec<_> = results.tier_aggregates.values().collect();
        tiers.sort_by(|a, b| a.avg_recall.partial_cmp(&b.avg_recall).unwrap().reverse());

        for tier in tiers {
            report.push_str(&format!(
                "| {} | {:.1}% | {:.1}% | {:.3} | {:.1}% | ${:.4} | {:.1} |\n",
                tier.tier,
                tier.avg_recall * 100.0,
                tier.avg_precision * 100.0,
                tier.avg_f1,
                tier.avg_actionability * 100.0,
                tier.total_cost_usd,
                tier.cost_effectiveness
            ));
        }

        report.push('\n');

        // Conclusion
        report.push_str("## Conclusion\n\n");
        match &results.conclusion {
            crate::scoring::ExperimentConclusion::Confirmed {
                winning_tier,
                evidence,
            } => {
                report.push_str("**Status: CONFIRMED**\n\n");
                report.push_str(&format!("**Winning Tier:** {}\n\n", winning_tier));
                report.push_str(&format!("**Evidence:** {}\n\n", evidence));
                report.push_str("The dumb critic hypothesis is validated: smaller models outperform larger ones for plan review tasks.\n\n");
            }
            crate::scoring::ExperimentConclusion::Refuted { evidence } => {
                report.push_str("**Status: REFUTED**\n\n");
                report.push_str(&format!("**Evidence:** {}\n\n", evidence));
                report.push_str("The dumb critic hypothesis is not supported: larger models perform better.\n\n");
            }
            crate::scoring::ExperimentConclusion::Nuanced { summary, notes } => {
                report.push_str("**Status: NUANCED**\n\n");
                report.push_str(&format!("**Summary:** {}\n\n", summary));
                report.push_str("**Details:**\n");
                for note in notes {
                    report.push_str(&format!("- {}\n", note));
                }
                report.push('\n');
            }
        }

        // Recommendations
        report.push_str("## Recommendations\n\n");
        match &results.conclusion {
            crate::scoring::ExperimentConclusion::Confirmed { winning_tier, .. } => {
                report.push_str(&format!(
                    "1. **Switch to {} for plan reviews** - Achieves higher quality at lower cost\n",
                    winning_tier
                ));
                report.push_str("2. **Update ADF configurations** - Replace Opus-based plan review with the winning tier\n");
                report.push_str("3. **Implement two-phase review** - Small model for plan pass, large model only for code pass\n");
                report.push_str("4. **Encode as design rule** - Document 'dumb critic, smart builder' in ADF guidelines\n");
            }
            crate::scoring::ExperimentConclusion::Refuted { .. } => {
                report.push_str(
                    "1. **Continue using Opus for plan reviews** - Maintains highest quality\n",
                );
                report.push_str("2. **Consider cost optimization** - Evaluate if recall difference justifies cost premium\n");
                report.push_str(
                    "3. **Monitor smaller models** - Re-run experiment as models improve\n",
                );
            }
            crate::scoring::ExperimentConclusion::Nuanced { .. } => {
                report.push_str("1. **Tiered review strategy** - Use different models for different defect types\n");
                report.push_str(
                    "2. **Cost-benefit analysis** - Evaluate tradeoffs for specific use cases\n",
                );
                report.push_str("3. **Further investigation** - Gather more data on edge cases\n");
            }
        }

        fs::write(output_path, report)?;
        info!("Report generated: {:?}", output_path);

        Ok(())
    }
}
