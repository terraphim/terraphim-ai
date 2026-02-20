//! Knowledge Graph Normalization Example
//!
//! This example demonstrates how to:
//! 1. Load markdown files from a knowledge corpus
//! 2. Extract entities and concepts using pattern matching
//! 3. Build a thesaurus/ontology from extracted terms
//! 4. Normalize entities to the ontology
//! 5. Compute coverage signals to judge quality
//!
//! Run with:
//! ```bash
//! cargo run --example kg_normalization -p terraphim_types --features "hgnc"
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Represents a document from the knowledge corpus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusDocument {
    pub path: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub linked_terms: Vec<String>,
}

/// Entry in the ontology/thesaurus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyEntry {
    pub term: String,
    pub normalized_term: String,
    pub definition: Option<String>,
    pub source: String,
    pub frequency: u32,
}

/// Knowledge graph node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub sources: Vec<String>,
}

/// Knowledge graph edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgEdge {
    pub source: String,
    pub target: String,
    pub relationship: String,
}

/// Extracted entity with normalization
#[derive(Debug, Clone)]
pub struct NormalizedEntity {
    pub raw_term: String,
    pub normalized_term: String,
    pub entity_type: String,
    pub grounding_uri: Option<String>,
    pub confidence: f32,
}

/// Knowledge graph normalizer
pub struct KgNormalizer {
    ontology: HashMap<String, OntologyEntry>,
    entities_by_type: HashMap<String, Vec<NormalizedEntity>>,
}

impl KgNormalizer {
    pub fn new() -> Self {
        Self {
            ontology: HashMap::new(),
            entities_by_type: HashMap::new(),
        }
    }

    /// Load documents from a directory
    pub fn load_corpus(&mut self, corpus_path: &str) -> Vec<CorpusDocument> {
        let mut documents = Vec::new();
        let path = Path::new(corpus_path);

        if !path.exists() {
            eprintln!("Warning: Corpus path does not exist: {}", corpus_path);
            return documents;
        }

        // Load markdown files recursively
        fn visit_dir(dir: &Path, docs: &mut Vec<CorpusDocument>) {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        visit_dir(&path, docs);
                    } else if path.extension().map_or(false, |e| e == "md") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let (title, tags, linked_terms) = parse_markdown_frontmatter(&content);
                            let doc = CorpusDocument {
                                path: path.to_string_lossy().to_string(),
                                title,
                                content: content.clone(),
                                tags,
                                linked_terms,
                            };
                            docs.push(doc);
                        }
                    }
                }
            }
        }

        visit_dir(path, &mut documents);
        println!("Loaded {} documents from corpus", documents.len());
        documents
    }

    /// Build ontology from corpus documents
    pub fn build_ontology(&mut self, documents: &[CorpusDocument]) {
        let mut term_freq: HashMap<String, (String, u32)> = HashMap::new();

        for doc in documents {
            // Extract terms from linked_terms in frontmatter
            for term in &doc.linked_terms {
                let normalized = normalize_term(term);
                let entry = term_freq
                    .entry(normalized.clone())
                    .or_insert((term.clone(), 0));
                entry.1 += 1;
            }

            // Extract terms from content (headers and key phrases)
            let terms = extract_terms_from_content(&doc.content);
            for term in terms {
                let normalized = normalize_term(&term);
                let entry = term_freq.entry(normalized.clone()).or_insert((term, 0));
                entry.1 += 1;
            }
        }

        // Build ontology entries
        for (normalized, (original, freq)) in term_freq {
            let entry = OntologyEntry {
                term: original,
                normalized_term: normalized.clone(),
                definition: None,
                source: "corpus".to_string(),
                frequency: freq,
            };
            self.ontology.insert(normalized, entry);
        }

        println!("Built ontology with {} terms", self.ontology.len());
    }

    /// Extract entities from text using the ontology - more selective version
    pub fn extract_and_normalize(&self, text: &str) -> Vec<NormalizedEntity> {
        let mut entities = Vec::new();

        // Filter out common stop words and short terms
        let stop_words = vec![
            "the", "and", "or", "for", "with", "from", "that", "this", "are", "was", "were",
            "been", "have", "has", "had", "will", "would", "could", "should", "can", "may",
            "might", "must", "all", "any", "some", "each", "every", "both", "few", "more", "most",
            "other", "such", "no", "not", "only", "same", "so", "than", "too", "very", "just",
            "but", "about", "into", "over", "after", "between", "out", "against", "during",
            "without", "before", "under", "around", "among", "per", "non", "pro", "dom", "ui",
            "rag", "us", "now", "low",
        ];

        // Find matches in ontology - only significant terms
        for (term, entry) in &self.ontology {
            // Skip very short terms or stop words
            if term.len() < 4 || stop_words.contains(&term.as_str()) {
                continue;
            }

            let term_lower = term.to_lowercase();
            let text_lower = text.to_lowercase();

            // Check for word boundary matches (more precise)
            let pattern = format!(" {}", term_lower);
            if text_lower.contains(&pattern) || text_lower.starts_with(&term_lower) {
                // Determine confidence based on match quality
                let confidence = if text_lower.contains(&format!(" {} ", term_lower)) {
                    1.0 // Full word match
                } else if text_lower.starts_with(&format!("{} ", term_lower)) {
                    0.9 // Starts with
                } else {
                    0.7 // Partial
                };

                entities.push(NormalizedEntity {
                    raw_term: term.clone(),
                    normalized_term: entry.normalized_term.clone(),
                    entity_type: "concept".to_string(),
                    grounding_uri: Some(format!(
                        "urn:terraphim:{}",
                        entry.normalized_term.replace(' ', "_")
                    )),
                    confidence,
                });
            }
        }

        // Sort by confidence and take top entities
        entities.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        entities.truncate(20); // Limit to top 20

        // Deduplicate by normalized term
        let mut seen = HashSet::new();
        entities.retain(|e| seen.insert(e.normalized_term.clone()));

        entities
    }

    /// Compute coverage signal for the normalization
    pub fn compute_coverage(
        &self,
        entities: &[NormalizedEntity],
        threshold: f32,
    ) -> CoverageResult {
        let total_terms = self.ontology.len();
        let matched = entities.len();
        let ratio = if total_terms > 0 {
            matched as f32 / total_terms as f32
        } else {
            0.0
        };

        CoverageResult {
            total_ontology_terms: total_terms,
            matched_terms: matched,
            coverage_ratio: ratio,
            threshold,
            needs_review: ratio < threshold,
        }
    }

    /// Get all entities by type
    pub fn get_entities_by_type(&self) -> &HashMap<String, Vec<NormalizedEntity>> {
        &self.entities_by_type
    }

    /// Get ontology entries
    pub fn get_ontology(&self) -> &HashMap<String, OntologyEntry> {
        &self.ontology
    }
}

