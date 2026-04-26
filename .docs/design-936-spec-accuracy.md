# Design Document: Task 1.4/1.5 Spec Accuracy (#936)

**Date**: 2026-04-26
**Issue**: #936
**Phase**: Phase 2 (Disciplined Design)
**Research Doc**: `.docs/research-936-spec-accuracy.md`

## Summary

Documentation-only update to `docs/specifications/terraphim-agent-session-search-tasks.md`.
All code changes are already complete; only spec checkboxes need updating.

## File Changes

### `docs/specifications/terraphim-agent-session-search-tasks.md`

#### Change 1: Task 1.4 subtask checkboxes (lines 159-177)
Change `[ ]` to `[x]` for subtasks 1.4.1 through 1.4.4.

| Line | Current | New |
|------|---------|-----|
| 159 | `- [ ] **1.4.1**` | `- [x] **1.4.1**` |
| 164 | `- [ ] **1.4.2**` | `- [x] **1.4.2**` |
| 169 | `- [ ] **1.4.3**` | `- [x] **1.4.3**` |
| 174 | `- [ ] **1.4.4**` | `- [x] **1.4.4**` |

#### Change 2: Task 1.4 acceptance criteria (lines 181-183)
Change `[ ]` to `[x]` for all three criteria.

| Line | Current | New |
|------|---------|-----|
| 181 | `- [ ] Interactive mode` | `- [x] Interactive mode` |
| 182 | `- [ ] Robot mode` | `- [x] Robot mode` |
| 183 | `- [ ] Exit codes` | `- [x] Exit codes` |

#### Change 3: Task 1.5 subtask checkboxes (lines 195-213)
Change `[ ]` to `[x]` for subtasks 1.5.1 through 1.5.4.

| Line | Current | New |
|------|---------|-----|
| 195 | `- [ ] **1.5.1**` | `- [x] **1.5.1**` |
| 199 | `- [ ] **1.5.2**` | `- [x] **1.5.2**` |
| 205 | `- [ ] **1.5.3**` | `- [x] **1.5.3**` |
| 210 | `- [ ] **1.5.4**` | `- [x] **1.5.4**` |

#### Change 4: Task 1.5 acceptance criteria (lines 217-219)
Change `[ ]` to `[x]` for all three criteria.

| Line | Current | New |
|------|---------|-----|
| 217 | `- [ ] \`--max-tokens 1000\`` | `- [x] \`--max-tokens 1000\`` |
| 218 | `- [ ] Truncated fields` | `- [x] Truncated fields` |
| 219 | `- [ ] Pagination` | `- [x] Pagination` |

#### Change 5: Progress tracking table (lines 627-629)

| Current | New |
|---------|-----|
| `\| 1.4 \| Partial \| - \| --robot/--format flags added; REPL dispatch pending \|` | `\| 1.4 \| Complete \| - \| REPL robot mode, forgiving parser, /robot command \|` |
| `\| 1.5 \| Not Started \| - \| Token budget \|` | `\| 1.5 \| Complete \| - \| Token budget, field filtering, truncation, pagination \|` |

#### Change 6: Document status and date (lines 5-6)

| Current | New |
|---------|-----|
| `> **Updated**: 2025-12-04` | `> **Updated**: 2026-04-26` |

## Test Strategy

No code tests needed. Verification is:
1. `grep -c '\- \[ \]' file` should show fewer unchecked boxes
2. Progress table matches codebase reality
3. Markdown renders correctly
