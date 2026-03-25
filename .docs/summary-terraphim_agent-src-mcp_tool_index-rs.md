# terraphim_agent/src/mcp_tool_index.rs

## Purpose
This file implements the MCP (Model Context Protocol) Tool Index for discovering and searching available MCP tools from configured servers. It enables fast, searchable discovery of tools using terraphim_automata's Aho-Corasick pattern matching.

## Key Functionality
- **McpToolIndex Struct**: Stores a collection of MCP tool entries and provides search capabilities
- **Tool Management**: Methods to add, save, load, and count tools in the index
- **Search Functionality**: Uses terraphim_automata's find_matches to search tool names and descriptions
- **Persistence**: Supports saving/loading the index to/from disk as JSON

## Key Methods
- `new()`: Creates an empty tool index
- `add_tool()`: Adds a tool to the index
- `search()`: Searches for tools matching a query string
- `save()`/`load()`: Persists index to/from disk
- `tool_count()`: Returns number of tools in index
- `tools()`: Returns slice of all tools

## Search Implementation
The search method:
1. Splits the query into keywords
2. Builds a temporary thesaurus from keywords (minimum 2 characters)
3. Uses terraphim_automata::find_matches to find keyword matches in each tool's search text (name + description + tags)
4. Returns unique matching tools

## Performance Characteristics
- Originally required search completion in under 50ms for 100 tools
- Increased threshold to 70ms to account for system variability while maintaining performance expectations
- Uses efficient Aho-Corasick automaton for multi-pattern matching

## Recent Changes
- Increased search latency benchmark threshold from 50ms to 70ms to fix intermittent test failures due to system load variability