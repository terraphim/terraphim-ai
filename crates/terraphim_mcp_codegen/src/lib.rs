//! MCP Code Generator - Generates TypeScript and Python wrappers for MCP tools
//!
//! This crate enables AI agents to use MCP tools as importable code modules,
//! achieving massive token reduction by allowing code-based tool usage instead
//! of traditional tool calling patterns.

pub mod introspection;
pub mod python_gen;
pub mod runtime;
pub mod typescript_gen;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Error types for MCP code generation
#[derive(Error, Debug)]
pub enum CodegenError {
    #[error("Template error: {0}")]
    Template(#[from] tera::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("MCP introspection error: {0}")]
    Introspection(String),

    #[error("Invalid tool specification: {0}")]
    InvalidSpec(String),

    #[error("Unsupported type: {0}")]
    UnsupportedType(String),
}

pub type Result<T> = std::result::Result<T, CodegenError>;

/// Metadata for a single MCP tool parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterMetadata {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// JSON Schema type (string, number, integer, boolean, array, object)
    pub json_type: String,
    /// Whether the parameter is required
    pub required: bool,
    /// Default value if any
    pub default_value: Option<serde_json::Value>,
    /// For arrays, the item type
    pub array_item_type: Option<String>,
    /// For objects, the properties
    pub object_properties: Option<HashMap<String, ParameterMetadata>>,
}

impl ParameterMetadata {
    /// Convert JSON type to TypeScript type
    pub fn to_typescript_type(&self) -> String {
        match self.json_type.as_str() {
            "string" => "string".to_string(),
            "number" => "number".to_string(),
            "integer" => "number".to_string(),
            "boolean" => "boolean".to_string(),
            "array" => {
                let item_type = self
                    .array_item_type
                    .as_deref()
                    .unwrap_or("any")
                    .to_string();
                format!("{}[]", item_type)
            }
            "object" => "Record<string, any>".to_string(),
            "null" => "null".to_string(),
            _ => "any".to_string(),
        }
    }

    /// Convert JSON type to Python type hint
    pub fn to_python_type(&self) -> String {
        match self.json_type.as_str() {
            "string" => "str".to_string(),
            "number" => "float".to_string(),
            "integer" => "int".to_string(),
            "boolean" => "bool".to_string(),
            "array" => {
                let item_type = self.array_item_type.as_deref().unwrap_or("Any");
                let python_item = match item_type {
                    "string" => "str",
                    "number" => "float",
                    "integer" => "int",
                    "boolean" => "bool",
                    _ => "Any",
                };
                format!("List[{}]", python_item)
            }
            "object" => "Dict[str, Any]".to_string(),
            "null" => "None".to_string(),
            _ => "Any".to_string(),
        }
    }
}

/// Metadata for a single MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Tool name (e.g., "search", "autocomplete_terms")
    pub name: String,
    /// Human-readable title
    pub title: Option<String>,
    /// Tool description
    pub description: String,
    /// Tool category for discovery
    pub category: ToolCategory,
    /// Tool capabilities for discovery
    pub capabilities: Vec<String>,
    /// Input parameters
    pub parameters: Vec<ParameterMetadata>,
    /// Return type description
    pub return_type: String,
    /// Example usage code
    pub examples: Vec<String>,
}

/// Tool category for progressive discovery
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    KnowledgeGraph,
    Autocomplete,
    TextProcessing,
    Configuration,
    Analysis,
    Serialization,
    Other(String),
}

impl std::fmt::Display for ToolCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolCategory::KnowledgeGraph => write!(f, "knowledge-graph"),
            ToolCategory::Autocomplete => write!(f, "autocomplete"),
            ToolCategory::TextProcessing => write!(f, "text-processing"),
            ToolCategory::Configuration => write!(f, "configuration"),
            ToolCategory::Analysis => write!(f, "analysis"),
            ToolCategory::Serialization => write!(f, "serialization"),
            ToolCategory::Other(s) => write!(f, "{}", s),
        }
    }
}

/// Complete MCP server metadata for code generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerMetadata {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
    /// All available tools
    pub tools: Vec<ToolMetadata>,
    /// Server description
    pub description: Option<String>,
}

/// Output format for code generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    TypeScript,
    Python,
}

impl std::str::FromStr for OutputFormat {
    type Err = CodegenError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "typescript" | "ts" => Ok(OutputFormat::TypeScript),
            "python" | "py" => Ok(OutputFormat::Python),
            _ => Err(CodegenError::InvalidSpec(format!(
                "Unknown output format: {}",
                s
            ))),
        }
    }
}

/// Configuration for code generation
#[derive(Debug, Clone)]
pub struct CodegenConfig {
    /// Output format (TypeScript or Python)
    pub format: OutputFormat,
    /// Output file path
    pub output_path: std::path::PathBuf,
    /// Module name to use
    pub module_name: String,
    /// Whether to generate async functions
    pub async_functions: bool,
    /// Include documentation comments
    pub include_docs: bool,
    /// Include usage examples
    pub include_examples: bool,
}

impl Default for CodegenConfig {
    fn default() -> Self {
        Self {
            format: OutputFormat::TypeScript,
            output_path: std::path::PathBuf::from("terraphim.ts"),
            module_name: "terraphim".to_string(),
            async_functions: true,
            include_docs: true,
            include_examples: true,
        }
    }
}

