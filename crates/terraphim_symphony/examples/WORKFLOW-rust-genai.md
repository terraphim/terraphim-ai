---
tracker:
  kind: gitea
  endpoint: https://git.terraphim.cloud
  api_key: $GITEA_TOKEN
  owner: terraphim
  repo: rust-genai

polling:
  interval_ms: 30000

agent:
  runner: claude-code
  max_concurrent_agents: 2
  max_turns: 50
  claude_flags: "--dangerously-skip-permissions --allowedTools Bash,Read,Write,Edit,Glob,Grep"
  settings: ~/.claude/symphony-settings.json

workspace:
  root: ~/symphony_workspaces

hooks:
  after_create: "git clone https://terraphim:${GITEA_TOKEN}@git.terraphim.cloud/terraphim/rust-genai.git . && cat > CLAUDE.md << CLAUDEEOF\n# rust-genai Agent Instructions\n\n## Your Task\nYou are implementing issue #${SYMPHONY_ISSUE_NUMBER} for the rust-genai Rust library.\nFocus ALL your effort on writing production-quality Rust code and tests.\nDo NOT run gitea-robot, tea, or any task-tracking commands.\nThe orchestrator handles all task tracking.\n\n## Project Overview\nrust-genai is a multi-AI provider Rust client library supporting OpenAI, Anthropic, Gemini,\nBedrock, Cerebras, DeepSeek, Groq, and more via a unified Adapter trait.\n\nThis is the Terraphim fork which adds: AWS Bedrock, Cerebras, Z.AI/Zhipu, OpenRouter adapters,\nand Bearer token authentication.\n\nThe fork needs to be synced with upstream v0.6.0-beta.8 which has breaking API changes.\n\n## Upstream Breaking Changes (v0.6.0-beta.8)\n- Adapter trait: added const DEFAULT_API_KEY_ENV_NAME: &str\n- Adapter trait: all_model_names now takes (kind, endpoint, auth) -- 3 params instead of 1\n- Adapter trait: default_auth() now returns AuthData (AuthData::None variant added)\n- ChatResponse: added stop_reason: Option<StopReason> field\n- InterStreamEnd: added captured_stop_reason: Option<StopReason> field\n- ChatMessage::tool renamed to ChatMessage::tool_response\n- ContentPart::ReasoningContent added alongside ContentPart::Text\n- ReasoningEffort::Max variant added\n- OpenRouter removed as dedicated adapter upstream (namespace dispatch only)\n- Aliyun adapter added upstream\n\n## Key Files\n- src/adapter/adapter_types.rs -- Adapter trait definition (source of truth for signatures)\n- src/adapter/adapter_kind.rs -- AdapterKind enum (all adapter variants)\n- src/adapter/dispatcher.rs -- Static dispatch to adapter implementations\n- src/resolver/auth_data.rs -- AuthData enum (BearerToken + None coexistence)\n- src/chat/chat_response.rs -- ChatResponse with stop_reason\n- src/adapter/inter_stream.rs -- InterStreamEnd with captured_stop_reason\n- src/adapter/adapters/bedrock/ -- Bedrock adapter (fork addition)\n- src/adapter/adapters/cerebras/ -- Cerebras adapter (fork addition)\n- src/adapter/adapters/openrouter/ -- OpenRouter adapter (fork addition)\n- src/adapter/adapters/zai/ -- Z.AI adapter (fork, has URL trailing slash issues)\n\n## Quality Requirements\n- cargo build must succeed\n- cargo test must pass\n- cargo clippy -- -D warnings must pass\n- cargo fmt -- --check must pass\n- Use British English in documentation\n- Never use mocks in tests\nCLAUDEEOF\ngit add CLAUDE.md && git commit -m 'chore: add agent CLAUDE.md with learnings' --no-verify 2>/dev/null || true"
  before_run: "set -e && git fetch origin && BRANCH=\"symphony/issue-${SYMPHONY_ISSUE_NUMBER}\" && git checkout main 2>/dev/null || true && git branch -D \"$BRANCH\" 2>/dev/null || true && if git rev-parse --verify \"origin/$BRANCH\" >/dev/null 2>&1; then git checkout -b \"$BRANCH\" \"origin/$BRANCH\" && git fetch origin main && git rebase origin/main 2>/dev/null || { git rebase --abort 2>/dev/null; git merge -X theirs origin/main --no-edit 2>/dev/null || true; }; else git fetch origin main && git checkout -b \"$BRANCH\" origin/main; fi"
  after_run: "set +e && BRANCH=\"symphony/issue-${SYMPHONY_ISSUE_NUMBER}\" && GITEA_API=\"https://git.terraphim.cloud/api/v1/repos/terraphim/rust-genai\" && AUTH=\"Authorization: token ${GITEA_TOKEN}\" && git add -A && git diff --cached --quiet || git commit -m \"symphony: ${SYMPHONY_ISSUE_IDENTIFIER} - ${SYMPHONY_ISSUE_TITLE}\" || true && git push -u origin \"$BRANCH\" 2>/dev/null || true && GATE=true && GATE_MSG=\"\" && RS_COUNT=$(find src -name '*.rs' 2>/dev/null | wc -l | tr -d ' ') && if [ \"$RS_COUNT\" -lt 1 ]; then GATE=false && GATE_MSG=\"No .rs files in src/. \"; fi && cargo build 2>/tmp/sym_build_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Build failed. \"; } && cargo test 2>/tmp/sym_test_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Tests failed. \"; } && cargo clippy -- -D warnings 2>/tmp/sym_clippy_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Clippy failed. \"; } && cargo fmt -- --check 2>/tmp/sym_fmt_${SYMPHONY_ISSUE_NUMBER}.log || { GATE=false; GATE_MSG=\"${GATE_MSG}Format check failed. \"; } && if [ \"$GATE\" = \"true\" ]; then MERGE_OK=false && git fetch origin main && git checkout main && git pull origin main && git merge -X theirs \"$BRANCH\" --no-ff -m \"Merge ${SYMPHONY_ISSUE_IDENTIFIER}: ${SYMPHONY_ISSUE_TITLE}\" && git push origin main && MERGE_OK=true; if [ \"$MERGE_OK\" = \"true\" ]; then curl --retry 3 --retry-delay 5 --max-time 30 -sf -X PATCH -H \"$AUTH\" -H 'Content-Type: application/json' -d '{\"state\":\"closed\"}' \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}\" >/dev/null || true; curl --retry 3 --retry-delay 5 --max-time 30 -sf -X POST -H \"$AUTH\" -H 'Content-Type: application/json' -d '{\"body\":\"Quality gate passed (build+test+clippy+fmt). Merged to main and closed.\"}' \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}/comments\" >/dev/null || true; cargo clean 2>/dev/null || true; else git checkout \"$BRANCH\" 2>/dev/null; curl --retry 2 --max-time 15 -sf -X POST -H \"$AUTH\" -H 'Content-Type: application/json' -d '{\"body\":\"Merge to main failed (concurrent merge race). Will retry.\"}' \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}/comments\" >/dev/null || true; fi; else curl --retry 2 --max-time 15 -sf -X POST -H \"$AUTH\" -H 'Content-Type: application/json' -d \"{\\\"body\\\":\\\"Quality gate failed: ${GATE_MSG}Issue left open for retry.\\\"}\" \"${GITEA_API}/issues/${SYMPHONY_ISSUE_NUMBER}/comments\" >/dev/null || true; fi"
  timeout_ms: 1200000

