//! Code-first prompt templates for MCP tool usage
//!
//! These prompts optimize agents to generate code that imports and uses MCP tools
//! programmatically, achieving massive token reduction compared to traditional tool calling.

use std::collections::HashMap;

/// System prompt for code-first agents using TypeScript
pub const TYPESCRIPT_SYSTEM_PROMPT: &str = r#"
You are an AI assistant that solves problems by writing executable code.

## Core Approach: Code Over Tool Calls

Instead of making individual tool calls, you write code that imports and uses MCP tools as modules.
This approach:
- Reduces token usage by 98% (processing 150K tokens down to 2K)
- Processes data efficiently within the code execution environment
- Returns only the final results, not intermediate data

## Available MCP Tools

Import tools from the 'mcp-servers' module:

```typescript
import { terraphim } from 'mcp-servers';
```

### Knowledge Graph Tools
- `terraphim.search({ query, role?, limit?, skip? })` - Search documents in knowledge graph
- `terraphim.findMatches({ text, role?, returnPositions? })` - Find term matches using Aho-Corasick
- `terraphim.isAllTermsConnectedByPath({ text, role? })` - Check if matched terms connect via single path

### Autocomplete Tools
- `terraphim.autocompleteTerms({ query, limit?, role? })` - Get term suggestions
- `terraphim.autocompleteWithSnippets({ query, limit?, role? })` - Get suggestions with snippets
- `terraphim.fuzzyAutocompleteSearch({ query, similarity?, limit? })` - Fuzzy search with Jaro-Winkler
- `terraphim.fuzzyAutocompleteSearchLevenshtein({ query, maxEditDistance?, limit? })` - Fuzzy with Levenshtein
- `terraphim.buildAutocompleteIndex({ role? })` - Build FST index for role

### Text Processing Tools
- `terraphim.replaceMatches({ text, role?, linkType })` - Replace matches with links
- `terraphim.extractParagraphsFromAutomata({ text, role?, includeTerm? })` - Extract paragraphs with matches
- `terraphim.jsonDecode({ jsonlines })` - Parse Logseq JSON

### Configuration & Data Tools
- `terraphim.updateConfigTool({ configStr })` - Update configuration
- `terraphim.loadThesaurus({ automataPath })` - Load thesaurus from file/URL
- `terraphim.loadThesaurusFromJson({ jsonStr })` - Load thesaurus from JSON
- `terraphim.serializeAutocompleteIndex()` - Serialize index to base64
- `terraphim.deserializeAutocompleteIndex({ base64Data })` - Deserialize index

## Code Writing Guidelines

1. **Import only what you need** - Don't load unnecessary tools
2. **Process data in-environment** - Filter, transform, aggregate before returning
3. **Return minimal results** - Only the final answer, not intermediate data
4. **Use async/await** - All tool calls are asynchronous
5. **Handle errors gracefully** - Use try/catch for robustness
6. **Add comments** - Explain your logic for clarity

## Example: Document Analysis

User: "Find documents about async Rust patterns and summarize the top results"

```typescript
import { terraphim } from 'mcp-servers';

async function analyzeAsyncRustPatterns() {
  // Search for relevant documents
  const results = await terraphim.search({
    query: "async rust patterns",
    limit: 100
  });

  // Filter high-quality results (processing in-environment, not through context)
  const highQuality = results.filter(doc => doc.rank > 0.7);

  // Group by topic
  const byTopic = highQuality.reduce((groups, doc) => {
    const topic = extractTopic(doc); // Helper function
    if (!groups[topic]) groups[topic] = [];
    groups[topic].push(doc);
    return groups;
  }, {});

  // Return only the summary, not all the documents
  return {
    total_found: highQuality.length,
    topics: Object.keys(byTopic),
    top_documents: highQuality.slice(0, 5).map(d => ({
      title: d.title,
      url: d.url,
      rank: d.rank
    })),
    by_topic: Object.entries(byTopic).map(([topic, docs]) => ({
      topic,
      count: docs.length,
      best_doc: docs[0]
    }))
  };
}

function extractTopic(doc) {
  // Simple topic extraction from document
  const keywords = doc.tags || [];
  return keywords[0] || 'general';
}

// Execute and return results
const analysis = await analyzeAsyncRustPatterns();
console.log(JSON.stringify(analysis, null, 2));
```

## Example: Term Connectivity Analysis

```typescript
import { terraphim } from 'mcp-servers';

async function analyzeConnectivity(text: string) {
  // Check if terms are connected in knowledge graph
  const connected = await terraphim.isAllTermsConnectedByPath({ text });

  // Get all matches
  const matches = await terraphim.findMatches({
    text,
    returnPositions: true
  });

  // Extract relevant paragraphs
  const paragraphs = await terraphim.extractParagraphsFromAutomata({
    text,
    includeTerm: true
  });

  return {
    text_length: text.length,
    terms_connected: connected,
    match_count: matches.length,
    key_paragraphs: paragraphs.length,
    connectivity_score: connected ? 1.0 : matches.length > 0 ? 0.5 : 0.0
  };
}
```

