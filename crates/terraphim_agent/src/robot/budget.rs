#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::output::{FieldMode, RobotConfig, RobotFormatter};
use super::schema::{Pagination, SearchResultItem, TokenBudget};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetedResults {
    pub results: Vec<serde_json::Value>,
    pub pagination: Pagination,
    pub token_budget: Option<TokenBudget>,
}

#[derive(Debug, thiserror::Error)]
pub enum BudgetError {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub struct BudgetEngine {
    config: RobotConfig,
    formatter: RobotFormatter,
}

const KNOWN_FIELDS: &[&str] = &[
    "rank",
    "id",
    "title",
    "url",
    "score",
    "preview",
    "source",
    "date",
    "preview_truncated",
];

impl BudgetEngine {
    pub fn new(config: RobotConfig) -> Self {
        let formatter = RobotFormatter::new(config.clone());
        Self { config, formatter }
    }

    pub fn apply(&self, results: &[SearchResultItem]) -> Result<BudgetedResults, BudgetError> {
        let total = results.len();

        let truncated: Vec<SearchResultItem> = results
            .iter()
            .map(|item| self.truncate_item(item))
            .collect();

        let filtered: Vec<serde_json::Value> = truncated
            .iter()
            .map(|item| filter_fields(item, &self.config.fields))
            .collect();

        let (capped, was_capped_by_results) = self.apply_max_results(&filtered);

        let (budgeted, token_budget) = self.apply_token_budget(&capped);

        let was_truncated =
            was_capped_by_results || token_budget.as_ref().is_some_and(|tb| tb.truncated);

        let pagination = Pagination::new(total, budgeted.len(), 0);

        Ok(BudgetedResults {
            results: budgeted,
            pagination,
            token_budget: token_budget.map(|mut tb| {
                tb.truncated = was_truncated;
                tb
            }),
        })
    }

    fn truncate_item(&self, item: &SearchResultItem) -> SearchResultItem {
        let mut item = item.clone();
        if let Some(ref preview) = item.preview {
            let (truncated, was_truncated) = self.formatter.truncate_content(preview);
            if was_truncated {
                item.preview = Some(truncated);
                item.preview_truncated = true;
            }
        }
        item
    }

    fn apply_max_results(&self, items: &[serde_json::Value]) -> (Vec<serde_json::Value>, bool) {
        if let Some(max) = self.config.max_results {
            if items.len() > max {
                return (items[..max].to_vec(), true);
            }
        }
        (items.to_vec(), false)
    }

    fn apply_token_budget(
        &self,
        items: &[serde_json::Value],
    ) -> (Vec<serde_json::Value>, Option<TokenBudget>) {
        let max_tokens = match self.config.max_tokens {
            Some(mt) => mt,
            None => return (items.to_vec(), None),
        };

        let mut output = Vec::new();
        let mut used_tokens = 0usize;

        for item in items {
            let serialized = serde_json::to_string(item).unwrap_or_default();
            let tokens = self.formatter.estimate_tokens(&serialized);

            if used_tokens + tokens > max_tokens && !output.is_empty() {
                break;
            }

            used_tokens += tokens;
            output.push(item.clone());
        }

        let budget = TokenBudget::new(max_tokens).with_estimate(used_tokens);
        (output, Some(budget))
    }
}

fn fields_for_mode(mode: &FieldMode) -> Vec<&'static str> {
    match mode {
        FieldMode::Full => KNOWN_FIELDS.to_vec(),
        FieldMode::Summary => vec!["rank", "id", "title", "url", "score"],
        FieldMode::Minimal => vec!["rank", "id", "title", "score"],
        FieldMode::Custom(fields) => fields
            .iter()
            .filter_map(|f| {
                let f_lower = f.to_lowercase();
                KNOWN_FIELDS.iter().find(|kf| kf == &&f_lower).copied()
            })
            .collect(),
    }
}

