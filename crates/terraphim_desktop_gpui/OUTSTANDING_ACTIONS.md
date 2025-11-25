# Outstanding Actions - GPUI Desktop

**Current Status:** 90% Complete - Critical fixes in progress
**Branch:** claude/plan-gpui-migration-01BgC7ez2NPwXiCuNB7b931a

---

## CRITICAL FIXES (Must Complete Now)

### 1. Fix Role Change Event Subscription (15 min)
**Status:** IN PROGRESS - Compilation error
**File:** `src/app.rs` line 60

**Error:**
```
closure is expected to take 4 arguments, but it takes 5 arguments
```

**Fix:**
```rust
// WRONG (current):
cx.subscribe(&role_selector, move |_app, event: &RoleChangeEvent, cx| {

// CORRECT (GPUI 0.2.2 pattern):
cx.subscribe(&role_selector, move |this: &mut TerraphimApp, event: &RoleChangeEvent, cx| {
```

**Store subscription:** Add `_subscriptions: Vec<Subscription>` to TerraphimApp struct

### 2. Fix SearchView update_role Type Annotation (5 min)
**Status:** IN PROGRESS - Compilation error
**File:** `src/views/search/mod.rs` line 42

**Already fixed** with explicit type annotation - just needs rebuild

### 3. Test Role-Based Search Works (10 min)
**Action:**
```bash
cargo build && cargo run
# Click role selector
# Select "Default" role
# Search for "test"
# Should get different results than Terraphim Engineer
# Log should show: "SearchState role changed from Terraphim Engineer to Default"
```

---

## HIGH PRIORITY (Session 2 - Next 3 hours)

### 4. Add Action Buttons to Search Results
**Tauri Reference:** `desktop/src/lib/Search/ResultItem.svelte` lines 94-183
**File:** `src/views/search/results.rs`

**Buttons Needed:**
- **"Add to Context"** (IconName::Plus) - CRITICAL
  - Pattern: ResultItem.svelte lines 137-153
  - Calls: add_context_to_conversation (cmd.rs:1124)

- **"Chat with Document"** (IconName::Bot) - CRITICAL
  - Pattern: ResultItem.svelte lines 155-180
  - Adds to context + navigates to chat

- **"Open URL"** (IconName::ExternalLink)
  - Opens document.url in browser
  - Use `webbrowser` crate or system open

- **"Download Markdown"** (IconName::Download) - Optional
  - Save to local file

**Implementation:**
```rust
fn render_result_actions(&self, doc: &Document, cx: &Context<Self>) -> impl IntoElement {
    div()
        .flex()
        .gap_2()
        .mt_2()
        .child(Button::new(("open", doc.id.as_str())).icon(IconName::ExternalLink).label("Open"))
        .child(Button::new(("add-ctx", doc.id.as_str())).icon(IconName::Plus).label("Add to Context"))
        .child(Button::new(("chat", doc.id.as_str())).icon(IconName::Bot).label("Chat"))
}
```

### 5. Wire "Add to Context" Button
**Pattern:** Tauri ResultItem.svelte lines 572-765

**Steps:**
a. Create ContextItem from Document
```rust
let context_item = ContextItem {
    id: ulid::Ulid::new().to_string(),
    context_type: ContextType::Document,
    title: document.title.clone(),
    summary: document.description.clone(),
    content: document.body.clone(),
    metadata: {
        let mut meta = ahash::AHashMap::new();
        meta.insert("document_id".to_string(), document.id.clone());
        if !document.url.is_empty() {
            meta.insert("url".to_string(), document.url.clone());
        }
        meta
    },
    created_at: chrono::Utc::now(),
    relevance_score: document.rank.map(|r| r as f64),
};
```

b. Get or create conversation ID
```rust
// Need access to ChatView's context_manager
// Option 1: Share context_manager via App
// Option 2: Emit event that App handles
```

c. Add to conversation
```rust
manager.lock().await.add_context(&conv_id, context_item)
```

### 6. Implement Article Modal
**Tauri Reference:** `desktop/src/lib/Search/ArticleModal.svelte`

**Features:**
- Modal overlay showing full document
- Markdown rendering
- Close button (Escape key)
- Double-click to edit (future)

