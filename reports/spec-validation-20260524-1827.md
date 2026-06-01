# Spec Validation Report: Issue #1827 -- Add Licence Field to terraphim_merge_coordinator

**Date**: 2026-05-24 11:35 CEST
**Branch**: `task/1827-add-license-merge-coordinator`
**Validator**: Carthos (spec-validator)
**Verdict**: PASS

---

## Requirements Enumerated

Derived from issue #1827 title (body empty) and the compliance-watchdog FAIL comment that triggered it:

| Req ID | Requirement | Source |
|--------|-------------|--------|
| REQ-1 | `crates/terraphim_merge_coordinator/Cargo.toml` must contain `license = "Apache-2.0"` in `[package]` | Issue #1827 title; compliance-watchdog comment |
| AC-1 | `cargo deny check licenses` must not report `terraphim_merge_coordinator` as `unlicensed` | compliance-watchdog finding L1 (BLOCKER, P1) |

---

## Context

The compliance-watchdog agent (2026-05-24 02:05 CEST) posted a FAIL verdict on issue #1827 with finding:

> **Bug Reporting: P1**
> Crate: `terraphim_merge_coordinator v1.19.3`
> Location: `crates/terraphim_merge_coordinator/Cargo.toml`
> Error: `cargo deny` reports `unlicensed` -- no license expression in manifest

The `terraphim_merge_coordinator` crate was introduced as a minimal skeleton in PR #1823 (commit `04648c24`) without a `license` field in its `[package]` section.

The workspace `[workspace.package]` in `Cargo.toml` carries no `license` field, so there is no workspace-level inheritance for this field. All other workspace-included crates declare an explicit licence. The dominant value across 36 crates is `Apache-2.0`.

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Evidence | Status |
|--------|-------------|------------|----------|----------|--------|
| REQ-1 | `license = "Apache-2.0"` in `[package]` | Issue #1827; compliance FAIL L1 | `crates/terraphim_merge_coordinator/Cargo.toml` line 5 | `git diff HEAD` confirms: `+license = "Apache-2.0"` at correct location | PASS |
| AC-1 | `cargo deny check licenses` passes | REQ-1 | `deny.toml` `[licenses]` section | `cargo deny check licenses` exits `licenses ok` with no `unlicensed` finding for this crate | PASS |

---

## Additional Observations

| Observation | Assessment |
|-------------|------------|
| `Apache-2.0` is used by 36 of the other workspace crates; 14 use `MIT`; 4 use `MIT OR Apache-2.0` | Choice is consistent with dominant workspace pattern |
| `terraphim_server` adds both `license = "Apache-2.0"` and `license-file = ["../LICENSE-Apache-2.0", "4"]`; the merge-coordinator adds only the SPDX expression | Acceptable -- `license-file` is only needed when the manifest is published to crates.io with a file reference; `terraphim_merge_coordinator` is a workspace-internal binary crate |
| `terraphim_lsp` (in `crates/`) also lacks a licence field | Out of scope -- this crate is excluded from the workspace (`exclude = [...]`) and is therefore invisible to `cargo deny`; no action needed here |
| `html2md v0.2.15` emits a deprecated SPDX `GPL-3.0+` parse warning from `cargo deny` | Pre-existing transitive issue; not caused by or related to this change |

---

## Gaps

None. All requirements are satisfied.

---

## Overall Verdict

**PASS**

The single-line change (`license = "Apache-2.0"`) directly satisfies the spec derived from issue #1827 and resolves the compliance-watchdog P1 blocker. `cargo deny check licenses` exits clean (`licenses ok`) on this branch with no `unlicensed` finding for `terraphim_merge_coordinator`.
