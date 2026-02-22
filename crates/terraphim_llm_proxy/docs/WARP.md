# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

Terraphim LLM Proxy is a production-ready intelligent LLM routing proxy with a sophisticated 6-phase routing architecture. It acts as a drop-in replacement for Claude Code, providing intelligent cost-optimized routing with sub-millisecond overhead (<1ms). The proxy features taxonomy-driven pattern matching, session-aware routing, multi-provider support via genai 0.4, and comprehensive token counting.

**Core Architecture:**
- **Multi-phase routing**: Explicit provider → Pattern matching (AI-driven) → Session-aware → Cost optimization → Performance optimization → Scenario fallback
- **Hybrid approach**: Static pattern matching (hot-reloadable taxonomy) + Dynamic algorithmic optimization (runtime metrics)
- **Request flow**: Client → Auth (16μs) → Token counting (124μs) → Analysis (50μs) → Router (5μs) → Transformer (16μs) → LLM → SSE Stream
- **Total overhead**: ~0.22ms (excluding LLM call)

## Essential Commands

### Building and Running

```bash
# Build release binary
cargo build --release

# Run with default config
cargo run --release

# Run with specific config
cargo run --release -- --config config.test.toml

# Run with custom host/port
cargo run --release -- --host 127.0.0.1 --port 3456

# Run with debug logging
LOG_LEVEL=debug cargo run --release

# Use with Claude Code (set environment variables)
export ANTHROPIC_BASE_URL=http://127.0.0.1:3456
export ANTHROPIC_API_KEY=your_proxy_api_key
claude "your query"
```

### Testing

```bash
# Run all tests (186 tests: 158 unit + 6 integration + 10 RoleGraph + 12 session)
cargo test

# Run specific test suite
cargo test --test integration_test
cargo test --test rolegraph_routing_integration_tests
cargo test --test session_management_e2e_tests

# Run tests with output
cargo test -- --nocapture

# Run unit tests only (in src/)
cargo test --lib

# Run specific test by name
cargo test test_pattern_matching_routing

# Run E2E tests (requires proxy running)
./scripts/run-e2e-tests.sh

# Run enhanced test suite
./scripts/run-enhanced-tests.sh
```

### Linting and Formatting

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run clippy linter
cargo clippy

# Clippy with warnings as errors
cargo clippy -- -D warnings

# Check without building
cargo check
```

### Development Tools

```bash
# Watch and rebuild on changes (requires cargo-watch)
cargo watch -x check -x test

# Build documentation
cargo doc --open

# Run benchmarks
cargo bench

# Check dependencies
cargo tree

# Update dependencies
cargo update
```

## Architecture Deep Dive

### Source Code Organization

**Core modules** (`src/`):
- `router.rs` (57KB): 6-phase routing engine with pattern matching, cost/performance optimization
- `server.rs` (63KB): Axum HTTP server with SSE streaming, authentication, rate limiting
- `rolegraph_client.rs` (20KB): Taxonomy pattern matching using Aho-Corasick automaton (200+ patterns)
- `token_counter.rs` (19KB): Fast token counting with tiktoken-rs (2.8M tokens/sec)
- `analyzer.rs` (12KB): Request analysis and hint generation for routing decisions
- `session.rs` (14KB): Session management for context-aware routing
- `client.rs` (23KB): LLM client integration via genai 0.4
- `transformer/` (15 files): Request/response transformers for each provider

**Supporting modules**:
- `cost/`: Budget management, pricing database, cost calculator
- `performance/`: Performance metrics, provider health monitoring
- `security/`: API key validation, rate limiting, SSRF protection
- `metrics.rs`, `production_metrics.rs`: Performance tracking and monitoring
- `retry.rs`: Exponential backoff retry logic
- `error.rs`: Comprehensive error handling with thiserror

### Multi-Phase Routing System

**Phase 0: Explicit Provider** - User specifies `provider:model` format
**Phase 1: Pattern-Based (RoleGraph)** - Taxonomy-driven pattern matching with Aho-Corasick
  - 52 taxonomy files in `docs/taxonomy/routing_scenarios/`
  - Patterns: background, think, low_cost, high_throughput, long_context, web_search, image
  - Score-based ranking: `length_score * position_score`
**Phase 2: Session-Aware** - Uses session history and provider preferences
**Phase 3: Cost Optimization** - Budget constraints, token usage estimation, price calculation
**Phase 4: Performance Optimization** - Metrics-based scoring (latency, throughput, success rate)
**Phase 5: Scenario Fallback** - Hint-based routing (background, thinking, long context, etc.)

See `docs/ROUTING_ARCHITECTURE.md` for complete routing documentation.

### Taxonomy Pattern Matching

Pattern files in `docs/taxonomy/routing_scenarios/` define routing rules:

```markdown
# low_cost_routing.md
route:: deepseek, deepseek-chat
synonyms:: low cost, budget, cheap, economy, cost-effective, ...
```

The RoleGraphClient loads these patterns into an Aho-Corasick automaton for <1ms matching. Patterns are scored by length and position, with the highest-scoring match determining the route.

### Provider Configuration

Providers are configured via TOML with genai 0.4's ServiceTargetResolver:

```toml
[[providers]]
name = "openrouter"
api_base_url = "https://openrouter.ai/api/v1"
api_key = "${OPENROUTER_API_KEY}"
models = ["anthropic/claude-sonnet-4.5", ...]

