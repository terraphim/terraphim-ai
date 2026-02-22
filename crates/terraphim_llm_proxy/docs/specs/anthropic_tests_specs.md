# Anthropic API Test Specification and Integration Plan

## Shared Requirements
- Every request must include `x-api-key` and `anthropic-version` headers; optional beta headers may be required for new features.
- Responses surface diagnostic headers such as `request-id` and `anthropic-organization-id`.

## Messages and Token Counting
| API | Activity | Input | Output | Description |
| --- | --- | --- | --- | --- |
| `POST /v1/messages` | Generate a Claude response to chat, tool, or multimodal prompts. | JSON including `model`, `max_tokens`, and `messages` (array of role/content items; text, image references, tool outputs). Optional fields: `system`, `temperature`, `top_p`, `top_k`, `metadata`, `stop_sequences`, `stream`, `tools`, `tool_choice`, `attachments`. | Message object with `id`, `type`, `role`, `content[]`, `stop_reason`, `stop_sequence`, and `usage` (input/output tokens). Streaming yields SSE chunks ending with the same structure. | Primary generation endpoint for conversations and stateless prompts; normalizes chat schema and tool-calling flow. |
| `POST /v1/messages/count_tokens` | Pre-flight token accounting for a prospective message. | JSON mirroring the messages payload (same fields except `stream`). | `{ "input_tokens": n, "output_tokens": 0, "cache_creation_input_tokens": m, "cache_read_input_tokens": k }`. | Validates prompts against context limits and pricing prior to execution. |

## Message Batches
| API | Activity | Input | Output | Description |
| --- | --- | --- | --- | --- |
| `POST /v1/messages/batches` | Submit many message requests for asynchronous processing. | `{ "requests": [{ "custom_id": "...", "params": { ...messages request... }}], "metadata": {...}, "completion_window": ... }`. | Batch descriptor containing `id`, `processing_status`, `request_counts`, timestamps, metadata. | Queues workloads that may run up to 24 hours; starts processing immediately. |
| `GET /v1/messages/batches/{batch_id}` | Check batch status. | Path `batch_id`. | Batch descriptor with status (`in_progress`, `completed`, `failed`, `canceled`), counts, timestamps, optional `cancel_reason`. | Poll endpoint to detect completion, failure, or cancellation. |
| `GET /v1/messages/batches/{batch_id}/results` | Retrieve per-request outputs. | Path `batch_id`, optional pagination query. | JSONL (or gzip) stream where each line includes `custom_id` plus result or error payload. | Access full responses/errors for each queued request once complete. |
| `GET /v1/messages/batches` | List batches. | Optional query (`limit`, `order`, `after_id`). | Paginated list of batch descriptors with `data[]`, `has_more`. | Administrative overview of historical batch runs. |
| `POST /v1/messages/batches/{batch_id}/cancel` | Cancel an in-progress batch. | Path `batch_id`, optional JSON `{ "cancel_reason": "..." }`. | Updated batch descriptor with status `canceled`. | Stops queued work gracefully; only valid while processing. |
| `DELETE /v1/messages/batches/{batch_id}` | Delete completed batch metadata/results. | Path `batch_id`. | Empty 204. | Removes stored batch artifacts; prior cancellation required if still running. |

## Files API
| API | Activity | Input | Output | Description |
| --- | --- | --- | --- | --- |
| `POST /v1/files` | Upload documents, images, or tool resources. | `multipart/form-data` with `file` binary and optional JSON metadata such as `type`. | File descriptor (`id`, `filename`, `purpose`, `bytes`, `created_at`, `workspace_id`). | Enables document and vision attachments referenced in messages or batches. |
| `GET /v1/files` | List files in the workspace. | Optional query (`limit`, `order`, `after_id`). | Paginated list of file descriptors. | Inventory and lifecycle management for uploaded assets. |
| `GET /v1/files/{file_id}` | Fetch metadata for a specific file. | Path `file_id`. | File descriptor. | Inspect single file details before reuse or deletion. |
| `GET /v1/files/{file_id}/content` | Download raw file data. | Path `file_id`. | Binary stream representing the uploaded content. | Required to verify integrity or rehydrate attachments. |
| `DELETE /v1/files/{file_id}` | Remove a file. | Path `file_id`. | Empty 204. | Cleanup unused artifacts; ensure not referenced by active requests. |

## Models API
| API | Activity | Input | Output | Description |
| --- | --- | --- | --- | --- |
| `GET /v1/models` | Enumerate available Claude models. | Optional filters (e.g., `include_archived`). | `{ "data": [{ "id", "display_name", "context_window", "input_cost_million_tokens", "output_cost_million_tokens", "modality", "supports_tools", ... }], "has_more": bool }`. | Discover supported capabilities, pricing, context windows, and availability. |
| `GET /v1/models/{model_id}` | Inspect a specific model or alias. | Path `model_id`. | Model descriptor with extended metadata (provider, status, release notes). | Resolve aliases and confirm feature support before issuing requests. |

