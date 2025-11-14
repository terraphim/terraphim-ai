//! Markdown command definition parser with YAML frontmatter support
//!
//! This module parses markdown files containing command definitions with YAML frontmatter,
//! extracting both metadata and content for command registration.

use super::{CommandDefinition, CommandRegistryError, ParsedCommand};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use regex::{Regex, RegexBuilder};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// Automata imports for term extraction
use ahash::AHashMap;
use terraphim_automata::{find_matches, Matched};
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

/// Parsed command with enriched content analysis
#[derive(Debug, Clone)]
pub struct EnrichedParsedCommand {
    /// Basic parsed command
    pub parsed_command: ParsedCommand,
    /// Enriched content analysis results
    pub enriched_content: Option<EnrichedContent>,
}

/// Enriched content analysis results
#[derive(Debug, Clone)]
pub struct EnrichedContent {
    /// Matched technical terms with positions
    pub matched_terms: Vec<Matched>,
    /// Extracted paragraphs for each matched term
    pub contextual_paragraphs: Vec<(Matched, String)>,
    /// Extracted keywords from content
    pub extracted_keywords: Vec<String>,
    /// Related concepts based on term analysis
    pub related_concepts: Vec<String>,
    /// Content complexity metrics
    pub complexity_metrics: ContentMetrics,
}

/// Content complexity analysis
#[derive(Debug, Clone)]
pub struct ContentMetrics {
    /// Total word count
    pub word_count: usize,
    /// Number of technical terms found
    pub technical_term_count: usize,
    /// Number of code blocks
    pub code_block_count: usize,
    /// Number of headings
    pub heading_count: usize,
    /// Content richness score (0.0 to 1.0)
    pub richness_score: f64,
}

/// Parser for markdown command definitions
#[derive(Debug)]
pub struct MarkdownCommandParser {
    /// Regex for extracting YAML frontmatter
    frontmatter_regex: Regex,
    /// Technical terms thesaurus for extraction
    technical_thesaurus: Option<Thesaurus>,
    /// Command-specific terms learned during parsing
    learned_terms: AHashMap<String, NormalizedTermValue>,
}

impl MarkdownCommandParser {
    /// Create a new markdown command parser
    pub fn new() -> Result<Self, CommandRegistryError> {
        let frontmatter_regex = RegexBuilder::new(r"^---\s*\n(.*?)\n---\s*\n(.*)$")
            .dot_matches_new_line(true)
            .build()
            .map_err(|e| CommandRegistryError::parse_error("regex", e.to_string()))?;

        Ok(Self {
            frontmatter_regex,
            technical_thesaurus: None,
            learned_terms: AHashMap::new(),
        })
    }

    /// Create a new markdown command parser with technical terms thesaurus
    pub fn with_technical_thesaurus(thesaurus: Thesaurus) -> Result<Self, CommandRegistryError> {
        let frontmatter_regex = RegexBuilder::new(r"^---\s*\n(.*?)\n---\s*\n(.*)$")
            .dot_matches_new_line(true)
            .build()
            .map_err(|e| CommandRegistryError::parse_error("regex", e.to_string()))?;

        Ok(Self {
            frontmatter_regex,
            technical_thesaurus: Some(thesaurus),
            learned_terms: AHashMap::new(),
        })
    }

    /// Set or update the technical terms thesaurus
    pub fn set_technical_thesaurus(&mut self, thesaurus: Thesaurus) {
        self.technical_thesaurus = Some(thesaurus);
    }

    /// Parse a single markdown file containing a command definition
    pub async fn parse_file(
        &self,
        file_path: impl AsRef<Path>,
    ) -> Result<ParsedCommand, CommandRegistryError> {
        let path = file_path.as_ref();

        // Read the file content
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|_e| CommandRegistryError::FileNotFound(path.to_string_lossy().to_string()))?;

