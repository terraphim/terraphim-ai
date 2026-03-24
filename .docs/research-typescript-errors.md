# Research Document: Fix Desktop Frontend TypeScript Errors

**Status**: Draft
**Author**: Claude Code Agent
**Date**: 2026-03-24

## Executive Summary

The desktop frontend at `/Users/alex/projects/terraphim/terraphim-ai/desktop` has approximately 50+ TypeScript errors preventing successful builds. These errors fall into 5 main categories: TiPTap editor type mismatches, missing dependencies, D3 visualization type issues, test configuration problems, and general type safety violations. The frontend builds to `desktop/dist/` and is embedded in the terraphim_server binary, making these errors blocking for deployment.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Blocking CI/CD and deployment; affects core user experience |
| Leverages strengths? | Yes | Frontend TypeScript expertise; existing test infrastructure |
| Meets real need? | Yes | Cannot build production assets with TypeScript errors |

**Proceed**: Yes - All 3 questions answered YES

## Problem Statement

### Description
The desktop frontend Svelte/TypeScript codebase has accumulated type errors that prevent `svelte-check` and the TypeScript compiler from producing valid output. This blocks the build process that creates `desktop/dist/` assets for embedding in terraphim_server.

### Impact
- **CI/CD**: Cannot complete automated builds
- **Developer Experience**: Red squiggles everywhere reduce productivity
- **Deployment**: Cannot ship terraphim_server with embedded UI
- **Quality**: Type errors may mask runtime bugs

### Success Criteria
1. `yarn check` (svelte-check) passes with zero errors
2. `yarn build` produces valid assets in `desktop/dist/`
3. All existing functionality preserved
4. Tests continue to pass

## Current State Analysis

### Error Categories (50+ total)

| Category | Count | Files Affected | Severity |
|----------|-------|----------------|----------|
| TiPTap Type Mismatches | 7+ | TerraphimSuggestion.ts, SlashCommand.ts | High |
| Missing Dependencies | 3+ | Multiple files | High |
| D3 Visualization Types | 4+ | RoleGraphVisualization.svelte | Medium |
| Test Configuration | 10+ | *.test.ts files | Medium |
| Implicit Any Types | 15+ | Various | Low |
| Variable Name Issues | 3+ | RoleGraphVisualization.svelte | Low |
| Unused Directives | 2+ | ArticleModal.svelte, ConfigJsonEditor.svelte | Low |

### Existing Implementation

#### TiPTap Editor Integration
**Location**: `desktop/src/lib/Editor/TerraphimSuggestion.ts`, `SlashCommand.ts`

**Current Pattern**:
```typescript
const suggestion: Partial<SuggestionOptions> = {
  editor: this.editor,  // Error: Type 'Editor | undefined' not assignable
  // ...
};
return [Suggestion(suggestion)];  // Error: Partial<SuggestionOptions> not assignable
```

**Root Cause**: TiPTap v2's `Suggestion` function expects `SuggestionOptions` but receives `Partial<SuggestionOptions>`. The `editor` property is optional in `Partial` but required in `SuggestionOptions`.

#### Missing @tomic/svelte
**Location**: `desktop/src/lib/config/*.svelte` files

**Issue**: Package is listed as peerDependency but not installed. The package provides Svelte components for Atomic Data integration.

#### RoleGraphVisualization Type Issues
**Location**: `desktop/src/lib/RoleGraphVisualization.svelte`

**Issues**:
1. Line 91, 98: Uses `_debugMessage` but variable is declared as `debugMessage` (typo)
2. Line 67: `nodeToDocument` returns partial Document type missing `summarization`, `source_haystack`
3. D3 zoom/drag behaviors have type incompatibilities with SVG element selections

#### Test Files
**Location**: `desktop/src/**/*.test.ts`

**Issues**:
- Cannot find name 'vi' (Vitest global not imported)
- Cannot use namespace 'jest' as a value (Jest types conflicting)
- Various implicit `any` types on event handlers

### Data Flow
1. TypeScript source files compiled via Vite + Svelte
2. Output written to `desktop/dist/`
3. terraphim_server embeds dist as static assets
4. Server serves UI at runtime

### Integration Points
- **TiPTap**: Rich text editor with autocomplete
- **D3**: Knowledge graph visualization
- **@tomic/svelte**: Atomic Data integration (optional peer dependency)
- **Tauri**: Native API bindings (though Tauri code was removed, types remain)

## Constraints

### Technical Constraints
1. **Svelte 5**: Using runes mode ($props, $state) - cannot revert to Svelte 4
2. **TypeScript 5**: Must maintain strict mode compliance
3. **TiPTap v2**: Core at v3.15.3 but extensions at v2.x - version mismatch
4. **Build Tool**: Vite 5.x with specific plugin ecosystem

