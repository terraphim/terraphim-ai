//! MCP Server Introspection - Extract tool metadata from MCP servers

use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    categorize_tool, extract_capabilities, CodegenError, McpServerMetadata, ParameterMetadata,
    Result, ToolMetadata,
};

/// Extract metadata from MCP server tool definitions
pub fn extract_server_metadata(
    tools: Vec<rmcp::model::Tool>,
    server_info: &rmcp::model::ServerInfo,
) -> Result<McpServerMetadata> {
    let mut tool_metadata = Vec::new();

    for tool in tools {
        let metadata = extract_tool_metadata(tool)?;
        tool_metadata.push(metadata);
    }

    Ok(McpServerMetadata {
        name: server_info.server_info.name.clone(),
        version: server_info.server_info.version.clone(),
        tools: tool_metadata,
        description: server_info.instructions.clone(),
    })
}

/// Extract metadata from a single MCP tool definition
fn extract_tool_metadata(tool: rmcp::model::Tool) -> Result<ToolMetadata> {
    let name = tool.name.to_string();

    // Extract parameters from input schema
    let parameters = extract_parameters_from_schema(&tool.input_schema)?;

    // Determine category and capabilities
    let category = categorize_tool(&name);

    // Create base tool metadata
    // Note: rmcp::model::Tool has fields: name, description, input_schema, annotations
    let mut metadata = ToolMetadata {
        name: name.clone(),
        title: None, // Not available in rmcp Tool, will derive from name if needed
        description: tool.description.map(|s| s.to_string()).unwrap_or_default(),
        category,
        capabilities: vec![], // Will be filled after creation
        parameters,
        return_type: "Promise<CallToolResult>".to_string(),
        examples: generate_examples(&name),
    };

    // Extract capabilities based on the metadata
    metadata.capabilities = extract_capabilities(&metadata);

    Ok(metadata)
}

/// Extract parameter metadata from JSON Schema
fn extract_parameters_from_schema(
    schema: &Arc<serde_json::Map<String, serde_json::Value>>,
) -> Result<Vec<ParameterMetadata>> {
    let mut parameters = Vec::new();

    // Get required fields
    let required_fields: Vec<String> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Get properties
    if let Some(properties) = schema.get("properties") {
        if let Some(props_obj) = properties.as_object() {
            for (param_name, param_schema) in props_obj {
                let param = extract_single_parameter(param_name, param_schema, &required_fields)?;
                parameters.push(param);
            }
        }
    }

    // Sort parameters: required first, then optional
    parameters.sort_by(|a, b| {
        match (a.required, b.required) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name), // Alphabetical within same required status
        }
    });

    Ok(parameters)
}