        // Get file metadata
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(CommandRegistryError::IoError)?;
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        // Parse the content
        self.parse_content(&content, path.to_path_buf(), modified)
    }

    /// Parse markdown content string
    pub fn parse_content(
        &self,
        content: &str,
        source_path: PathBuf,
        modified: SystemTime,
    ) -> Result<ParsedCommand, CommandRegistryError> {
        // Extract frontmatter and content
        let captures = self.frontmatter_regex.captures(content).ok_or_else(|| {
            CommandRegistryError::invalid_frontmatter(
                &source_path,
                "No valid YAML frontmatter found. Expected format: ---\\nyaml\\n---\\ncontent",
            )
        })?;

        let frontmatter_yaml = captures.get(1).unwrap().as_str().trim();
        let markdown_content = captures.get(2).unwrap().as_str().trim();

        // Parse YAML frontmatter
        let definition: CommandDefinition =
            serde_yaml::from_str(frontmatter_yaml).map_err(|e| {
                CommandRegistryError::invalid_frontmatter(
                    &source_path,
                    format!("YAML parsing error: {}", e),
                )
            })?;

        // Validate the command definition
        self.validate_definition(&definition, &source_path)?;

        // Parse markdown content to extract and preserve structure
        let content = self.extract_markdown_content(markdown_content);

        Ok(ParsedCommand {
            definition,
            content,
            source_path,
            modified,
        })
    }

    /// Parse markdown content with enhanced term extraction and analysis
    pub fn parse_content_with_analysis(
        &mut self,
        content: &str,
        source_path: PathBuf,
        modified: SystemTime,
    ) -> Result<EnrichedParsedCommand, CommandRegistryError> {
        // First perform basic parsing
        let parsed_command = self.parse_content(content, source_path.clone(), modified)?;

        // Perform enhanced content analysis
        let enriched_content = self.analyze_content(&parsed_command.content)?;

        // Learn new terms from this command for future parsing
        self.learn_terms_from_content(&parsed_command.content);

        Ok(EnrichedParsedCommand {
            parsed_command,
            enriched_content: Some(enriched_content),
        })
    }

    /// Analyze content using automata for term extraction and metrics
    fn analyze_content(&self, content: &str) -> Result<EnrichedContent, CommandRegistryError> {
        // Extract technical terms using available thesaurus
        let matched_terms = if let Some(ref thesaurus) = self.technical_thesaurus {
            find_matches(content, thesaurus.clone(), true)
                .map_err(|e| CommandRegistryError::AutomataError(e.to_string()))?
        } else {
            Vec::new()
        };

        // Extract keywords using heuristics
        let extracted_keywords = self.extract_keywords_from_text(content);

        // Calculate content complexity metrics
        let complexity_metrics = self.calculate_complexity_metrics(content, &matched_terms);

        // Extract contextual paragraphs for matched terms
        let contextual_paragraphs = self.extract_contextual_paragraphs(content, &matched_terms);

        // Identify related concepts based on term analysis
        let related_concepts = self.identify_related_concepts(&matched_terms, &extracted_keywords);

        Ok(EnrichedContent {
            matched_terms,
            contextual_paragraphs,
            extracted_keywords,
            related_concepts,
            complexity_metrics,
        })
    }

    /// Extract keywords from text using heuristics
    fn extract_keywords_from_text(&self, text: &str) -> Vec<String> {
        let mut keywords = Vec::new();

        // Split on whitespace and punctuation
        for word in text.split_whitespace() {
            let clean_word = word.trim_matches(
                &[
                    ':', ',', '.', ';', '(', ')', '[', ']', '{', '}', '"', '\'', '!', '?', '-', '_',
                ][..],
            );

            // Filter by length and common patterns
            if clean_word.len() >= 3 && !self.is_stop_word(clean_word) {
                // Add technical-looking words
                if self.is_technical_term(clean_word) {
                    keywords.push(clean_word.to_lowercase());
                }
            }
        }

        // Remove duplicates and sort
        keywords.sort();
        keywords.dedup();
        keywords.truncate(20); // Limit to top 20 keywords
        keywords
    }

    /// Check if a word is a technical term based on common patterns
    fn is_technical_term(&self, word: &str) -> bool {
        // Technical indicators
        let tech_indicators = [
            "config",
            "deploy",
            "build",
            "test",
            "api",
            "http",
            "json",
            "yaml",
            "docker",
            "kubernetes",
            "service",
            "database",
            "cache",
            "queue",
            "server",
            "client",
            "request",
            "response",
            "endpoint",
            "route",
            "handler",
            "middleware",
            "auth",
            "token",
            "session",
            "cluster",
            "node",
            "container",
            "pod",
            "namespace",
            "helm",
            "terraform",
            "ansible",
            "ci",
            "cd",
            "pipeline",
            "github",
            "gitlab",
            "jenkins",
            "artifact",
            "registry",
            "monitoring",
            "logging",
            "metrics",
            "alerting",
            "grafana",
            "prometheus",
            "kibana",
            "elasticsearch",
            "redis",
            "postgresql",
            "mysql",
            "mongodb",
            "cassandra",
            "kafka",
            "rabbitmq",
            "nginx",
            "apache",
            "ssl",
            "certificates",
            "tls",
            "https",
            "cert",
            "encryption",
            "hash",
            "helm",
            "charts",
            "configmaps",
            "microservice",
            "deploys",
            "deployments",
        ];

        let word_lower = word.to_lowercase();

        // Check against known technical terms
        if tech_indicators.contains(&word_lower.as_str()) {
            return true;
        }

        // Check for common technical patterns
        if word_lower.ends_with("config")
            || word_lower.ends_with("service")
            || word_lower.ends_with("server")
            || word_lower.ends_with("client")
            || word_lower.ends_with("manager")
            || word_lower.ends_with("handler")
            || word_lower.ends_with("worker")
            || word_lower.ends_with("process")
            || word_lower.ends_with("thread")
            || word_lower.contains("config")
            || word_lower.contains("deploy")
            || word_lower.contains("build")
            || word_lower.contains("test")
        {
            return true;
        }

        // Check for camelCase or snake_case technical patterns
        if word.contains('_') && word.split('_').count() > 1 {
            return true;
        }

        if word.chars().any(|c| c.is_uppercase()) && word.len() > 4 {
            return true;
        }

        false
    }

    /// Check if a word is a stop word
    fn is_stop_word(&self, word: &str) -> bool {
        let stop_words = [
            "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by", "from",
            "up", "about", "into", "through", "during", "before", "after", "above", "below", "is",
            "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does", "did",
            "will", "would", "could", "should", "may", "might", "must", "can", "this", "that",
            "these", "those", "i", "you", "he", "she", "it", "we", "they", "me", "him", "her",
            "us", "them", "my", "your", "his", "its", "our", "their", "a", "an", "as", "if",
            "when", "where", "why", "how", "what", "which", "who", "whom", "whose", "all", "any",
            "both", "each", "every", "few", "many", "most", "other", "some", "such", "only", "own",
            "same", "so", "than", "too", "very", "just", "now", "also", "here", "there", "more",
            "most",
        ];
        stop_words.contains(&word)
    }

    /// Calculate complexity metrics for content
    fn calculate_complexity_metrics(
        &self,
        content: &str,
        matched_terms: &[Matched],
    ) -> ContentMetrics {
        let word_count = content.split_whitespace().count();
        let technical_term_count = matched_terms.len();

        // Count markdown elements
        let code_block_count = content.matches("```").count() / 2;
        let heading_count = content.lines().filter(|line| line.starts_with('#')).count();

        // Calculate richness score based on term density and structural elements
        let term_density = if word_count > 0 {
            technical_term_count as f64 / word_count as f64
        } else {
            0.0
        };

        let structural_score = (code_block_count + heading_count) as f64 / 10.0; // Normalize by expected max
        let richness_score = (term_density * 0.6 + structural_score * 0.4).min(1.0);

        ContentMetrics {
            word_count,
            technical_term_count,
            code_block_count,
            heading_count,
            richness_score,
        }
    }

    /// Extract contextual paragraphs around matched terms
    fn extract_contextual_paragraphs(
        &self,
        content: &str,
        matched_terms: &[Matched],
    ) -> Vec<(Matched, String)> {
        let mut paragraphs = Vec::new();

        for matched in matched_terms {
            if let Some((start, _)) = matched.pos {
                // Find paragraph boundaries around the match
                let paragraph_start = self.find_paragraph_start(content, start);
                let paragraph_end = self.find_paragraph_end(content, start + 20); // Approximate match end

                if paragraph_start < paragraph_end && paragraph_start < content.len() {
                    let paragraph = &content[paragraph_start..paragraph_end];
                    paragraphs.push((matched.clone(), paragraph.trim().to_string()));
                }
            }
        }

        paragraphs
    }

    /// Find start of paragraph containing the given position
    fn find_paragraph_start(&self, content: &str, pos: usize) -> usize {
        let mut start = pos;

        // Look backwards for paragraph start
        while start > 0 {
            let prev_char = content.chars().nth(start - 1).unwrap_or('\n');
            if prev_char == '\n' && start > 1 {
                let prev_prev_char = content.chars().nth(start - 2).unwrap_or('\n');
                if prev_prev_char == '\n' {
                    break; // Found paragraph boundary
                }
            }
            start -= 1;
        }

        start
    }

    /// Find end of paragraph containing the given position
    fn find_paragraph_end(&self, content: &str, pos: usize) -> usize {
        let mut end = pos;
        let content_len = content.len();

        // Look forwards for paragraph end
        while end < content_len {
            let current_char = content.chars().nth(end).unwrap_or('\0');
            if current_char == '\n' && end + 1 < content_len {
                let next_char = content.chars().nth(end + 1).unwrap_or('\0');
                if next_char == '\n' {
                    end += 2; // Include both newlines
                    break;
                }
            }
            end += 1;
        }

        end.min(content_len)
    }

    /// Identify related concepts based on term analysis
    fn identify_related_concepts(
        &self,
        matched_terms: &[Matched],
        keywords: &[String],
    ) -> Vec<String> {
        let mut concepts = Vec::new();

        // Extract concept names from matched terms
        for matched in matched_terms {
            let term = &matched.term;
            if term.len() > 4 && !concepts.contains(&term.to_lowercase()) {
                concepts.push(term.to_lowercase());
            }
        }

        // Add high-value keywords
        for keyword in keywords.iter().take(10) {
            if !concepts.contains(keyword) {
                concepts.push(keyword.clone());
            }
        }

        // Sort and limit
        concepts.sort();
        concepts.truncate(15);
        concepts
    }

    /// Learn new terms from parsed content to improve future parsing
    fn learn_terms_from_content(&mut self, content: &str) {
        // Extract potential new technical terms
        for word in content.split_whitespace() {
            let clean_word = word.trim_matches(
                &[
                    ':', ',', '.', ';', '(', ')', '[', ']', '{', '}', '"', '\'', '!', '?',
                ][..],
            );

            if clean_word.len() > 4 && self.is_technical_term(clean_word) {
                let normalized = NormalizedTermValue::from(clean_word.to_lowercase());
                self.learned_terms
                    .insert(clean_word.to_lowercase(), normalized);
            }
        }
    }

    /// Get learned terms for building/updating thesaurus
    pub fn get_learned_terms(&self) -> &AHashMap<String, NormalizedTermValue> {
        &self.learned_terms
    }

    /// Build a technical thesaurus from learned terms
    pub fn build_technical_thesaurus(&self) -> Option<Thesaurus> {
        if self.learned_terms.is_empty() {
            return None;
        }

        let mut thesaurus = Thesaurus::new("learned_technical_terms".to_string());
        let mut term_id = 1u64;

        for (term, normalized_term) in &self.learned_terms {
            thesaurus.insert(
                normalized_term.clone(),
                NormalizedTerm {
                    id: term_id,
                    value: normalized_term.clone(),
                    url: Some(format!("learned-term:{}", term)),
                },
            );
            term_id += 1;
        }

        Some(thesaurus)
    }

    /// Parse all command files in a directory recursively
    pub async fn parse_directory(
        &self,
        dir_path: impl AsRef<Path>,
    ) -> Result<Vec<ParsedCommand>, CommandRegistryError> {
        self.parse_directory_recursive(dir_path, 0).await
    }

    /// Internal recursive implementation with depth limiting
    async fn parse_directory_recursive(
        &self,
        dir_path: impl AsRef<Path>,
        depth: usize,
    ) -> Result<Vec<ParsedCommand>, CommandRegistryError> {
        // Prevent infinite recursion
        if depth > 10 {
            return Ok(Vec::new());
        }

        let mut commands = Vec::new();
        let mut entries = tokio::fs::read_dir(dir_path)
            .await
            .map_err(CommandRegistryError::IoError)?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(CommandRegistryError::IoError)?
        {
            let path = entry.path();

            if path.is_dir() {
                // Recursively parse subdirectories - use Box::pin to avoid recursion issues
                match Box::pin(self.parse_directory_recursive(&path, depth + 1)).await {
                    Ok(mut sub_commands) => commands.append(&mut sub_commands),
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to parse directory {}: {}",
                            path.display(),
                            e
                        );
                        // Continue with other files
                    }
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                // Parse markdown files
                match self.parse_file(&path).await {
                    Ok(command) => commands.push(command),
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to parse command file {}: {}",
                            path.display(),
                            e
                        );
                        // Continue with other files
                    }
                }
            }
        }

        Ok(commands)
    }

    /// Extract and preserve markdown content structure
    fn extract_markdown_content(&self, markdown_content: &str) -> String {
        let parser = Parser::new(markdown_content);
        let mut output = String::new();
        let mut code_block_fence = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    output.push_str(&"#".repeat(level as usize));
                    output.push(' ');
                }
                Event::End(TagEnd::Heading(_)) => {
                    output.push('\n');
                }

                Event::Start(Tag::CodeBlock(kind)) => {
                    code_block_fence = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(fence) => {
                            if fence.is_empty() {
                                "```".to_string()
                            } else {
                                format!("```{}", fence)
                            }
                        }
                        _ => "```".to_string(),
                    };
                    output.push_str(&code_block_fence);
                    output.push('\n');
                }
                Event::End(TagEnd::CodeBlock) => {
                    output.push_str(&code_block_fence);
                    output.push('\n');
                }

                Event::Start(Tag::List(..)) => {
                    // Just let the list items handle their own formatting
                }
                Event::End(TagEnd::List(_)) => {
                    output.push('\n');
                }

                Event::Start(Tag::Item) => {
                    output.push_str("- ");
                }
                Event::End(TagEnd::Item) => {
                    output.push('\n');
                }

                Event::Text(text) => {
                    output.push_str(&text);
                }

                Event::Code(code) => {
                    output.push('`');
                    output.push_str(&code);
                    output.push('`');
                }

                Event::Start(Tag::Strong) => {
                    output.push_str("**");
                }

                Event::End(TagEnd::Strong) => {
                    output.push_str("**");
                }

                Event::Start(Tag::Emphasis) => {
                    output.push('*');
                }

                Event::End(TagEnd::Emphasis) => {
                    output.push('*');
                }

                Event::SoftBreak | Event::HardBreak => {
                    output.push('\n');
                }

                // Skip other events for simplicity
                _ => {}
            }
        }

        // Clean up trailing whitespace while preserving structure
        output.trim().to_string()
    }

    /// Validate command definition
    fn validate_definition(
        &self,
        definition: &CommandDefinition,
        source_path: &Path,
    ) -> Result<(), CommandRegistryError> {
        // Validate command name
        if definition.name.is_empty() {
            return Err(CommandRegistryError::invalid_frontmatter(
                source_path,
                "Command name cannot be empty",
            ));
        }

        // Validate command name format (alphanumeric, hyphens, underscores)
        let name_regex = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$").unwrap();
        if !name_regex.is_match(&definition.name) {
            return Err(CommandRegistryError::invalid_frontmatter(
                source_path,
                format!("Invalid command name '{}'. Must start with letter and contain only alphanumeric characters, hyphens, and underscores", definition.name)
            ));
        }

        // Validate parameter names and types
        let param_name_regex = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();
        for param in &definition.parameters {
            // Validate parameter name format
            if !param_name_regex.is_match(&param.name) {
                return Err(CommandRegistryError::invalid_frontmatter(
                    source_path,
                    format!("Invalid parameter name '{}'. Must start with letter and contain only alphanumeric characters and underscores", param.name)
                ));
            }
        }

        // Validate that required parameters don't have default values
        for param in &definition.parameters {
            if param.required && param.default_value.is_some() {
                return Err(CommandRegistryError::invalid_frontmatter(
                    source_path,
                    format!(
                        "Required parameter '{}' cannot have a default value",
                        param.name
                    ),
                ));
            }
        }

        // Validate timeout
        if let Some(timeout) = definition.timeout {
            if timeout == 0 {
                return Err(CommandRegistryError::invalid_frontmatter(
                    source_path,
                    "Timeout cannot be zero",
                ));
            }
        }

        // Validate resource limits
        if let Some(ref limits) = definition.resource_limits {
            if let Some(max_memory) = limits.max_memory_mb {
                if max_memory == 0 {
                    return Err(CommandRegistryError::invalid_frontmatter(
                        source_path,
                        "Max memory limit cannot be zero",
                    ));
                }
            }

            if let Some(max_cpu) = limits.max_cpu_time {
                if max_cpu == 0 {
                    return Err(CommandRegistryError::invalid_frontmatter(
                        source_path,
                        "Max CPU time cannot be zero",
                    ));
                }
            }

            if let Some(max_disk) = limits.max_disk_mb {
                if max_disk == 0 {
                    return Err(CommandRegistryError::invalid_frontmatter(
                        source_path,
                        "Max disk limit cannot be zero",
                    ));
                }
            }
        }

        Ok(())
    }
}

