# Quick Fixes for GPUI 0.2.2 Compatibility

## Common API Issues Found

### 1. Non-existent Methods (Remove or Replace)
- `.test_id()` - Doesn't exist on Stateful<E>
- `.with_alpha()` - Doesn't exist on Hsla
- `.opacity_70()` - Doesn't exist
- `.overflow_y_scroll()` - Doesn't exist
- `.key()` - Doesn't exist on Div
- `.font_medium()` - Doesn't exist
- `button()` function - Use Button::new()
- `transparent()` - Not in scope
- `.text_white()` - Doesn't exist

### 2. Type Mismatches
- `&String` doesn't implement IntoElement - Use `.clone()` or pass by value
- `Div is not an iterator` - Don't use .iter() on divs
- Closure takes 4 args, but takes 3 - Add |_window| parameter

### 3. Field/Method Errors in Autocomplete
- `AutocompleteSuggestion` has no field `source`
- `AutocompleteSuggestion` has no field `context`
- `Arc<AutocompleteEngine>` has no method `suggest`
- `Thesaurus` has no method `fuzzy_search`
- `Thesaurus` has no method `find_term`

---

## Quick Fixes for Critical Files

### File: src/components/search.rs

**Problem**:
```rust
// Won't work:
.test_id("...")
.with_alpha(0.x)
.opacity_70()
.overflow_y_scroll()
.key("...")
.font_medium()
children(if ... { div() } else { None })
```

**Fix**:
```rust
// Use .when() instead:
.when(true, |this| this.child(div()))

// Remove test_id calls
// Remove with_alpha, opacity_70, overflow_y_scroll
// Remove font_medium - use standard text sizing
// Use .children() only with iterators/vectors
```

### File: src/views/search/input.rs

**Problem**:
```rust
// Event handlers need 4 parameters
cx.listener(|this, text, cx| { ... })  // WRONG - 3 params
```

**Fix**:
```rust
// Add window parameter
cx.listener(|this, text, _window, cx| { ... })  // CORRECT - 4 params
```

### File: src/views/search/results.rs

**Problem**:
```rust
// Button usage is wrong
button(
    "...",
    ButtonSize::Medium,
    ButtonStyle::Primary,
    on_click
)
```

**Fix**:
```rust
// Use Button::new() pattern
Button::new("btn-id")
    .label("Add to Context")
    .primary()
    .on_click(cx.listener(|this, _ev, _window, cx| {
        // handle click
    }))
```

---

## Files Critical for Demo (Fix These)

1. **src/components/search.rs** (main search component)
   - Remove non-existent methods
   - Fix string references
   - Fix button usage

2. **src/views/search/input.rs** (search input)
   - Fix event handler signatures
   - Add missing parameters

3. **src/views/search/results.rs** (results display)
   - Fix button rendering
   - Fix conditional rendering
   - Fix "Add to Context" button

4. **src/views/search/autocomplete.rs** (dropdown)
   - Fix dropdown rendering
   - Fix event handlers

---

## Files to Temporarily Disable (If Needed)

If compilation takes too long, can comment out:
- Everything in `src/components/*_optimizer.rs`
- Everything in `src/components/advanced_*.rs`
- Everything in `src/components/kg_search_modal.rs`
- Everything in `src/components/knowledge_graph.rs` (except basic types)

Keep:
- ✅ `src/app.rs`
- ✅ `src/views/role_selector.rs`
- ✅ `src/views/search/*.rs`
- ✅ `src/views/chat/*.rs`
- ✅ `src/state/*.rs`
- ✅ `src/models.rs`
- ✅ `src/autocomplete.rs`
- ✅ `src/search_service.rs`