impl Default for KgNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Coverage result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageResult {
    pub total_ontology_terms: usize,
    pub matched_terms: usize,
    pub coverage_ratio: f32,
    pub threshold: f32,
    pub needs_review: bool,
}

/// Parse markdown frontmatter to extract title, tags, and linked_terms
fn parse_markdown_frontmatter(content: &str) -> (String, Vec<String>, Vec<String>) {
    let mut title = String::new();
    let mut tags = Vec::new();
    let mut linked_terms = Vec::new();

    if content.starts_with("---") {
        if let Some(end) = content[3..].find("---") {
            let frontmatter = &content[3..end + 3];
            for line in frontmatter.lines() {
                let line = line.trim();
                if line.starts_with("title:") {
                    title = line
                        .trim_start_matches("title:")
                        .trim()
                        .trim_matches('"')
                        .to_string();
                } else if line.starts_with("tags:") {
                    // Parse tags array
                    if let Some(start) = line.find('[') {
                        if let Some(end) = line.find(']') {
                            let tags_str = &line[start + 1..end];
                            tags = tags_str
                                .split(',')
                                .map(|s| s.trim().trim_matches('"').to_string())
                                .collect();
                        }
                    }
                } else if line.starts_with("linked_terms:") {
                    // Parse linked_terms array
                    if let Some(start) = line.find('[') {
                        if let Some(end) = line.find(']') {
                            let terms_str = &line[start + 1..end];
                            linked_terms = terms_str
                                .split(',')
                                .map(|s| s.trim().trim_matches('"').to_string())
                                .collect();
                        }
                    }
                }
            }
        }
    }

    // If no title from frontmatter, try to get first header
    if title.is_empty() {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("# ") {
                title = trimmed[2..].to_string();
                break;
            }
        }
    }

    (title, tags, linked_terms)
}

/// Normalize a term for matching
fn normalize_term(term: &str) -> String {
    term.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract potential terms from markdown content
fn extract_terms_from_content(content: &str) -> Vec<String> {
    let mut terms = Vec::new();

    // Extract headers
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            let header = trimmed[3..].trim();
            // Split header into potential terms
            for word in header.split(&[',', '-', '/'][..]) {
                let word = word.trim();
                if word.len() > 2 && !word.contains("://") {
                    terms.push(word.to_string());
                }
            }
        }
    }

    terms
}

