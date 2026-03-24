# Design & Implementation Plan: Fix Desktop Frontend TypeScript Errors

**Status**: Draft
**Author**: Claude Code Agent
**Date**: 2026-03-24
**Based on**: Research Document `.docs/research-typescript-errors.md`

---

## 1. Summary of Target Behavior

After implementation, the desktop frontend will:
- Pass `yarn check` (svelte-check) with **zero TypeScript errors**
- Build successfully to `desktop/dist/` for embedding in terraphim_server
- Preserve all existing functionality (no runtime changes)
- Maintain backward compatibility with existing APIs

---

## 2. Key Invariants and Acceptance Criteria

### Invariants (Must Not Violate)
| Invariant | Rationale | Verification |
|-----------|-----------|--------------|
| No runtime behavior changes | Type fixes only, no logic changes | Code review + manual testing |
| All tests pass | Cannot reduce quality | `yarn test` passes |
| Build produces valid output | Required for deployment | `ls -la dist/` has assets |
| TiPTap editor functionality preserved | Core user feature | Manual editor test |

### Acceptance Criteria
| ID | Criterion | Test Method |
|----|-----------|-------------|
| AC1 | `yarn check` exits 0 | Automated |
| AC2 | `yarn build` completes without errors | Automated |
| AC3 | Editor autocomplete works (++) | Manual E2E |
| AC4 | Slash commands work (/) | Manual E2E |
| AC5 | Knowledge graph visualization renders | Manual E2E |
| AC6 | Config wizard loads | Manual E2E |
| AC7 | Unit tests pass | `yarn test` |

---

## 3. High-Level Design and Boundaries

### Change Boundaries

**Inside Scope (Modifying):**
- `desktop/package.json` - Dependency version alignment
- `desktop/src/lib/Editor/TerraphimSuggestion.ts` - Type fixes
- `desktop/src/lib/Editor/SlashCommand.ts` - Type fixes
- `desktop/src/lib/RoleGraphVisualization.svelte` - Type fixes + typo fix
- `desktop/src/lib/ConfigJsonEditor.svelte` - Type fixes
- `desktop/src/lib/Search/ArticleModal.svelte` - Remove unused directive
- Test files - Fix Vitest/Jest configuration

**Outside Scope (Not Touching):**
- Runtime logic (only types change)
- Rust backend code
- CI/CD workflows (unless build breaks)
- CSS/styling (unless required by type changes)

### Component Relationships

```
TiPTap Editor (TerraphimSuggestion, SlashCommand)
    ↓ depends on
@tiptap/core, @tiptap/suggestion (versions must align)
    ↓ depends on
prosemirror-* packages (transitive)

RoleGraphVisualization
    ↓ depends on
d3, @types/d3
    ↓ uses
stores.ts (is_tauri, role)

Config Components
    ↓ depends on
@tomic/svelte (peer dependency - install or stub)
```

### Complexity Areas
1. **TiPTap version alignment** - Core v3 vs extensions v2 mismatch
2. **D3 generic types** - Complex SVG element type constraints
3. **Test globals** - Vitest vs Jest type conflicts

---

## 4. File/Module-Level Change Plan

### Priority 1: Critical (Blocks Build)

| File | Action | Change Description | Dependencies |
|------|--------|-------------------|--------------|
| `package.json` | Modify | Downgrade @tiptap/core to v2.11.5 to match extensions | All TiPTap packages |
| `src/lib/Editor/TerraphimSuggestion.ts:173` | Modify | Fix `Partial<SuggestionOptions>` type assertion | TiPTap downgrade |
| `src/lib/Editor/SlashCommand.ts:150` | Modify | Fix `Partial<SuggestionOptions>` type assertion | TiPTap downgrade |

### Priority 2: High (Feature Breaking)

| File | Action | Change Description | Dependencies |
|------|--------|-------------------|--------------|
| `package.json` | Modify | Add @tomic/svelte to dependencies | Config components |
| `src/lib/RoleGraphVisualization.svelte:91,98` | Modify | Fix `_debugMessage` → `debugMessage` typo | None |
| `src/lib/RoleGraphVisualization.svelte:67` | Modify | Add missing Document properties to nodeToDocument return | None |
| `src/lib/RoleGraphVisualization.svelte:148,226` | Modify | Add type assertions for D3 zoom/drag behaviors | @types/d3 |

### Priority 3: Medium (Quality)

| File | Action | Change Description | Dependencies |
|------|--------|-------------------|--------------|
| `src/lib/ConfigJsonEditor.svelte:5` | Modify | Remove unused `@ts-expect-error` directive | None |
| `src/lib/ConfigJsonEditor.svelte:13` | Modify | Add explicit `any` type to updatedContent parameter | None |
| `src/lib/Search/ArticleModal.svelte:4` | Modify | Remove unused `@ts-expect-error` directive | None |
| `tsconfig.json` | Modify | Add test globals if needed | Test files |

