# MCP Integration

This guide explains how to use Model Context Protocol (MCP) haystacks in Terraphim via the middleware MCP indexer.

## Features
- Transports:
  - SSE (localhost) with optional OAuth bearer
- stdio (optional; behind `mcp-rust-sdk` feature) — uses rust-sdk `StdioTransport` to spawn `npx -y @modelcontextprotocol/server-everything stdio`
  - HTTP fallback invoking `search` then `list`
- Result mapping to `terraphim_types::Document` (id, url, title, body, description)

## Enable Features
- Default: no MCP
- SSE/HTTP client only:
  ```bash
  cargo build -p terraphim_middleware --features mcp
  ```
- Full rust-sdk client (optional):
  ```bash
  cargo build -p terraphim_middleware --features mcp-rust-sdk
  ```

## Start a local Everything Server (SSE)
```bash
npx -y @modelcontextprotocol/server-everything sse
# Server is running on port 3001
```

## Role Config Haystack
Add a haystack with service `Mcp` and extra parameters:

- `base_url`: e.g. `http://127.0.0.1:3001`
- `transport`: `sse` | `oauth` | `stdio`
- `oauth_token`: (optional) when `transport = oauth`

Example (Rust test snippet):
```rust
Haystack::new(base_url.clone(), ServiceType::Mcp, true)
    .with_extra_parameter("base_url".into(), base_url.clone())
    .with_extra_parameter("transport".into(), "sse".into())
```

## Live Test
```bash
export MCP_SERVER_URL="http://127.0.0.1:3001"
cargo test -p terraphim_middleware --test mcp_haystack_test -- --ignored --nocapture
```

## Behavior
- The indexer probes `/{base}/sse` for reachability when using `transport = sse`.
- It then calls `/{base}/search` with `{ query: needle }`, falling back to `/{base}/list`.
- JSON arrays or `{ items: [...] }` payloads are supported.
- Each item is mapped best-effort to `Document`.

## Rust SDK (optional)
If you enable `mcp-rust-sdk`, the indexer includes gated functions to:
- Connect via SSE transport using the SDK (`SseTransport` + `McpService` + `McpClient`)
- Connect via stdio to a spawned `server-everything` (`StdioTransport` + `McpService` + `McpClient`)

Note: SDK imports may vary by version; code is feature-gated to avoid impacting default builds.
