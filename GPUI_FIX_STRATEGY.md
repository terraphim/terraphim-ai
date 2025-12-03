# GPUI Fix Strategy Using Component Documentation

## Key API Patterns from gpui-component Documentation

### 1. Conditional Rendering
```rust
// CORRECT - Using .when() with FluentBuilder
use gpui::prelude::FluentBuilder;

div().when(condition, |this| {
    this.child(content)
})

// CORRECT - Using .children() with Option
.children(if condition { Some(element) } else { None })

// INCORRECT - Don't use .child(Option<...>)
.child(if condition { Some(element) } else { None }) // WRONG
```

### 2. Input Component Pattern
```rust
// CORRECT - InputState creation
let state = cx.new(|cx| InputState::new(window, cx));

// Use with Input component
Input::new(&state)
    .placeholder("Search...")
    .on_text_change(cx.listener(|this, text, window, cx| {
        this.query = text.to_string();
    }))
```

### 3. Button Styling
```rust
Button::new("btn")
    .label("Search")
    .primary()
    .when(loading, |this| this.disabled())
    .on_click(cx.listener(|this, event, window, cx| {
        // Handle click
    }))
```

### 4. Modal/Dialog
```rust
// Open article modal
window.open_dialog(cx, |dialog, _, cx| {
    dialog
        .title(document.title)
        .child(render_document_content(document))
        .on_close(cx.listener(|this, _, window, cx| {
            // Handle close
        }))
})
```

## Critical Files to Fix (Priority Order)

### Priority 1: Core Search Workflow (Must Fix)

#### 1. `src/views/search/input.rs`
**Issue**: Missing `FluentBuilder` import, invalid conditional patterns
**Fix**:
- Add `use gpui::prelude::FluentBuilder;`
- Replace `.child(Option<...>)` with `.when()` or `.children()`
- Add proper InputState usage

#### 2. `src/views/search/results.rs`  
**Issue**: "Add to Context" button rendering, result list styling
**Fix**:
- Use Button::new() with proper styling
- Use .when() for conditional states
- Proper list rendering with .children()

#### 3. `src/views/search/autocomplete.rs`
**Issue**: Autocomplete dropdown rendering, keyboard navigation
**Fix**:
- Use FluentBuilder patterns
- Proper conditional rendering
- Event handling for selection

#### 4. `src/views/search/mod.rs`
**Issue**: Main view composition
**Fix**: 
- Ensure all sub-components compile
- Proper layout with div() and gap/padding methods

### Priority 2: Term Chips & KG Indicators (Should Fix)

#### 5. `src/models.rs` or new `term_chip.rs`
**Feature**: Port term chip UI from desktop
```rust
// Desktop has:
// - Term chips with KG indicator (📚)
// - AND/OR operator visualization
// - Click to remove functionality
//
// Need: GPUI equivalent with proper styling
```

#### 6. `src/views/search/results.rs` enhancements
- Add KG icon (📚) next to KG terms in results
- Show term chips from parsed query
- Visual feedback for selections

### Priority 3: Context Management (Verify Working)

#### 7. Verify `AddToContextEvent` flow
- Already implemented in app.rs
- Should work once search results compile
- Test: Click "Add to Context" → navigates to Chat → appears in context

## Files That Can Be Skipped for Demo

### Optimization Components (Not Critical)
- `advanced_virtualization.rs` - Advanced performance feature
- `memory_optimizer.rs` - Memory management
- `render_optimizer.rs` - Render performance
- `performance_*.rs` - Benchmarking code

### Advanced Features (Nice-to-Have)
- `kg_search_modal.rs` - Separate KG search modal
- `knowledge_graph.rs` - Complex KG visualization
- `search_performance.rs` - Performance metrics

## Quick Verification Checklist

After fixing Priority 1 files:
```bash
# Should compile successfully
cargo check -p terraphim_desktop_gpui --lib

# Run the app
cargo run --bin terraphim-gpui
```

Expected behavior:
1. ✅ App launches without crashes
2. ✅ Role selector shows dropdown with icons
3. ✅ Can type in search input
4. ✅ Autocomplete dropdown appears with suggestions
5. ✅ Search executes and shows results
6. ✅ "Add to Context" button appears on results
7. ✅ Clicking "Add to Context" navigates to Chat
8. ✅ Document appears in chat context
9. ✅ Can chat with context

## Implementation Order for Maximum Impact

1. **30 minutes**: Fix `FluentBuilder` imports in all search view files
2. **1 hour**: Fix conditional rendering patterns (.when vs .child)
3. **1 hour**: Fix Input/Button component usage
4. **30 minutes**: Verify compilation
5. **1 hour**: Add term chips UI (port from desktop)
6. **30 minutes**: Test complete workflow
7. **30 minutes**: Polish and bug fixes

**Total**: ~4 hours for working demo
