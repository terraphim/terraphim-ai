//! Enhanced file operations with semantic awareness for Terraphim TUI
//!
//! This module provides intelligent file operations that go beyond basic file manipulation,
//! incorporating semantic understanding, content analysis, and relationship discovery.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// File classification types based on content analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileCategory {
    /// Code and programming files
    Code {
        language: String,
        frameworks: Vec<String>,
    },
    /// Documentation and text files
    Documentation {
        format: String,     // "markdown", "rst", "txt", etc.
        complexity: String, // "simple", "technical", "academic"
    },
    /// Configuration files
    Configuration {
        config_type: String, // "json", "yaml", "toml", "ini", etc.
        purpose: String,     // "app", "build", "deploy", etc.
    },
    /// Data files
    Data {
        format: String,    // "csv", "json", "xml", "binary", etc.
        structure: String, // "structured", "semi-structured", "unstructured"
    },
    /// Media files
    Media {
        media_type: String, // "image", "video", "audio"
        format: String,
    },
    /// Archive files
    Archive {
        archive_type: String, // "zip", "tar", "gz", etc.
        compression: String,
    },
    /// Script files
    Script {
        interpreter: String, // "bash", "python", "node", etc.
        purpose: String,     // "build", "deploy", "utility", etc.
    },
    /// Other or uncategorized files
    Other {
        mime_type: String,
        description: String,
    },
}

/// Semantic metadata extracted from file content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMetadata {
    /// Primary concepts identified in the file
    pub concepts: Vec<String>,
    /// Named entities (people, places, organizations)
    pub entities: Vec<FileEntity>,
    /// Important keywords and terms
    pub keywords: Vec<String>,
    /// File's semantic similarity score to queries
    pub relevance_score: Option<f64>,
    /// File's semantic fingerprint for similarity matching
    pub semantic_fingerprint: Option<String>,
    /// Content summary
    pub summary: Option<String>,
    /// Estimated reading time in minutes
    pub reading_time_minutes: Option<u32>,
    /// Content complexity score (0.0-1.0)
    pub complexity_score: Option<f64>,
}

/// Named entity extracted from file content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntity {
    /// Entity text
    pub text: String,
    /// Entity type (PERSON, ORG, LOCATION, etc.)
    pub entity_type: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Position in the file (line, character)
    pub position: Option<FilePosition>,
}

/// Position reference within a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePosition {
    pub line_number: usize,
    pub character_start: usize,
    pub character_end: usize,
}

/// File relationship analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRelationships {
    /// Files with similar content
    pub similar_files: Vec<SimilarFile>,
    /// Files that are related thematically
    pub related_files: Vec<RelatedFile>,
    /// Files that reference this file
    pub referenced_by: Vec<FileReference>,
    /// Files that this file references
    pub references: Vec<FileReference>,
}

/// Similar file with similarity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarFile {
    pub file_path: PathBuf,
    pub similarity_score: f64,
    pub similarity_type: SimilarityType,
    pub shared_concepts: Vec<String>,
}

/// Types of similarity between files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimilarityType {
    /// Content similarity (text overlap)
    Content,
    /// Semantic similarity (meaning overlap)
    Semantic,
    /// Structural similarity (format/organization)
    Structural,
    /// Topic similarity (subject matter)
    Topic,
}

/// Related file with relationship metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedFile {
    pub file_path: PathBuf,
    pub relationship_type: RelationshipType,
    pub confidence: f64,
    pub explanation: String,
}

/// Types of relationships between files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    /// Sequential dependency (imports, includes)
    Dependency,
    /// Thematic relationship (same topic/domain)
    Thematic,
    /// Temporal relationship (created/modified around same time)
    Temporal,
    /// Structural relationship (same directory/project structure)
    Structural,
    /// Functional relationship (serves similar purpose)
    Functional,
}

