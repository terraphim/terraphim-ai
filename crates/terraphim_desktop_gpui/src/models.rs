use serde::{Deserialize, Serialize};
use terraphim_types::Document;

/// Term chip for multi-term queries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TermChip {
    pub value: String,
    pub is_from_kg: bool,
}

/// Term chip collection with operator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TermChipSet {
    pub chips: Vec<TermChip>,
    pub operator: Option<ChipOperator>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChipOperator {
    And,
    Or,
}

impl TermChipSet {
    pub fn new() -> Self {
        Self {
            chips: vec![],
            operator: None,
        }
    }

    pub fn add_chip(&mut self, chip: TermChip) {
        self.chips.push(chip);

        // Set default operator if we have multiple chips
        if self.chips.len() > 1 && self.operator.is_none() {
            self.operator = Some(ChipOperator::And);
        }
    }

    pub fn remove_chip(&mut self, index: usize) {
        if index < self.chips.len() {
            self.chips.remove(index);

            // Clear operator if only one chip left
            if self.chips.len() <= 1 {
                self.operator = None;
            }
        }
    }

    pub fn clear(&mut self) {
        self.chips.clear();
        self.operator = None;
    }

    pub fn to_query_string(&self) -> String {
        if self.chips.is_empty() {
            return String::new();
        }

        if self.chips.len() == 1 {
            return self.chips[0].value.clone();
        }

        let operator_str = match self.operator {
            Some(ChipOperator::And) => " AND ",
            Some(ChipOperator::Or) => " OR ",
            None => " AND ",
        };

        self.chips
            .iter()
            .map(|c| c.value.as_str())
            .collect::<Vec<_>>()
            .join(operator_str)
    }

    pub fn from_query_string(query: &str, is_kg_check: impl Fn(&str) -> bool) -> Self {
        let query_lower = query.to_lowercase();

        let (operator, terms) = if query_lower.contains(" and ") {
            (
                Some(ChipOperator::And),
                query_lower
                    .split(" and ")
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>(),
            )
        } else if query_lower.contains(" or ") {
            (
                Some(ChipOperator::Or),
                query_lower
                    .split(" or ")
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>(),
            )
        } else {
            (None, vec![query.trim()])
        };

        let chips = terms
            .into_iter()
            .map(|term| TermChip {
                value: term.to_string(),
                is_from_kg: is_kg_check(term),
            })
            .collect();

        Self { chips, operator }
    }
}

impl Default for TermChipSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Result item view model
#[derive(Clone, Debug)]
pub struct ResultItemViewModel {
    pub document: Document,
    pub highlighted_title: String,
    pub highlighted_description: String,
    pub show_details: bool,
}

impl From<Document> for ResultItemViewModel {
    fn from(document: Document) -> Self {
        Self {
            highlighted_title: document.title.clone(),
            highlighted_description: document
                .description
                .clone()
                .unwrap_or_else(|| "No description".to_string()),
            document,
            show_details: false,
        }
    }
}

impl ResultItemViewModel {
    pub fn new(document: Document) -> Self {
        Self::from(document)
    }

    pub fn with_highlights(mut self, query: &str) -> Self {
        // Simple highlighting - wrap matches in markers
        let query_lower = query.to_lowercase();

        if self.document.title.to_lowercase().contains(&query_lower) {
            self.highlighted_title = self
                .document
                .title
                .replace(&query, &format!("**{}**", query));
        }

        if let Some(desc) = &self.document.description {
            if desc.to_lowercase().contains(&query_lower) {
                self.highlighted_description =
                    desc.replace(&query, &format!("**{}**", query));
            }
        }

        self
    }

    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_chip_set_add_remove() {
        let mut set = TermChipSet::new();

        set.add_chip(TermChip {
            value: "rust".to_string(),
            is_from_kg: true,
        });

        assert_eq!(set.chips.len(), 1);
        assert!(set.operator.is_none());

        set.add_chip(TermChip {
            value: "tokio".to_string(),
            is_from_kg: true,
        });

        assert_eq!(set.chips.len(), 2);
        assert_eq!(set.operator, Some(ChipOperator::And));

        set.remove_chip(0);
        assert_eq!(set.chips.len(), 1);
        assert!(set.operator.is_none());
    }

    #[test]
    fn test_query_string_conversion() {
        let mut set = TermChipSet::new();
        set.add_chip(TermChip {
            value: "rust".to_string(),
            is_from_kg: true,
        });
        set.add_chip(TermChip {
            value: "async".to_string(),
            is_from_kg: true,
        });

        assert_eq!(set.to_query_string(), "rust AND async");

        set.operator = Some(ChipOperator::Or);
        assert_eq!(set.to_query_string(), "rust OR async");
    }

    #[test]
    fn test_from_query_string() {
        let set = TermChipSet::from_query_string("rust AND tokio", |_| false);

        assert_eq!(set.chips.len(), 2);
        assert_eq!(set.chips[0].value, "rust");
        assert_eq!(set.chips[1].value, "tokio");
        assert_eq!(set.operator, Some(ChipOperator::And));
    }
}
