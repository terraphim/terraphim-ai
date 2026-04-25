use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use terraphim_types::{DocumentType, MarkdownDirectives, RouteDirective};
use walkdir::WalkDir;

/// A non-fatal warning produced while parsing a markdown directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDirectiveWarning {
    pub path: PathBuf,
    pub line: Option<usize>,
    pub message: String,
}

/// The result of parsing all markdown directives in a directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkdownDirectivesParseResult {
    pub directives: HashMap<String, MarkdownDirectives>,
    pub warnings: Vec<MarkdownDirectiveWarning>,
}

/// Recursively parses all markdown directive files under `root`.
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

/// Extract the first `# Heading` from a markdown file at the given path.
///
/// Reads the file and delegates to `terraphim_markdown_parser::extract_first_heading`
/// for proper AST-based heading extraction. Returns `None` if the file cannot be read
/// or has no H1 heading.
pub fn extract_heading_from_path(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    terraphim_markdown_parser::extract_first_heading(&content)
}

fn parse_markdown_directives_content(
    path: &Path,
    content: &str,
    warnings: &mut Vec<MarkdownDirectiveWarning>,
) -> MarkdownDirectives {
    let mut doc_type: Option<DocumentType> = None;
    let mut synonyms: Vec<String> = Vec::new();
    let mut routes: Vec<RouteDirective> = Vec::new();
    let mut priority: Option<u8> = None;
    let mut trigger: Option<String> = None;
    let mut pinned: bool = false;

    // Use AST parser for proper heading extraction (handles inline code, emphasis, etc.)
    let heading = terraphim_markdown_parser::extract_first_heading(content);

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
                routes.push(RouteDirective {
                    provider: provider.to_ascii_lowercase(),
                    model: model.to_string(),
                    action: None,
                });
            }
            continue;
        }

        if lower.starts_with("action::") {
            let value = trimmed["action::".len()..].trim();
            if !value.is_empty() {
                // Attach action to the most recently parsed route
                if let Some(last_route) = routes.last_mut() {
                    last_route.action = Some(value.to_string());
                } else {
                    warnings.push(MarkdownDirectiveWarning {
                        path: path.to_path_buf(),
                        line: Some(idx + 1),
                        message: "action:: directive without a preceding route:: directive"
                            .to_string(),
                    });
                }
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
            continue;
        }

        if lower.starts_with("trigger::") {
            if trigger.is_some() {
                continue; // First trigger wins
            }
            let value = trimmed["trigger::".len()..].trim();
            if !value.is_empty() {
                trigger = Some(value.to_string());
            }
            continue;
        }

        if lower.starts_with("pinned::") {
            let value = trimmed["pinned::".len()..].trim().to_ascii_lowercase();
            pinned = matches!(value.as_str(), "true" | "yes" | "1");
            continue;
        }
    }

    // Primary route is the first in the list (backward compatible)
    let route = routes.first().cloned();

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
        routes,
        priority,
        trigger,
        pinned,
        heading,
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
                action: None,
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

    #[test]
    fn parses_trigger_directive() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, "trigger:: when managing dependencies").unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        let directives = result.directives.get("test").unwrap();
        assert_eq!(
            directives.trigger,
            Some("when managing dependencies".to_string())
        );
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn parses_pinned_directive() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, "pinned:: true").unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        let directives = result.directives.get("test").unwrap();
        assert!(directives.pinned);
    }

    #[test]
    fn pinned_false_variants() {
        let dir = tempdir().unwrap();

        for (filename, value) in [("false", "false"), ("no", "no"), ("zero", "0")] {
            let path = dir.path().join(format!("{filename}.md"));
            fs::write(&path, format!("pinned:: {value}")).unwrap();
        }

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        assert!(!result.directives.get("false").unwrap().pinned);
        assert!(!result.directives.get("no").unwrap().pinned);
        assert!(!result.directives.get("zero").unwrap().pinned);
    }

    #[test]
    fn trigger_and_synonyms_coexist() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(
            &path,
            "synonyms:: alpha, beta\ntrigger:: when using alphas\n",
        )
        .unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        let directives = result.directives.get("test").unwrap();
        assert_eq!(
            directives.synonyms,
            vec!["alpha".to_string(), "beta".to_string()]
        );
        assert_eq!(directives.trigger, Some("when using alphas".to_string()));
    }

    #[test]
    fn empty_trigger_ignored() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, "trigger::").unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        let directives = result.directives.get("test").unwrap();
        assert_eq!(directives.trigger, None);
    }

    #[test]
    fn extracts_heading_from_markdown() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bun.md");
        fs::write(
            &path,
            "# Bun Package Manager\n\nsynonyms:: npm, yarn, pnpm\n",
        )
        .unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        let directives = result.directives.get("bun").unwrap();
        assert_eq!(directives.heading, Some("Bun Package Manager".to_string()));
        assert_eq!(
            directives.synonyms,
            vec!["npm".to_string(), "yarn".to_string(), "pnpm".to_string()]
        );
    }

    #[test]
    fn heading_none_when_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("noheading.md");
        fs::write(&path, "synonyms:: alpha, beta\n").unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        let directives = result.directives.get("noheading").unwrap();
        assert_eq!(directives.heading, None);
    }

    #[test]
    fn parses_multiple_routes_with_actions() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("implementation.md");
        fs::write(
            &path,
            r#"# Implementation Routing

priority:: 50

synonyms:: implement, build, code

route:: kimi, kimi-for-coding/k2p5
action:: opencode -m {{ model }} -p "{{ prompt }}"

route:: anthropic, claude-sonnet-4-6
action:: claude --model {{ model }} -p "{{ prompt }}" --max-turns 50
"#,
        )
        .unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        let directives = result.directives.get("implementation").unwrap();

        // Primary route (backward compatible)
        assert_eq!(directives.route.as_ref().unwrap().provider, "kimi");
        assert_eq!(
            directives.route.as_ref().unwrap().model,
            "kimi-for-coding/k2p5"
        );

        // All routes
        assert_eq!(directives.routes.len(), 2);
        assert_eq!(directives.routes[0].provider, "kimi");
        assert_eq!(
            directives.routes[0].action.as_deref(),
            Some(r#"opencode -m {{ model }} -p "{{ prompt }}""#)
        );
        assert_eq!(directives.routes[1].provider, "anthropic");
        assert_eq!(directives.routes[1].model, "claude-sonnet-4-6");
        assert_eq!(
            directives.routes[1].action.as_deref(),
            Some(r#"claude --model {{ model }} -p "{{ prompt }}" --max-turns 50"#)
        );

        assert!(result.warnings.is_empty());
    }

    #[test]
    fn action_without_route_warns() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("orphan_action.md");
        fs::write(&path, r#"action:: opencode -m foo -p "{{ prompt }}""#).unwrap();

        let result = parse_markdown_directives_dir(dir.path()).unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert!(
            result.warnings[0]
                .message
                .contains("without a preceding route")
        );
    }

    #[test]
    fn extract_heading_from_path_works() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, "# My Heading\n\nSome content\n").unwrap();

        assert_eq!(
            extract_heading_from_path(&path),
            Some("My Heading".to_string())
        );
    }

    #[test]
    fn extract_heading_from_path_returns_none_without_heading() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, "Just content\n").unwrap();

        assert_eq!(extract_heading_from_path(&path), None);
    }
}
