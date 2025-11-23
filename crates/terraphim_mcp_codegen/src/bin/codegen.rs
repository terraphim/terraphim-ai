//! MCP Code Generator CLI
//!
//! Generates TypeScript and Python wrappers for MCP tools.

use std::path::PathBuf;

use terraphim_mcp_codegen::{
    generate_code,
    runtime::{McpRuntime, RuntimeConfig},
    CodegenConfig, OutputFormat,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "generate" => {
            let format = args.get(2).map(|s| s.as_str()).unwrap_or("typescript");
            let output_path = args.get(3).map(PathBuf::from).unwrap_or_else(|| {
                if format == "python" || format == "py" {
                    PathBuf::from("terraphim.py")
                } else {
                    PathBuf::from("terraphim.ts")
                }
            });

            generate_wrappers(format, output_path).await?;
        }
        "package" => {
            let output_dir = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("mcp-runtime"));

            create_package(output_dir).await?;
        }
        "introspect" => {
            introspect_tools().await?;
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_usage() {
    eprintln!(
        r#"
MCP Code Generator - Generate TypeScript/Python wrappers for MCP tools

USAGE:
    mcp-codegen <command> [options]

COMMANDS:
    generate [format] [output]  Generate wrapper code
        format: typescript (default), python
        output: output file path (default: terraphim.ts or terraphim.py)

    package [output_dir]        Create complete code execution package
        output_dir: directory for the package (default: mcp-runtime)

    introspect                  List all available MCP tools

    help                        Show this help message

EXAMPLES:
    mcp-codegen generate typescript ./workspace/terraphim.ts
    mcp-codegen generate python ./workspace/terraphim.py
    mcp-codegen package ./workspace/mcp-runtime
    mcp-codegen introspect
"#
    );
}

async fn generate_wrappers(format: &str, output_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating {} wrappers...", format);

    // Create MCP service to introspect tools
    let metadata = get_mcp_metadata().await?;

    let output_format: OutputFormat = format.parse().map_err(|e| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Invalid format: {}", e),
        ))
    })?;

    let config = CodegenConfig {
        format: output_format,
        output_path: output_path.clone(),
        module_name: "terraphim".to_string(),
        async_functions: true,
        include_docs: true,
        include_examples: true,
    };

    let code = generate_code(&metadata, &config)?;

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&output_path, code)?;

    println!("Generated {} tools to {}", metadata.tools.len(), output_path.display());
    println!("\nTools generated:");
    for tool in &metadata.tools {
        println!("  - {} ({})", tool.name, tool.category);
    }

    Ok(())
}