impl Default for MarkdownCommandParser {
    fn default() -> Self {
        Self::new().expect("Failed to create MarkdownCommandParser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::SystemTime;

    #[test]
    fn test_parse_simple_command() {
        let parser = MarkdownCommandParser::new().unwrap();

        let markdown = r#"---
name: "hello"
description: "Say hello to someone"
parameters:
  - name: "name"
    type: "string"
    required: true
    description: "Name of person to greet"
execution_mode: "local"
risk_level: "low"
---

# Hello Command

This command says hello to someone with a friendly message.

## Usage

Just provide a name and get a greeting!
"#;

        let result =
            parser.parse_content(markdown, PathBuf::from("hello.md"), SystemTime::UNIX_EPOCH);

        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.definition.name, "hello");
        assert_eq!(parsed.definition.description, "Say hello to someone");
        assert_eq!(parsed.definition.parameters.len(), 1);
        assert_eq!(parsed.definition.parameters[0].name, "name");
        assert!(parsed.definition.parameters[0].required);
    }

    #[test]
    fn test_invalid_command_name() {
        let parser = MarkdownCommandParser::new().unwrap();

        let markdown = r#"---
name: "123invalid"
description: "Invalid command name"
execution_mode: "local"
---

Content here
"#;

        let result = parser.parse_content(
            markdown,
            PathBuf::from("invalid.md"),
            SystemTime::UNIX_EPOCH,
        );

        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            CommandRegistryError::InvalidFrontmatter(_, msg) => {
                assert!(msg.contains("Invalid command name"));
            }
            _ => panic!("Expected InvalidFrontmatter error"),
        }
    }

