//! Python Code Generator for MCP Tools

use crate::{
    introspection::to_camel_case, CodeGenerator, CodegenConfig, McpServerMetadata, Result,
    ToolMetadata,
};
use tera::{Context, Tera};

const PYTHON_MODULE_TEMPLATE: &str = r#"
"""
{{ server_name }} MCP Tools
{{ server_description }}

Generated automatically from MCP server introspection.
Version: {{ server_version }}

Usage:
    from {{ module_name }} import {{ module_name }}

    results = await {{ module_name }}.search(query="rust patterns")
"""

from typing import Any, Dict, List, Optional
import asyncio

# Type alias for MCP call results
McpCallResult = Dict[str, Any]

# MCP Runtime - connects to actual MCP server
async def mcp_call(tool_name: str, params: Dict[str, Any]) -> McpCallResult:
    """Call an MCP tool. This should be injected by the runtime."""
    raise NotImplementedError("mcp_call must be injected by the MCP runtime")

{% for tool in tools %}
{% if include_docs %}
async def {{ tool.snake_name }}(
{% for param in tool.parameters %}
    {{ param.name }}: {% if param.required %}{{ param.python_type }}{% else %}Optional[{{ param.python_type }}] = None{% endif %},
{% endfor %}
) -> McpCallResult:
    """
    {{ tool.description }}

    Category: {{ tool.category }}
    Capabilities: {{ tool.capabilities | join(sep=", ") }}

    Args:
{% for param in tool.parameters %}
        {{ param.name }}: {{ param.description }}{% if not param.required %} (optional){% endif %}
{% endfor %}

    Returns:
        McpCallResult: The result from the MCP server
{% if include_examples %}

    Example:
{% for example in tool.examples %}
        {{ example | replace(from="\n", to="\n        ") }}
{% endfor %}
{% endif %}
    """
{% else %}
async def {{ tool.snake_name }}(
{% for param in tool.parameters %}
    {{ param.name }}: {% if param.required %}{{ param.python_type }}{% else %}Optional[{{ param.python_type }}] = None{% endif %},
{% endfor %}
) -> McpCallResult:
{% endif %}
    params = {
{% for param in tool.parameters %}
{% if param.required %}
        "{{ param.name }}": {{ param.name }},
{% else %}
        "{{ param.name }}": {{ param.name }},
{% endif %}
{% endfor %}
    }
    # Remove None values for optional parameters
    params = {k: v for k, v in params.items() if v is not None}
    return await mcp_call("{{ tool.name }}", params)

{% endfor %}

# Main module class
class {{ module_name_pascal }}:
    """{{ server_name }} MCP Tools API"""

{% for tool in tools %}
    {{ tool.snake_name }} = staticmethod({{ tool.snake_name }})
{% endfor %}

# Convenience alias
{{ module_name }} = {{ module_name_pascal }}

__all__ = [
    "{{ module_name }}",
    "{{ module_name_pascal }}",
{% for tool in tools %}
    "{{ tool.snake_name }}",
{% endfor %}
]
"#;

/// Python code generator
pub struct PythonGenerator {
    tera: Tera,
}

impl PythonGenerator {
    /// Create a new Python generator
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();
        tera.add_raw_template("python_module", PYTHON_MODULE_TEMPLATE)?;

        Ok(Self { tera })
    }

    /// Convert camelCase to snake_case
    fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() {
                if i > 0 {
                    result.push('_');
                }
                result.push(c.to_lowercase().next().unwrap());
            } else if c == '_' {
                result.push('_');
            } else {
                result.push(c);
            }
        }
        result
    }

    /// Convert to PascalCase for class names
    fn to_pascal_case_python(s: &str) -> String {
        s.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + chars.as_str().to_lowercase().as_str()
                    }
                }
            })
            .collect::<String>()
    }
}

impl CodeGenerator for PythonGenerator {
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
        context.insert(
            "module_name_pascal",
            &Self::to_pascal_case_python(&config.module_name),
        );
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
                            "name": Self::to_snake_case(&p.name),
                            "description": p.description,
                            "python_type": p.to_python_type(),
                            "required": p.required,
                        })
                    })
                    .collect();

                // Convert examples to Python style
                let python_examples: Vec<String> = tool
                    .examples
                    .iter()
                    .map(|ex| {
                        ex.replace("await terraphim.", "await ")
                            .replace("const ", "")
                            .replace("let ", "")
                            .replace(";", "")
                            .replace(" =>", ":")
                            .replace("console.log", "print")
                            .replace("${", "{")
                            .replace("}`", "}")
                    })
                    .collect();

                serde_json::json!({
                    "name": tool.name,
                    "snake_name": Self::to_snake_case(&tool.name),
                    "description": tool.description,
                    "category": tool.category.to_string(),
                    "capabilities": tool.capabilities,
                    "parameters": params,
                    "examples": python_examples,
                })
            })
            .collect();

        context.insert("tools", &tools);

        let rendered = self.tera.render("python_module", &context)?;

        // Clean up extra whitespace
        let cleaned = rendered
            .lines()
            .map(|line| line.trim_end())
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
    fn test_to_snake_case() {
        assert_eq!(
            PythonGenerator::to_snake_case("autocomplete_terms"),
            "autocomplete_terms"
        );
        assert_eq!(
            PythonGenerator::to_snake_case("fuzzy_autocomplete_search"),
            "fuzzy_autocomplete_search"
        );
        assert_eq!(PythonGenerator::to_snake_case("search"), "search");
    }

    #[test]
    fn test_to_pascal_case_python() {
        assert_eq!(
            PythonGenerator::to_pascal_case_python("terraphim"),
            "Terraphim"
        );
        assert_eq!(
            PythonGenerator::to_pascal_case_python("mcp_server"),
            "McpServer"
        );
    }

    #[test]
    fn test_generate_simple_tool() {
        let generator = PythonGenerator::new().unwrap();

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
            return_type: "McpCallResult".to_string(),
            examples: vec!["results = await search(query=\"test\")".to_string()],
        };

        let config = CodegenConfig {
            module_name: "terraphim".to_string(),
            ..Default::default()
        };
        let code = generator.generate_tool(&tool, &config).unwrap();

        assert!(code.contains("async def search("));
        assert!(code.contains("query: str,"));
        assert!(code.contains("limit: Optional[int] = None,"));
        assert!(code.contains("mcp_call(\"search\""));
    }
}