async fn create_package(output_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating code execution package...");

    let metadata = get_mcp_metadata().await?;

    // Create both TypeScript and Python wrappers
    let ts_config = CodegenConfig {
        format: OutputFormat::TypeScript,
        module_name: "terraphim".to_string(),
        ..Default::default()
    };

    let py_config = CodegenConfig {
        format: OutputFormat::Python,
        module_name: "terraphim".to_string(),
        ..Default::default()
    };

    let ts_code = generate_code(&metadata, &ts_config)?;
    let py_code = generate_code(&metadata, &py_config)?;

    // Create runtime configuration
    let runtime_config = RuntimeConfig::default();
    let runtime = McpRuntime::new(runtime_config.clone());

    // Setup directory structure
    std::fs::create_dir_all(&output_dir)?;
    std::fs::create_dir_all(output_dir.join("typescript"))?;
    std::fs::create_dir_all(output_dir.join("python"))?;

    // Write TypeScript package
    std::fs::write(output_dir.join("typescript/terraphim.ts"), ts_code)?;
    runtime.write_javascript_runtime(&output_dir.join("typescript/runtime.js"))?;

    // Write Python package
    std::fs::write(output_dir.join("python/terraphim.py"), py_code)?;
    runtime.write_python_runtime(&output_dir.join("python/runtime.py"))?;

    // Write package.json for TypeScript
    let package_json = serde_json::json!({
        "name": "terraphim-mcp",
        "version": "1.0.0",
        "type": "module",
        "main": "terraphim.ts",
        "dependencies": {}
    });
    std::fs::write(
        output_dir.join("typescript/package.json"),
        serde_json::to_string_pretty(&package_json)?,
    )?;

    // Write requirements.txt for Python
    std::fs::write(
        output_dir.join("python/requirements.txt"),
        "aiohttp>=3.8.0\n",
    )?;

    // Write README
    let readme = format!(
        r#"# Terraphim MCP Code Execution Package

This package contains TypeScript and Python wrappers for {} MCP tools.

## TypeScript Usage

```typescript
import {{ terraphim }} from './typescript/terraphim';
import './typescript/runtime';

const results = await terraphim.search({{ query: "rust patterns", limit: 10 }});
```

## Python Usage

```python
from python.runtime import mcp_call
from python.terraphim import terraphim

results = await terraphim.search(query="rust patterns", limit=10)
```

## Available Tools

{}
"#,
        metadata.tools.len(),
        metadata
            .tools
            .iter()
            .map(|t| format!("- **{}**: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n")
    );
    std::fs::write(output_dir.join("README.md"), readme)?;

    println!(
        "Package created at {} with {} tools",
        output_dir.display(),
        metadata.tools.len()
    );

    Ok(())
}

async fn introspect_tools() -> Result<(), Box<dyn std::error::Error>> {
    println!("Introspecting MCP tools...\n");

    let metadata = get_mcp_metadata().await?;

    println!("Server: {} v{}", metadata.name, metadata.version);
    if let Some(desc) = &metadata.description {
        println!("Description: {}", desc);
    }
    println!("\nAvailable Tools ({}):\n", metadata.tools.len());

    for tool in &metadata.tools {
        println!("  {} - {}", tool.name, tool.category);
        println!("    {}", tool.description);
        if !tool.parameters.is_empty() {
            println!("    Parameters:");
            for param in &tool.parameters {
                let required = if param.required { "required" } else { "optional" };
                println!(
                    "      - {} ({}): {} [{}]",
                    param.name, param.json_type, param.description, required
                );
            }
        }
        println!("    Capabilities: {}", tool.capabilities.join(", "));
        println!();
    }

    Ok(())
}

async fn get_mcp_metadata(
) -> Result<terraphim_mcp_codegen::McpServerMetadata, Box<dyn std::error::Error>> {
    // Build metadata directly from known MCP server tools
    // This avoids needing to create a runtime context
    use terraphim_mcp_codegen::{
        categorize_tool, extract_capabilities, McpServerMetadata, ParameterMetadata, ToolMetadata,
    };

    let tools = vec![
        create_tool_metadata(
            "search",
            "Search for documents in the Terraphim knowledge graph",
            vec![
                ("query", "string", "The search query", true),
                ("role", "string", "Optional role to filter by", false),
                ("limit", "integer", "Maximum number of results to return", false),
                ("skip", "integer", "Number of results to skip", false),
            ],
        ),
        create_tool_metadata(
            "update_config_tool",
            "Update the Terraphim configuration",
            vec![("config_str", "string", "JSON configuration string", true)],
        ),
        create_tool_metadata(
            "build_autocomplete_index",
            "Build FST-based autocomplete index from role's knowledge graph",
            vec![("role", "string", "Optional role name to build autocomplete index for", false)],
        ),
        create_tool_metadata(
            "autocomplete_terms",
            "Autocomplete terms using FST prefix + fuzzy fallback",
            vec![
                ("query", "string", "Prefix or term for suggestions", true),
                ("limit", "integer", "Max suggestions (default 10)", false),
                ("role", "string", "Optional role name to use for autocomplete", false),
            ],
        ),
        create_tool_metadata(
            "autocomplete_with_snippets",
            "Autocomplete and return short snippets from matching documents",
            vec![
                ("query", "string", "Prefix or term for suggestions with snippets", true),
                ("limit", "integer", "Max suggestions (default 10)", false),
                ("role", "string", "Optional role name to use for autocomplete", false),
            ],
        ),
        create_tool_metadata(
            "fuzzy_autocomplete_search",
            "Perform fuzzy autocomplete search using Jaro-Winkler similarity",
            vec![
                ("query", "string", "The text to get autocomplete suggestions for", true),
                ("similarity", "number", "Minimum similarity threshold (0.0-1.0, default: 0.6)", false),
                ("limit", "integer", "Maximum number of suggestions to return (default: 10)", false),
            ],
        ),
        create_tool_metadata(
            "fuzzy_autocomplete_search_levenshtein",
            "Perform fuzzy autocomplete search using Levenshtein distance",
            vec![
                ("query", "string", "The text to get autocomplete suggestions for", true),
                ("max_edit_distance", "integer", "Maximum Levenshtein edit distance allowed (default: 2)", false),
                ("limit", "integer", "Maximum number of suggestions to return (default: 10)", false),
            ],
        ),
        create_tool_metadata(
            "fuzzy_autocomplete_search_jaro_winkler",
            "Perform fuzzy autocomplete search using Jaro-Winkler similarity (explicit)",
            vec![
                ("query", "string", "The text to get autocomplete suggestions for", true),
                ("similarity", "number", "Minimum similarity threshold (0.0-1.0, default: 0.6)", false),
                ("limit", "integer", "Maximum number of suggestions to return (default: 10)", false),
            ],
        ),
        create_tool_metadata(
            "serialize_autocomplete_index",
            "Serialize the current autocomplete index to a base64-encoded string",
            vec![],
        ),
        create_tool_metadata(
            "deserialize_autocomplete_index",
            "Deserialize an autocomplete index from a base64-encoded string",
            vec![("base64_data", "string", "The base64-encoded string of the serialized index", true)],
        ),
        create_tool_metadata(
            "find_matches",
            "Find all term matches in text using Aho-Corasick algorithm",
            vec![
                ("text", "string", "The text to search in", true),
                ("role", "string", "Optional role to filter by", false),
                ("return_positions", "boolean", "Whether to return positions (default: false)", false),
            ],
        ),
        create_tool_metadata(
            "replace_matches",
            "Replace matched terms in text with links using specified format",
            vec![
                ("text", "string", "The text to replace terms in", true),
                ("role", "string", "Optional role to filter by", false),
                ("link_type", "string", "The type of link to use (wiki, html, markdown)", true),
            ],
        ),
        create_tool_metadata(
            "extract_paragraphs_from_automata",
            "Extract paragraphs containing matched terms from text",
            vec![
                ("text", "string", "The text to extract paragraphs from", true),
                ("role", "string", "Optional role to filter by", false),
                ("include_term", "boolean", "Whether to include the matched term (default: true)", false),
            ],
        ),
        create_tool_metadata(
            "json_decode",
            "Parse Logseq JSON output using terraphim_automata",
            vec![("jsonlines", "string", "The JSON lines string to decode", true)],
        ),
        create_tool_metadata(
            "load_thesaurus",
            "Load thesaurus from a local file or remote URL",
            vec![("automata_path", "string", "The path to the automata file (local or remote URL)", true)],
        ),
        create_tool_metadata(
            "load_thesaurus_from_json",
            "Load thesaurus from a JSON string",
            vec![("json_str", "string", "The JSON string to load thesaurus from", true)],
        ),
        create_tool_metadata(
            "is_all_terms_connected_by_path",
            "Check if all matched terms in text can be connected by a single path in the knowledge graph",
            vec![
                ("text", "string", "The text to check for term connectivity", true),
                ("role", "string", "Optional role to use for thesaurus and graph", false),
            ],
        ),
    ];

    Ok(McpServerMetadata {
        name: "terraphim-mcp".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        tools,
        description: Some("Terraphim MCP Server - Knowledge graph search and autocomplete tools".to_string()),
    })
}

fn create_tool_metadata(
    name: &str,
    description: &str,
    params: Vec<(&str, &str, &str, bool)>,
) -> terraphim_mcp_codegen::ToolMetadata {
    use terraphim_mcp_codegen::{
        categorize_tool, extract_capabilities, ParameterMetadata, ToolMetadata,
    };

    let parameters: Vec<ParameterMetadata> = params
        .into_iter()
        .map(|(pname, ptype, pdesc, required)| ParameterMetadata {
            name: pname.to_string(),
            description: pdesc.to_string(),
            json_type: ptype.to_string(),
            required,
            default_value: None,
            array_item_type: None,
            object_properties: None,
        })
        .collect();

    let category = categorize_tool(name);

    let mut metadata = ToolMetadata {
        name: name.to_string(),
        title: None,
        description: description.to_string(),
        category,
        capabilities: vec![],
        parameters,
        return_type: "Promise<CallToolResult>".to_string(),
        examples: terraphim_mcp_codegen::introspection::generate_examples(name),
    };

    metadata.capabilities = extract_capabilities(&metadata);
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_wrappers() {
        // This would test the wrapper generation
        // For now, just a placeholder
    }
}
