# Specification Validation Report

**Date:** 2026-03-24
**Branch:** task/58-handoff-context-fields
**Validated by:** Carthos (Domain Architect)

---

## Executive Summary

8 specifications validated against crate implementations. The system shows strong implementation in its core domain (session search, knowledge graph, learning capture) but significant gaps in the service orchestration and desktop integration layers -- the boundaries between subsystems remain unwired.

| Specification | Status | Coverage | Priority Gaps |
|---|---|---|---|
| Chat Session History | PARTIAL | ~30% | Service layer, API endpoints, Tauri commands |
| Chat Session History QuickRef | PARTIAL | ~30% | (Same as above) |
| Agent Session Search Spec | IMPLEMENTED | ~85% | Token budget, Tantivy, additional connectors |
| Agent Session Search Architecture | IMPLEMENTED | ~85% | (Aligned with spec above) |
| Agent Session Search Tasks | PHASES 1-3 DONE | ~90% | Phase 1 tests, token budget |
| Learning Capture Interview | IMPLEMENTED | ~85% | CLI surface area verification |
| Codebase Evaluation Check | NOT IMPLEMENTED | ~5% | Aspirational -- entire framework missing |
| Desktop Application | SUBSTANTIALLY DONE | ~75% | Tauri IPC layer, system integration |

---

## Detailed Findings

### 1. Chat Session History Specification

**Source:** `docs/specifications/chat-session-history-spec.md`, `chat-session-history-quickref.md`

**Bounded context:** Conversation lifecycle management -- creation, persistence, search, export.

#### Implemented (foundation layer)

- `terraphim_types`: `Conversation`, `ChatMessage`, `ContextItem` data models exist
- `terraphim_persistence/src/conversation.rs`: `ConversationPersistence` trait with `OpenDALConversationPersistence` (SQLite, DashMap, Memory, optional S3)
- `desktop/src/lib/Chat/SessionList.svelte`: Full session list UI with filtering, timestamps, message counts
- `desktop/src/lib/Chat/Chat.svelte`: Chat component with session sidebar integration

#### Missing (service and integration layers)

| Gap | Spec Section | Severity |
|---|---|---|
| `ConversationService` orchestration layer | Service Layer | HIGH |
| REST API endpoints (`GET/POST/PUT/DELETE /api/conversations`) | API Layer | HIGH |
| Tauri IPC commands (9 specified: `list_all_conversations`, `create_new_conversation`, etc.) | Desktop Integration | HIGH |
| Auto-save with 2-second debounce | UX | MEDIUM |
| Full-text search across conversations | Search | MEDIUM |
| Export/Import (JSON serialization) | Data Portability | MEDIUM |
| Archive/Restore workflow | Lifecycle | LOW |
| Clone/branch conversations | Lifecycle | LOW |
| Statistics aggregation | Analytics | LOW |

#### Diagnosis

The aggregate root (`Conversation`) and its persistence boundary are correctly implemented. The gap is the **application service layer** -- the invariant-enforcing orchestrator that sits between UI/API and persistence. The UI components exist; the persistence exists; the middle is hollow.

---

### 2. Agent Session Search Specification

**Source:** `docs/specifications/terraphim-agent-session-search-spec.md`, `-architecture.md`, `-tasks.md`

**Bounded context:** Multi-agent session import, indexing, search, and knowledge graph enrichment.

#### Implemented (Phases 1-3 substantially complete)

- **Robot Mode** (`crates/terraphim_agent/src/robot/`): JSON/JSONL/Minimal/Table output, exit codes, response schemas, self-documentation API
- **Forgiving CLI** (`crates/terraphim_agent/src/forgiving/`): Jaro-Winkler fuzzy matching, alias management, command suggestions
- **Session Search** (`crates/terraphim_sessions/`, `crates/terraphim-session-analyzer/`): Claude Code JSONL connector, Cursor SQLite connector, REPL commands (`/sessions sources|import|list|search|stats|show`)
- **KG Enrichment** (`crates/terraphim_sessions/src/enrichment/`): Concept extraction via terraphim_automata, confidence scoring, dominant topic identification

#### Missing

| Gap | Phase | Severity |
|---|---|---|
| Token budget management (`--max-tokens`, `--max-results`, field modes) | 1.5 | MEDIUM |
| Tantivy full-text index integration | 2.5 | MEDIUM |
| Aider connector (Markdown parsing) | 2.5 | LOW |
| Cline connector (JSON parsing) | 2.5 | LOW |
| Phase 1 integration tests | 1.6 | MEDIUM |

#### Divergences

- **Connector architecture**: Spec designed from-scratch connectors; implementation pragmatically wraps `terraphim-session-analyzer` (CLA) as git subtree with feature gates. Architecturally sound deviation -- reduces duplication.
- **Search engine**: Spec specifies Tantivy; implementation uses existing `terraphim_automata` matching. Functional but lacks full-text ranking capabilities Tantivy would provide.

---

### 3. Learning Capture Specification

**Source:** `docs/specifications/learning-capture-specification-interview.md`

**Bounded context:** Automated failure capture from shell hooks, with redaction, correction, and query.

#### Implemented (core pipeline)

- `crates/terraphim_agent/src/learnings/capture.rs`: Capture logic with chained command parsing
- `crates/terraphim_agent/src/learnings/redaction.rs`: Secret auto-redaction via `terraphim_automata::replace_matches()` (AWS, GCP, Azure, API keys, connection strings)
- `crates/terraphim_agent/src/learnings/hook.rs`: Hook integration for post-tool-use capture
- `crates/terraphim_agent/src/learnings/install.rs`: Hook installation
- Data types: `CapturedLearning`, `LearningSource`, `LearningCaptureConfig`