## Experimental Prompt Tools
| API | Activity | Input | Output | Description |
| --- | --- | --- | --- | --- |
| `POST /v1/experimental/generate_prompt` | Draft a new prompt from supplied goals. | `{ "concept": "...", "context": "...", "constraints": [...], "metadata": {...} }`. | `{ "prompt": "...", "explanation": "...", "tokens": {...} }`. | Produces a tailored prompt aligned with user goals. |
| `POST /v1/experimental/improve_prompt` | Refine an existing prompt using feedback. | `{ "prompt": "...", "feedback": "...", "metadata": {...} }`. | `{ "prompt": "...", "changes": "...", "tokens": {...} }`. | Iterates on prompt quality or policy compliance. |
| `POST /v1/experimental/templatize_prompt` | Convert a prompt into a reusable template. | `{ "prompt": "...", "placeholders": ["field", ...], "metadata": {...} }`. | `{ "template": "...", "parameters": [...], "tokens": {...} }`. | Extracts variables and emits a reusable template. |

## Organization and Workspace Management
| API | Activity | Input | Output | Description |
| --- | --- | --- | --- | --- |
| `GET /v1/organizations/me` | Retrieve current user, organization, and default workspace info. | None. | `{ "user": {...}, "organization": {...}, "default_workspace_id": "...", "roles": [...] }`. | Confirms identity tied to the API key. |
| `GET /v1/organizations/users` | List organization members. | Pagination filters (e.g., `workspace_id`). | Paginated array of user records (id, email, roles, status). | Admin roster management. |
| `GET /v1/organizations/users/{user_id}` | Inspect a specific member. | Path `user_id`. | Single user record. | Verify roles and workspace assignments. |
| `POST /v1/organizations/users/{user_id}` | Update user membership or roles. | JSON such as `{ "roles": [...], "default_workspace_id": "...", "suspended": bool }`. | Updated user record. | Adjust permissions or workspace mapping. |
| `DELETE /v1/organizations/users/{user_id}` | Remove user from organization. | Path `user_id`. | Empty 204. | Offboard a member. |
| `GET /v1/organizations/invites` | List pending invites. | Pagination query. | Paginated invite list (`id`, `email`, `roles`, `status`, expiration). | Track outstanding invitations. |
| `GET /v1/organizations/invites/{invite_id}` | Inspect a specific invite. | Path `invite_id`. | Invite record with status and expiry metadata. | Review invitation details before revocation. |
| `POST /v1/organizations/invites` | Create invitation. | `{ "email": "...", "roles": [...], "workspace_id": "...", "expires_at": "ISO8601" }`. | Invite record including `link`. | Onboard new users. |
| `DELETE /v1/organizations/invites/{invite_id}` | Revoke invite. | Path `invite_id`. | Empty 204. | Cancel pending invite. |
| `GET /v1/organizations/workspaces` | List workspaces. | Pagination query. | Paginated array of workspaces (`id`, `name`, `is_default`, `archived`, etc.). | Manage multi-workspace setups. |
| `GET /v1/organizations/workspaces/{workspace_id}` | Inspect workspace. | Path `workspace_id`. | Workspace record. | Review workspace configuration. |
| `POST /v1/organizations/workspaces` | Create workspace. | `{ "name": "...", "description": "...", "default_model": "...", "metadata": {...} }`. | Workspace record. | Provision new workspace containers. |
| `POST /v1/organizations/workspaces/{workspace_id}` | Update workspace. | JSON with fields to mutate (name, default_model, metadata). | Updated workspace record. | Keep workspace metadata current. |
| `POST /v1/organizations/workspaces/{workspace_id}/archive` | Archive workspace. | Optional `{ "archive_reason": "..." }`. | Workspace record with `archived: true`. | Soft-delete while retaining history. |
| `GET /v1/organizations/workspaces/{workspace_id}/members` | List workspace members. | Path `workspace_id`. | Membership list (`user_id`, `role`, `joined_at`). | Audit workspace access. |
| `GET /v1/organizations/workspaces/{workspace_id}/members/{user_id}` | Inspect a particular membership. | Paths `workspace_id`, `user_id`. | Membership record. | Verify roles for one member. |
| `POST /v1/organizations/workspaces/{workspace_id}/members` | Add member to workspace. | `{ "user_id": "...", "role": "...", "permissions": [...] }`. | Membership record. | Grant workspace access. |
| `POST /v1/organizations/workspaces/{workspace_id}/members/{user_id}` | Update member role or permissions. | `{ "role": "...", "permissions": [...] }`. | Membership record. | Adjust workspace privileges. |
| `DELETE /v1/organizations/workspaces/{workspace_id}/members/{user_id}` | Remove member from workspace. | Paths `workspace_id`, `user_id`. | Empty 204. | Revoke workspace access. |
| `GET /v1/organizations/api_keys` | List API key metadata. | Pagination query. | Paginated key list (`id`, `name`, `created_at`, `workspace_id`, `last_used_at`, `status`). | Track credentials and usage. |
| `GET /v1/organizations/api_keys/{api_key_id}` | Inspect key metadata. | Path `api_key_id`. | API key record (non-secret fields). | Review scope and status. |
| `POST /v1/organizations/api_keys/{api_key_id}` | Update key (rename, rotate, suspend). | `{ "name": "...", "workspace_id": "...", "revoked": bool, "rotate": bool }`. | Updated record (rotation responses include new secret). | Manage credential lifecycle. |
| `GET /v1/organizations/usage_report/messages` | Retrieve aggregated Claude message usage. | Query `start_date`, `end_date`, optional `workspace_id`. | JSON with daily rows (`date`, `input_tokens`, `output_tokens`, `total_requests`, `cost_usd`). | Supports dashboards and billing reconciliation. |
| `GET /v1/organizations/usage_report/claude_code` | Retrieve Claude Code usage metrics. | Same query structure as messages report. | JSON with metrics (`sessions`, `duration_minutes`, `active_users`, etc.). | Analyze developer productivity with Claude Code. |
| `GET /v1/organizations/cost_report` | Retrieve cost breakdown across the organization. | Query `start_date`, `end_date`, optional `group_by`. | JSON summarizing spend by model or workspace along with totals. | Finance reconciliation and chargebacks. |

