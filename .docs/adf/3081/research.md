# Research Document: Session Analyzer JSONL Parser Fix (#3081)

**Status**: Draft
**Author**: opencode session
**Date**: 2026-07-06
**Issue**: [terraphim/terraphim-ai#3081](https://git.terraphim.cloud/terraphim/terraphim-ai/issues/3081)

## Executive Summary

`terraphim_session_analyzer::parser` produces ~2430 WARN lines on every `terraphim-agent sessions search` invocation because its `SessionEntry` deserialization struct requires `uuid` and `message` fields that 13 Claude Code JSONL entry types do not contain. These are metadata entries (mode changes, permission toggles, file snapshots) that carry no searchable message content. The fix is a pre-filter that skips known non-message entry types before attempting deserialization.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Every sessions search produces 2430 noise lines, obscuring real errors |
| Leverages strengths? | Yes | Terraphim parser codebase -- our own crate, our own format knowledge |
| Meets real need? | Yes | Session search is a core feature referenced in CLAUDE.md and skills |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

The `SessionEntry` struct in `models.rs:133-145` requires two fields that many Claude Code JSONL entry types lack:

```rust
pub struct SessionEntry {
    pub uuid: String,           // REQUIRED -- metadata entries use leafUuid or nothing
    pub message: Message,       // REQUIRED -- metadata entries have no message
    ...
}
```

When `parser.rs:42` calls `serde_json::from_str::<SessionEntry>(&line)` on every non-empty line, entries without these fields fail deserialization and emit a WARN log.

### Impact

- ~2430 WARN lines per sessions scan, obscuring genuinely malformed data
- Silent data loss: valid `attachment` and some `assistant` entries that lack `message` are dropped from the index
- Poor UX: sessions search output is flooded with parser warnings on stderr

### Success Criteria

- Zero WARN lines for known Claude Code metadata entry types
- All conversational messages (`user`, `assistant` with message content, `tool-result`) are still parsed and indexed
- Genuinely malformed lines still produce WARN (not silently swallowed)
- No change to downstream consumers (`extract_agent_invocations`, `extract_file_operations`, etc.)

## Current State Analysis

### Crate Location

The `terraphim_session_analyzer` crate was extracted from terraphim-ai in commit `aa7ba99e8` (E4a/E4b/E5 dir-removal, Refs #1910). Source is available from:
- Registry cache: `~/.cargo/registry/src/index.crates.io-*/terraphim-session-analyzer-1.20.3/`
- Git history: `terraphim-ai` repo, commit before `aa7ba99e8`
- Consumed by: `terraphim-clients/crates/terraphim_sessions` (feature `tsa-full`, version `1.19.2`)

### Code Locations

| Component | Location (registry cache) | Purpose |
|-----------|--------------------------|---------|
| `SessionEntry` struct | `src/models.rs:133-145` | Deserialization target for every JSONL line |
| `Message` enum | `src/models.rs:147-166` | `#[serde(untagged)]` -- requires role + content |
| `from_file` parser | `src/parser.rs:27-85` | Line-by-line deserialize with warn-on-fail |
| Downstream consumers | `src/parser.rs:143-266` | All check `if let Message::Assistant { .. }` before use |

### Data Flow

```
Claude Code JSONL file
    |
    v
parser.rs:from_file() -- reads line by line
    |
    v
serde_json::from_str::<SessionEntry>(&line)  <-- FAILS HERE for 13 entry types
    |                    |
    | Ok(entry)          | Err(e)
    v                    v
entries.push(entry)   warn!("Failed to parse...")
```

### Affected Entry Types (measured)

| Entry type | Error count | Root cause |
|---|---|---|
| `attachment` | 473 | Has `uuid`, lacks `message` |
| `assistant` | 427 | Some entries lack `message` (likely malformed or different format) |
| `last-prompt` | 330 | Lacks `uuid`, has `leafUuid` |
| `permission-mode` | 287 | Lacks `uuid` and `message` |
| `ai-title` | 272 | Lacks `uuid` and `message` |
| `mode` | 251 | Lacks `uuid` and `message` |
| `file-history-snapshot` | 150 | Lacks `uuid` and `message` |
| `system` | 73 | Lacks `uuid` and `message` |
| `queue-operation` | 70 | Lacks `uuid` and `message` |
| `agent-name` | 45 | Lacks `uuid` and `message` |
| `pr-link` | 44 | Lacks `uuid` and `message` |
| `tool_reference` | 7 | Lacks `uuid` and `message` |
| `text` | 1 | Lacks `uuid` and `message` |

**Total: ~2430 errors.** Of these, ~2003 are metadata entries that should be skipped entirely. The ~427 `assistant` failures warrant investigation but may be genuinely malformed/truncated lines.

## Constraints

### Technical Constraints

- **Crate is published to registry**: Fix requires republishing `terraphim-session-analyzer` with bumped version
- **Consumer version pin**: `terraphim_sessions` depends on `1.19.2`; the installed binary uses `1.20.3`
- **No mocks in tests**: Test data must use real Claude Code JSONL fixtures
- **Serde untagged enum**: The `Message` enum uses `#[serde(untagged)]` which tries each variant; making `message` optional is feasible

### Non-Functional Requirements

| Requirement | Target | Current |
|---|---|---|
| WARN lines per scan | 0 for known types | ~2430 |
| Parse latency | No increase | ~80ms per file |
| Memory | No increase | Linear in entry count |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|---|---|---|
| Must not lose conversational messages | `user`/`assistant` entries are the searchable content | parser.rs downstream consumers all filter on Message::Assistant |
| Must eliminate noise for known metadata types | 2430 WARN lines is the reported bug | Issue #3081 |
| Must keep WARN for genuinely unknown types | Silent failure on new formats is worse than noise | LLM coding discipline: surface confusion |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|---|---|
| Making `message` field `Option<Message>` on SessionEntry | Ripples through all downstream code; metadata entries have no message anyway |
| Making `uuid` field `Option<String>` | Metadata entries don't need a uuid; downstream code uses it only for message-bearing entries |
| Parsing metadata entry types into new structs | These entries carry no searchable content; indexing them adds noise |
| Investigating the 427 `assistant` failures separately | Likely genuinely malformed/truncated lines; the pre-filter won't affect them, and WARN is appropriate for those |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| New Claude Code version adds new metadata type not in skip list | Medium | Low (WARN reappears for new type only) | Skip set is easy to extend; one-line addition |
| A metadata type starts carrying useful content in future | Low | Low | Re-evaluate skip set when Claude Code format changes |
| `assistant` failures are a real format bug, not malformed data | Medium | Medium | Pre-filter doesn't touch `assistant`; those WARNs remain visible for investigation |

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|---|---|---|---|
| Metadata entry types never carry searchable message content | Inspected all 13 types in production JSONL files | If wrong, we miss indexing some content | Yes |
| `user` and `assistant` entry types always have `uuid` and `message` | Existing test fixtures, Claude Code format spec | If wrong, we'd need Option fields instead | Yes (for valid entries) |
| The 427 `assistant` failures are malformed/truncated, not a format change | They fail at varying column positions suggesting truncation | If wrong, a separate format compatibility fix is needed | Partially -- needs spike |

## Research Findings

### Key Insights

1. The downstream consumers (`extract_agent_invocations`, `extract_file_operations`, `extract_tool_invocations`) ALL filter with `if let Message::Assistant { .. }`. Non-message entries are never used even when successfully parsed. Skipping them at parse time loses nothing.

2. The pre-filter approach is a one-location change in `from_file()` at `parser.rs:42`. No struct changes, no downstream code changes.

3. A lightweight `EntryTypePeek` struct can extract just the `"type"` field before full deserialization, costing negligible overhead.

### Relevant Prior Art

- The `connectors/` module in the same crate already handles format-specific parsing for Cursor, Aider, OpenCode, and Codex. The Claude Code parser is the only one with the all-lines-must-fit pattern.
- The `terraphim_sessions` crate (the newer one in terraphim-clients) has its own `model.rs` with a different `SessionEntry` that may not have this issue -- needs verification.

## Recommendations

### Proceed/No-Proceed

**Proceed.** The fix is a single-location pre-filter with no downstream impact. Estimated effort: 2-3 hours including tests.

### Scope Recommendations

- Fix applies to `terraphim_session_analyzer` crate source
- Crate source needs to be restored from git history or worked on in the publishing repo
- Publish as `1.20.6` or `1.21.9` patch bump

### Risk Mitigation Recommendations

- Add test fixtures for each of the 13 metadata entry types
- Add a test verifying zero WARN output for a fixture file containing only metadata types
- Keep the WARN path for genuinely unknown types (anything not in the skip set)