fn main() {
    println!("=== Knowledge Graph Normalization Example\n");

    // Initialize the normalizer
    let mut normalizer = KgNormalizer::new();

    // Load corpus from the knowledge directory
    let corpus_path = "/Users/alex/cto-executive-system/knowledge";
    println!("Loading corpus from: {}", corpus_path);

    let documents = normalizer.load_corpus(corpus_path);

    if documents.is_empty() {
        println!("No documents loaded. Please check the corpus path.");
        return;
    }

    // Show sample documents
    println!("\n--- Sample Documents ---");
    for doc in documents.iter().take(3) {
        println!("  - {} ({})", doc.title, doc.tags.join(", "));
    }

    // Build ontology from corpus
    println!("\n--- Building Ontology ---");
    normalizer.build_ontology(&documents);

    // Show ontology statistics
    let ontology = normalizer.get_ontology();
    println!("\n--- Ontology Statistics ---");
    println!("  Total terms: {}", ontology.len());

    // Get most frequent terms
    let mut terms: Vec<_> = ontology.values().collect();
    terms.sort_by(|a, b| b.frequency.cmp(&a.frequency));

    println!("\n--- Top Terms by Frequency ---");
    for entry in terms.iter().take(10) {
        println!("  {} (freq: {})", entry.term, entry.frequency);
    }

    // Extract and normalize from sample content
    println!("\n--- Entity Extraction & Normalization ---");

    let sample_texts = [
        "Knowledge graphs enable context-rich AI systems by connecting entities through relationships. \
         The schema-first approach ensures quality and consistency in entity extraction. \
         Ontology-based reasoning allows for sophisticated inference over connected data.",
        "Terraphim uses dynamic ontology with schema-first extraction. \
         Coverage signals judge extraction quality. \
         Normalization grounds entities to canonical URIs for interoperability.",
        "Context engineering provides the framework for building AI systems that understand \
         domain-specific terminology. Linked terms enable cross-referencing between concepts.",
    ];

    let mut all_entities: Vec<NormalizedEntity> = Vec::new();

    for (i, text) in sample_texts.iter().enumerate() {
        println!("\n  Sample {}:", i + 1);
        println!(
            "    \"{}\"",
            text.chars().take(60).collect::<String>() + "..."
        );

        let entities = normalizer.extract_and_normalize(text);
        println!("    Extracted {} entities:", entities.len());

        for entity in &entities {
            println!(
                "      - {} -> {} (conf: {:.2})",
                entity.raw_term, entity.normalized_term, entity.confidence
            );
        }

        all_entities.extend(entities);
    }

    // Compute coverage
    println!("\n--- Coverage Analysis ---");
    let coverage = normalizer.compute_coverage(&all_entities, 0.3);

    println!("  Total ontology terms: {}", coverage.total_ontology_terms);
    println!("  Matched terms: {}", coverage.matched_terms);
    println!("  Coverage ratio: {:.1}%", coverage.coverage_ratio * 100.0);
    println!("  Threshold: {:.0}%", coverage.threshold * 100.0);
    println!("  Needs review: {}", coverage.needs_review);

    // Judgement based on coverage
    println!("\n--- Quality Judgement ---");
    if coverage.coverage_ratio >= 0.7 {
        println!("  [EXCELLENT] High coverage - ontology well-suited for this corpus");
    } else if coverage.coverage_ratio >= 0.4 {
        println!("  [GOOD] Moderate coverage - ontology captures main concepts");
    } else if coverage.coverage_ratio >= 0.2 {
        println!("  [NEEDS IMPROVEMENT] Low coverage - consider expanding ontology");
    } else {
        println!("  [POOR] Very low coverage - corpus may need different approach");
    }

    // Export thesaurus for Terraphim
    println!("\n--- Exporting Thesaurus ---");
    let mut thesaurus_json =
        String::from("{\n  \"name\": \"Knowledge Graph Terms\",\n  \"data\": {\n");
    let mut first = true;
    for entry in terms.iter().take(50) {
        if !first {
            thesaurus_json.push_str(",\n");
        }
        first = false;
        thesaurus_json.push_str(&format!(
            "    \"{}\": {{\"id\": {}, \"nterm\": \"{}\", \"url\": \"urn:terraphim:{}\"}}",
            entry.term,
            entry.frequency,
            entry.normalized_term,
            entry.normalized_term.replace(' ', "-")
        ));
    }
    thesaurus_json.push_str("\n  }\n}\n");

    println!("  Generated thesaurus with top 50 terms");
    println!("  (Can be saved and used with Terraphim automata)");

    println!("\n=== Example Complete ===");
}