### Priority 4: Low (Test Files)

| File | Action | Change Description | Dependencies |
|------|--------|-------------------|--------------|
| `src/lib/ThemeSwitcher.test.ts` | Modify | Import `vi` from vitest | Test config |
| `src/lib/BackButton.test.ts` | Modify | Import `vi` from vitest | Test config |
| `src/**/*.test.ts` | Modify | Fix implicit any on event handlers | Test config |

---

## 5. Step-by-Step Implementation Sequence

### Phase A: Dependency Alignment (P1 - Critical)
**Goal**: Fix TiPTap version mismatch

**Step A1**: Update package.json
```bash
cd desktop
# Edit package.json to align TiPTap versions
```
- Change `@tiptap/core` from `^3.15.3` to `^2.11.5`
- Verify all TiPTap packages use v2.x

**Step A2**: Reinstall dependencies
```bash
rm -rf node_modules yarn.lock
yarn install
```

**Step A3**: Verify TiPTap types resolve
```bash
yarn check 2>&1 | grep -i tiptap
# Should show fewer/no TiPTap errors
```

**Deployable?** Yes - dependencies only

---

### Phase B: Editor Type Fixes (P1 - Critical)
**Goal**: Fix TiPTap Suggestion type errors

**Step B1**: Fix TerraphimSuggestion.ts line 173
```typescript
// Change from:
return [Suggestion(suggestion)];
// To:
return [Suggestion(suggestion as SuggestionOptions)];
```

**Step B2**: Fix SlashCommand.ts line 150
```typescript
// Same change as above
return [Suggestion(suggestion as SuggestionOptions)];
```

**Step B3**: Verify editor errors resolved
```bash
yarn check 2>&1 | grep -E "TerraphimSuggestion|SlashCommand"
# Should show no errors
```

**Deployable?** Yes - type assertions only

---

### Phase C: Visualization Fixes (P2 - High)
**Goal**: Fix RoleGraphVisualization type issues

**Step C1**: Fix typo line 91
```svelte
<!-- Change from: -->
_debugMessage = `Right-clicked: ${nodeData.label}`;
<!-- To: -->
debugMessage = `Right-clicked: ${nodeData.label}`;
```

**Step C2**: Fix typo line 98
```svelte
<!-- Change from: -->
_debugMessage = '';
<!-- To: -->
debugMessage = '';
```

**Step C3**: Fix nodeToDocument return type (line 67)
```typescript
// Add missing properties to return object:
return {
  id: `kg-node-${node.id}`,
  // ... existing fields
  summarization: '', // Add this
  source_haystack: 'knowledge-graph', // Add this
};
```

**Step C4**: Fix D3 zoom behavior types (line 148)
```typescript
// Add type assertion:
svg.call(zoom as any);
```

**Step C5**: Fix D3 drag behavior types (line 226)
```typescript
// Add type assertion:
circle.call(drag as any);
```

**Step C6**: Verify
```bash
yarn check 2>&1 | grep RoleGraphVisualization
```

**Deployable?** Yes - type fixes only

---

### Phase D: Config & Modal Fixes (P3 - Medium)
**Goal**: Clean up unused directives and add types

**Step D1**: Remove unused @ts-expect-error (ArticleModal.svelte:4)
```svelte
<!-- Remove this line: -->
<!-- @ts-expect-error -->
```

**Step D2**: Remove unused @ts-expect-error (ConfigJsonEditor.svelte:5)
```svelte
<!-- Remove this line -->
```

**Step D3**: Add type to updatedContent (ConfigJsonEditor.svelte:13)
```typescript
// Change from:
function _handleChange(updatedContent) {
// To:
function _handleChange(updatedContent: any) {
```

**Deployable?** Yes - cleanup only

---

### Phase E: @tomic/svelte (P2 - High)
**Goal**: Resolve missing peer dependency

**Step E1**: Check if @tomic/svelte is actually used
```bash
grep -r "@tomic/svelte" src/ --include="*.svelte" --include="*.ts"
```

**Step E2**: If used, add to dependencies
```json
{
  "dependencies": {
    "@tomic/svelte": "^0.35.2"
  }
}
```

**Step E3**: If not used, create stub
Create `src/lib/stores/tomic-stub.ts`:
```typescript
// Stub for @tomic/svelte if not used
export {};
```

**Step E4**: Install
```bash
yarn install
```

**Deployable?** Yes - adds missing dependency

---

### Phase F: Test Configuration (P4 - Low)
**Goal**: Fix test file TypeScript errors

**Step F1**: Update tsconfig.json test globals
```json
{
  "compilerOptions": {
    "types": ["vitest/globals"]
  }
}
```

**Step F2**: Fix ThemeSwitcher.test.ts
```typescript
// Add import:
import { vi } from 'vitest';
```