/// File reference information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReference {
    pub file_path: PathBuf,
    pub reference_type: ReferenceType,
    pub context: String,
    pub line_number: Option<usize>,
}

/// Types of file references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReferenceType {
    /// Import/include statement
    Import,
    /// Link/hyperlink
    Link,
    /// File path reference
    Path,
    /// Documentation reference
    Documentation,
    /// Configuration reference
    Configuration,
}

/// File search result with semantic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchResult {
    /// File path
    pub file_path: PathBuf,
    /// File name
    pub file_name: String,
    /// File category
    pub category: FileCategory,
    /// Relevance score to search query
    pub relevance_score: f64,
    /// Match type (exact, semantic, partial)
    pub match_type: MatchType,
    /// Matching lines with context
    pub matches: Vec<FileMatch>,
    /// File metadata
    pub metadata: SemanticMetadata,
    /// File size in bytes
    pub file_size: u64,
    /// Last modified timestamp
    pub last_modified: String,
}

/// Match information for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMatch {
    /// Line number where match was found
    pub line_number: usize,
    /// Matching text
    pub matched_text: String,
    /// Context around the match
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    /// Match confidence score
    pub confidence: f64,
}

/// Types of file matches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchType {
    /// Exact text match
    Exact,
    /// Semantic match (meaning-based)
    Semantic,
    /// Partial match (substring)
    Partial,
    /// Fuzzy match (with typos/variation)
    Fuzzy,
}

/// File indexing status and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIndexStatus {
    /// Total files indexed
    pub total_files: u64,
    /// Files currently being processed
    pub processing_files: u64,
    /// Files that failed to index
    pub failed_files: u64,
    /// Index size in bytes
    pub index_size_bytes: u64,
    /// Last index update time
    pub last_update: String,
    /// Indexing operation status
    pub status: IndexingStatus,
}

/// Current indexing operation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexingStatus {
    /// Indexing is idle
    Idle,
    /// Currently indexing files
    Indexing { progress: f64, current_file: String },
    /// Indexing completed successfully
    Completed,
    /// Indexing failed with error
    Failed { error: String },
    /// Indexing paused
    Paused,
}

/// File operation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationConfig {
    /// Directories to include in operations
    pub include_directories: Vec<PathBuf>,
    /// Directories to exclude from operations
    pub exclude_directories: Vec<PathBuf>,
    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
    /// Maximum file size for processing (in MB)
    pub max_file_size_mb: u64,
    /// Enable semantic analysis
    pub enable_semantic_analysis: bool,
    /// Similarity threshold for file relationships
    pub similarity_threshold: f64,
    /// Maximum number of similar files to return
    pub max_similar_files: usize,
    /// Cache directory for semantic data
    pub cache_directory: Option<PathBuf>,
}

impl Default for FileOperationConfig {
    fn default() -> Self {
        Self {
            include_directories: vec![],
            exclude_directories: vec![
                PathBuf::from(".git"),
                PathBuf::from("target"),
                PathBuf::from("node_modules"),
                PathBuf::from(".vscode"),
                PathBuf::from(".idea"),
            ],
            exclude_patterns: vec![
                "*.tmp".to_string(),
                "*.log".to_string(),
                "*.cache".to_string(),
            ],
            max_file_size_mb: 100, // 100MB
            enable_semantic_analysis: true,
            similarity_threshold: 0.7,
            max_similar_files: 10,
            cache_directory: Some(PathBuf::from(".terraphim_file_cache")),
        }
    }
}

/// File analysis request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysisRequest {
    /// File path to analyze
    pub file_path: PathBuf,
    /// Analysis types to perform
    pub analysis_types: Vec<AnalysisType>,
    /// Configuration options
    pub config: FileOperationConfig,
}

/// Types of analysis that can be performed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnalysisType {
    /// Categorize file by content type
    Classification,
    /// Extract semantic metadata
    SemanticExtraction,
    /// Find similar files
    SimilarityAnalysis,
    /// Find related files
    RelationshipAnalysis,
    /// Summarize content
    Summarization,
    /// Extract entities and concepts
    EntityExtraction,
}

