# MCP Namespace Configuration Example

This example demonstrates how to use MCP namespaces in Terraphim AI to aggregate multiple MCP servers.

## Overview

MCP namespaces allow you to:
- **Aggregate multiple MCP servers** into a single role
- **Prefix tools automatically** to avoid naming conflicts (e.g., `filesystem__read_file`)
- **Override tool names and descriptions** per namespace
- **Enable/disable individual tools** or entire namespaces
- **Support multiple transports**: STDIO, SSE, HTTP, OAuth

## Configuration Structure

```json
{
  "name": "Role Name",
  "mcp_namespaces": [
    {
      "name": "namespace-name",
      "enabled": true,
      "servers": [...],
      "tool_overrides": {...}
    }
  ]
}
```

## Server Configuration

### STDIO Transport
```json
{
  "name": "filesystem",
  "transport": "STDIO",
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-filesystem", "/workspace"]
}
```

### SSE Transport
```json
{
  "name": "github",
  "transport": "SSE",
  "url": "http://localhost:3001/mcp",
  "bearer_token": "${GITHUB_MCP_TOKEN}"
}
```

### HTTP Transport
```json
{
  "name": "perplexity",
  "transport": "HTTP",
  "url": "https://api.perplexity.ai/mcp",
  "bearer_token": "${PERPLEXITY_API_KEY}"
}
```

## Tool Overrides

### Rename a Tool
```json
"filesystem__read_file": {
  "name": "read_code",
  "description": "Read source code file"
}
```

### Disable a Tool
```json
"filesystem__write_file": {
  "status": "INACTIVE"
}
```

### Update Description Only
```json
"github__create_pr": {
  "description": "Create a new pull request on GitHub"
}
```

## Environment Variables

The configuration supports environment variable interpolation using `${VAR_NAME}` syntax:

```json
"bearer_token": "${GITHUB_MCP_TOKEN}"
```

Before using, set the environment variables:
```bash
export GITHUB_MCP_TOKEN="your_token_here"
export PERPLEXITY_API_KEY="your_api_key_here"
export OPENROUTER_API_KEY="your_openrouter_key_here"
```

## Tool Naming

Tools are automatically prefixed with their server name to avoid conflicts:

- Original: `read_file` from `filesystem` server
- Prefixed: `filesystem__read_file`
- After override: `read_code` (if renamed)

## Using the Configuration

1. Enable the mcp-proxy feature:
```bash
cargo build --features mcp-proxy
```

2. Set environment variables:
```bash
source .env
```

3. Load the configuration:
```bash
terraphim-server --config examples/mcp_namespace_config.json
```

## Multiple Namespaces

You can define multiple namespaces for different purposes:

- `dev-tools`: Development-related MCP servers (filesystem, git, github)
- `ai-tools`: AI-powered services (perplexity, claude, etc.)
- `data-tools`: Data processing servers (databases, APIs, etc.)

Each namespace can be enabled/disabled independently and has its own tool override configuration.

## Benefits

1. **No Naming Conflicts**: Automatic prefixing prevents tool name collisions
2. **Fine-Grained Control**: Enable/disable tools and namespaces independently
3. **Customization**: Rename and redescribe tools to match your workflow
4. **Security**: Use environment variables for sensitive credentials
5. **Organization**: Group related MCP servers logically

## See Also

- [MetaMCP Documentation](https://github.com/metatool-ai/metamcp)
- [MCP Protocol Specification](https://spec.modelcontextprotocol.io/)
- [Terraphim AI Documentation](https://terraphim.ai)
