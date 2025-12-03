# Focused Implementation Plan for Quick Demo

## Current Status (As of Dec 2)

### ✅ What's Working
- **Architecture**: Solid backend integration, event system, state management
- **Role Selector**: Fully implemented with dropdown and icons
- **Navigation**: App tabs (Search/Chat/Editor) working
- **Search State**: Backend search with TerraphimService integrated
- **Context Manager**: Full CRUD operations ready
- **Chat Integration**: Ready to receive context
- **System Tray**: Working with role switching

### ❌ Blockers for Demo
- **src/components/search.rs**: 30+ GPUI API errors (non-existent methods)
- **src/views/search/*.rs**: Various GPUI syntax errors
- **Term Chips UI**: Not implemented in GPUI yet

## Realistic 4-Hour Plan

### Hour 1: Fix Main Search Component (src/components/search.rs)
**Approach**: Comment out or simplify problematic code
- Remove `.test_id()` calls
- Remove `.with_alpha()`, `.opacity_70()`, `.overflow_y_scroll()`
- Remove `.font_medium()`, `.text_white()`
- Replace `transparent()` with `rgb(0xffffff)` or similar
- Fix `&String` references (clone them)
- Use `.when()` for conditionals (already imported)
- Comment out advanced features if needed

**Goal**: Get search.rs to compile, even if some features disabled

### Hour 2: Fix Search View Files
**Priority**: Fix only critical errors
- src/views/search/input.rs - Fix event handler signatures
- src/views/search/results.rs - Fix button rendering
- src/views/search/autocomplete.rs - Fix dropdown

**Goal**: Search view compiles and basic functionality works

### Hour 3: Basic Term Chips Implementation
**Minimum Viable Feature**:
- Port term chip logic from desktop (Search.svelte lines 696-729)
- Show chips for parsed terms (AND/OR queries)
- KG indicator (📚) for terms from knowledge graph
- Don't need all fancy styling, just basic display

**Goal**: Visual feedback that query parsing works

### Hour 4: Integration & Testing
**Workflow Testing**:
1. Launch app
2. Select role
3. Type search query
4. See autocomplete suggestions
5. Execute search
6. See results
7. Click "Add to Context" on result
8. Navigate to Chat (automatic)
9. Verify document in context
10. Send chat message with context

**Goal**: Complete Search → Add to Context → Chat journey works

## What to Cut for Demo (Temporarily)
- Advanced styling and animations
- Complex KG visualizations
- Performance optimizations
- Full error handling
- Edge cases

## What Must Work
- ✅ Role selection
- ✅ Search input with autocomplete
- ✅ Search execution with real backend
- ✅ Results display with "Add to Context"
- ✅ Context addition working
- ✅ Chat context integration
- ✅ Navigation between views

## Success Criteria
1. **Application launches** without panics
2. **Search workflow** completes end-to-end
3. **Context management** works reliably
4. **Role switching** affects search results
5. **UI is functional** even if not pixel-perfect

## Risk Mitigation
- If Hour 1 takes too long, comment out more features
- If term chips are complex, skip to Hour 4 focus on core workflow
- Can ship demo without advanced features
- Focus on "working" over "perfect"

## Files to Focus On
1. src/components/search.rs (200 lines to fix)
2. src/views/search/input.rs (50 lines to fix)
3. src/views/search/results.rs (100 lines to fix)
4. src/views/search/autocomplete.rs (80 lines to fix)

Total: ~430 lines of focused fixes

## Files to IGNORE for Demo
- All *_optimizer.rs (not needed)
- advanced_virtualization.rs (not needed)
- kg_search_modal.rs (not needed)
- performance_*.rs (not needed)
- knowledge_graph.rs advanced features (not needed)

These can be fixed after demo is working