/// File analysis response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysisResponse {
    /// File that was analyzed
    pub file_path: PathBuf,
    /// Analysis results
    pub results: FileAnalysisResults,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Any warnings or issues
    pub warnings: Vec<String>,
}

/// Complete file analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysisResults {
    /// File category
    pub category: Option<FileCategory>,
    /// Semantic metadata
    pub semantic_metadata: Option<SemanticMetadata>,
    /// File relationships
    pub relationships: Option<FileRelationships>,
    /// Content summary
    pub summary: Option<String>,
    /// Extraction errors
    pub errors: Vec<String>,
}

/// File search request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchRequest {
    /// Search query
    pub query: String,
    /// Search path (directory or file)
    pub search_path: Option<PathBuf>,
    /// File type filters
    pub file_types: Option<Vec<String>>,
    /// Enable semantic search
    pub semantic_search: bool,
    /// Maximum results to return
    pub max_results: Option<usize>,
    /// Search configuration
    pub config: FileOperationConfig,
}

/// File search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchResponse {
    /// Search results
    pub results: Vec<FileSearchResult>,
    /// Total number of matches found
    pub total_matches: usize,
    /// Search time in milliseconds
    pub search_time_ms: u64,
    /// Query used for search (may be expanded)
    pub expanded_query: Option<String>,
}

/// File tagging request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTaggingRequest {
    /// File path to tag
    pub file_path: PathBuf,
    /// Tags to apply
    pub tags: Vec<String>,
    /// Auto-suggest additional tags
    pub auto_suggest: bool,
    /// Configuration
    pub config: FileOperationConfig,
}

/// File tagging response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTaggingResponse {
    /// File that was tagged
    pub file_path: PathBuf,
    /// Applied tags
    pub applied_tags: Vec<String>,
    /// Auto-suggested tags
    pub suggested_tags: Vec<String>,
    /// Tag confidence scores
    pub tag_confidence: HashMap<String, f64>,
}

/// Utility functions for file operations
pub mod utils {
    use super::*;

