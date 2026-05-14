# Spec Validation Report: #547 QualityScores in IndexedDocument

**Date**: 2026-05-14  
**Agent**: Carthos (spec-validator)  
**Issue**: terraphim/terraphim-ai#547  
**Branch**: task/547-quality-scores-indexed-document  
**Verdict**: PARTIAL PASS — core structure present, three deviations from spec

---

## Traceability Matrix

| Req ID | Requirement | Impl Ref | Tests | Status |
|--------|-------------|----------|-------|--------|
| REQ-001 | `QualityScore` struct with K/L/S dimensions | `crates/terraphim_types/src/lib.rs:888` | `test_quality_score_composite`, `test_quality_score_serialization` | ⚠️ Field naming deviation |
| REQ-002 | Add `quality_scores` field to `IndexedDocument` | `crates/terraphim_types/src/lib.rs:954` | `test_indexed_document_with_quality_score` | ⚠️ Field named `quality_score` (singular) |
| REQ-003 | `overall: Option<f32>` weighted average field | Missing — `composite()` method exists but is unweighted | No test for weighted composite | ❌ Missing field |
| REQ-004 | `last_evaluated: Option<DateTime<Utc>>` field | Missing entirely | No test | ❌ Missing field |
| REQ-005 | `min_quality` search parameter | `crates/terraphim_types/src/lib.rs:1133` | `test_min_quality_serialization` (inline) | ✅ PASS |
| REQ-006 | `--min-quality` CLI flag | `crates/terraphim_cli/src/main.rs:101`, `crates/terraphim_agent/src/main.rs:721` | — | ✅ PASS |
| REQ-007 | Filter documents below `min_quality` threshold | `crates/terraphim_service/src/lib.rs:1518` | Unit test in `lib.rs` | ✅ PASS |

---

## Findings

### PASS — Core Structure

`QualityScore` exists at `crates/terraphim_types/src/lib.rs:888`:

```rust
pub struct QualityScore {
    pub knowledge: Option<f64>,
    pub learning: Option<f64>,
    pub synthesis: Option<f64>,
}
```

`IndexedDocument` carries `quality_score: Option<QualityScore>` at line 954.  
`Document` also carries it at line 596 (positive extension, not in spec).  
`SearchQuery.min_quality: Option<f64>` present at line 1133.  
`apply_min_quality_filter` in `terraphim_service::lib` filters on `Document.quality_score.composite()`.  
`--min-quality` flag present in both `terraphim_cli` and `terraphim_agent`.  
`composite()` returns 0.0 when no scores are set — NaN guard confirmed.

---

### DEVIATION 1 — Dimension field naming (Medium, Spec Contract)

**Spec** (#547 issue body):
- `logic: Option<f32>` — L dimension
- `structure: Option<f32>` — S dimension

**Implementation**:
- `learning: Option<f64>` (not `logic`)
- `synthesis: Option<f64>` (not `structure`)

ADF Nightwatch is listed as a consumer of quality scores. If it serialises or deserialises using field names `logic` / `structure`, it will silently receive `None` from this implementation. This is a contract-breaking deviation.

**Mitigation path**: Either rename `learning`→`logic` and `synthesis`→`structure` in `terraphim_types`, or document the deliberate rename and update ADF Nightwatch accordingly.

---

### DEVIATION 2 — Missing `overall` weighted average field (Follow-up)

**Spec**: `overall: Option<f32>` — weighted average  
**Implementation**: `composite()` method provides unweighted average

The spec description says "weighted average". The current `composite()` is unweighted (equal weight per populated dimension). This is a behavioural gap. If Nightwatch or external judges write `overall` scores derived from domain-specific weights, there is no field to persist them — the implementation always recomputes.

---

### DEVIATION 3 — Missing `last_evaluated` field (Blocker for temporal tracking)

**Spec**: `last_evaluated: Option<DateTime<Utc>>`  
**Implementation**: completely absent

This field enables consumers to know when quality was last assessed, supporting freshness-based re-evaluation and caching strategies. Without it, Nightwatch cannot record evaluation timestamps and must re-evaluate every document on every cycle.

**Resolution**: Add `last_evaluated: Option<chrono::DateTime<chrono::Utc>>` to `QualityScore`.

---

### DEVIATION 4 — Naming divergence: singular vs plural (Low, Style)

**Spec**: struct `QualityScores`, field `quality_scores`  
**Implementation**: struct `QualityScore`, field `quality_score`

The singular form is consistent throughout the implementation (struct, field, all references). This is a style deviation without functional impact unless external consumers use the plural field name in serialised JSON. Since `quality_score` is the JSON key name, any consumer expecting `quality_scores` will receive `null`.

---

### DEVIATION 5 — Type precision: `f32` vs `f64` (Low, Upgrade)

**Spec**: `Option<f32>`  
**Implementation**: `Option<f64>`

Higher precision is generally preferable. No functional regression. However, if ADF or external systems serialise scores as 32-bit floats and compare by exact equality, rounding differences may surface.

---

## Gaps Summary

| Gap | Severity | Action |
|-----|----------|--------|
| `logic`/`structure` naming vs `learning`/`synthesis` | Medium — ADF contract | Rename fields or document deliberate divergence |
| Missing `overall` weighted average field | Follow-up | Add field; `composite()` serves as fallback |
| Missing `last_evaluated` field | Blocker — temporal tracking | Add `last_evaluated: Option<DateTime<Utc>>` |
| Singular vs plural naming | Low — JSON key contract | Verify no external consumer uses plural key |

---

## Positive Extensions

- `quality_score` on `Document` (not in spec) — enables filtering before index construction
- `composite()` computed method with correct NaN guard
- Backward-compatible `#[serde(default)]` on `quality_score` — old data without the field deserialises cleanly
- Tests: `test_quality_score_composite`, `test_quality_score_serialization`, `test_indexed_document_with_quality_score`, `test_indexed_document_from_document_quality_score_none`

---

## Recommendation

Address the two blockers before merging:

1. Add `last_evaluated: Option<chrono::DateTime<chrono::Utc>>` to `QualityScore`
2. Decide and document: rename `learning`→`logic` and `synthesis`→`structure`, or explicitly document the deliberate rename and update all consumers

The `overall` field and precision type can be addressed in a follow-up issue if preferred.