/// Extract a single parameter from its JSON Schema definition
fn extract_single_parameter(
    name: &str,
    schema: &serde_json::Value,
    required_fields: &[String],
) -> Result<ParameterMetadata> {
    let schema_obj = schema.as_object().ok_or_else(|| {
        CodegenError::InvalidSpec(format!("Parameter {} schema is not an object", name))
    })?;

    let json_type = schema_obj
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("any")
        .to_string();

    let description = schema_obj
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let default_value = schema_obj.get("default").cloned();

    let array_item_type = if json_type == "array" {
        schema_obj
            .get("items")
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("type"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    let object_properties = if json_type == "object" {
        schema_obj
            .get("properties")
            .and_then(|v| v.as_object())
            .map(|props| {
                let mut nested_params = HashMap::new();
                for (prop_name, prop_schema) in props {
                    if let Ok(nested_param) =
                        extract_single_parameter(prop_name, prop_schema, &[])
                    {
                        nested_params.insert(prop_name.clone(), nested_param);
                    }
                }
                nested_params
            })
    } else {
        None
    };

    Ok(ParameterMetadata {
        name: name.to_string(),
        description,
        json_type,
        required: required_fields.contains(&name.to_string()),
        default_value,
        array_item_type,
        object_properties,
    })
}

/// Generate usage examples for a tool
pub fn generate_examples(tool_name: &str) -> Vec<String> {
    match tool_name {
        "search" => vec![
            r#"
const results = await terraphim.search({
  query: "rust async patterns",
  limit: 10
});
console.log(`Found ${results.length} documents`);
"#
            .trim()
            .to_string(),
        ],
        "autocomplete_terms" => vec![
            r#"
const suggestions = await terraphim.autocompleteTerms({
  query: "tera",
  limit: 5
});
suggestions.forEach(s => console.log(s));
"#
            .trim()
            .to_string(),
        ],
        "find_matches" => vec![
            r#"
const matches = await terraphim.findMatches({
  text: "This document discusses async rust patterns with tokio",
  returnPositions: true
});
console.log(`Found ${matches.length} term matches`);
"#
            .trim()
            .to_string(),
        ],
        "fuzzy_autocomplete_search" => vec![
            r#"
const suggestions = await terraphim.fuzzyAutocompleteSearch({
  query: "asynch",  // typo intentional
  similarity: 0.7,
  limit: 5
});
"#
            .trim()
            .to_string(),
        ],
        "replace_matches" => vec![
            r#"
const linkedText = await terraphim.replaceMatches({
  text: "Learn about async rust and tokio patterns",
  linkType: "markdown"
});
// Returns: "Learn about [async rust](url) and [tokio patterns](url)"
"#
            .trim()
            .to_string(),
        ],
        "extract_paragraphs_from_automata" => vec![
            r#"
const paragraphs = await terraphim.extractParagraphsFromAutomata({
  text: longDocument,
  includeTerm: true
});
paragraphs.forEach(p => console.log(p.term, p.paragraph));
"#
            .trim()
            .to_string(),
        ],
        "build_autocomplete_index" => vec![
            r#"
await terraphim.buildAutocompleteIndex({
  role: "engineer"
});
console.log("Index built successfully");
"#
            .trim()
            .to_string(),
        ],
        "is_all_terms_connected_by_path" => vec![
            r#"
const connected = await terraphim.isAllTermsConnectedByPath({
  text: "async programming with tokio runtime"
});
console.log(`Terms are connected: ${connected}`);
"#
            .trim()
            .to_string(),
        ],
        _ => vec![format!(
            r#"
const result = await terraphim.{}(params);
console.log(result);
"#,
            to_camel_case(tool_name)
        )
        .trim()
        .to_string()],
    }
}

/// Convert snake_case to camelCase
pub fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// Convert snake_case to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    let camel = to_camel_case(s);
    if let Some(first) = camel.chars().next() {
        format!("{}{}", first.to_uppercase(), &camel[1..])
    } else {
        camel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("search"), "search");
        assert_eq!(to_camel_case("autocomplete_terms"), "autocompleteTerms");
        assert_eq!(
            to_camel_case("fuzzy_autocomplete_search"),
            "fuzzyAutocompleteSearch"
        );
        assert_eq!(
            to_camel_case("is_all_terms_connected_by_path"),
            "isAllTermsConnectedByPath"
        );
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("search"), "Search");
        assert_eq!(to_pascal_case("autocomplete_terms"), "AutocompleteTerms");
        assert_eq!(
            to_pascal_case("fuzzy_autocomplete_search"),
            "FuzzyAutocompleteSearch"
        );
    }

    #[test]
    fn test_extract_single_parameter_string() {
        let schema = serde_json::json!({
            "type": "string",
            "description": "The search query"
        });

        let param = extract_single_parameter("query", &schema, &["query".to_string()]).unwrap();

        assert_eq!(param.name, "query");
        assert_eq!(param.json_type, "string");
        assert_eq!(param.description, "The search query");
        assert!(param.required);
    }

    #[test]
    fn test_extract_single_parameter_optional() {
        let schema = serde_json::json!({
            "type": "integer",
            "description": "Maximum results"
        });

        let param = extract_single_parameter("limit", &schema, &[]).unwrap();

        assert_eq!(param.name, "limit");
        assert_eq!(param.json_type, "integer");
        assert!(!param.required);
    }

    #[test]
    fn test_generate_examples() {
        let examples = generate_examples("search");
        assert!(!examples.is_empty());
        assert!(examples[0].contains("terraphim.search"));

        let fuzzy_examples = generate_examples("fuzzy_autocomplete_search");
        assert!(fuzzy_examples[0].contains("fuzzyAutocompleteSearch"));
    }
}
