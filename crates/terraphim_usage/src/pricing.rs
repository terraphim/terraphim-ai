use std::path::Path;
use terraphim_types::ModelPricing;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PricingTable {
    pub entries: Vec<ModelPricing>,
}

impl PricingTable {
    pub fn embedded_defaults() -> Self {
        Self {
            entries: vec![
                ModelPricing {
                    model_pattern: "openai/gpt-4o*".into(),
                    input_cost_per_million_tokens: 2.50,
                    output_cost_per_million_tokens: 10.00,
                },
                ModelPricing {
                    model_pattern: "openai/gpt-4o-mini*".into(),
                    input_cost_per_million_tokens: 0.15,
                    output_cost_per_million_tokens: 0.60,
                },
                ModelPricing {
                    model_pattern: "openai/gpt-4-turbo*".into(),
                    input_cost_per_million_tokens: 10.00,
                    output_cost_per_million_tokens: 30.00,
                },
                ModelPricing {
                    model_pattern: "openai/gpt-4".into(),
                    input_cost_per_million_tokens: 30.00,
                    output_cost_per_million_tokens: 60.00,
                },
                ModelPricing {
                    model_pattern: "openai/gpt-3.5-turbo*".into(),
                    input_cost_per_million_tokens: 0.50,
                    output_cost_per_million_tokens: 1.50,
                },
                ModelPricing {
                    model_pattern: "openai/o1*".into(),
                    input_cost_per_million_tokens: 15.00,
                    output_cost_per_million_tokens: 60.00,
                },
                ModelPricing {
                    model_pattern: "openai/o3-mini*".into(),
                    input_cost_per_million_tokens: 1.10,
                    output_cost_per_million_tokens: 4.40,
                },
                ModelPricing {
                    model_pattern: "anthropic/claude-sonnet*".into(),
                    input_cost_per_million_tokens: 3.00,
                    output_cost_per_million_tokens: 15.00,
                },
                ModelPricing {
                    model_pattern: "anthropic/claude-opus*".into(),
                    input_cost_per_million_tokens: 15.00,
                    output_cost_per_million_tokens: 75.00,
                },
                ModelPricing {
                    model_pattern: "anthropic/claude-haiku*".into(),
                    input_cost_per_million_tokens: 0.80,
                    output_cost_per_million_tokens: 4.00,
                },
                ModelPricing {
                    model_pattern: "anthropic/claude-3*".into(),
                    input_cost_per_million_tokens: 3.00,
                    output_cost_per_million_tokens: 15.00,
                },
                ModelPricing {
                    model_pattern: "google/gemini-2.5-pro*".into(),
                    input_cost_per_million_tokens: 1.25,
                    output_cost_per_million_tokens: 10.00,
                },
                ModelPricing {
                    model_pattern: "google/gemini-2.0-flash*".into(),
                    input_cost_per_million_tokens: 0.10,
                    output_cost_per_million_tokens: 0.40,
                },
                ModelPricing {
                    model_pattern: "google/gemini-1.5-pro*".into(),
                    input_cost_per_million_tokens: 1.25,
                    output_cost_per_million_tokens: 5.00,
                },
                ModelPricing {
                    model_pattern: "google/gemini-1.5-flash*".into(),
                    input_cost_per_million_tokens: 0.075,
                    output_cost_per_million_tokens: 0.30,
                },
                ModelPricing {
                    model_pattern: "meta/llama-3.3-70b*".into(),
                    input_cost_per_million_tokens: 0.56,
                    output_cost_per_million_tokens: 0.76,
                },
                ModelPricing {
                    model_pattern: "meta/llama-3.1-405b*".into(),
                    input_cost_per_million_tokens: 2.40,
                    output_cost_per_million_tokens: 2.40,
                },
                ModelPricing {
                    model_pattern: "deepseek/deepseek-chat*".into(),
                    input_cost_per_million_tokens: 0.27,
                    output_cost_per_million_tokens: 1.10,
                },
                ModelPricing {
                    model_pattern: "deepseek/deepseek-r1*".into(),
                    input_cost_per_million_tokens: 0.55,
                    output_cost_per_million_tokens: 2.19,
                },
                ModelPricing {
                    model_pattern: "mistral/mistral-large*".into(),
                    input_cost_per_million_tokens: 2.00,
                    output_cost_per_million_tokens: 6.00,
                },
                ModelPricing {
                    model_pattern: "mistral/mistral-small*".into(),
                    input_cost_per_million_tokens: 0.20,
                    output_cost_per_million_tokens: 0.60,
                },
                ModelPricing {
                    model_pattern: "ollama/*".into(),
                    input_cost_per_million_tokens: 0.0,
                    output_cost_per_million_tokens: 0.0,
                },
            ],
        }
    }

    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<PricingTable>(&content) {
                Ok(table) => {
                    let mut merged = Self::embedded_defaults();
                    merged.entries.extend(table.entries);
                    merged
                }
                Err(e) => {
                    tracing::warn!("Failed to parse pricing TOML {}: {}", path.display(), e);
                    Self::embedded_defaults()
                }
            },
            Err(_) => Self::embedded_defaults(),
        }
    }

    pub fn load_default_path() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        let path = Path::new(&home).join(".config/terraphim/pricing.toml");
        Self::load(&path)
    }

    pub fn find_pricing(&self, model: &str) -> Option<&ModelPricing> {
        let model_lower = model.to_lowercase();
        let mut best_match: Option<&ModelPricing> = None;
        let mut best_prefix_len: usize = 0;

        for entry in &self.entries {
            let pattern = entry.model_pattern.to_lowercase();
            if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                if model_lower.starts_with(prefix) && prefix.len() > best_prefix_len {
                    best_prefix_len = prefix.len();
                    best_match = Some(entry);
                }
            } else if model_lower == pattern {
                return Some(entry);
            }
        }
        best_match
    }

    pub fn calculate_cost(
        &self,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) -> Option<f64> {
        self.find_pricing(model)
            .map(|p| p.calculate_cost(input_tokens, output_tokens))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_defaults_count() {
        let table = PricingTable::embedded_defaults();
        assert!(table.entries.len() >= 20);
    }

    #[test]
    fn test_exact_match() {
        let table = PricingTable::embedded_defaults();
        let p = table.find_pricing("openai/gpt-4").unwrap();
        assert_eq!(p.input_cost_per_million_tokens, 30.0);
    }

    #[test]
    fn test_glob_match() {
        let table = PricingTable::embedded_defaults();
        let p = table.find_pricing("openai/gpt-4o-mini-2024-07-18").unwrap();
        assert_eq!(p.input_cost_per_million_tokens, 0.15);
    }

    #[test]
    fn test_case_insensitive() {
        let table = PricingTable::embedded_defaults();
        assert!(table.find_pricing("OpenAI/GPT-4o-mini").is_some());
    }

    #[test]
    fn test_ollama_free() {
        let table = PricingTable::embedded_defaults();
        let cost = table.calculate_cost("ollama/llama3", 1000, 500).unwrap();
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_unknown_model() {
        let table = PricingTable::embedded_defaults();
        assert!(table.find_pricing("unknown/model").is_none());
    }

    #[test]
    fn test_calculate_cost_known() {
        let table = PricingTable::embedded_defaults();
        let cost = table
            .calculate_cost("anthropic/claude-sonnet-4-20250514", 1_000_000, 500_000)
            .unwrap();
        assert!((cost - 10.50).abs() < 0.001);
    }

    #[test]
    fn test_load_missing_file() {
        let table = PricingTable::load(Path::new("/nonexistent/pricing.toml"));
        assert!(table.entries.len() >= 20);
    }

    #[test]
    fn test_toml_roundtrip() {
        let table = PricingTable::embedded_defaults();
        let toml_str = toml::to_string(&table).unwrap();
        let parsed: PricingTable = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.entries.len(), table.entries.len());
    }
}