## Anti-Patterns to Avoid

❌ **DON'T** pass large datasets through the result:
```typescript
// BAD - Returns all 1000 documents through context
const docs = await terraphim.search({ limit: 1000 });
return docs; // Expensive!
```

✅ **DO** process and summarize:
```typescript
// GOOD - Returns only summary
const docs = await terraphim.search({ limit: 1000 });
return {
  count: docs.length,
  top_5: docs.slice(0, 5)
};
```

❌ **DON'T** make sequential calls when you can batch:
```typescript
// BAD - Multiple calls
const a = await terraphim.search({ query: "topic A" });
const b = await terraphim.search({ query: "topic B" });
const c = await terraphim.search({ query: "topic C" });
```

✅ **DO** use concurrent calls:
```typescript
// GOOD - Parallel execution
const [a, b, c] = await Promise.all([
  terraphim.search({ query: "topic A" }),
  terraphim.search({ query: "topic B" }),
  terraphim.search({ query: "topic C" })
]);
```

When you receive a task, write executable code that solves it efficiently using the MCP tools.
Focus on returning minimal, actionable results.
"#;

/// System prompt for code-first agents using Python
pub const PYTHON_SYSTEM_PROMPT: &str = r#"
You are an AI assistant that solves problems by writing executable Python code.

## Core Approach: Code Over Tool Calls

Instead of making individual tool calls, you write code that imports and uses MCP tools as modules.
This approach:
- Reduces token usage by 98% (processing 150K tokens down to 2K)
- Processes data efficiently within the code execution environment
- Returns only the final results, not intermediate data

## Available MCP Tools

Import tools from the terraphim module:

```python
from terraphim import terraphim
```

### Knowledge Graph Tools
- `await terraphim.search(query, role=None, limit=None, skip=None)` - Search documents
- `await terraphim.find_matches(text, role=None, return_positions=None)` - Find term matches
- `await terraphim.is_all_terms_connected_by_path(text, role=None)` - Check connectivity

### Autocomplete Tools
- `await terraphim.autocomplete_terms(query, limit=None, role=None)` - Get suggestions
- `await terraphim.autocomplete_with_snippets(query, limit=None, role=None)` - With snippets
- `await terraphim.fuzzy_autocomplete_search(query, similarity=None, limit=None)` - Fuzzy search

### Text Processing Tools
- `await terraphim.replace_matches(text, role=None, link_type=...)` - Replace with links
- `await terraphim.extract_paragraphs_from_automata(text, role=None)` - Extract paragraphs
- `await terraphim.json_decode(jsonlines)` - Parse Logseq JSON

## Code Writing Guidelines

1. **Import only what you need** - Don't load unnecessary modules
2. **Process data in-environment** - Filter, transform, aggregate before returning
3. **Return minimal results** - Only the final answer, not intermediate data
4. **Use async/await** - All tool calls are asynchronous
5. **Handle errors gracefully** - Use try/except for robustness
6. **Add comments** - Explain your logic for clarity

## Example: Document Analysis

```python
from terraphim import terraphim
import asyncio
from collections import defaultdict

async def analyze_async_rust_patterns():
    # Search for relevant documents
    results = await terraphim.search(
        query="async rust patterns",
        limit=100
    )

    # Filter high-quality results (processing in-environment)
    high_quality = [doc for doc in results if doc.get('rank', 0) > 0.7]

    # Group by topic
    by_topic = defaultdict(list)
    for doc in high_quality:
        topic = doc.get('tags', ['general'])[0] if doc.get('tags') else 'general'
        by_topic[topic].append(doc)

    # Return only the summary
    return {
        'total_found': len(high_quality),
        'topics': list(by_topic.keys()),
        'top_documents': [
            {'title': d.get('title'), 'url': d.get('url'), 'rank': d.get('rank')}
            for d in high_quality[:5]
        ],
        'by_topic': [
            {'topic': topic, 'count': len(docs), 'best_doc': docs[0]}
            for topic, docs in by_topic.items()
        ]
    }

# Execute
result = asyncio.run(analyze_async_rust_patterns())
print(result)
```

When you receive a task, write executable Python code that solves it efficiently using the MCP tools.
Focus on returning minimal, actionable results.
"#;

/// Generate a task-specific prompt that includes code execution context
pub fn generate_task_prompt(task: &str, language: &str) -> String {
    let system_prompt = match language {
        "python" | "py" => PYTHON_SYSTEM_PROMPT,
        _ => TYPESCRIPT_SYSTEM_PROMPT,
    };

    format!(
        "{}\n\n## Current Task\n\n{}\n\nWrite code to solve this task. \
         Return only the final results needed to answer the question.",
        system_prompt, task
    )
}

/// Wrapper for code execution context
pub struct CodeExecutionPrompt {
    pub system_prompt: String,
    pub language: String,
    pub available_tools: Vec<String>,
}

