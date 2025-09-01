# Comprehensive Test: Tauri Parameter Naming Fixes

## Test Objective
Verify that all Tauri command parameter naming issues have been resolved.

## Fixed Commands and Parameters

### 1. `find_documents_for_kg_term`
- **Fixed**: `roleName` → `role_name`
- **Files**: `ResultItem.svelte`, `ArticleModal.svelte`
- **Test**: Click on graph tags in search results

### 2. `publish_thesaurus`
- **Fixed**: `roleName` → `role_name`
- **Files**: `ThemeSwitcher.svelte`
- **Test**: Switch roles using theme switcher

### 3. `select_role`
- **Fixed**: `roleName` → `role_name`
- **Files**: `ThemeSwitcher.svelte`
- **Test**: Switch roles using theme switcher

### 4. `get_rolegraph`
- **Fixed**: `roleName` → `role_name`
- **Files**: `RoleGraphVisualization.svelte`
- **Test**: View role graph visualization

### 5. `get_document`
- **Fixed**: `documentId` → `document_id`
- **Files**: `ResultItem.svelte`
- **Test**: Open document modals

## Test Steps

### 1. Start Tauri App
```bash
cd desktop
yarn run tauri dev
```

### 2. Test Graph Tags (find_documents_for_kg_term)
1. Navigate to http://localhost:5173
2. Search for "service haystack knowledge"
3. Look for graph tags in results
4. Click on a graph tag
5. **Expected**: Modal opens with KG document details
6. **Check Console**: Should see `role_name` parameter, not `roleName`

### 3. Test Role Switching (select_role, publish_thesaurus)
1. Use the theme switcher to change roles
2. **Expected**: Role changes without errors
3. **Check Console**: Should see `role_name` parameters, not `roleName`

### 4. Test Document Opening (get_document)
1. Click on a document to open it
2. **Expected**: Document modal opens without errors
3. **Check Console**: Should see `document_id` parameter, not `documentId`

### 5. Test Role Graph (get_rolegraph)
1. Navigate to role graph visualization
2. **Expected**: Graph loads without errors
3. **Check Console**: Should see `role_name` parameter, not `roleName`

## Expected Console Output (Success)
```
🔍 Clicking on graph tag: "haystack"
  Tauri params: { role_name: "Terraphim Engineer", term: "haystack" }
  📥 Tauri response received:
    Status: success
    Results count: 1
    Total: 1
```

## Error Indicators (If Still Broken)
```
❌ Error fetching KG document:
  Error message: "invalid args `roleName` for command `find_documents_for_kg_term`"

❌ Error selecting role:
  Error message: "invalid args `roleName` for command `select_role`"

❌ Failed to load document:
  Error message: "invalid args `documentId` for command `get_document`"
```

## Success Criteria
- ✅ No parameter naming errors in console
- ✅ Graph tags are clickable and functional
- ✅ Role switching works without errors
- ✅ Document modals open correctly
- ✅ Role graph visualization loads
- ✅ All Tauri commands use snake_case parameters

## Notes
- All parameter names must match exactly between frontend and backend
- Tauri commands use snake_case in Rust backend
- Frontend TypeScript must use snake_case for Tauri command parameters
- Caching issues can cause old code to persist - clear caches if needed
