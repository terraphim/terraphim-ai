//! EnhanceTool Transformer
//!
//! Enhances tool definitions with additional metadata, validation schemas,
//! and improved descriptions for better model understanding and usage.

use crate::{server::ChatResponse, token_counter::ChatRequest, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::debug;

/// EnhanceTool transformer for improving tool definitions
pub struct EnhanceToolTransformer {
    /// Add examples to tool definitions
    add_examples: bool,
    /// Add validation schemas
    add_validation: bool,
    /// Enhance descriptions with usage hints
    enhance_descriptions: bool,
    /// Add categorization metadata
    add_categories: bool,
}

impl EnhanceToolTransformer {
    pub fn new() -> Self {
        Self {
            add_examples: true,
            add_validation: true,
            enhance_descriptions: true,
            add_categories: true,
        }
    }

    pub fn with_examples(mut self, add_examples: bool) -> Self {
        self.add_examples = add_examples;
        self
    }

    pub fn with_validation(mut self, add_validation: bool) -> Self {
        self.add_validation = add_validation;
        self
    }

    pub fn with_description_enhancement(mut self, enhance: bool) -> Self {
        self.enhance_descriptions = enhance;
        self
    }

    pub fn with_categories(mut self, add_categories: bool) -> Self {
        self.add_categories = add_categories;
        self
    }
}

impl Default for EnhanceToolTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::transformer::Transformer for EnhanceToolTransformer {
    fn name(&self) -> &str {
        "enhancetool"
    }

    async fn transform_request(&self, mut req: ChatRequest) -> Result<ChatRequest> {
        debug!("Applying EnhanceTool transformer - improving tool definitions");

        if let Some(tools) = req.tools.as_mut() {
            for tool in tools.iter_mut() {
                // Convert Tool to Value for processing
                let mut tool_value = serde_json::to_value(&*tool)?;
                self.enhance_tool_definition(&mut tool_value);
                // Convert back to Tool
                *tool = serde_json::from_value(tool_value)?;
            }
        }

        Ok(req)
    }

    async fn transform_response(&self, resp: ChatResponse) -> Result<ChatResponse> {
        debug!("EnhanceTool transformer - response pass-through");
        Ok(resp)
    }
}

impl EnhanceToolTransformer {
    fn enhance_tool_definition(&self, tool_value: &mut Value) {
        if let Some(tool_obj) = tool_value.as_object_mut() {
            // Check if this is OpenAI format with function wrapper
            if let Some(function) = tool_obj.get_mut("function") {
                if let Some(func_obj) = function.as_object_mut() {
                    let tool_name = func_obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Enhance description within function
                    if self.enhance_descriptions {
                        self.enhance_function_description(func_obj, &tool_name);
                    }

                    // Add examples within function
                    if self.add_examples {
                        self.add_function_examples(func_obj, &tool_name);
                    }

                    // Add validation within function (using "parameters" field)
                    if self.add_validation {
                        self.add_function_validation_schemas(func_obj, &tool_name);
                    }

                    // Add categories within function
                    if self.add_categories {
                        self.add_function_categories(func_obj, &tool_name);
                    }
                }
            } else {
                // Legacy format - enhance at root level
                let tool_name = tool_obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Enhance description
                if self.enhance_descriptions {
                    self.enhance_description(tool_obj, &tool_name);
                }

                // Add examples
                if self.add_examples {
                    self.add_tool_examples(tool_obj, &tool_name);
                }

                // Add validation
                if self.add_validation {
                    self.add_validation_schemas(tool_obj, &tool_name);
                }

                // Add categories
                if self.add_categories {
                    self.add_tool_categories(tool_obj, &tool_name);
                }
            }
        }
    }

    fn enhance_description(&self, tool_obj: &mut serde_json::Map<String, Value>, tool_name: &str) {
        let current_desc = tool_obj
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let enhanced_desc = match tool_name {
            name if name.contains("calculator") || name.contains("calculate") => {
                format!("{} - Use this tool for mathematical calculations, expressions, and numerical computations.", current_desc)
            }
            name if name.contains("search") || name.contains("browse") => {
                format!("{} - Use this tool to search for information, browse web content, or retrieve external data.", current_desc)
            }
            name if name.contains("file") || name.contains("read") || name.contains("write") => {
                format!("{} - Use this tool for file operations including reading, writing, and managing files.", current_desc)
            }
            name if name.contains("code") || name.contains("execute") || name.contains("run") => {
                format!("{} - Use this tool to execute code, run programs, or perform computational tasks.", current_desc)
            }
            name if name.contains("database") || name.contains("query") || name.contains("sql") => {
                format!(
                    "{} - Use this tool for database operations, queries, and data retrieval.",
                    current_desc
                )
            }
            _ => {
                format!("{} - Use this tool when you need to perform the specific function it provides.", current_desc)
            }
        };

        tool_obj.insert("description".to_string(), json!(enhanced_desc));
    }

    fn add_tool_examples(&self, tool_obj: &mut serde_json::Map<String, Value>, tool_name: &str) {
        let examples = match tool_name {
            name if name.contains("calculator") => json!([
                {
                    "expression": "2 + 2 * 3",
                    "description": "Basic arithmetic"
                },
                {
                    "expression": "sqrt(16) + pow(2, 3)",
                    "description": "Complex calculation"
                }
            ]),
            name if name.contains("search") => json!([
                {
                    "query": "latest developments in AI",
                    "description": "Search for recent AI news"
                },
                {
                    "query": "weather forecast New York",
                    "description": "Get weather information"
                }
            ]),
            name if name.contains("file") && name.contains("read") => json!([
                {
                    "path": "/path/to/file.txt",
                    "description": "Read a text file"
                },
                {
                    "path": "./config.json",
                    "description": "Read configuration file"
                }
            ]),
            _ => json!([]),
        };

        if !examples.as_array().unwrap().is_empty() {
            tool_obj.insert("examples".to_string(), examples);
        }
    }

    fn add_validation_schemas(
        &self,
        tool_obj: &mut serde_json::Map<String, Value>,
        _tool_name: &str,
    ) {
        if let Some(parameters) = tool_obj.get_mut("input_schema") {
            if let Some(schema_obj) = parameters.as_object_mut() {
                // Add additionalProperties: false for stricter validation
                if !schema_obj.contains_key("additionalProperties") {
                    schema_obj.insert("additionalProperties".to_string(), json!(false));
                }

                // Enhance property descriptions if they exist
                if let Some(properties) = schema_obj.get_mut("properties") {
                    if let Some(props_obj) = properties.as_object_mut() {
                        for (prop_name, prop_schema) in props_obj.iter_mut() {
                            if let Some(schema) = prop_schema.as_object_mut() {
                                if !schema.contains_key("description") {
                                    let desc = match prop_name.as_str() {
                                        "query" | "search" => {
                                            "The search query or term to look for"
                                        }
                                        "path" | "file" => "The file path or location",
                                        "expression" | "calculation" => {
                                            "The mathematical expression to evaluate"
                                        }
                                        "url" | "link" => "The URL or web address",
                                        "text" | "content" => "The text content to process",
                                        _ => "The parameter value",
                                    };
                                    schema.insert("description".to_string(), json!(desc));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn add_tool_categories(&self, tool_obj: &mut serde_json::Map<String, Value>, tool_name: &str) {
        let categories = match tool_name {
            name if name.contains("calculator") || name.contains("math") => {
                vec!["mathematics", "calculation"]
            }
            name if name.contains("search") || name.contains("browse") => {
                vec!["information", "web"]
            }
            name if name.contains("file") => vec!["file_system", "io"],
            name if name.contains("code") || name.contains("execute") => {
                vec!["development", "execution"]
            }
            name if name.contains("database") || name.contains("query") => vec!["data", "database"],
            name if name.contains("api") || name.contains("http") => vec!["network", "api"],
            _ => vec!["general"],
        };

        tool_obj.insert("categories".to_string(), json!(categories));
    }

    // OpenAI format helper functions that work within the "function" object

    fn enhance_function_description(
        &self,
        func_obj: &mut serde_json::Map<String, Value>,
        tool_name: &str,
    ) {
        let current_desc = func_obj
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let enhanced_desc = match tool_name {
            name if name.contains("calculator") || name.contains("calculate") => {
                format!("{} - Use this tool for mathematical calculations, expressions, and numerical computations.", current_desc)
            }
            name if name.contains("search") || name.contains("browse") => {
                format!("{} - Use this tool to search for information, browse web content, or retrieve external data.", current_desc)
            }
            name if name.contains("file") || name.contains("read") || name.contains("write") => {
                format!("{} - Use this tool for file operations including reading, writing, and managing files.", current_desc)
            }
            name if name.contains("code") || name.contains("execute") || name.contains("run") => {
                format!("{} - Use this tool to execute code, run programs, or perform computational tasks.", current_desc)
            }
            name if name.contains("database") || name.contains("query") || name.contains("sql") => {
                format!(
                    "{} - Use this tool for database operations, queries, and data retrieval.",
                    current_desc
                )
            }
            _ => {
                format!("{} - Use this tool when you need to perform the specific function it provides.", current_desc)
            }
        };

        func_obj.insert("description".to_string(), json!(enhanced_desc));
    }

    fn add_function_examples(
        &self,
        func_obj: &mut serde_json::Map<String, Value>,
        tool_name: &str,
    ) {
        // Add examples to the parameters object
        if let Some(parameters) = func_obj.get_mut("parameters") {
            if let Some(params_obj) = parameters.as_object_mut() {
                let examples = match tool_name {
                    name if name.contains("calculator") => json!([
                        {
                            "expression": "2 + 2 * 3",
                            "description": "Basic arithmetic"
                        },
                        {
                            "expression": "sqrt(16) + pow(2, 3)",
                            "description": "Complex calculation"
                        }
                    ]),
                    name if name.contains("search") => json!([
                        {
                            "query": "latest developments in AI",
                            "description": "Search for recent AI news"
                        },
                        {
                            "query": "weather forecast New York",
                            "description": "Get weather information"
                        }
                    ]),
                    name if name.contains("file") && name.contains("read") => json!([
                        {
                            "path": "/path/to/file.txt",
                            "description": "Read a text file"
                        },
                        {
                            "path": "./config.json",
                            "description": "Read configuration file"
                        }
                    ]),
                    _ => json!([]),
                };

                if !examples.as_array().unwrap().is_empty() {
                    params_obj.insert("examples".to_string(), examples);
                }
            }
        }
    }

    fn add_function_validation_schemas(
        &self,
        func_obj: &mut serde_json::Map<String, Value>,
        _tool_name: &str,
    ) {
        if let Some(parameters) = func_obj.get_mut("parameters") {
            if let Some(schema_obj) = parameters.as_object_mut() {
                // Add additionalProperties: false for stricter validation
                if !schema_obj.contains_key("additionalProperties") {
                    schema_obj.insert("additionalProperties".to_string(), json!(false));
                }

                // Enhance property descriptions if they exist
                if let Some(properties) = schema_obj.get_mut("properties") {
                    if let Some(props_obj) = properties.as_object_mut() {
                        for (prop_name, prop_schema) in props_obj.iter_mut() {
                            if let Some(schema) = prop_schema.as_object_mut() {
                                if !schema.contains_key("description") {
                                    let desc = match prop_name.as_str() {
                                        "query" | "search" => {
                                            "The search query or term to look for"
                                        }
                                        "path" | "file" => "The file path or location",
                                        "expression" | "calculation" => {
                                            "The mathematical expression to evaluate"
                                        }
                                        "url" | "link" => "The URL or web address",
                                        "text" | "content" => "The text content to process",
                                        _ => "The parameter value",
                                    };
                                    schema.insert("description".to_string(), json!(desc));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn add_function_categories(
        &self,
        func_obj: &mut serde_json::Map<String, Value>,
        tool_name: &str,
    ) {
        let categories = match tool_name {
            name if name.contains("calculator") || name.contains("math") => {
                vec!["mathematics", "calculation"]
            }
            name if name.contains("search") || name.contains("browse") => {
                vec!["information", "web"]
            }
            name if name.contains("file") => vec!["file_system", "io"],
            name if name.contains("code") || name.contains("execute") => {
                vec!["development", "execution"]
            }
            name if name.contains("database") || name.contains("query") => vec!["data", "database"],
            name if name.contains("api") || name.contains("http") => vec!["network", "api"],
            _ => vec!["general"],
        };

        // Add categories to parameters object
        if let Some(parameters) = func_obj.get_mut("parameters") {
            if let Some(params_obj) = parameters.as_object_mut() {
                params_obj.insert("categories".to_string(), json!(categories));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::Transformer;
    use serde_json::json;

    #[tokio::test]
    async fn test_enhances_calculator_tool() {
        let transformer = EnhanceToolTransformer::new();

        let req = ChatRequest {
            model: "test-model".to_string(),
            tools: Some(vec![crate::token_counter::Tool {
                tool_type: Some("function".to_string()),
                function: Some(crate::token_counter::FunctionTool {
                    name: "calculator".to_string(),
                    description: Some("Perform calculations".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "expression": {"type": "string"}
                        },
                        "required": ["expression"]
                    }),
                }),
                name: None,
                description: None,
                input_schema: None,
            }]),
            ..Default::default()
        };

        let transformed = transformer.transform_request(req).await.unwrap();

        let tools = transformed.tools.unwrap();
        let tool = &tools[0];

        // Check enhanced description in function
        let func = tool.function.as_ref().unwrap();
        let description = func.description.as_ref().unwrap();
        assert!(description.contains("mathematical calculations"));

        // Check examples were added to parameters
        if let Some(examples) = func.parameters.get("examples") {
            assert!(examples.is_array());
        }

        // Check categories were added to parameters
        if let Some(categories) = func.parameters.get("categories") {
            assert!(categories.is_array());
            let categories_array = categories.as_array().unwrap();
            assert!(categories_array
                .iter()
                .any(|c| c.as_str() == Some("mathematics")));
        }
    }

    #[tokio::test]
    async fn test_adds_validation_schemas() {
        let transformer = EnhanceToolTransformer::new();

        let req = ChatRequest {
            model: "test-model".to_string(),
            tools: Some(vec![crate::token_counter::Tool {
                tool_type: Some("function".to_string()),
                function: Some(crate::token_counter::FunctionTool {
                    name: "search_tool".to_string(),
                    description: Some("Search for information".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "query": {"type": "string"}
                        }
                    }),
                }),
                name: None,
                description: None,
                input_schema: None,
            }]),
            ..Default::default()
        };

        let transformed = transformer.transform_request(req).await.unwrap();

        let tools = transformed.tools.unwrap();
        let tool = &tools[0];
        let func = tool.function.as_ref().unwrap();
        let schema = func.parameters.as_object().unwrap();

        // Check additionalProperties was added
        assert_eq!(schema.get("additionalProperties").unwrap(), &json!(false));

        // Check property description was enhanced
        let properties = schema.get("properties").unwrap().as_object().unwrap();
        let query_prop = properties.get("query").unwrap().as_object().unwrap();
        assert!(query_prop
            .get("description")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("search query"));
    }

    #[tokio::test]
    async fn test_handles_unknown_tool() {
        let transformer = EnhanceToolTransformer::new();

        let req = ChatRequest {
            model: "test-model".to_string(),
            tools: Some(vec![crate::token_counter::Tool {
                tool_type: Some("function".to_string()),
                function: Some(crate::token_counter::FunctionTool {
                    name: "unknown_tool".to_string(),
                    description: Some("Does something".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {}
                    }),
                }),
                name: None,
                description: None,
                input_schema: None,
            }]),
            ..Default::default()
        };

        let transformed = transformer.transform_request(req).await.unwrap();

        let tools = transformed.tools.unwrap();
        let tool = &tools[0];
        let func = tool.function.as_ref().unwrap();

        // Should still enhance description
        let description = func.description.as_ref().unwrap();
        assert!(description.contains("specific function"));

        // Should add general category to parameters
        if let Some(categories) = func.parameters.get("categories").and_then(|c| c.as_array()) {
            assert!(categories.iter().any(|c| c.as_str() == Some("general")));
        }
    }
}