    /// Determine file category based on path and content
    pub fn categorize_file(file_path: &std::path::Path, content: Option<&str>) -> FileCategory {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let file_name = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        match extension {
            "rs" => FileCategory::Code {
                language: "Rust".to_string(),
                frameworks: detect_rust_frameworks(content.unwrap_or("")),
            },
            "js" | "ts" | "jsx" | "tsx" => FileCategory::Code {
                language: "JavaScript".to_string(),
                frameworks: detect_js_frameworks(content.unwrap_or("")),
            },
            "py" => FileCategory::Code {
                language: "Python".to_string(),
                frameworks: detect_python_frameworks(content.unwrap_or("")),
            },
            "md" | "markdown" => FileCategory::Documentation {
                format: "markdown".to_string(),
                complexity: estimate_documentation_complexity(content.unwrap_or("")),
            },
            "json" => FileCategory::Configuration {
                config_type: "json".to_string(),
                purpose: infer_json_purpose(file_name, content.unwrap_or("")),
            },
            "yaml" | "yml" => FileCategory::Configuration {
                config_type: "yaml".to_string(),
                purpose: infer_yaml_purpose(file_name, content.unwrap_or("")),
            },
            "toml" => FileCategory::Configuration {
                config_type: "toml".to_string(),
                purpose: infer_toml_purpose(file_name, content.unwrap_or("")),
            },
            "csv" => FileCategory::Data {
                format: "csv".to_string(),
                structure: "structured".to_string(),
            },
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" => FileCategory::Media {
                media_type: "image".to_string(),
                format: extension.to_string(),
            },
            "mp4" | "avi" | "mkv" | "mov" => FileCategory::Media {
                media_type: "video".to_string(),
                format: extension.to_string(),
            },
            "mp3" | "wav" | "flac" | "ogg" => FileCategory::Media {
                media_type: "audio".to_string(),
                format: extension.to_string(),
            },
            "zip" | "tar" | "gz" | "bz2" | "xz" => FileCategory::Archive {
                archive_type: detect_archive_type(extension),
                compression: extension.to_string(),
            },
            "sh" | "bash" | "zsh" | "fish" => FileCategory::Script {
                interpreter: "bash".to_string(),
                purpose: infer_script_purpose(file_name, content.unwrap_or("")),
            },
            _ => FileCategory::Other {
                mime_type: infer_mime_type(extension),
                description: format!("File with extension: {}", extension),
            },
        }
    }

    /// Detect Rust frameworks from content
    fn detect_rust_frameworks(content: &str) -> Vec<String> {
        let mut frameworks = Vec::new();

        if content.contains("tokio") {
            frameworks.push("tokio".to_string());
        }
        if content.contains("serde") {
            frameworks.push("serde".to_string());
        }
        if content.contains("actix") {
            frameworks.push("actix".to_string());
        }
        if content.contains("rocket") {
            frameworks.push("rocket".to_string());
        }
        if content.contains("clap") {
            frameworks.push("clap".to_string());
        }
        if content.contains("tracing") {
            frameworks.push("tracing".to_string());
        }

        frameworks
    }

    /// Detect JavaScript frameworks from content
    fn detect_js_frameworks(content: &str) -> Vec<String> {
        let mut frameworks = Vec::new();

        if content.contains("react") || content.contains("React") {
            frameworks.push("react".to_string());
        }
        if content.contains("vue") || content.contains("Vue") {
            frameworks.push("vue".to_string());
        }
        if content.contains("angular") || content.contains("Angular") {
            frameworks.push("angular".to_string());
        }
        if content.contains("express") {
            frameworks.push("express".to_string());
        }
        if content.contains("node") || content.contains("Node") {
            frameworks.push("node".to_string());
        }
        if content.contains("webpack") {
            frameworks.push("webpack".to_string());
        }
        if content.contains("typescript") || content.contains("TypeScript") {
            frameworks.push("typescript".to_string());
        }

        frameworks
    }

    /// Detect Python frameworks from content
    fn detect_python_frameworks(content: &str) -> Vec<String> {
        let mut frameworks = Vec::new();

        if content.contains("django") || content.contains("Django") {
            frameworks.push("django".to_string());
        }
        if content.contains("flask") || content.contains("Flask") {
            frameworks.push("flask".to_string());
        }
        if content.contains("fastapi") || content.contains("FastAPI") {
            frameworks.push("fastapi".to_string());
        }
        if content.contains("pytest") {
            frameworks.push("pytest".to_string());
        }
        if content.contains("pandas") {
            frameworks.push("pandas".to_string());
        }
        if content.contains("numpy") {
            frameworks.push("numpy".to_string());
        }
        if content.contains("requests") {
            frameworks.push("requests".to_string());
        }

        frameworks
    }

    /// Estimate documentation complexity
    fn estimate_documentation_complexity(content: &str) -> String {
        let word_count = content.split_whitespace().count();
        let code_block_count = content.matches("```").count() / 2;
        let _heading_count = content.matches('#').count();

        if word_count > 2000 || code_block_count > 10 {
            "technical".to_string()
        } else if word_count > 500 || code_block_count > 3 {
            "detailed".to_string()
        } else {
            "simple".to_string()
        }
    }

    /// Infer JSON configuration purpose
    fn infer_json_purpose(file_name: &str, _content: &str) -> String {
        if file_name.contains("package") {
            "package".to_string()
        } else if file_name.contains("tsconfig") || file_name.contains("config") {
            "build".to_string()
        } else if file_name.contains("settings") {
            "application".to_string()
        } else if file_name.contains("manifest") {
            "project".to_string()
        } else {
            "configuration".to_string()
        }
    }

    /// Infer YAML configuration purpose
    fn infer_yaml_purpose(file_name: &str, _content: &str) -> String {
        if file_name.contains("docker-compose") {
            "deployment".to_string()
        } else if file_name.contains("github-actions") || file_name.contains(".github") {
            "ci-cd".to_string()
        } else if file_name.contains("k8s") || file_name.contains("kubernetes") {
            "orchestration".to_string()
        } else {
            "configuration".to_string()
        }
    }

    /// Infer TOML configuration purpose
    fn infer_toml_purpose(file_name: &str, _content: &str) -> String {
        if file_name.contains("Cargo") {
            "build".to_string()
        } else if file_name.contains("pyproject") {
            "project".to_string()
        } else if file_name.contains("toolchain") {
            "development".to_string()
        } else {
            "configuration".to_string()
        }
    }

    /// Detect archive type from extension
    fn detect_archive_type(extension: &str) -> String {
        match extension {
            "zip" => "zip".to_string(),
            "tar" => "tar".to_string(),
            "gz" | "bz2" | "xz" => "compressed".to_string(),
            _ => "archive".to_string(),
        }
    }

    /// Infer script purpose
    fn infer_script_purpose(file_name: &str, content: &str) -> String {
        if file_name.contains("build")
            || content.contains("cargo build")
            || content.contains("npm build")
        {
            "build".to_string()
        } else if file_name.contains("deploy") || content.contains("deploy") {
            "deployment".to_string()
        } else if file_name.contains("test")
            || content.contains("pytest")
            || content.contains("jest")
        {
            "testing".to_string()
        } else if file_name.contains("setup") || content.contains("install") {
            "setup".to_string()
        } else {
            "utility".to_string()
        }
    }

    /// Infer MIME type from extension
    fn infer_mime_type(extension: &str) -> String {
        match extension {
            "txt" => "text/plain".to_string(),
            "bin" => "application/octet-stream".to_string(),
            "exe" => "application/x-executable".to_string(),
            "dll" => "application/x-msdownload".to_string(),
            _ => format!("application/x-{}", extension),
        }
    }

    /// Calculate file reading time estimate
    pub fn estimate_reading_time(content: &str) -> u32 {
        let word_count = content.split_whitespace().count() as u32;
        // Average reading speed: 200-250 words per minute
        (word_count / 220).max(1)
    }

    /// Calculate content complexity score
    pub fn calculate_complexity_score(content: &str) -> f64 {
        let mut score = 0.0;

        // Factor in line count
        let line_count = content.lines().count() as f64;
        score += (line_count / 1000.0).min(0.3);

        // Factor in unique words
        let unique_words: std::collections::HashSet<&str> = content.split_whitespace().collect();
        let word_diversity = unique_words.len() as f64 / content.split_whitespace().count() as f64;
        score += word_diversity * 0.4;

        // Factor in code structures (brackets, punctuation)
        let code_chars = content
            .matches(|c| c == '{' || c == '}' || c == '(' || c == ')')
            .count() as f64;
        score += (code_chars / 100.0).min(0.3);

        score.min(1.0)
    }

    /// Generate semantic fingerprint for content
    pub fn generate_semantic_fingerprint(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Simple fingerprint based on word frequencies
        let mut word_counts: HashMap<String, u32> = HashMap::new();
        for word in content.split_whitespace().take(1000) {
            // Limit to first 1000 words
            *word_counts.entry(word.to_lowercase()).or_insert(0) += 1;
        }

        // Create a hash from the top words
        let mut hasher = DefaultHasher::new();
        let mut top_words: Vec<_> = word_counts.into_iter().collect::<Vec<_>>();
        top_words.sort_by(|a, b| b.1.cmp(&a.1));

        for (word, count) in top_words.into_iter().take(50) {
            word.hash(&mut hasher);
            count.hash(&mut hasher);
        }

        format!("{:x}", hasher.finish())
    }

    /// Extract key concepts from content
    pub fn extract_concepts(content: &str) -> Vec<String> {
        // Simple concept extraction - in a real implementation,
        // this would use NLP techniques
        let mut concepts = Vec::new();

        // Look for common technical terms
        let technical_terms = [
            "api",
            "database",
            "algorithm",
            "function",
            "class",
            "interface",
            "service",
            "client",
            "server",
            "protocol",
            "authentication",
            "authorization",
            "encryption",
            "security",
            "performance",
            "scalability",
            "architecture",
            "design",
            "pattern",
            "framework",
            "library",
            "dependency",
            "module",
        ];

        for term in technical_terms {
            if content.to_lowercase().contains(term) {
                concepts.push(term.to_string());
            }
        }

        concepts.sort();
        concepts.dedup();
        concepts
    }

    /// Validate file path exists and is accessible
    pub fn validate_file_path(path: &str) -> Result<PathBuf> {
        let path_buf = PathBuf::from(path);

        if !path_buf.exists() {
            anyhow::bail!("File does not exist: {}", path);
        }

        if !path_buf.is_file() {
            anyhow::bail!("Path is not a file: {}", path);
        }

        Ok(path_buf)
    }

    /// Validate directory path exists and is accessible
    pub fn validate_directory_path(path: &str) -> Result<PathBuf> {
        let path_buf = PathBuf::from(path);

        if !path_buf.exists() {
            anyhow::bail!("Directory does not exist: {}", path);
        }

        if !path_buf.is_dir() {
            anyhow::bail!("Path is not a directory: {}", path);
        }

        Ok(path_buf)
    }
}