#### Gaps requiring verification

| Gap | Detail | Severity |
|---|---|---|
| CLI command surface area | `learn capture/query/correct/list/stats/prune` -- present in module but full CLI wiring unverified | MEDIUM |
| Configuration file | `.terraphim/learning-capture.toml` support unverified | LOW |
| KG-based synonym expansion for queries | Spec promises automata-enriched search | LOW |

---

### 4. Codebase Evaluation Check

**Source:** `docs/specifications/terraphim-codebase-eval-check.md`

**Bounded context:** Automated before/after codebase evaluation with role-based scoring.

#### Status: NOT IMPLEMENTED

This is an **aspirational specification** describing a future evaluation framework. No corresponding implementation exists:

- No evaluation orchestrator service
- No before/after comparison logic
- No verdict engine with scoring heuristics
- No role-based evaluation workflows (Code Reviewer, Performance Analyst, Security Auditor, Documentation Steward)
- No CI integration for automated evaluation
- No artifact storage convention

**Prerequisite components exist** (terraphim backend, metrics tooling, TUI) but the evaluation domain itself is unbuilt.

---

### 5. Desktop Application Specification

**Source:** `docs/specifications/terraphim-desktop-spec.md`

**Bounded context:** Privacy-first desktop application with search, chat, KG visualization, and configuration.

#### Implemented (frontend + backend, gap in middle)

- **Frontend**: Svelte + TypeScript + Vite + Bulma -- complete component set (Search, Chat, RoleGraphVisualization, ConfigWizard, ThemeSwitcher, Novel Editor, SessionList)
- **Backend**: terraphim_server with health, config, search, chat endpoints
- **Storage**: OpenDAL multi-backend persistence
- **AI**: Ollama + OpenRouter integration
- **Themes**: 22 variants via ThemeSwitcher
- **KG Visualization**: D3.js-based RoleGraphVisualization

#### Missing

| Gap | Detail | Severity |
|---|---|---|
| Tauri command handlers | 9+ conversation management commands specified but not wired | HIGH |
| System tray integration | Not found | LOW |
| Global keyboard shortcuts | System-level shortcuts not verified | LOW |
| MCP autocomplete in Novel editor | Editor exists, MCP wiring unclear | MEDIUM |
| Session persistence commands | UI exists without backend handlers | HIGH |

---

## Cross-Cutting Observations

### 1. The Hollow Middle Pattern

Multiple specs reveal the same structural gap: **persistence layer exists, UI exists, but the service/command layer between them is missing**. This is most acute for conversation management where `ConversationPersistence` trait is implemented and `SessionList.svelte` renders conversations, but no `ConversationService` or Tauri commands bridge them.

### 2. Specification Freshness

- **Active and aligned**: Agent Session Search (3 docs) -- implementation tracks spec closely
- **Partially stale**: Chat Session History -- spec written ahead of implementation, foundation built but orchestration not started
- **Aspirational**: Codebase Evaluation Check -- design document without implementation timeline

### 3. Pragmatic Divergences (Acceptable)

- CLA git subtree instead of from-scratch connectors (less code, same capability)
- `terraphim_automata` instead of Tantivy for session search (simpler, sufficient for current scale)

### 4. Spec-to-Crate Mapping

| Specification Domain | Primary Crates | Status |
|---|---|---|
| Conversation Lifecycle | `terraphim_types`, `terraphim_persistence`, `terraphim_service` | Persistence done, service missing |
| Session Search | `terraphim_agent`, `terraphim_sessions`, `terraphim-session-analyzer` | Substantially complete |
| Learning Capture | `terraphim_agent` (learnings module) | Core complete, CLI surface unclear |
| Codebase Evaluation | (none) | Not started |
| Desktop Application | `desktop/`, `terraphim_server` | Frontend complete, IPC layer gaps |

---

## Recommended Actions (Priority Order)

### HIGH -- Unblock Features

1. **Implement `ConversationService`** in `terraphim_service` -- the missing aggregate root orchestrator. Wire CRUD operations from persistence trait to API surface.
2. **Add REST endpoints** for conversation management in `terraphim_server` -- 5 core routes minimum.
3. **Wire Tauri IPC commands** (if desktop mode is active) -- connect `SessionList.svelte` to actual persistence.

### MEDIUM -- Complete Coverage

4. **Token budget management** for agent session search -- needed for AI-agent consumption.
5. **Verify learning capture CLI** -- ensure `learn query/correct/list/stats/prune` subcommands are fully wired.
6. **Add Phase 1 integration tests** for robot mode and forgiving CLI.
7. **Auto-save with debounce** for chat conversations.

### LOW -- Future Enhancement

8. **Tantivy integration** for session full-text search (when scale demands it).
9. **Additional session connectors** (Aider, Cline) -- community-driven priority.
10. **Codebase Evaluation framework** -- requires dedicated design sprint; spec is sound but scope is large.
11. **System tray and global shortcuts** for desktop.

---

## Methodology

- Read all 8 specification documents in `docs/specifications/`
- Cross-referenced against source files in 10+ crates (`terraphim_types`, `terraphim_persistence`, `terraphim_service`, `terraphim_agent`, `terraphim_sessions`, `terraphim-session-analyzer`, `terraphim_tui`, `terraphim_mcp_server`, `terraphim_server`, `desktop/`)
- Verified module structure, trait implementations, and public API surface
- Checked for divergences between specified data models and implemented types
- Assessed implementation completeness by feature, not by line count