### Business Constraints
1. **No Runtime Changes**: Must preserve all existing functionality
2. **No Major Refactors**: Fix types only, don't redesign
3. **Test Coverage**: Must not reduce existing test coverage

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Build Time | < 2 min | Unknown (fails before completion) |
| Type Errors | 0 | 50+ |
| Bundle Size | < 5MB | Unknown |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| TiPTap type compatibility | Blocks editor functionality | 7 errors in core editor files |
| @tomic/svelte availability | Required for config UI | 3+ files cannot compile without it |
| Variable name fixes | Runtime bugs | `_debugMessage` typo will crash on right-click |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Upgrade TiPTap to v3 | Major version upgrade out of scope; focus on type fixes |
| Migrate to Svelte 5 runes everywhere | Only fix errors, don't migrate working Svelte 4 syntax |
| Add comprehensive type coverage | Fix existing errors only, don't add types to untyped code |
| Fix all deprecation warnings | Warnings don't block build; focus on errors only |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_server | Consumes dist/ output | High - cannot embed broken build |
| crates/* | Rust backend APIs | Medium - types must match API |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| @tiptap/core | 3.15.3 | High | Downgrade to 2.x or upgrade all to 3.x |
| @tiptap/suggestion | 2.22.1 | High | Must match core version |
| @tiptap/starter-kit | 3.17.1 | High | Must match core version |
| @tomic/svelte | 0.35.2 (peer) | High | Install or stub |
| d3 | 7.9.0 | Medium | Type fixes or @types/d3 updates |
| @types/node | 25.0.9 | Low | Update to match Node version |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| TiPTap version mismatch deeper than types | High | High | Align all TiPTap packages to same major version |
| @tomic/svelte installation breaks other deps | Medium | Medium | Install as optional, provide stub fallback |
| D3 type fixes require runtime changes | Low | Medium | Use type assertions where necessary |
| Fixing types reveals runtime bugs | Medium | High | Thorough testing after fixes |

### Open Questions
1. **TiPTap versions**: Should we downgrade core to v2.x or upgrade extensions to v3.x? - Check CHANGELOG for breaking changes
2. **@tomic/svelte**: Is Atomic Data integration actually used, or can we stub it? - Check if config UI uses it
3. **Test environment**: Should tests use Vitest or Jest globals? - Check vite.config.ts test setup

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| TiPTap v2 and v3 have compatible APIs | Version numbers suggest minor differences | Editor features broken | No |
| @tomic/svelte is optional | Marked optional in peerDependencies | Config UI crashes | No |
| D3 types can be fixed with assertions | Complex generic types | Visualization broken | No |
| Tests use Vitest | vitest in devDependencies | Test suite broken | Partial |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Upgrade all TiPTap to v3 | Cleanest solution, may have breaking API changes | Too risky without testing |
| Downgrade TiPTap core to v2 | Aligns versions, likely safest | **Chosen** - minimal change |
| Use type assertions/casts | Quick fix, hides real issues | Rejected - prefer proper types |
| Add @ts-ignore comments | Fastest, creates technical debt | Rejected - proper fixes preferred |

## Research Findings

### Key Insights

1. **TiPTap Version Mismatch**: Core is at v3.15.3 but suggestion/starter-kit are at v2.x. This is the root cause of type errors.

2. **@tomic/svelte Missing**: Listed as peerDependency but never installed. Either install it or the feature using it should be stubbed.

3. **Typo in RoleGraphVisualization**: `_debugMessage` is written with underscore prefix but variable is `debugMessage`.

4. **Test Globals Mismatch**: Some files use Vitest globals (`vi`) others reference Jest. Need consistent test setup.

5. **TypeScript Strictness**: tsconfig.json doesn't explicitly enable strict mode, but svelte-check applies strict checks.

### Relevant Prior Art
- TiPTap migration guide: https://tiptap.dev/guide/migrate
- Svelte 5 runes documentation: https://svelte.dev/docs/svelte/what-are-runes
- @tomic/svelte README: https://github.com/atomicdata-dev/atomic-svelte

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| TiPTap v2 vs v3 API diff | Determine if downgrade or upgrade is safer | 30 min |
| @tomic/svelte usage audit | Find if it's actually used in UI | 15 min |
| Test runner configuration | Verify Vitest setup and globals | 15 min |

## Recommendations

### Proceed/No-Proceed
**Proceed** - The errors are fixable with focused effort. No architectural changes needed.

### Scope Recommendations
1. Fix TiPTap versions first (highest impact)
2. Install missing @tomic/svelte or stub it
3. Fix typos and variable names
4. Address test configuration
5. Fix remaining implicit any types

### Risk Mitigation Recommendations
1. Create feature branch for all changes
2. Run full test suite after each category of fixes
3. Test editor functionality manually after TiPTap fixes
4. Test knowledge graph visualization after D3 fixes

## Next Steps

If approved:
1. Create feature branch: `fix/desktop-typescript-errors`
2. Phase 2: Detailed implementation plan with file-by-file changes
3. Execute fixes in dependency order (TiPTap → @tomic → others)
4. Run quality gates after each fix batch
5. Request human review before merge

## Appendix

### Reference Materials
- TiPTap Documentation: https://tiptap.dev/
- Svelte Check Documentation: https://github.com/sveltejs/language-tools
- Vite Config: `desktop/vite.config.ts`

### Code Snippets

#### TerraphimSuggestion.ts Error Location (line 173)
```typescript
return [Suggestion(suggestion)];
//      ^ Error: Partial<SuggestionOptions> not assignable
```

#### SlashCommand.ts Error Location (line 150)
```typescript
return [Suggestion(suggestion)];
//      ^ Same error as above
```

#### RoleGraphVisualization.svelte Typo (lines 91, 98)
```svelte
<script>
  let debugMessage = $state('');  // Declared as debugMessage
  // ...
  _debugMessage = `Right-clicked: ${nodeData.label}`;  // Used as _debugMessage
</script>
```

### Error Summary by File
| File | Errors | Priority |
|------|--------|----------|
| TerraphimSuggestion.ts | 2 | Critical |
| SlashCommand.ts | 2 | Critical |
| RoleGraphVisualization.svelte | 5 | High |
| ConfigJsonEditor.svelte | 2 | Medium |
| ArticleModal.svelte | 1 | Low |
| ThemeSwitcher.test.ts | 3 | Medium |
| Various *.test.ts | 10+ | Medium |
| Other files | 25+ | Low |
