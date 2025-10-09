# Linting Fixes Plan

## Summary

**Rust Linting Status**: ✅ PASSED
- `cargo fmt --check`: No formatting issues
- `cargo clippy --workspace --all-targets --all-features`: No clippy errors
- Only minor dependency warnings (future incompatibility in `opendal` and `redis`)

**Frontend Linting Status**: ❌ FAILED
- `svelte-check`: 17 errors, 3 warnings
- Multiple type errors, module resolution issues, and accessibility warnings

---

## Rust Issues (Low Priority)

### Issue 1: Future Incompatibility Warnings
**Files**: Dependencies `opendal v0.44.2`, `redis v0.23.3`
**Severity**: Low (future-facing, not blocking)
**Description**: These dependencies contain code that will be rejected by future Rust versions.

**Fix Strategy**:
1. Update `opendal` to latest version (check for v0.45+)
2. Update `redis` to latest version (check for v0.24+)
3. Run `cargo update` and test compatibility
4. Consider using `redistack` library per project guidelines

---

## Frontend Issues (High Priority)

### Issue 1: Missing Type Definitions in Generated Types ⚠️ CRITICAL
**File**: `desktop/src/lib/generated/types.ts`
**Lines**: 49, 62
**Severity**: Critical - Breaks type system

**Errors**:
```
Line 49: Cannot find name 'AHashMap'
Line 49: Cannot find name 'Value'
Line 62: Cannot find name 'AHashMap'
```

**Root Cause**: The generated TypeScript types reference Rust types (`AHashMap`, `Value`) that don't have TypeScript equivalents defined.

**Fix Strategy**:
1. **Option A**: Update the Rust type generation (tsify) to properly export these types
   - Check `crates/terraphim_types/src/lib.rs` or wherever types are generated
   - Ensure `AHashMap` is mapped to TypeScript `Record<K, V>` or `Map<K, V>`
   - Ensure `Value` type is properly exported
   
2. **Option B**: Add missing type definitions manually to `types.ts`:
   ```typescript
   // Add at top of file:
   export type Value = string | number | boolean | null | Value[] | { [key: string]: Value };
   export type AHashMap<K extends string | number, V> = Record<K, V>;
   ```

3. **Option C**: Modify generated types to use standard TypeScript types:
   ```typescript
   // Line 49: Change from
   export interface Role extends AHashMap<string, Value> {
   // To:
   export interface Role {
   ```

**Recommended Approach**: Option B (quick fix) + Option A (long-term fix)

---

### Issue 2: Module Import Errors in FetchTabs.svelte
**File**: `desktop/src/lib/Fetchers/FetchTabs.svelte`
**Lines**: 9, 64
**Severity**: High

**Errors**:
```
Line 9: Cannot find module '$lib/stores' or its corresponding type declarations
Line 64: Cannot find module '$workers/postmessage.ts' or its corresponding type declarations
```

**Root Cause**: 
1. TypeScript path aliases (`$lib`, `$workers`) not properly configured in `tsconfig.json`
2. Files may be missing or misnamed

**Fix Strategy**:
1. Check `desktop/tsconfig.json` for path mappings:
   ```json
   {
     "compilerOptions": {
       "paths": {
         "$lib/*": ["src/lib/*"],
         "$workers/*": ["src/workers/*"]
       }
     }
   }
   ```
2. Verify `src/workers/postmessage.ts` exists, or update import path
3. Check if `svelte.config.js` has proper alias configuration

---

### Issue 3: @tomic/lib Agent Type Incompatibility
**File**: `desktop/src/lib/Fetchers/FetchTabs.svelte`
**Line**: 52
**Severity**: Medium

**Error**: Different versions of `@tomic/lib` in dependencies causing type conflicts

**Root Cause**: `@tomic/svelte` has its own bundled version of `@tomic/lib` that differs from the direct dependency.

**Fix Strategy**:
1. Check `package.json` versions:
   - `@tomic/lib`: ^0.40.0
   - `@tomic/svelte`: ^0.35.2
2. Try to align versions or use only one source
3. Alternative: Use `$store.setAgent(agent as any)` with type assertion (not ideal)
4. Better: Update both packages to latest compatible versions

---

### Issue 4: Route Component Type Errors (Svelte Routing)
**File**: `desktop/src/lib/Fetchers/FetchTabs.svelte`
**Lines**: 110, 135, 161
**Severity**: Medium

**Error**: `svelte-routing` Route component not compatible with type checking

**Root Cause**: Project uses Svelte 3 without proper TypeScript definitions for `svelte-routing`

**Fix Strategy**:
1. Add type augmentation file `desktop/src/types/svelte-routing.d.ts`:
   ```typescript
   declare module 'svelte-routing' {
     import type { SvelteComponentTyped } from 'svelte';
     
     export class Route extends SvelteComponentTyped<{
       path?: string;
       component?: any;
     }> {}
     
     export class Router extends SvelteComponentTyped<{}> {}
     export class Link extends SvelteComponentTyped<{ to: string }> {}
   }
   ```
2. Alternatively: Consider migrating to `tinro` (already in dependencies) which has better TypeScript support

---

### Issue 5: ThemeSwitcher Role Import and Type Errors
**File**: `desktop/src/lib/ThemeSwitcher.svelte`
**Lines**: 5, 148, 160
**Severity**: High

**Errors**:
```
Line 5: '"./stores"' has no exported member named 'Role'
Line 148: This comparison appears to be unintentional because the types 'RoleName' and 'string' have no overlap
Line 160: Type 'string' is not assignable to type 'RoleName'
```