fn filter_fields(item: &SearchResultItem, mode: &FieldMode) -> serde_json::Value {
    let mut value = serde_json::to_value(item).unwrap_or(serde_json::Value::Null);

    let allowed = fields_for_mode(mode);

    if let serde_json::Value::Object(ref mut map) = value {
        map.retain(|k, _| allowed.contains(&k.as_str()));
    }

    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item() -> SearchResultItem {
        SearchResultItem {
            rank: 1,
            id: "test-001".to_string(),
            title: "Test Result".to_string(),
            url: Some("https://example.com".to_string()),
            score: 0.95,
            preview: Some("This is a preview of the search result content".to_string()),
            source: Some("claude-code".to_string()),
            date: Some("2026-04-22".to_string()),
            preview_truncated: false,
        }
    }

    fn many_items(count: usize) -> Vec<SearchResultItem> {
        (0..count)
            .map(|i| SearchResultItem {
                rank: i + 1,
                id: format!("test-{:03}", i),
                title: format!("Result {}", i),
                url: Some(format!("https://example.com/{}", i)),
                score: 1.0 - (i as f64 * 0.01),
                preview: Some(format!("Preview content for result number {}", i)),
                source: Some("claude-code".to_string()),
                date: Some("2026-04-22".to_string()),
                preview_truncated: false,
            })
            .collect()
    }

    #[test]
    fn test_field_mode_full_returns_all_fields() {
        let item = SearchResultItem {
            preview_truncated: true,
            ..sample_item()
        };
        let value = filter_fields(&item, &FieldMode::Full);

        for field in KNOWN_FIELDS {
            assert!(
                value.get(field).is_some(),
                "Full mode should include field '{}'",
                field
            );
        }
    }

    #[test]
    fn test_field_mode_summary_excludes_preview() {
        let item = sample_item();
        let value = filter_fields(&item, &FieldMode::Summary);

        assert!(value.get("rank").is_some());
        assert!(value.get("id").is_some());
        assert!(value.get("title").is_some());
        assert!(value.get("url").is_some());
        assert!(value.get("score").is_some());

        assert!(value.get("preview").is_none());
        assert!(value.get("source").is_none());
        assert!(value.get("date").is_none());
        assert!(value.get("preview_truncated").is_none());
    }

    #[test]
    fn test_field_mode_minimal_only_core() {
        let item = sample_item();
        let value = filter_fields(&item, &FieldMode::Minimal);

        assert!(value.get("rank").is_some());
        assert!(value.get("id").is_some());
        assert!(value.get("title").is_some());
        assert!(value.get("score").is_some());

        assert!(value.get("url").is_none());
        assert!(value.get("preview").is_none());
    }

    #[test]
    fn test_field_mode_custom_selects_specified() {
        let item = sample_item();
        let value = filter_fields(
            &item,
            &FieldMode::Custom(vec!["title".to_string(), "score".to_string()]),
        );

        assert!(value.get("title").is_some());
        assert!(value.get("score").is_some());
        assert!(value.get("rank").is_none());
        assert!(value.get("id").is_none());
    }

    #[test]
    fn test_field_mode_custom_ignores_unknown() {
        let item = sample_item();
        let value = filter_fields(
            &item,
            &FieldMode::Custom(vec!["title".to_string(), "nonexistent".to_string()]),
        );

        assert!(value.get("title").is_some());
        assert_eq!(value.as_object().unwrap().len(), 1);
    }

    #[test]
    fn test_truncate_content_marks_truncated() {
        let config = RobotConfig::new().with_max_content_length(10);
        let engine = BudgetEngine::new(config);

        let item = SearchResultItem {
            preview: Some("This is definitely longer than ten characters".to_string()),
            ..sample_item()
        };

        let result = engine.truncate_item(&item);
        assert!(result.preview_truncated);
        assert!(result.preview.unwrap().ends_with("..."));
    }

    #[test]
    fn test_truncate_content_short_unchanged() {
        let config = RobotConfig::new().with_max_content_length(1000);
        let engine = BudgetEngine::new(config);

        let item = sample_item();
        let result = engine.truncate_item(&item);
        assert!(!result.preview_truncated);
        assert_eq!(result.preview, item.preview);
    }

    #[test]
    fn test_max_results_limits_count() {
        let config = RobotConfig::new().with_max_results(3);
        let engine = BudgetEngine::new(config);

        let results = many_items(10);
        let output = engine.apply(&results).unwrap();

        assert_eq!(output.results.len(), 3);
        assert_eq!(output.pagination.total, 10);
        assert_eq!(output.pagination.returned, 3);
        assert!(output.pagination.has_more);
    }

    #[test]
    fn test_max_tokens_progressive_budget() {
        let config = RobotConfig::new().with_max_tokens(5);
        let engine = BudgetEngine::new(config);

        let results = many_items(100);
        let output = engine.apply(&results).unwrap();

        assert!(output.results.len() < 100);
        assert!(output.token_budget.is_some());
        let tb = output.token_budget.unwrap();
        assert!(tb.truncated);
    }

    #[test]
    fn test_max_tokens_includes_partial_results() {
        let config = RobotConfig::new().with_max_tokens(1000);
        let engine = BudgetEngine::new(config);

        let results = many_items(3);
        let output = engine.apply(&results).unwrap();

        assert_eq!(output.results.len(), 3);
        let tb = output.token_budget.unwrap();
        assert!(!tb.truncated);
    }

    #[test]
    fn test_no_budget_returns_all() {
        let config = RobotConfig::new();
        let engine = BudgetEngine::new(config);

        let results = many_items(10);
        let output = engine.apply(&results).unwrap();

        assert_eq!(output.results.len(), 10);
        assert!(output.token_budget.is_none());
    }

    #[test]
    fn test_pagination_metadata_populated() {
        let mut config = RobotConfig::new();
        config.max_results = None;
        let engine = BudgetEngine::new(config);

        let results = many_items(25);
        let output = engine.apply(&results).unwrap();

        assert_eq!(output.pagination.total, 25);
        assert_eq!(output.pagination.returned, 25);
        assert_eq!(output.pagination.offset, 0);
        assert!(!output.pagination.has_more);
    }

    #[test]
    fn test_token_budget_metadata_populated() {
        let config = RobotConfig::new().with_max_tokens(500);
        let engine = BudgetEngine::new(config);

        let results = many_items(5);
        let output = engine.apply(&results).unwrap();

        let tb = output.token_budget.unwrap();
        assert_eq!(tb.max_tokens, 500);
        assert!(tb.estimated_tokens > 0);
    }

    #[test]
    fn test_token_budget_truncated_flag() {
        let config = RobotConfig::new().with_max_tokens(2);
        let engine = BudgetEngine::new(config);

        let results = many_items(10);
        let output = engine.apply(&results).unwrap();

        let tb = output.token_budget.unwrap();
        assert!(tb.truncated);
    }

    #[test]
    fn test_empty_results() {
        let config = RobotConfig::new();
        let engine = BudgetEngine::new(config);

        let output = engine.apply(&[]).unwrap();

        assert_eq!(output.results.len(), 0);
        assert_eq!(output.pagination.total, 0);
        assert_eq!(output.pagination.returned, 0);
        assert!(!output.pagination.has_more);
    }

    #[test]
    fn test_custom_fields_includes_preview_truncated_with_preview() {
        let config = RobotConfig::new().with_fields(FieldMode::Custom(vec![
            "preview".to_string(),
            "title".to_string(),
        ]));
        let engine = BudgetEngine::new(config);

        let item = SearchResultItem {
            preview: Some("Short".to_string()),
            ..sample_item()
        };
        let output = engine.apply(&[item]).unwrap();

        let result = &output.results[0];
        assert!(result.get("title").is_some());
        assert!(result.get("preview").is_some());
        assert!(result.get("rank").is_none());
    }

    #[test]
    fn test_combined_max_results_and_tokens() {
        let config = RobotConfig::new().with_max_results(5).with_max_tokens(10);
        let engine = BudgetEngine::new(config);

        let results = many_items(20);
        let output = engine.apply(&results).unwrap();

        assert!(output.results.len() <= 5);
        let tb = output.token_budget.unwrap();
        assert!(tb.truncated);
    }
}