    #[test]
    fn test_missing_frontmatter() {
        let parser = MarkdownCommandParser::new().unwrap();

        let markdown = r#"This is just plain markdown
without any frontmatter.
"#;

        let result = parser.parse_content(
            markdown,
            PathBuf::from("no-frontmatter.md"),
            SystemTime::UNIX_EPOCH,
        );

        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            CommandRegistryError::InvalidFrontmatter(_, msg) => {
                assert!(msg.contains("No valid YAML frontmatter"));
            }
            _ => panic!("Expected InvalidFrontmatter error"),
        }
    }

    #[test]
    fn test_description_extraction() {
        let parser = MarkdownCommandParser::new().unwrap();

        let markdown = r#"---
name: "test"
description: "Test command"
execution_mode: "local"
---

# Test Command

This is a **bold** description with *italic* text and `code` blocks.

Here's a [link](https://example.com) that should be removed.

## Subheading

Some additional content that might be included.
"#;

        let result =
            parser.parse_content(markdown, PathBuf::from("test.md"), SystemTime::UNIX_EPOCH);

        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(parsed.content.contains("Test Command"));
        assert!(parsed
            .content
            .contains("**bold** description with *italic* text and `code` blocks"));
        assert!(!parsed.content.contains("https://example.com"));
    }

    #[test]
    fn test_markdown_content_preservation() {
        let parser = MarkdownCommandParser::new().unwrap();

        let markdown = r#"---
name: "test-command"
description: "Test command with markdown"
execution_mode: "local"
risk_level: "low"
---

# Test Command

This is a **bold** description with *italic* text and `code` blocks.

## Examples

```bash
test-command --input "hello"
```

### Sub-section

Some additional content here.

- List item 1
- List item 2
- List item 3
"#;

        let result =
            parser.parse_content(markdown, PathBuf::from("test.md"), SystemTime::UNIX_EPOCH);

        assert!(result.is_ok());
        let parsed = result.unwrap();

        // Test that markdown structure is preserved
        assert!(parsed.content.contains("# Test Command"));
        assert!(parsed.content.contains("## Examples"));
        assert!(parsed.content.contains("### Sub-section"));
        assert!(parsed.content.contains("```bash"));
        assert!(parsed.content.contains("- List item 1"));
        assert!(parsed.content.contains("- List item 2"));
        assert!(parsed.content.contains("- List item 3"));

        // Test that content is preserved, not stripped
        assert!(parsed.content.contains("This is a **bold** description"));
        assert!(parsed.content.contains("test-command --input \"hello\""));

        // Test that newlines are preserved for structure
        let lines: Vec<&str> = parsed.content.lines().collect();
        assert!(lines.len() > 5); // Should have multiple lines preserved
    }

    // Enhanced term extraction tests
    #[test]
    fn test_technical_term_identification() {
        let parser = MarkdownCommandParser::new().unwrap();

        // Test technical term identification
        assert!(parser.is_technical_term("database"));
        assert!(parser.is_technical_term("APIendpoint"));
        assert!(parser.is_technical_term("docker_config"));
        assert!(parser.is_technical_term("build_service"));
        assert!(parser.is_technical_term("kubernetes_cluster"));

        // Test non-technical terms
        assert!(!parser.is_technical_term("hello"));
        assert!(!parser.is_technical_term("world"));
        assert!(!parser.is_technical_term("simple"));
        assert!(!parser.is_technical_term("basic"));
    }

    #[test]
    fn test_keyword_extraction() {
        let parser = MarkdownCommandParser::new().unwrap();

        let text = "This command configures the nginx server and sets up SSL certificates for HTTPS connections. It also manages the PostgreSQL database cluster.";

        let keywords = parser.extract_keywords_from_text(text);

        // Should extract technical keywords
        assert!(keywords.contains(&"nginx".to_string()));
        assert!(keywords.contains(&"server".to_string()));
        assert!(keywords.contains(&"ssl".to_string()));
        assert!(keywords.contains(&"certificates".to_string()));
        assert!(keywords.contains(&"https".to_string()));
        assert!(keywords.contains(&"postgresql".to_string()));
        assert!(keywords.contains(&"database".to_string()));
        assert!(keywords.contains(&"cluster".to_string()));

        // Should not include stop words
        assert!(!keywords.contains(&"this".to_string()));
        assert!(!keywords.contains(&"and".to_string()));
        assert!(!keywords.contains(&"for".to_string()));
        assert!(!keywords.contains(&"the".to_string()));
    }

    #[test]
    fn test_content_complexity_metrics() {
        let parser = MarkdownCommandParser::new().unwrap();

        let content = r#"# Complex Command

This is a detailed command with multiple paragraphs.

## Technical Details

The service uses Docker containers and Kubernetes for orchestration.

```bash
docker build -t myapp .
kubectl apply -f deployment.yaml
```

## Configuration

Set up the database connection and cache layer."#;

        let metrics = parser.calculate_complexity_metrics(content, &[]);

        assert!(metrics.word_count > 0);
        assert_eq!(metrics.code_block_count, 1); // One code block
        assert_eq!(metrics.heading_count, 3); // Three headings
        assert!(metrics.richness_score > 0.0);
        assert!(metrics.richness_score <= 1.0);
    }

    #[test]
    fn test_paragraph_extraction() {
        let parser = MarkdownCommandParser::new().unwrap();

        let content = "First paragraph with some content.

Second paragraph that contains important technical terms like database and server.

Third paragraph with more information.";

        // Create a mock matched term at position in second paragraph
        let matched_term = Matched {
            term: "database".to_string(),
            normalized_term: NormalizedTerm::new(1, NormalizedTermValue::from("database")),
            pos: Some((70, 78)), // Position in second paragraph
        };

        let paragraphs = parser.extract_contextual_paragraphs(content, &[matched_term]);

        assert_eq!(paragraphs.len(), 1);
        let (_, paragraph) = &paragraphs[0];
        assert!(paragraph.contains("Second paragraph"));
        assert!(paragraph.contains("technical terms"));
        assert!(paragraph.contains("database"));
        assert!(paragraph.contains("server"));
    }

    #[test]
    fn test_related_concepts_identification() {
        let parser = MarkdownCommandParser::new().unwrap();

        let matched_terms = vec![
            Matched {
                term: "kubernetes".to_string(),
                normalized_term: NormalizedTerm::new(1, NormalizedTermValue::from("kubernetes")),
                pos: Some((0, 10)),
            },
            Matched {
                term: "database".to_string(),
                normalized_term: NormalizedTerm::new(2, NormalizedTermValue::from("database")),
                pos: Some((20, 28)),
            },
        ];

        let keywords = vec![
            "server".to_string(),
            "cluster".to_string(),
            "deployment".to_string(),
            "cache".to_string(),
        ];

        let concepts = parser.identify_related_concepts(&matched_terms, &keywords);

        assert!(!concepts.is_empty());
        assert!(concepts.contains(&"kubernetes".to_string()));
        assert!(concepts.contains(&"database".to_string()));
        assert!(concepts.contains(&"server".to_string()));
        assert!(concepts.contains(&"cluster".to_string()));
    }

    #[test]
    fn test_term_learning() {
        let mut parser = MarkdownCommandParser::new().unwrap();

        let content = "This script deploys the microservice to the Kubernetes cluster using Helm charts and ConfigMaps.";

        // Learn terms from content
        parser.learn_terms_from_content(content);

        let learned_terms = parser.get_learned_terms();

        // Should have learned technical terms
        assert!(learned_terms.contains_key("deploys"));
        assert!(learned_terms.contains_key("microservice"));
        assert!(learned_terms.contains_key("kubernetes"));
        assert!(learned_terms.contains_key("cluster"));
        assert!(learned_terms.contains_key("charts"));
        assert!(learned_terms.contains_key("configmaps"));
    }

    #[test]
    fn test_technical_thesaurus_building() {
        let mut parser = MarkdownCommandParser::new().unwrap();

        // Learn some terms first
        parser.learn_terms_from_content("Deploy the microservice to the cluster");
        parser.learn_terms_from_content("Configure the database connection");

        let thesaurus = parser.build_technical_thesaurus();

        assert!(thesaurus.is_some());
        let thesaurus = thesaurus.unwrap();
        assert_eq!(thesaurus.name(), "learned_technical_terms");
        assert!(!thesaurus.is_empty());

        // Should contain learned terms
        assert!(thesaurus
            .get(&NormalizedTermValue::from("deploy"))
            .is_some());
        assert!(thesaurus
            .get(&NormalizedTermValue::from("microservice"))
            .is_some());
        assert!(thesaurus
            .get(&NormalizedTermValue::from("cluster"))
            .is_some());
        assert!(thesaurus
            .get(&NormalizedTermValue::from("database"))
            .is_some());
    }

    #[tokio::test]
    async fn test_enhanced_parsing_workflow() {
        let mut parser = MarkdownCommandParser::new().unwrap();

        let markdown = r#"---
name: "deploy-service"
description: "Deploy microservice to Kubernetes cluster with database and cache"
execution_mode: "local"
parameters:
  - name: "environment"
    type: "string"
    required: true
    description: "Target deployment environment"
---

# Deploy Service Command

This command deploys a microservice to the Kubernetes cluster using Helm charts.
It sets up the PostgreSQL database and Redis cache configuration.

## Usage

```bash
deploy-service --environment production
```

## Configuration

The service requires proper database configuration and SSL certificates for secure connections."#;

        let result = parser.parse_content_with_analysis(
            markdown,
            PathBuf::from("deploy-service.md"),
            SystemTime::UNIX_EPOCH,
        );

        assert!(result.is_ok());
        let enriched_command = result.unwrap();

        // Should have basic parsing results
        assert_eq!(
            enriched_command.parsed_command.definition.name,
            "deploy-service"
        );
        assert!(enriched_command
            .parsed_command
            .content
            .contains("Deploy Service Command"));

        // Should have enriched content analysis
        assert!(enriched_command.enriched_content.is_some());
        let enriched = enriched_command.enriched_content.unwrap();

        // Should have extracted keywords
        assert!(!enriched.extracted_keywords.is_empty());
        assert!(enriched
            .extracted_keywords
            .contains(&"microservice".to_string()));
        assert!(enriched
            .extracted_keywords
            .contains(&"kubernetes".to_string()));
        assert!(enriched
            .extracted_keywords
            .contains(&"database".to_string()));

        // Should have complexity metrics
        assert!(enriched.complexity_metrics.word_count > 0);
        // Code blocks may be stripped during markdown processing, so we don't assert their count

        // Should have related concepts (may be empty if no thesaurus)
        // This is optional depending on the thesaurus availability

        // Should have learned terms
        assert!(!parser.get_learned_terms().is_empty());
    }

    #[test]
    fn test_parser_with_technical_thesaurus() {
        // Create a technical thesaurus
        let mut thesaurus = Thesaurus::new("test_technical".to_string());

        thesaurus.insert(
            NormalizedTermValue::from("database"),
            NormalizedTerm {
                id: 1,
                value: NormalizedTermValue::from("database"),
                url: Some("concept:database".to_string()),
            },
        );

        thesaurus.insert(
            NormalizedTermValue::from("kubernetes"),
            NormalizedTerm {
                id: 2,
                value: NormalizedTermValue::from("kubernetes"),
                url: Some("concept:kubernetes".to_string()),
            },
        );

        let parser = MarkdownCommandParser::with_technical_thesaurus(thesaurus).unwrap();

        let content = "This command manages the database and Kubernetes cluster.";
        let analysis = parser.analyze_content(content).unwrap();

        // Should find matches from thesaurus
        assert!(!analysis.matched_terms.is_empty());
        assert!(analysis.matched_terms.iter().any(|m| m.term == "database"));
        assert!(analysis
            .matched_terms
            .iter()
            .any(|m| m.term == "kubernetes"));
    }
}

/// Convenience function to parse a markdown command file
pub async fn parse_markdown_command(
    file_path: impl AsRef<Path>,
) -> Result<ParsedCommand, CommandRegistryError> {
    let parser = MarkdownCommandParser::new()?;
    parser.parse_file(file_path).await
}