**Root Cause**: 
1. Trying to import `Role` type from stores, but stores exports it from generated types
2. Mixing `string` and `RoleName` types

**Fix Strategy**:
1. Fix import on line 5:
   ```typescript
   // Change from:
   import { ..., type Role as RoleInterface } from "./stores";
   // To:
   import type { Role as RoleInterface } from "./generated/types";
   ```

2. Fix type handling:
   ```typescript
   // Line 148-160: Instead of comparing string to RoleName
   const roleSettings = $roles.find((r) => r.name.lowercase === newRoleName.toLowerCase());
   
   // Or convert string to RoleName:
   const roleNameObj: RoleName = {
     original: newRoleName,
     lowercase: newRoleName.toLowerCase()
   };
   cfg.selected_role = roleNameObj;
   ```

---

### Issue 6: DOM Type Errors in ResultItem.svelte
**File**: `desktop/src/lib/Search/ResultItem.svelte`
**Lines**: 442, 445, 447, 458
**Severity**: Medium

**Errors**:
```
Line 442: Property 'createElement' does not exist on type 'Document'
Line 445: Property 'appendChild' does not exist on type 'string'
Line 447: Property 'removeChild' does not exist on type 'string'
```

**Root Cause**: TypeScript not recognizing DOM types properly - likely `document` shadowed by a local variable or type

**Fix Strategy**:
1. Check for variable shadowing:
   ```typescript
   // Look for: let document = ...
   // Rename to: let documentData = ...
   ```
2. Add explicit DOM lib reference in `tsconfig.json`:
   ```json
   {
     "compilerOptions": {
       "lib": ["ES2020", "DOM", "DOM.Iterable"]
     }
   }
   ```
3. Use explicit window reference:
   ```typescript
   const a = window.document.createElement('a');
   window.document.body.appendChild(a);
   ```

---

### Issue 7: NovelWrapper Import Error in ArticleModal
**File**: `desktop/src/lib/Search/ArticleModal.svelte`
**Line**: 3
**Severity**: Medium

**Error**: `Cannot find module '$lib/Editor/NovelWrapper.svelte'`

**Fix Strategy**:
1. Check if file exists: `desktop/src/lib/Editor/NovelWrapper.svelte`
2. If missing, create wrapper component for `@paralect/novel-svelte`
3. If exists but path wrong, update import path
4. If not needed, remove import

---

### Issue 8: Accessibility Warnings (A11y)
**Files**: 
- `desktop/src/lib/Search/ArticleModal.svelte:320`
- `desktop/src/lib/Search/AtomicSaveModal.svelte:281, 366`
**Severity**: Low (warnings, not errors)

**Warnings**:
1. Line 320: Clickable div needs keyboard handler
2. Lines 281, 366: Form labels not associated with controls

**Fix Strategy**:

**For ArticleModal.svelte:320**:
```svelte
<!-- Add keyboard event handler -->
<div
  class="content-viewer"
  on:click={handleContentClick}
  on:keydown={(e) => e.key === 'Enter' && handleContentClick(e)}
  tabindex="0"
  role="button"
  aria-label="KG document content - click KG links to explore further"
>
```

**For AtomicSaveModal.svelte labels**:
```svelte
<!-- Option 1: Add for attribute -->
<label class="label" for="document-preview">Document to Save</label>
<div id="document-preview" class="box document-preview">

<!-- Option 2: Wrap input in label -->
<label class="label">
  Parent Collection
  <div class="control">
    <!-- input here -->
  </div>
</label>
```

---

### Issue 9: Package.json License Field Warning
**Files**: Root `package.json`, `desktop/package.json`
**Severity**: Low

**Warning**: `No license field`

**Fix Strategy**:
Add license field matching LICENSE files (Apache-2.0 OR MIT):
```json
{
  "license": "Apache-2.0 OR MIT"
}
```

---

### Issue 10: Sass Deprecation Warnings
**Severity**: Low (future-facing)

**Warning**: Legacy JS API deprecated in Sass

**Fix Strategy**:
Update `sass` dependency when Dart Sass 2.0 stable is available. Current version (1.92.1) is modern, just using legacy API through build tools.

---

## Implementation Priority

### Phase 1: Critical Type Fixes (Required for build)
1. ✅ Fix missing type definitions (`AHashMap`, `Value`) - **Issue 1**
2. Fix ThemeSwitcher type errors - **Issue 5**
3. Fix module import errors - **Issue 2**

### Phase 2: Component Fixes (Required for functionality)
4. Fix NovelWrapper import - **Issue 7**
5. Fix DOM type errors in ResultItem - **Issue 6**
6. Fix Route component types - **Issue 4**

### Phase 3: Dependency & Compatibility
7. Fix @tomic Agent incompatibility - **Issue 3**
8. Update Rust dependencies - **Rust Issue 1**

### Phase 4: Quality & Standards
9. Fix accessibility warnings - **Issue 8**
10. Add license fields - **Issue 9**

---

## Testing Strategy

After each phase:
1. Run `yarn run check` in desktop directory
2. Run `cargo clippy --workspace` in root
3. Test affected components in browser
4. Run relevant e2e tests

---

## Commands Reference

```bash
# Rust linting
cargo fmt --check
cargo clippy --workspace --all-targets --all-features

# Frontend linting
cd desktop && yarn run check

# Run tests after fixes
cd desktop && yarn test
cargo test --workspace
```

---

## Notes

- Most issues are TypeScript configuration and type generation related
- Core functionality likely works at runtime despite type errors
- Generated types file is the root cause of cascade failures
- Consider regenerating TypeScript bindings from Rust after fixing type definitions

