# Documentation Audit Report

**Date:** 2026-06-04
**Agent:** documentation-generator
**Scope:** 6 key crates (consumer-facing, high-traffic)

---

## Executive Summary

| Metric | Value |
|--------|-------|
| Crates scanned | 6 |
| Total public items | 882 |
| Undocumented | 339 |
| Coverage | 62% |
| SIGNIFICANT_GAPS | 5 crates |
| MINOR_GAPS | 1 crate |
| CLEAN | 0 crates |

---

## Per-Crate Results

| Crate | Public Items | Undocumented | % Gap | Verdict |
|-------|-------------|--------------|-------|---------|
| `terraphim_types` | 344 | 128 | 37% | SIGNIFICANT_GAPS |
| `terraphim_service` | 250 | 89 | 36% | SIGNIFICANT_GAPS |
| `terraphim_sessions` | 111 | 44 | 40% | SIGNIFICANT_GAPS |
| `terraphim_automata` | 142 | 62 | 44% | SIGNIFICANT_GAPS |
| `terraphim_persistence` | 34 | 15 | 44% | SIGNIFICANT_GAPS |
| `haystack_core` | 1 | 1 | 100% | MINOR_GAPS |

---

## Priority Undocumented Items

### `terraphim_persistence` — highest-density critical types

| File | Line | Type | Name |
|------|------|------|------|
| `lib.rs` | 66 | struct | `DeviceStorage` |
| `lib.rs` | 220 | trait | `Persistable` |
| `error.rs` | 5 | enum | `Error` |
| `conversation.rs` | 10 | trait | `ConversationPersistence` |
| `conversation.rs` | 32 | struct | `ConversationIndex` |
| `lib.rs` | 18-24 | mod | 7 undocumented `pub mod` re-exports |

### `terraphim_types` — most undocumented items (128)

Top priority (referenced workspace-wide):

| File | Line | Type | Name |
|------|------|------|------|
| `lib.rs` | 171 | struct | `RoleName` |
| `lib.rs` | 262 | struct | `NormalizedTermValue` |
| `lib.rs` | 306 | struct | `NormalizedTerm` |
| `lib.rs` | 438 | struct | `Concept` |
| `lib.rs` | 476 | enum | `DocumentType` |
| `lib.rs` | 488 | struct | `RouteDirective` |
| `lib.rs` | 606 | struct | `MarkdownDirectives` |
| `medical_types.rs` | 36 | enum | `MedicalNodeType` |
| `medical_types.rs` | 141 | enum | `MedicalEdgeType` |
| `medical_types.rs` | 383 | struct | `MedicalNodeMetadata` |
| `hgnc.rs` | 12 | struct | `HgncGene` |

### `terraphim_sessions` — core domain types (44 gaps)

| File | Line | Type | Name |
|------|------|------|------|
| `model.rs` | 258 | struct | `Session` |
| `model.rs` | 165 | struct | `Message` |
| `model.rs` | 220 | struct | `SessionMetadata` |
| `model.rs` | 21 | enum | `MessageRole` |
| `model.rs` | 62 | enum | `ContentBlock` |
| `model.rs` | 756 | enum | `FileOperation` |
| `service.rs` | — | fn | all public methods |

### `terraphim_automata` — evaluation and UMLS types (62 gaps)

| File | Line | Type | Name |
|------|------|------|------|
| `evaluation.rs` | 17 | struct | `GroundTruthDocument` |
| `evaluation.rs` | 28 | struct | `ExpectedMatch` |
| `evaluation.rs` | 37 | struct | `ClassificationMetrics` |
| `evaluation.rs` | 48 | struct | `TermReport` |
| `evaluation.rs` | 55 | struct | `EvaluationResult` |
| `umls.rs` | 14 | struct | `UmlsConcept` |
| `umls.rs` | 47 | struct | `UmlsDataset` |
| `url_protector.rs` | 55 | struct | `ProtectedUrl` |

### `haystack_core` — trivial fix

| File | Line | Type | Name |
|------|------|------|------|
| `lib.rs` | 8 | trait | `HaystackProvider` |

---

## CHANGELOG.md Updates

Added under `### Fixed` (2026-06-04):
- `fix(agent)`: HTTP 4xx classified as `ErrorGeneral` not `ErrorNetwork`; integration test `server_http_error_exits_1` added (Refs #1992)

The `docs(specs)` commit (98fa93b32) is an internal documentation update with no user-facing changelog entry.

---

## Recommendations

1. **Quick wins** (< 30 min combined):
   - `haystack_core::HaystackProvider` — one line
   - `terraphim_persistence::Error` enum — one line
   - `terraphim_persistence` module re-exports (7 lines)

2. **Medium effort** (half-day):
   - `terraphim_persistence` core traits: `DeviceStorage`, `Persistable`, `ConversationPersistence`
   - `terraphim_types` core structs: `RoleName`, `NormalizedTerm`, `Concept`, `DocumentType`

3. **Batch effort** (track as issue):
   - `terraphim_sessions::model` — all domain types
   - `terraphim_automata::evaluation` — metrics structs
   - `terraphim_service` module re-exports

See `.docs/api-reference-snippets.md` for proposed doc comments on key types.

---

## Gitea

Findings posted as comment on issue #2137 (Theme-ID: doc-gap).
