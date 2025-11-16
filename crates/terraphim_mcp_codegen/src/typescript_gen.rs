//! TypeScript Code Generator for MCP Tools

use crate::{
    introspection::{to_camel_case, to_pascal_case},
    CodeGenerator, CodegenConfig, McpServerMetadata, Result, ToolMetadata,
};
use tera::{Context, Tera};

const TYPESCRIPT_MODULE_TEMPLATE: &str = r#"
/**
 * {{ server_name }} MCP Tools
 * {{ server_description }}
 *
 * Generated automatically from MCP server introspection.
 * Version: {{ server_version }}
 *
 * Usage:
 * ```typescript
 * import { {{ module_name }} } from './{{ module_name }}';
 *
 * const results = await {{ module_name }}.search({ query: "rust patterns" });
 * ```
 */

// Runtime type for MCP call results
interface McpCallResult {
  content: Array<{ type: string; text?: string; resource?: any }>;
  isError?: boolean;
}

// MCP Runtime - connects to actual MCP server
declare const mcpCall: (toolName: string, params: Record<string, any>) => Promise<McpCallResult>;

{% for tool in tools %}
{% if include_docs %}
/**
 * {{ tool.description }}
 *
 * Category: {{ tool.category }}
 * Capabilities: {{ tool.capabilities | join(sep=", ") }}
{% for param in tool.parameters %}
 * @param {{ param.name }} - {{ param.description }}{% if not param.required %} (optional){% endif %}
{% endfor %}
 * @returns Promise<McpCallResult>
{% if include_examples %}
 *
 * @example
 * ```typescript
{% for example in tool.examples %}
 * {{ example | replace(from="\n", to="\n * ") }}
{% endfor %}
 * ```
{% endif %}
 */
{% endif %}
export interface {{ tool.pascal_name }}Params {
{% for param in tool.parameters %}
  {{ param.name }}{% if not param.required %}?{% endif %}: {{ param.typescript_type }};
{% endfor %}
}

export async function {{ tool.camel_name }}(
  params: {{ tool.pascal_name }}Params
): Promise<McpCallResult> {
  return await mcpCall('{{ tool.name }}', params);
}

{% endfor %}

// Main module export
export const {{ module_name }} = {
{% for tool in tools %}
  {{ tool.camel_name }},
{% endfor %}
};

// Default export
export default {{ module_name }};
"#;

/// TypeScript code generator
pub struct TypeScriptGenerator {
    tera: Tera,
}

impl TypeScriptGenerator {
    /// Create a new TypeScript generator
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();
        tera.add_raw_template("typescript_module", TYPESCRIPT_MODULE_TEMPLATE)?;

        Ok(Self { tera })
    }
}

impl CodeGenerator for TypeScriptGenerator {
    fn generate(&self, metadata: &McpServerMetadata, config: &CodegenConfig) -> Result<String> {
        let mut context = Context::new();

        // Server info
        context.insert("server_name", &metadata.name);
        context.insert("server_version", &metadata.version);
        context.insert(
            "server_description",
            metadata.description.as_deref().unwrap_or(""),
        );
        context.insert("module_name", &config.module_name);
        context.insert("include_docs", &config.include_docs);
        context.insert("include_examples", &config.include_examples);

        // Transform tools for template
        let tools: Vec<serde_json::Value> = metadata
            .tools
            .iter()
            .map(|tool| {
                let params: Vec<serde_json::Value> = tool
                    .parameters
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "name": p.name,
                            "description": p.description,
                            "typescript_type": p.to_typescript_type(),
                            "required": p.required,
                        })
                    })
                    .collect();

                serde_json::json!({
                    "name": tool.name,
                    "camel_name": to_camel_case(&tool.name),
                    "pascal_name": to_pascal_case(&tool.name),
                    "description": tool.description,
                    "category": tool.category.to_string(),
                    "capabilities": tool.capabilities,
                    "parameters": params,
                    "examples": tool.examples,
                })
            })
            .collect();

        context.insert("tools", &tools);

        let rendered = self.tera.render("typescript_module", &context)?;

        // Clean up extra whitespace
        let cleaned = rendered
            .lines()
            .filter(|line| !line.trim().is_empty() || line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(cleaned.trim().to_string())
    }

    fn generate_tool(&self, tool: &ToolMetadata, config: &CodegenConfig) -> Result<String> {
        // Create single-tool metadata
        let metadata = McpServerMetadata {
            name: "terraphim".to_string(),
            version: "1.0.0".to_string(),
            tools: vec![tool.clone()],
            description: None,
        };

        self.generate(&metadata, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ParameterMetadata, ToolCategory};

    #[test]
    fn test_generate_simple_tool() {
        let generator = TypeScriptGenerator::new().unwrap();

        let tool = ToolMetadata {
            name: "search".to_string(),
            title: Some("Search".to_string()),
            description: "Search for documents".to_string(),
            category: ToolCategory::KnowledgeGraph,
            capabilities: vec!["search".to_string()],
            parameters: vec![
                ParameterMetadata {
                    name: "query".to_string(),
                    description: "The search query".to_string(),
                    json_type: "string".to_string(),
                    required: true,
                    default_value: None,
                    array_item_type: None,
                    object_properties: None,
                },
                ParameterMetadata {
                    name: "limit".to_string(),
                    description: "Max results".to_string(),
                    json_type: "integer".to_string(),
                    required: false,
                    default_value: None,
                    array_item_type: None,
                    object_properties: None,
                },
            ],
            return_type: "Promise<SearchResult>".to_string(),
            examples: vec!["const r = await terraphim.search({...})".to_string()],
        };

        let config = CodegenConfig::default();
        let code = generator.generate_tool(&tool, &config).unwrap();

        assert!(code.contains("export interface SearchParams"));
        assert!(code.contains("query: string;"));
        assert!(code.contains("limit?: number;"));
        assert!(code.contains("export async function search"));
        assert!(code.contains("mcpCall('search'"));
    }

    #[test]
    fn test_generate_multiple_tools() {
        let generator = TypeScriptGenerator::new().unwrap();

        let metadata = McpServerMetadata {
            name: "terraphim-mcp".to_string(),
            version: "0.1.0".to_string(),
            tools: vec![
                ToolMetadata {
                    name: "search".to_string(),
                    title: None,
                    description: "Search documents".to_string(),
                    category: ToolCategory::KnowledgeGraph,
                    capabilities: vec!["search".to_string()],
                    parameters: vec![],
                    return_type: "Promise<any>".to_string(),
                    examples: vec![],
                },
                ToolMetadata {
                    name: "autocomplete_terms".to_string(),
                    title: None,
                    description: "Get suggestions".to_string(),
                    category: ToolCategory::Autocomplete,
                    capabilities: vec!["autocomplete".to_string()],
                    parameters: vec![],
                    return_type: "Promise<any>".to_string(),
                    examples: vec![],
                },
            ],
            description: Some("Terraphim MCP Server".to_string()),
        };

        let config = CodegenConfig::default();
        let code = generator.generate(&metadata, &config).unwrap();

        assert!(code.contains("export async function search"));
        assert!(code.contains("export async function autocompleteTerms"));
        assert!(code.contains("search,"));
        assert!(code.contains("autocompleteTerms,"));
    }
}