**Implementation:**
```rust
pub struct ArticleModal {
    document: Option<Document>,
    is_open: bool,
}

impl Render for ArticleModal {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_open {
            return div();
        }

        div()
            .absolute()
            .inset_0()
            .bg(rgba(0, 0, 0, 0.5))  // Overlay
            .child(
                div()
                    .max_w(px(1200.0))
                    .m_auto()
                    .bg(rgb(0xffffff))
                    .p_6()
                    .rounded_lg()
                    .child(/* Document content */)
            )
    }
}
```

---

## MEDIUM PRIORITY (Session 3 - Optional)

### 7. System Tray Icon
**Tauri Reference:** `desktop/src-tauri/src/main.rs` lines 109-131

**Requires:**
- Platform-specific code (macOS NSStatusBar)
- tauri-plugin-tray or manual implementation
- Not critical for core functionality

**Defer:** Can use existing desktop app for now

### 8. Global Keyboard Shortcuts
**Tauri Reference:** `desktop/src-tauri/src/main.rs` lines 280-296

**Requires:**
- Platform-specific global hotkey registration
- Not available in GPUI out of box

**Defer:** Not critical, use mouse for now

---

## CURRENT COMPILATION ERRORS TO FIX

### Error 1: Subscribe closure signature
**File:** `src/app.rs:60`
```rust
// Current (wrong - 5 args):
cx.subscribe(&role_selector, move |_app, event: &RoleChangeEvent, cx| {

// Fix (4 args):
let _role_sub = cx.subscribe(&role_selector, move |this: &mut TerraphimApp, event: &RoleChangeEvent, cx| {
    search_view_clone.update(cx, |view, _| {
        view.update_role(event.new_role.clone());
    });
});
```

**Store subscription:** Add to TerraphimApp:
```rust
pub struct TerraphimApp {
    // ...
    _subscriptions: Vec<Subscription>,
}

// In new():
Self {
    // ...
    _subscriptions: vec![_role_sub],
}
```

### Error 2: Type annotation (already fixed)
Just needs rebuild after fixing Error 1.

---

## TEST PLAN

### Test 1: Role-Based Search (After fixes)
1. Launch app
2. Search "test" with Terraphim Engineer → note result count
3. Click role dropdown
4. Select "Default"
5. Search "test" again → should get different results
6. Log should show: "SearchState role changed from Terraphim Engineer to Default"

### Test 2: Add to Context (After button implementation)
1. Search for "async"
2. Click "Add to Context" on first result
3. Navigate to Chat
4. Verify context item appears in panel
5. Send message
6. Verify context injected

### Test 3: Modal (After implementation)
1. Search results showing
2. Click result item
3. Modal opens with full content
4. Press Escape
5. Modal closes

---

## ESTIMATED TIME REMAINING

**Critical Fixes:** 30 minutes
- Fix subscribe signature: 15 min
- Test role-based search: 10 min
- Commit and verify: 5 min

**Action Buttons:** 1.5 hours
- Add buttons to results: 30 min
- Wire add-to-context: 45 min
- Test complete flow: 15 min

**Modal:** 1 hour
- Create modal component: 30 min
- Wire to results: 15 min
- Test: 15 min

**Total to Full Parity:** ~3 hours

---

## FILES TO MODIFY

1. ✅ `src/app.rs` - Fix subscribe, store subscription
2. ✅ `src/state/search.rs` - Role change logic (done)
3. ✅ `src/views/search/mod.rs` - update_role method (done)
4. ✅ `src/views/role_selector.rs` - Event emission (done)
5. ⏳ `src/views/search/results.rs` - Add action buttons
6. ⏳ `src/views/article_modal.rs` - NEW - Modal component
7. ⏳ `src/app.rs` - Handle add-to-context event

---

## PRIORITY ORDER

**NOW (Next 30 min):**
1. Fix compilation errors
2. Test role-based search works
3. Commit working state

**NEXT (1-2 hours):**
1. Add action buttons
2. Wire add-to-context
3. Test complete flow

**LATER (1 hour):**
1. Article modal
2. Final polish

---

**Last Updated:** 2025-11-25 11:50 UTC
**Next Action:** Fix subscribe signature and test role-based search