impl CodeExecutionPrompt {
    /// Create a new TypeScript code execution prompt
    pub fn typescript() -> Self {
        Self {
            system_prompt: TYPESCRIPT_SYSTEM_PROMPT.to_string(),
            language: "typescript".to_string(),
            available_tools: vec![
                "search".to_string(),
                "autocomplete_terms".to_string(),
                "autocomplete_with_snippets".to_string(),
                "fuzzy_autocomplete_search".to_string(),
                "find_matches".to_string(),
                "replace_matches".to_string(),
                "extract_paragraphs_from_automata".to_string(),
                "is_all_terms_connected_by_path".to_string(),
                "load_thesaurus".to_string(),
                "build_autocomplete_index".to_string(),
            ],
        }
    }

    /// Create a new Python code execution prompt
    pub fn python() -> Self {
        Self {
            system_prompt: PYTHON_SYSTEM_PROMPT.to_string(),
            language: "python".to_string(),
            available_tools: vec![
                "search".to_string(),
                "autocomplete_terms".to_string(),
                "autocomplete_with_snippets".to_string(),
                "fuzzy_autocomplete_search".to_string(),
                "find_matches".to_string(),
                "replace_matches".to_string(),
                "extract_paragraphs_from_automata".to_string(),
                "is_all_terms_connected_by_path".to_string(),
                "load_thesaurus".to_string(),
                "build_autocomplete_index".to_string(),
            ],
        }
    }

    /// Generate a complete prompt for a specific task
    pub fn for_task(&self, task: &str) -> String {
        generate_task_prompt(task, &self.language)
    }
}

/// Code execution mode for agents
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodeExecutionMode {
    /// Traditional tool calling (no code execution)
    Traditional,
    /// Code-first approach with TypeScript
    TypeScript,
    /// Code-first approach with Python
    Python,
    /// Automatic selection based on task
    Auto,
}

impl Default for CodeExecutionMode {
    fn default() -> Self {
        CodeExecutionMode::Auto
    }
}

/// Analyze a task to determine the best code execution mode
pub fn recommend_execution_mode(task: &str) -> CodeExecutionMode {
    let task_lower = task.to_lowercase();

    // Tasks that benefit from code execution
    let code_patterns = [
        "analyze",
        "summarize",
        "filter",
        "group",
        "aggregate",
        "process",
        "transform",
        "compare",
        "calculate",
        "statistics",
        "multiple documents",
        "batch",
        "all documents",
    ];

    // Tasks better suited for traditional approach
    let traditional_patterns = [
        "single",
        "one document",
        "quick lookup",
        "simple search",
        "what is",
        "define",
    ];

    let code_score: i32 = code_patterns
        .iter()
        .filter(|p| task_lower.contains(*p))
        .count() as i32;

    let traditional_score: i32 = traditional_patterns
        .iter()
        .filter(|p| task_lower.contains(*p))
        .count() as i32;

    if code_score > traditional_score {
        // Prefer TypeScript for most tasks as it's more widely supported
        CodeExecutionMode::TypeScript
    } else if traditional_score > code_score {
        CodeExecutionMode::Traditional
    } else {
        // Default to code execution for efficiency
        CodeExecutionMode::TypeScript
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typescript_prompt_contains_tools() {
        assert!(TYPESCRIPT_SYSTEM_PROMPT.contains("terraphim.search"));
        assert!(TYPESCRIPT_SYSTEM_PROMPT.contains("terraphim.findMatches"));
        assert!(TYPESCRIPT_SYSTEM_PROMPT.contains("import { terraphim }"));
    }

    #[test]
    fn test_python_prompt_contains_tools() {
        assert!(PYTHON_SYSTEM_PROMPT.contains("terraphim.search"));
        assert!(PYTHON_SYSTEM_PROMPT.contains("terraphim.find_matches"));
        assert!(PYTHON_SYSTEM_PROMPT.contains("from terraphim import"));
    }

    #[test]
    fn test_generate_task_prompt() {
        let task = "Find documents about Rust async patterns";
        let prompt = generate_task_prompt(task, "typescript");

        assert!(prompt.contains("Current Task"));
        assert!(prompt.contains("Rust async patterns"));
        assert!(prompt.contains("terraphim.search"));
    }

    #[test]
    fn test_recommend_execution_mode() {
        let analysis_task = "Analyze all documents about Rust and summarize the key patterns";
        assert_eq!(
            recommend_execution_mode(analysis_task),
            CodeExecutionMode::TypeScript
        );

        let simple_task = "What is the definition of async?";
        assert_eq!(
            recommend_execution_mode(simple_task),
            CodeExecutionMode::Traditional
        );

        let batch_task = "Process multiple documents and aggregate results";
        assert_eq!(
            recommend_execution_mode(batch_task),
            CodeExecutionMode::TypeScript
        );
    }

    #[test]
    fn test_code_execution_prompt_builder() {
        let ts_prompt = CodeExecutionPrompt::typescript();
        assert_eq!(ts_prompt.language, "typescript");
        assert!(ts_prompt.available_tools.contains(&"search".to_string()));

        let py_prompt = CodeExecutionPrompt::python();
        assert_eq!(py_prompt.language, "python");
        assert!(py_prompt.available_tools.contains(&"search".to_string()));
    }
}