## Rust Integration Test Plan
- **Harness and Shared Utilities**
  - Create `tests/anthropic` suite using `tokio` and `reqwest`; load `ANTHROPIC_API_KEY`, `ANTHROPIC_VERSION`, and feature toggles from the environment, skipping when unset.
  - Centralize header injection and logging helpers that redact secrets while capturing `request-id` for debugging.
  - Mirror Anthropic schemas with `serde` types for messages, batches, files, and admin APIs to validate responses.
- **Messages and Token Counting**
  - Implement baseline chat test verifying `stop_reason`, `usage`, and assistant output for a simple exchange.
  - Add tool-call scenario with mock function definition, asserting tool invocation payload in response.
  - Cover attachments (vision or document references) by uploading files and referencing them in `messages`.
  - Exercise streaming via SSE, accumulating deltas and ensuring final event carries usage totals.
  - Compare `count_tokens` results with actual usage to detect drift.
- **Message Batches Lifecycle**
  - Submit a small batch, poll status with exponential backoff, and assert completion semantics.
  - Download results stream, deserialize JSONL, and validate each `custom_id` alongside success or error payload.
  - Cover list, cancel, and delete flows to guarantee cleanup logic and error handling.
- **Files API**
  - Upload a temporary file, verify metadata, download content for integrity, then delete.
  - Reuse file IDs in message attachments to ensure end-to-end handling.
  - Gate large or binary-heavy tests behind feature flags to keep CI efficient.
- **Models and Prompt Tools**
  - Snapshot `GET /v1/models` to confirm target models are available and deserialize capability flags.
  - Resolve a known alias via `GET /v1/models/{id}` to check alias mapping.
  - Hit each experimental prompt tool endpoint when enabled, asserting returned prompt text and token metadata.
- **Organization and Reporting APIs**
  - Detect admin credential availability; when present, run guarded tests covering users, invites, workspaces, memberships, API keys, and reporting endpoints.
  - Validate usage and cost report schema, ensuring date ranges and totals align with expectations.
  - Provide skip messaging or fixtures when admin access is unavailable so suites remain green.
- **Cross-Cutting Concerns**
  - Add retry logic for 429/5xx responses with jitter and respect Cloudflare rate limits.
  - Ensure teardown removes created files, batches, invites, and other artifacts to leave the organization clean.

## GitHub Issues
- Set up Anthropic integration test harness — <https://github.com/terraphim/terraphim-llm-proxy/issues/17>
- Cover Messages and token counting APIs — <https://github.com/terraphim/terraphim-llm-proxy/issues/18>
- Exercise message batches lifecycle — <https://github.com/terraphim/terraphim-llm-proxy/issues/19>
- Implement Files and prompt tools tests — <https://github.com/terraphim/terraphim-llm-proxy/issues/21>
- Assess organization admin and reporting APIs — <https://github.com/terraphim/terraphim-llm-proxy/issues/22>
