use serde::{Deserialize, Serialize};

/// Model tier for the experiment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    /// GPT-5 Nano (cheapest, smallest)
    Nano,
    /// MiniMax M2.5
    Small,
    /// Kimi K2.5
    Medium,
    /// Claude Sonnet 4.6
    Large,
    /// Claude Opus 4.6 (most expensive, smartest)
    Oracle,
}

#[allow(dead_code)]
impl ModelTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelTier::Nano => "nano",
            ModelTier::Small => "small",
            ModelTier::Medium => "medium",
            ModelTier::Large => "large",
            ModelTier::Oracle => "oracle",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ModelTier::Nano => "GPT-5 Nano",
            ModelTier::Small => "MiniMax M2.5",
            ModelTier::Medium => "Kimi K2.5",
            ModelTier::Large => "Claude Sonnet 4.6",
            ModelTier::Oracle => "Claude Opus 4.6",
        }
    }

    pub fn model_identifier(&self) -> &'static str {
        match self {
            // These are OpenRouter model identifiers
            ModelTier::Nano => "openai/gpt-4o-mini",
            ModelTier::Small => "minimax/minimax-01",
            ModelTier::Medium => "moonshotai/kimi-k2",
            ModelTier::Large => "anthropic/claude-3-5-sonnet",
            ModelTier::Oracle => "anthropic/claude-3-opus",
        }
    }

    /// Cost rank (1 = cheapest)
    pub fn cost_rank(&self) -> u8 {
        match self {
            ModelTier::Nano => 1,
            ModelTier::Small => 2,
            ModelTier::Medium => 3,
            ModelTier::Large => 4,
            ModelTier::Oracle => 5,
        }
    }

    /// Intelligence rank (1 = smallest)
    pub fn intelligence_rank(&self) -> u8 {
        match self {
            ModelTier::Nano => 1,
            ModelTier::Small => 2,
            ModelTier::Medium => 3,
            ModelTier::Large => 4,
            ModelTier::Oracle => 5,
        }
    }

    /// Estimated cost per 1K input tokens (USD)
    pub fn estimated_input_cost_per_1k(&self) -> f64 {
        match self {
            ModelTier::Nano => 0.00015,
            ModelTier::Small => 0.0005,
            ModelTier::Medium => 0.001,
            ModelTier::Large => 0.003,
            ModelTier::Oracle => 0.015,
        }
    }

    /// Estimated cost per 1K output tokens (USD)
    pub fn estimated_output_cost_per_1k(&self) -> f64 {
        match self {
            ModelTier::Nano => 0.0006,
            ModelTier::Small => 0.0015,
            ModelTier::Medium => 0.004,
            ModelTier::Large => 0.015,
            ModelTier::Oracle => 0.075,
        }
    }

    /// All tiers in order of cost
    pub fn all() -> Vec<ModelTier> {
        vec![
            ModelTier::Nano,
            ModelTier::Small,
            ModelTier::Medium,
            ModelTier::Large,
            ModelTier::Oracle,
        ]
    }
}

/// Configuration for model reviews
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewConfig {
    /// Temperature for generation (lower = more focused)
    pub temperature: f64,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// System prompt for the review
    pub system_prompt: String,
    /// Whether to allow the model to refuse
    pub allow_refusal: bool,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            temperature: 0.3,
            max_tokens: 2000,
            system_prompt: default_system_prompt(),
            allow_refusal: false,
        }
    }
}

/// Default skeptical system prompt for plan reviews
fn default_system_prompt() -> String {
    r#"You are a code reviewer analyzing software implementation plans. Your job is to identify defects and problems. Be skeptical and critical. Do NOT praise or compliment. Focus ONLY on finding issues.

For each defect you find, provide:
1. Defect type (one of: missing_prerequisite, ambiguous_acceptance_criteria, wrong_ordering, scope_creep, missing_rollback, contradictory_statements, stale_reference)
2. Location in the plan (section, line, or paragraph)
3. Clear description of the problem
4. Suggested fix

If you find no defects, explicitly state "No defects found."

Output format:
DEFECT 1:
- Type: <type>
- Location: <location>
- Description: <description>
- Suggested fix: <fix>

Continue for all defects found."#.to_string()
}

/// Review result from a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelReview {
    /// Model tier that produced this review
    pub model_tier: ModelTier,
    /// Plan being reviewed
    pub plan_id: String,
    /// Raw text output from the model
    pub raw_output: String,
    /// Parsed defects found by the model
    pub parsed_defects: Vec<ParsedDefect>,
    /// Token usage (if available)
    pub token_usage: Option<TokenUsage>,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Timestamp of the review
    pub reviewed_at: String,
}

/// A defect as parsed from model output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDefect {
    /// Defect type (as reported by model)
    pub defect_type: String,
    /// Location in the plan
    pub location: String,
    /// Description of the defect
    pub description: String,
    /// Suggested fix
    pub suggested_fix: String,
    /// Whether this defect matched a ground truth defect
    pub matches_ground_truth: Option<bool>,
    /// Which ground truth defect it matched (if any)
    pub matched_gt_id: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    /// Estimated cost in USD
    pub fn estimated_cost(&self, tier: &ModelTier) -> f64 {
        let input_cost = (self.prompt_tokens as f64 / 1000.0) * tier.estimated_input_cost_per_1k();
        let output_cost =
            (self.completion_tokens as f64 / 1000.0) * tier.estimated_output_cost_per_1k();
        input_cost + output_cost
    }
}