/// Main code generator trait
pub trait CodeGenerator {
    /// Generate code for all tools in the metadata
    fn generate(&self, metadata: &McpServerMetadata, config: &CodegenConfig) -> Result<String>;

    /// Generate code for a single tool
    fn generate_tool(&self, tool: &ToolMetadata, config: &CodegenConfig) -> Result<String>;
}

/// Generate code based on configuration
pub fn generate_code(metadata: &McpServerMetadata, config: &CodegenConfig) -> Result<String> {
    match config.format {
        OutputFormat::TypeScript => {
            let generator = typescript_gen::TypeScriptGenerator::new()?;
            generator.generate(metadata, config)
        }
        OutputFormat::Python => {
            let generator = python_gen::PythonGenerator::new()?;
            generator.generate(metadata, config)
        }
    }
}

/// Categorize tools based on their names and descriptions
pub fn categorize_tool(tool_name: &str) -> ToolCategory {
    match tool_name {
        "search" | "find_matches" | "is_all_terms_connected_by_path" => {
            ToolCategory::KnowledgeGraph
        }
        name if name.contains("autocomplete") => ToolCategory::Autocomplete,
        "replace_matches" | "extract_paragraphs_from_automata" | "json_decode" => {
            ToolCategory::TextProcessing
        }
        "update_config_tool" => ToolCategory::Configuration,
        "load_thesaurus" | "load_thesaurus_from_json" | "build_autocomplete_index" => {
            ToolCategory::Analysis
        }
        name if name.contains("serialize") || name.contains("deserialize") => {
            ToolCategory::Serialization
        }
        _ => ToolCategory::Other("uncategorized".to_string()),
    }
}

/// Extract capabilities from tool metadata
pub fn extract_capabilities(tool: &ToolMetadata) -> Vec<String> {
    let mut capabilities = Vec::new();

    // Based on tool name patterns
    if tool.name.contains("search") {
        capabilities.push("search".to_string());
    }
    if tool.name.contains("autocomplete") {
        capabilities.push("autocomplete".to_string());
        capabilities.push("suggestions".to_string());
    }
    if tool.name.contains("fuzzy") {
        capabilities.push("fuzzy-matching".to_string());
    }
    if tool.name.contains("find") || tool.name.contains("match") {
        capabilities.push("pattern-matching".to_string());
    }
    if tool.name.contains("replace") {
        capabilities.push("text-transformation".to_string());
    }
    if tool.name.contains("extract") {
        capabilities.push("text-extraction".to_string());
    }
    if tool.name.contains("load") {
        capabilities.push("data-loading".to_string());
    }
    if tool.name.contains("serialize") || tool.name.contains("deserialize") {
        capabilities.push("serialization".to_string());
    }
    if tool.name.contains("config") {
        capabilities.push("configuration".to_string());
    }

    // Add read/write based on likely side effects
    if tool.name.starts_with("update") || tool.name.starts_with("build") {
        capabilities.push("write".to_string());
    } else {
        capabilities.push("read".to_string());
    }

    capabilities
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_tool() {
        assert_eq!(categorize_tool("search"), ToolCategory::KnowledgeGraph);
        assert_eq!(
            categorize_tool("autocomplete_terms"),
            ToolCategory::Autocomplete
        );
        assert_eq!(
            categorize_tool("fuzzy_autocomplete_search"),
            ToolCategory::Autocomplete
        );
        assert_eq!(
            categorize_tool("replace_matches"),
            ToolCategory::TextProcessing
        );
        assert_eq!(
            categorize_tool("update_config_tool"),
            ToolCategory::Configuration
        );
        assert_eq!(
            categorize_tool("serialize_autocomplete_index"),
            ToolCategory::Serialization
        );
    }

    #[test]
    fn test_extract_capabilities() {
        let tool = ToolMetadata {
            name: "fuzzy_autocomplete_search".to_string(),
            title: None,
            description: "Fuzzy search".to_string(),
            category: ToolCategory::Autocomplete,
            capabilities: vec![],
            parameters: vec![],
            return_type: "string[]".to_string(),
            examples: vec![],
        };

        let caps = extract_capabilities(&tool);
        assert!(caps.contains(&"autocomplete".to_string()));
        assert!(caps.contains(&"fuzzy-matching".to_string()));
        assert!(caps.contains(&"read".to_string()));
    }

    #[test]
    fn test_parameter_type_conversion() {
        let param = ParameterMetadata {
            name: "items".to_string(),
            description: "Array of strings".to_string(),
            json_type: "array".to_string(),
            required: true,
            default_value: None,
            array_item_type: Some("string".to_string()),
            object_properties: None,
        };

        assert_eq!(param.to_typescript_type(), "string[]");
        assert_eq!(param.to_python_type(), "List[str]");
    }

    #[test]
    fn test_output_format_parsing() {
        assert_eq!(
            "typescript".parse::<OutputFormat>().unwrap(),
            OutputFormat::TypeScript
        );
        assert_eq!(
            "ts".parse::<OutputFormat>().unwrap(),
            OutputFormat::TypeScript
        );
        assert_eq!(
            "python".parse::<OutputFormat>().unwrap(),
            OutputFormat::Python
        );
        assert_eq!(
            "py".parse::<OutputFormat>().unwrap(),
            OutputFormat::Python
        );
    }
}
