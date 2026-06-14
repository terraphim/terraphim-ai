//! Completion helpers for Terraphim LSP.
//!
//! Builds LSP completion items from a knowledge-graph thesaurus and the
//! current document context.

use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Position};

use terraphim_types::Thesaurus;

/// Build completion items for the given thesaurus.
///
/// When `word` is non-empty, only terms whose normalized value starts with the
/// word (case-insensitive) are returned. Otherwise all thesaurus entries are
/// returned.
pub fn build_completions(thesaurus: &Thesaurus, word: &str) -> Vec<CompletionItem> {
    let prefix = word.to_lowercase();
    thesaurus
        .into_iter()
        .filter(|(key, _)| {
            if prefix.is_empty() {
                true
            } else {
                key.to_string().to_lowercase().starts_with(&prefix)
            }
        })
        .map(|(key, term)| CompletionItem {
            label: key.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: term.display_value.clone().map(|d| d.to_string()),
            documentation: term
                .url
                .as_ref()
                .map(|url| tower_lsp::lsp_types::Documentation::String(format!("See: {}", url))),
            ..CompletionItem::default()
        })
        .collect()
}

/// Extract the word prefix at the given position.
///
/// Returns the contiguous run of alphanumeric characters (and hyphens/underscores)
/// immediately preceding or containing the cursor.
pub fn word_at_position(text: &str, position: Position) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let line = lines.get(position.line as usize).unwrap_or(&"");
    let col = position.character as usize;
    let before = &line[..col.min(line.len())];

    before
        .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
        .next_back()
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

    fn sample_thesaurus() -> Thesaurus {
        let mut thesaurus = Thesaurus::new("programming".to_string());
        thesaurus.insert(
            NormalizedTermValue::from("rust"),
            NormalizedTerm::with_auto_id(NormalizedTermValue::from("rust programming language")),
        );
        thesaurus.insert(
            NormalizedTermValue::from("tokio"),
            NormalizedTerm::with_auto_id(NormalizedTermValue::from("tokio async runtime")),
        );
        thesaurus
    }

    #[test]
    fn test_completions_empty_prefix_returns_all() {
        let thesaurus = sample_thesaurus();
        let items = build_completions(&thesaurus, "");
        assert_eq!(items.len(), 2);
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(labels.contains(&"rust".to_string()));
        assert!(labels.contains(&"tokio".to_string()));
    }

    #[test]
    fn test_completions_prefix_filters() {
        let thesaurus = sample_thesaurus();
        let items = build_completions(&thesaurus, "to");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "tokio");
    }

    #[test]
    fn test_word_at_position() {
        let text = "rust and tokio";
        let pos = Position {
            line: 0,
            character: 14,
        };
        assert_eq!(word_at_position(text, pos), "tokio");
    }

    #[test]
    fn test_word_at_position_mid_word() {
        let text = "rust and tokio";
        let pos = Position {
            line: 0,
            character: 12,
        };
        assert_eq!(word_at_position(text, pos), "tok");
    }
}
