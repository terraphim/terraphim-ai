# Requirement Trace: Issue #3148 Agent Web Operations

**Issue**: Gitea `terraphim/terraphim-ai#3148`
**Branch**: `task/3148-agent-web-operations`
**Date**: 2026-08-12

## Requirements

| ID | Requirement | Evidence / Test |
| --- | --- | --- |
| R-3148-1 | TinyClaw must use a genuine web/API backend for navigate, extract, and API operations. | Existing `BrowserTool` reqwest implementation; hermetic axum tests `browser_navigate_returns_title_and_preview`, `browser_extract_returns_text`, `browser_api_post_round_trip`. |
| R-3148-2 | TinyClaw must not emit simulated clicked/typed/screenshot success. | Add fail-closed tests for browser-native operations and placeholder `terraphim-agent` subprocess output. |
| R-3148-3 | TinyClaw must inspect real `terraphim-agent` API/feature/binary capability before relying on browser automation. | `terraphim-agent robot capabilities` reports `"web_operations": false`; `terraphim-agent web ...` is not a recognized command; no `terraphim_agent` crate appears in `cargo metadata`. |
| R-3148-4 | If web operations are unavailable, browser-native operations must fail closed with exact evidence. | `ToolError::BackendUnavailable` includes capability/protocol evidence from the probe. |
| R-3148-5 | Tests must use hermetic local fixtures for network side effects. | Axum fixture in `crates/terraphim_tinyclaw/tests/browser_contracts.rs`; subprocess probe tests use local shell shims. |

## Backend Availability Findings

| Check | Result | Impact |
| --- | --- | --- |
| `cargo metadata --format-version 1 --no-deps` for `terraphim_agent` | No package returned. | No library dependency can be wired from TinyClaw in this checkout. |
| `crates/terraphim_agent/src/repl/web_operations.rs` | File absent. | The documented `WebOperationRequest` API is not present locally. |
| `crates/terraphim_agent/src/repl/handler.rs` web branch | Prints "functionality is not yet implemented" for web ops. | Subprocess stdout cannot be treated as success. |
| `terraphim-agent robot capabilities` | `"web_operations": false`. | Browser-native backend must be unavailable. |
| `terraphim-agent web get http://127.0.0.1:1/` | Unrecognized subcommand. | No robust CLI protocol exists for web operations. |

## Implementation Trace

| Requirement | Implementation |
| --- | --- |
| R-3148-1 | `BrowserTool::execute` keeps real reqwest GET/POST behavior for `navigate`, `extract`, and `api`. |
| R-3148-2 | Browser-native operations call the agent capability/protocol probe and return `BackendUnavailable`; no success path exists without a proven backend. |
| R-3148-3 | Probe executes `terraphim-agent --robot --format json robot capabilities` and validates that the binary both advertises web operations and exposes a `web` subcommand. |
| R-3148-4 | Probe failures are formatted into the returned backend-unavailable message. |
| R-3148-5 | Contract tests use local axum and local executable shims; no external network or real browser driver is required. |
