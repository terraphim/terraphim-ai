/// Configuration for the knowledge-graph path scorer.
#[derive(Debug, Clone)]
pub struct KgScorerConfig {
    /// Score added per unique matched term.
    pub weight_per_term: i32,
    /// Maximum boost the scorer can contribute regardless of match count.
    pub max_boost: i32,
}

impl Default for KgScorerConfig {
    fn default() -> Self {
        Self {
            weight_per_term: 5,
            max_boost: 30,
        }
    }
}