/// File operation builder for creating analysis requests
pub struct FileOperationBuilder {
    analysis_types: Vec<AnalysisType>,
    config: FileOperationConfig,
}

impl FileOperationBuilder {
    pub fn new() -> Self {
        Self {
            analysis_types: vec![
                AnalysisType::Classification,
                AnalysisType::SemanticExtraction,
            ],
            config: FileOperationConfig::default(),
        }
    }

    pub fn with_analysis_types(mut self, types: Vec<AnalysisType>) -> Self {
        self.analysis_types = types;
        self
    }

    pub fn with_config(mut self, config: FileOperationConfig) -> Self {
        self.config = config;
        self
    }

    pub fn add_classification(mut self) -> Self {
        if !self.analysis_types.contains(&AnalysisType::Classification) {
            self.analysis_types.push(AnalysisType::Classification);
        }
        self
    }

    pub fn add_semantic_extraction(mut self) -> Self {
        if !self
            .analysis_types
            .contains(&AnalysisType::SemanticExtraction)
        {
            self.analysis_types.push(AnalysisType::SemanticExtraction);
        }
        self
    }

    pub fn add_similarity_analysis(mut self) -> Self {
        if !self
            .analysis_types
            .contains(&AnalysisType::SimilarityAnalysis)
        {
            self.analysis_types.push(AnalysisType::SimilarityAnalysis);
        }
        self
    }

    pub fn add_relationship_analysis(mut self) -> Self {
        if !self
            .analysis_types
            .contains(&AnalysisType::RelationshipAnalysis)
        {
            self.analysis_types.push(AnalysisType::RelationshipAnalysis);
        }
        self
    }

    pub fn add_summarization(mut self) -> Self {
        if !self.analysis_types.contains(&AnalysisType::Summarization) {
            self.analysis_types.push(AnalysisType::Summarization);
        }
        self
    }

    pub fn add_entity_extraction(mut self) -> Self {
        if !self
            .analysis_types
            .contains(&AnalysisType::EntityExtraction)
        {
            self.analysis_types.push(AnalysisType::EntityExtraction);
        }
        self
    }

    pub fn build(self) -> FileOperationConfig {
        self.config
    }
}

impl Default for FileOperationBuilder {
    fn default() -> Self {
        Self::new()
    }
}