codex:
  turn_timeout_ms: 3600000
  stall_timeout_ms: 1200000
---
You are working on issue {{ issue.identifier }}: {{ issue.title }}.

{% if issue.description %}
## Issue Description

{{ issue.description }}
{% endif %}

## Project Context

This is a Rust library (`genai`) providing a unified client for multiple AI providers.
The library uses an Adapter trait pattern where each provider implements its own adapter.

**Crate name**: `genai`
**Rust edition**: 2024
**Build**: `cargo build`
**Test**: `cargo test`
**Lint**: `cargo clippy -- -D warnings`
**Format**: `cargo fmt -- --check`

### Key architecture
- `src/adapter/adapter_types.rs` -- The Adapter trait all providers implement
- `src/adapter/adapter_kind.rs` -- AdapterKind enum mapping model names to adapters
- `src/adapter/dispatcher.rs` -- Static dispatch calling the right adapter
- `src/resolver/` -- Authentication and endpoint resolution
- `src/chat/` -- Chat request/response types
- `src/webc/` -- HTTP client and SSE streaming

### Adapter trait signature (upstream v0.6.0-beta.8)
```rust
pub trait Adapter {
    const DEFAULT_API_KEY_ENV_NAME: &str;
    fn default_endpoint() -> Endpoint;
    fn default_auth() -> AuthData;
    async fn all_model_names(kind: AdapterKind, endpoint: Endpoint, auth: AuthData) -> Result<Vec<String>>;
    fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String>;
    fn to_web_request_data(target: ServiceTarget, service_type: ServiceType, chat_req: ChatRequest, options_set: ChatOptionsSet<'_, '_>) -> Result<WebRequestData>;
    fn to_chat_response(model_iden: ModelIden, web_response: WebResponse, options_set: ChatOptionsSet<'_, '_>) -> Result<ChatResponse>;
    fn to_chat_stream(model_iden: ModelIden, reqwest_builder: RequestBuilder, options_set: ChatOptionsSet<'_, '_>) -> Result<ChatStreamResponse>;
    fn to_embed_request_data(service_target: ServiceTarget, embed_req: EmbedRequest, options_set: EmbedOptionsSet<'_, '_>) -> Result<WebRequestData>;
    fn to_embed_response(model_iden: ModelIden, web_response: WebResponse, options_set: EmbedOptionsSet<'_, '_>) -> Result<EmbedResponse>;
}
```

## CRITICAL Instructions

1. Examine ALL existing code in this workspace first. Previous agents have already built parts of this project. Do NOT recreate files that already exist. Build on what is there.
2. Read `src/adapter/adapter_types.rs` to understand the EXACT current Adapter trait signature.
3. Implement the feature described in the issue with REAL, COMPLETE Rust code.
4. Ensure `cargo build` succeeds before finishing.
5. Ensure `cargo test` passes before finishing.
6. Ensure `cargo clippy -- -D warnings` passes before finishing.
7. Ensure `cargo fmt -- --check` passes (run `cargo fmt` to auto-fix).
8. Commit with message: feat(module): description (Refs {{ issue.identifier }})
9. Do NOT run gitea-robot, tea, or any issue-tracking commands -- the orchestrator handles that.

{% if attempt %}
## RETRY ATTEMPT {{ attempt }}

This is retry attempt {{ attempt }}. Previous attempts did not pass the quality gate.
Check the existing code, fix any build errors, fix any test failures, fix clippy warnings.
Focus on making `cargo build && cargo test && cargo clippy -- -D warnings && cargo fmt -- --check` all succeed.
{% endif %}