[[providers]]
name = "deepseek"
api_base_url = "https://api.deepseek.com"
api_key = "${DEEPSEEK_API_KEY}"
models = ["deepseek-chat", "deepseek-reasoner"]
```

Supports: OpenRouter, Anthropic, DeepSeek, Ollama, Gemini, OpenAI, and any OpenAI-compatible API.

## Configuration

### Environment Setup

1. Copy `.env.example` to `.env`:
```bash
cp .env.example .env
```

2. Configure API keys:
```bash
PROXY_API_KEY=sk_your_proxy_api_key_here_minimum_32_characters
OPENROUTER_API_KEY=sk_your_openrouter_key
ANTHROPIC_API_KEY=sk_your_anthropic_key
DEEPSEEK_API_KEY=sk_your_deepseek_key
```

3. Use example config:
```bash
cp config.example.toml config.toml
# Edit config.toml with your settings
```

### Config Files

- `config.test.toml` - Testing configuration
- `config.example.toml` - Production example
- `config.multi-provider.toml` - Multi-provider setup
- `.env.e2e.example` - E2E test environment

## Development Workflow

### Making Changes

1. **Create feature branch**: Work on specific features/fixes
2. **Write tests first**: No mocks allowed - use real integrations or test doubles
3. **Run tests**: `cargo test` must pass (186/186 tests)
4. **Check linting**: `cargo clippy` must have zero warnings
5. **Format code**: `cargo fmt`
6. **Update docs**: Maintain inline comments (per project rules)
7. **Track in GitHub**: Use `gh` tool to update issues

### Testing Strategy

- **Unit tests**: In `src/` modules with `#[cfg(test)] mod tests`
- **Integration tests**: In `tests/` directory
- **No mocks**: Test against real services or test instances
- **Coverage**: Check after implementation with `cargo tarpaulin` (if available)

### Key Testing Patterns

```rust
// Unit test example
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        // Test logic
    }
}

// Async test example
#[tokio::test]
async fn test_async_function() {
    // Async test logic
}
```

### Using tmux for Development

Instead of background processes with sleep, use tmux:

```bash
# Start proxy in tmux session
tmux new-session -d -s proxy 'cargo run --release -- --config config.test.toml'

# Run tests while proxy runs
cargo test --test integration_test

# Check proxy logs
tmux capture-pane -t proxy -p

# Kill session when done
tmux kill-session -t proxy
```

## Important Notes

### Rust Version
- **Minimum**: Rust 1.70 (for dependencies compatibility)
- **Recommended**: Rust 1.80+
- **Current**: Rust 1.90.0

### Dependencies
- `genai` from Terraphim fork: Better multi-provider support with ServiceTargetResolver
- `tiktoken-rs 0.5`: Fast token counting
- `axum 0.7`: HTTP framework
- `aho-corasick 1.1`: Pattern matching engine
- `terraphim_types`: Knowledge graph integration

### Performance Targets
- **Routing overhead**: <1ms (measured: 0.22ms)
- **Token counting**: 2.8M tokens/sec
- **Request throughput**: >4,000 req/sec
- **Memory overhead**: <2MB

### Security Features
- API key authentication (minimum 32 characters)
- Rate limiting (configurable)
- SSRF protection (blocks private IPs)
- Input sanitization
- Secure error messages

### Streaming
Full Claude API SSE format support:
- `message_start`, `content_block_delta`, `message_delta`, `message_stop` events
- Real-time chunk streaming
- Graceful error handling and stream termination

## Documentation

- **README.md** - Main project overview with quick start
- **docs/ROUTING_ARCHITECTURE.md** - Complete routing system documentation
- **docs/integration_design.md** - System design
- **docs/cost_based_prioritization_spec.md** - Cost optimization
- **PHASE2_WEEK1_COMPLETE.md** - Implementation reports
- **STREAMING_IMPLEMENTATION.md** - Streaming guide
- **GENAI_04_SUCCESS.md** - genai 0.4 integration

## Common Issues

### Tests Failing
- Ensure all dependencies are up to date: `cargo update`
- Check Rust version: `rustc --version` (need 1.70+)
- Verify test configuration: `.env.test` or `.env.e2e.example`

### Routing Not Working
- Check taxonomy files: `docs/taxonomy/routing_scenarios/`
- Verify RoleGraph initialization in logs
- Test pattern matching: `cargo test test_rolegraph_pattern_matching`

### Provider Errors
- Validate API keys in `.env`
- Check provider config in TOML: `api_base_url`, `models`
- Review genai 0.4 ServiceTargetResolver logs

## Project Rules

From project documentation:
- **No sleep command**: Use tmux for background tasks
- **No mocks in tests**: Real integrations or test doubles only
- **Use jiff instead of chrono**: (Note: Currently using chrono due to Rust version)
- **Keep memory.md and scratchpad.md updated**: Track progress (if applicable)
- **Maintain feature parity**: Native and WASM targets
- **Comprehensive inline comments**: Never remove existing comments
- **Update GitHub issues**: Use `gh` tool for every change
- **Commit every change**: Keep issues updated with progress

## Key Patterns

### Error Handling
Use `Result<T, ProxyError>` with `?` operator. Custom errors defined in `error.rs` with thiserror.

### Async Operations
All I/O uses tokio async runtime. Use `async fn` and `.await`. Spawn tasks with `tokio::spawn`.

### Configuration Loading
Uses `twelf` crate for layered config: file → environment → CLI args.

### Logging
Structured logging with `tracing`. Use `debug!`, `info!`, `warn!`, `error!` macros.

### Provider Integration
All providers go through genai 0.4 `ServiceTargetResolver` for consistent interface.