**Step F3**: Fix BackButton.test.ts
```typescript
// Add import:
import { vi } from 'vitest';
```

**Step F4**: Verify
```bash
yarn check 2>&1 | grep -E "\.test\.ts"
```

**Deployable?** Yes - test-only changes

---

### Phase G: Final Verification
**Goal**: Ensure zero errors and build passes

**Step G1**: Full type check
```bash
yarn check
# Expect: 0 errors
```

**Step G2**: Build
```bash
yarn build
# Expect: Success, dist/ folder created
```

**Step G3**: Run tests
```bash
yarn test
# Expect: All pass
```

**Step G4**: Manual E2E test
- Editor autocomplete (++)
- Slash commands (/)
- Knowledge graph visualization
- Config wizard

**Deployable?** ✅ Yes - all criteria met

---

## 6. Testing & Verification Strategy

### Automated Tests
| Criterion | Test Type | Location | Command |
|-----------|-----------|----------|---------|
| Type check passes | Static | All files | `yarn check` |
| Build succeeds | Integration | Build pipeline | `yarn build` |
| Unit tests pass | Unit | *.test.ts | `yarn test` |
| No regressions | Regression | CI | GitHub Actions |

### Manual E2E Tests
| Feature | Test Steps | Expected Result |
|---------|------------|-----------------|
| Editor autocomplete | Type "++" in editor | Suggestion dropdown appears |
| Slash commands | Type "/" in editor | Command menu appears |
| KG Visualization | Navigate to graph page | D3 graph renders |
| Node interaction | Click/right-click nodes | Modal opens, no console errors |
| Config wizard | Open settings | Form loads without errors |

### Rollback Plan
If issues detected:
1. Revert specific commit using `git revert`
2. Rebuild and verify
3. Create issue for failed fix

---

## 7. Risk & Complexity Review

### Risks from Research
| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| TiPTap downgrade breaks features | Test editor thoroughly after Phase A | Low - v2 to v3 should be compatible |
| @tomic/svelte missing features | Test config wizard after Phase E | Low - stub available if needed |
| D3 type assertions hide bugs | Manual test all interactions | Low - assertions only on complex generics |
| Test fixes break test runner | Run full test suite after Phase F | Very Low - only adding imports |

### Complexity Assessment
| Area | Complexity | Reason |
|------|------------|--------|
| TiPTap versions | Medium | Version alignment across packages |
| Type assertions | Low | Simple `as Type` additions |
| Variable typo | Very Low | Single character change |
| Dependency install | Low | Standard yarn workflow |
| Test globals | Very Low | Configuration change |

**Overall Complexity**: Low-Medium
**Estimated Effort**: 2-3 hours
**Risk Level**: Low

---

## 8. Open Questions / Decisions for Human Review

### Questions
1. **TiPTap Strategy**: Confirm downgrade to v2.x is preferred over upgrading extensions to v3.x?
   - Research recommendation: Downgrade (safer)
   - Alternative: Upgrade (may have new features)

2. **@tomic/svelte**: Should we install it or stub it?
   - Need: Quick grep to verify usage
   - If used < 3 places: Install
   - If unused: Create stub

3. **D3 Types**: Are type assertions acceptable, or prefer proper generic fixing?
   - Current: Assertions (quick fix)
   - Alternative: Fix generic constraints (better long-term)

### Decisions Needed
- [ ] **DECISION 1**: Approve TiPTap downgrade approach
- [ ] **DECISION 2**: Approve @tomic/svelte handling (install vs stub)
- [ ] **DECISION 3**: Approve D3 type assertion approach

---

## 9. Implementation Checklist

### Before Starting
- [ ] Create feature branch: `fix/desktop-typescript-errors`
- [ ] Run initial `yarn check` to document baseline error count
- [ ] Run initial `yarn build` to confirm build fails

### Phase Execution
- [ ] Phase A: Dependency alignment
- [ ] Phase B: Editor type fixes
- [ ] Phase C: Visualization fixes
- [ ] Phase D: Config & modal fixes
- [ ] Phase E: @tomic/svelte resolution
- [ ] Phase F: Test configuration
- [ ] Phase G: Final verification

### Quality Gates
- [ ] After each phase: `yarn check` error count decreased
- [ ] After Phase B: Manual editor test
- [ ] After Phase C: Manual KG visualization test
- [ ] After Phase G: Full test suite passes
- [ ] Final: Build produces valid dist/

### Documentation
- [ ] Update CHANGELOG.md
- [ ] Update AGENTS.md if new patterns introduced

---

## Summary

This plan fixes 50+ TypeScript errors through systematic dependency alignment, type assertions for complex generics, variable name fixes, and test configuration updates. The approach prioritizes critical path (TiPTap) first, then moves through high-priority fixes to final verification.

**Estimated Timeline**: 2-3 hours
**Risk Level**: Low
**Success Criteria**: Zero errors, successful build, all tests pass
