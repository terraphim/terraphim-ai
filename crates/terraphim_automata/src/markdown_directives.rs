use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use terraphim_types::{DocumentType, MarkdownDirectives, RouteDirective};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDirectiveWarning {
    pub path: PathBuf,
    pub line: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkdownDirectivesParseResult {
    pub directives: HashMap<String, MarkdownDirectives>,
    pub warnings: Vec<MarkdownDirectiveWarning>,
}

pub fn parse_markdown_directives_dir(root: &Path) -> crate::Result<MarkdownDirectivesParseResult> {
    let mut directives = HashMap::new();
    let mut warnings = Vec::new();

    for entry in WalkDir::new(root) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                let path = err.path().unwrap_or(root).to_path_buf();
                warnings.push(MarkdownDirectiveWarning {
                    path,
                    line: None,
                    message: format!("Failed to read directory entry: {}", err),
                });
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if extension != "md" {
            continue;
        }

        let concept = match path.file_stem().and_then(|stem| stem.to_str()) {
            Some(stem) => stem.to_string(),
            None => {
                warnings.push(MarkdownDirectiveWarning {
                    path: path.to_path_buf(),
                    line: None,
                    message: "Failed to read file stem for concept key".to_string(),
                });
                continue;
            }
        };

        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                warnings.push(MarkdownDirectiveWarning {
                    path: path.to_path_buf(),
                    line: None,
                    message: format!("Failed to read markdown file: {}", err),
                });
                continue;
            }
        };

        let doc_directives = parse_markdown_directives_content(path, &content, &mut warnings);
        directives.insert(concept, doc_directives);
    }

    Ok(MarkdownDirectivesParseResult {
        directives,
        warnings,
    })
}

fn parse_markdown_directives_content(
    path: &Path,
    content: &str,
    warnings: &mut Vec<MarkdownDirectiveWarning>,
) -> MarkdownDirectives {
    let mut doc_type: Option<DocumentType> = None;
    let mut synonyms: Vec<String> = Vec::new();
    let mut route: Option<RouteDirective> = None;
    let mut priority: Option<u8> = None;

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("type:::") {
            if doc_type.is_some() {
                continue;
            }
            let value = trimmed["type:::".len()..].trim();
            let value_lower = value.to_ascii_lowercase();
            let parsed = match value_lower.as_str() {
                "kg_entry" => Some(DocumentType::KgEntry),
                "document" => Some(DocumentType::Document),
                "config_document" => Some(DocumentType::ConfigDocument),
                _ => None,
            };

            match parsed {
                Some(parsed) => doc_type = Some(parsed),
                None => warnings.push(MarkdownDirectiveWarning {
                    path: path.to_path_buf(),
                    line: Some(idx + 1),
                    message: format!("Invalid type directive '{}'", value),
                }),
            }
            continue;
        }

        if lower.starts_with("synonyms::") {
            let value = trimmed["synonyms::".len()..].trim();
            for raw in value.split(',') {
                let normalized = raw.trim().to_ascii_lowercase();
                if !normalized.is_empty() {
                    synonyms.push(normalized);
                }
            }
            continue;
        }

        if lower.starts_with("route::") || lower.starts_with("routing::") {
            if route.is_some() {
                continue;
            }
            let prefix_len = if lower.starts_with("route::") {
                "route::".len()
            } else {
                "routing::".len()
            };
            let value = trimmed[prefix_len..].trim();
            let mut parts = value.splitn(2, ',');
            let provider = parts.next().unwrap_or("").trim();
            let model = parts.next().unwrap_or("").trim();

            if provider.is_empty() || model.is_empty() {
                warnings.push(MarkdownDirectiveWarning {
                    path: path.to_path_buf(),
                    line: Some(idx + 1),
                    message: format!("Invalid route directive '{}'", value),
                });
            } else {
                route = Some(RouteDirective {
                    provider: provider.to_ascii_lowercase(),
                    model: model.to_string(),
                });
            }
            continue;
        }

        if lower.starts_with("priority::") {
            if priority.is_some() {
                continue;
            }
            let value = trimmed["priority::".len()..].trim();
            match value.parse::<u8>() {
                Ok(parsed) if parsed <= 100 => priority = Some(parsed),
                _ => warnings.push(MarkdownDirectiveWarning {
                    path: path.to_path_buf(),
                    line: Some(idx + 1),
                    message: format!("Invalid priority directive '{}'", value),
                }),
            }
        }
    }

    let doc_type = doc_type.unwrap_or_else(|| {
        if route.is_some() {
            DocumentType::ConfigDocument
        } else {
            DocumentType::KgEntry
        }
    });

    MarkdownDirectives {
        doc_type,
        synonyms,
        route,
        priority,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_synonyms_only() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("alpha.md");
        fs::write(
            &path,
            "synonyms:: Alpha, beta\nsynonyms:: gamma\n\nSome content",
        )
        .unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        let directives = result.directives.get("alpha").unwrap();
        assert_eq!(directives.doc_type, DocumentType::KgEntry);
        assert_eq!(
            directives.synonyms,
            vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
        );
        assert!(directives.route.is_none());
        assert!(directives.priority.is_none());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn parses_config_route_priority() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("plan.md");
        fs::write(
            &path,
            "type::: config_document\nroute:: openai, gpt-4o\npriority:: 80",
        )
        .unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        let directives = result.directives.get("plan").unwrap();
        assert_eq!(directives.doc_type, DocumentType::ConfigDocument);
        assert_eq!(
            directives.route,
            Some(RouteDirective {
                provider: "openai".to_string(),
                model: "gpt-4o".to_string(),
            })
        );
        assert_eq!(directives.priority, Some(80));
        assert!(directives.synonyms.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn infers_config_document_when_route_present() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("route_only.md");
        fs::write(&path, "route:: anthropic, claude-3-5-sonnet").unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        let directives = result.directives.get("route_only").unwrap();
        assert_eq!(directives.doc_type, DocumentType::ConfigDocument);
        assert!(directives.route.is_some());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn warns_on_invalid_priority() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad_priority.md");
        fs::write(&path, "priority:: 200").unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        let directives = result.directives.get("bad_priority").unwrap();
        assert!(directives.priority.is_none());
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn warns_on_invalid_route() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad_route.md");
        fs::write(&path, "route:: only_provider").unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        let directives = result.directives.get("bad_route").unwrap();
        assert!(directives.route.is_none());
        assert_eq!(directives.doc_type, DocumentType::KgEntry);
        assert_eq!(result.warnings.len(), 1);
    }
}
