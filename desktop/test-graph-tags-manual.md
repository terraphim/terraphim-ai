# Manual Test: Graph Tags in Tauri App

## Test Objective
Verify that graph tags are clickable and functional in the Tauri app after fixing parameter naming issues.

## Prerequisites
- Tauri app running on http://localhost:5173
- Terraphim Engineer role configured with knowledge graph
- Some documents with graph tags available

## Test Steps

### 1. Start Tauri App
```bash
cd desktop
yarn run tauri dev
```

### 2. Navigate to App
- Open browser to http://localhost:5173
- Wait for app to load completely

### 3. Test Graph Tags in Search Results
1. Enter a search term that should return results with graph tags (e.g., "service haystack")
2. Look for graph tags in search results (they appear as clickable links)
3. Click on a graph tag
4. **Expected Result**: Should open a modal with knowledge graph document details
5. Check browser console for any error messages

### 4. Test KG Links in Document Content
1. Open a document that contains KG links in the content
2. Look for KG links (they appear as `[term](kg:term)` format)
3. Click on a KG link
4. **Expected Result**: Should open a modal with knowledge graph document details
5. Check browser console for any error messages

### 5. Test Role Switching
1. Switch between different roles using the theme switcher
2. **Expected Result**: Role switching should work without errors
3. Check browser console for any error messages

## Expected Console Output (Success)
```
🔍 Clicking on graph tag: "haystack"
  Tauri params: { role_name: "Terraphim Engineer", term: "haystack" }
  📥 Tauri response received:
    Status: success
    Results count: 1
    Total: 1
  ✅ Found KG document:
    Title: Haystack Service
    Rank: 117
    Body length: 2048 characters
```

## Error Indicators (If Still Broken)
```
❌ Error fetching KG document:
  Error type: String
  Error message: "invalid args `roleName` for command `find_documents_for_kg_term`"
```

## Success Criteria
- ✅ Graph tags are clickable in search results
- ✅ KG links are clickable in document content
- ✅ Clicking opens modal with document details
- ✅ No console errors related to parameter naming
- ✅ Role switching works without errors

## Notes
- The fix involved changing `roleName` to `role_name` in all Tauri command calls
- Multiple commands were affected: `find_documents_for_kg_term`, `publish_thesaurus`, `select_role`, `get_rolegraph`
- Dependency issues in `terraphim_onepassword_cli` were also resolved
