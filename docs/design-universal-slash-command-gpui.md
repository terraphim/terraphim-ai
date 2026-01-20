# Design & Implementation Plan: Universal Slash Command System for GPUI

## 1. Summary of Target Behavior

Implement a universal slash command system in the GPUI desktop application that provides:

1. **Slash Command Palette**: When users type `/` at the start of a line in the chat input, display a command palette overlay with available commands (formatting, AI actions, search, etc.)

2. **Knowledge Graph Autocomplete**: When users type `++` anywhere, display KG-powered autocomplete suggestions using the existing `AutocompleteEngine` and `KGAutocompleteComponent`

3. **Command Registry**: Centralized registry of commands with metadata, execution logic, and GPUI keybinding integration

4. **Trigger System**: Detect character triggers (`/`, `++`) and auto-triggers (typing) with debouncing

5. **Popup Overlay Rendering**: GPUI-native overlay/popup for displaying suggestions with keyboard navigation, positioned inline below cursor

6. **KG-Enhanced Commands**: Commands integrate with Knowledge Graph for contextual suggestions (e.g., `/search rust` suggests KG-related terms)

The system will be **Rust-native** using GPUI patterns, reusing existing components like `AutocompleteState`, `KGAutocompleteComponent`, and modal patterns from `ContextEditModal`.

**Key Decisions:**
- Commands are **view-scoped** (Chat vs Search have different command sets)
- Popup displays **inline below cursor** (like autocomplete)
- KG integration is **enabled from start** for richer contextual commands
- **No feature flag** - this is core functionality

---

## 2. Key Invariants and Acceptance Criteria

### Invariants

| Category | Guarantee |
|----------|-----------|
| **Performance** | Command palette opens in <50ms, suggestions render in <100ms |
| **Responsiveness** | Keyboard navigation (Up/Down/Enter/Tab/Escape) responds immediately |
| **State Consistency** | Slash command popup closes when: (1) command selected, (2) Escape pressed, (3) focus lost, (4) trigger deleted |
| **Thread Safety** | All async operations use GPUI's spawn pattern with proper Entity updates |
| **Graceful Degradation** | If KG service unavailable, show static commands only |
| **View Scoping** | Chat commands stay in Chat, Search commands stay in Search |

### Acceptance Criteria

| ID | Criterion | Testable |
|----|-----------|----------|
| AC-1 | Typing `/` at line start in chat input shows command palette | Unit test + E2E |
| AC-2 | Command palette shows matching commands as user types filter | Unit test |
| AC-3 | Arrow Up/Down navigates command list, Enter/Tab selects | Unit test |
| AC-4 | Escape closes command palette without action | Unit test |
| AC-5 | Typing `++term` shows KG autocomplete suggestions | Integration test |
| AC-6 | Selected autocomplete term inserts into input | E2E test |
| AC-7 | Commands execute correct actions (insert text, trigger search, etc.) | Unit test |
| AC-8 | System integrates with existing `SearchInput` and `ChatView` | Integration test |
| AC-9 | `/search query` shows KG-enhanced suggestions from query | Integration test |
| AC-10 | Commands are view-scoped (Chat commands in Chat, Search in Search) | Unit test |

---

## 3. High-Level Design and Boundaries

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                     GPUI Application Layer                          │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    Universal Command System                      ││
│  │  ┌─────────────────┐  ┌──────────────────┐  ┌─────────────────┐││
│  │  │ CommandRegistry │  │ SuggestionSystem │  │  TriggerEngine  │││
│  │  │ - commands: Vec │  │ - providers: Vec │  │ - char_triggers │││
│  │  │ - view_scope    │  │ - cache: LRU     │  │ - debouncer     │││
│  │  │ - categories    │  │ - async fetch    │  │ - position_track│││
│  │  └─────────────────┘  └──────────────────┘  └─────────────────┘││
│  └─────────────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    Suggestion Providers                         ││
│  │  ┌─────────────────┐  ┌──────────────────┐  ┌─────────────────┐││
│  │  │CommandPalette   │  │KnowledgeGraph    │  │ KGEnhanced      │││
│  │  │Provider         │  │Provider          │  │ CommandProvider │││
│  │  │(static commands)│  │(AutocompleteEng) │  │(contextual cmds)│││
│  │  └─────────────────┘  └──────────────────┘  └─────────────────┘││
│  └─────────────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    GPUI UI Components                           ││
│  │  ┌─────────────────┐  ┌──────────────────┐  ┌─────────────────┐││
│  │  │SlashCommandPopup│  │ AutocompletePopup│  │  InputIntegrat. │││
│  │  │(inline below    │  │ (KG suggestions) │  │ (trigger detect)│││
│  │  │ cursor)         │  │Entity<Overlay>   │  │ on_key_down     │││
│  │  └─────────────────┘  └──────────────────┘  └─────────────────┘││
│  └─────────────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────────┤
│                    Existing Components (Reuse)                      │
│  ┌─────────────────┐  ┌──────────────────┐  ┌─────────────────────┐│
│  │AutocompleteState│  │ SearchInput      │  │ ChatView            ││
│  │(views/search/)  │  │ (input handling) │  │ (message input)     ││
│  └─────────────────┘  └──────────────────┘  └─────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Reuse Strategy |
|-----------|----------------|----------------|
| `CommandRegistry` | Store/lookup commands by id, category, trigger, view_scope | New module |
| `SuggestionProvider` (trait) | Async interface for suggestion sources | New trait, impl existing engines |
| `KGEnhancedCommandProvider` | Commands that query KG for contextual suggestions | New, wraps AutocompleteEngine |
| `TriggerEngine` | Detect triggers, manage debounce, track position | New module |
| `SlashCommandPopup` | GPUI overlay for command palette (inline below cursor) | New view, pattern from autocomplete dropdown |
| `AutocompletePopup` | GPUI overlay for KG suggestions | Enhance existing autocomplete rendering |
| `AutocompleteEngine` | Thesaurus-based autocomplete | Reuse `autocomplete.rs` |
| `KGAutocompleteComponent` | Rich KG suggestions | Reuse `kg_autocomplete.rs` |

### View Scope Design

```rust
pub enum ViewScope {
    Chat,      // Commands available in ChatView
    Search,    // Commands available in SearchInput
    Global,    // Available everywhere (future)
}

// Commands are tagged with their scope
pub struct UniversalCommand {
    pub id: String,
    pub scope: ViewScope,
    // ...
}
```

### Boundaries

**Inside Scope:**
- Command registry with view-scoped commands
- Three suggestion providers: CommandPalette, KnowledgeGraph, KGEnhancedCommand
- Trigger detection for `/` and `++`
- GPUI inline popup UI with keyboard navigation
- Integration with `ChatView` and `SearchInput`
- KG-enhanced commands from start

**Outside Scope (Future Work):**
- Zed editor plugin (separate implementation)
- Global commands (cross-view)
- Command history/favorites
- Custom command scripting/plugins

---

## 4. File/Module-Level Change Plan

### New Files

| File | Action | Responsibility | Dependencies |
|------|--------|----------------|--------------|
| `src/slash_command/mod.rs` | Create | Module root, re-exports | - |
| `src/slash_command/types.rs` | Create | Core types: UniversalCommand, UniversalSuggestion, ViewScope, CommandCategory | - |
| `src/slash_command/registry.rs` | Create | CommandRegistry with view-scoped lookup | `types.rs`, `actions.rs` |
| `src/slash_command/providers.rs` | Create | SuggestionProvider trait + impls | `autocomplete.rs`, `search_service.rs` |
| `src/slash_command/kg_enhanced.rs` | Create | KGEnhancedCommandProvider for contextual commands | `autocomplete.rs`, `providers.rs` |
| `src/slash_command/trigger.rs` | Create | TriggerEngine, debounce logic | - |
| `src/slash_command/popup.rs` | Create | SlashCommandPopup view (inline positioning) | `theme.rs`, gpui-component |

### Modified Files

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `src/lib.rs` | Modify | No slash_command module | Add `pub mod slash_command;` | - |
| `src/views/chat/mod.rs` | Modify | Basic input handling | Add trigger detection + popup | `slash_command/` |
| `src/views/search/input.rs` | Modify | Autocomplete only | Add slash command integration | `slash_command/` |
| `src/actions.rs` | Modify | Basic navigation actions | Add command execution actions | - |

### Existing Files (Reference Only)

| File | Role | Reuse |
|------|------|-------|
| `src/autocomplete.rs` | AutocompleteEngine | Direct reuse for KG providers |
| `src/components/kg_autocomplete.rs` | KGAutocompleteComponent | Reference for async patterns |
| `src/state/search.rs` | SearchState, suggestions | Extend for command suggestions |
| `src/views/search/input.rs` | SearchInput with dropdown | Pattern for inline popup |
| `src/theme/colors.rs` | Theme colors | Reuse for popup styling |

---

## 5. Step-by-Step Implementation Sequence

### Phase 1: Core Types and Registry (Foundation)

| Step | Purpose | Deployable? |
|------|---------|-------------|
| 1.1 | Create `slash_command/types.rs` with core types (UniversalCommand, UniversalSuggestion, ViewScope, CommandCategory, CommandResult) | Yes |
| 1.2 | Create `slash_command/registry.rs` with CommandRegistry struct, view-scoped lookup, and built-in commands | Yes |
| 1.3 | Create `slash_command/mod.rs` with module exports | Yes |
| 1.4 | Add unit tests for registry lookup and filtering by view scope | Yes |

### Phase 2: Suggestion Provider System with KG Integration

| Step | Purpose | Deployable? |
|------|---------|-------------|
| 2.1 | Create `slash_command/providers.rs` with SuggestionProvider trait | Yes |
| 2.2 | Implement CommandPaletteProvider (static command suggestions) | Yes |
| 2.3 | Implement KnowledgeGraphProvider wrapping AutocompleteEngine | Yes |
| 2.4 | Create `slash_command/kg_enhanced.rs` with KGEnhancedCommandProvider | Yes |
| 2.5 | Implement `/search` command with KG term suggestions | Yes |
| 2.6 | Add unit and integration tests for all providers | Yes |

### Phase 3: Trigger Detection System

| Step | Purpose | Deployable? |
|------|---------|-------------|
| 3.1 | Create `slash_command/trigger.rs` with TriggerEngine | Yes |
| 3.2 | Implement character trigger detection (`/` at line start, `++` anywhere) | Yes |
| 3.3 | Implement debounce manager using GPUI timers | Yes |
| 3.4 | Add start-of-line detection for `/` trigger | Yes |
| 3.5 | Add unit tests for trigger detection edge cases | Yes |

### Phase 4: GPUI Inline Popup UI Component

| Step | Purpose | Deployable? |
|------|---------|-------------|
| 4.1 | Create `slash_command/popup.rs` with SlashCommandPopup struct | Yes |
| 4.2 | Implement inline popup rendering (absolute positioning below cursor) | Yes |
| 4.3 | Render suggestion list with icons, titles, descriptions | Yes |
| 4.4 | Implement keyboard navigation (Up/Down/Enter/Tab/Escape) | Yes |
| 4.5 | Add KG context display for enhanced commands | Yes |
| 4.6 | Style popup using theme system (match existing autocomplete) | Yes |

### Phase 5: Integration with ChatView

| Step | Purpose | Deployable? |
|------|---------|-------------|
| 5.1 | Add SlashCommandPopup entity to ChatView | Yes |
| 5.2 | Hook into input's on_key_down for `/` trigger detection | Yes |
| 5.3 | Connect popup events to command execution | Yes |
| 5.4 | Handle focus management and popup lifecycle | Yes |
| 5.5 | Add Chat-scoped commands (AI, context, formatting) | Yes |

### Phase 6: Integration with SearchInput

| Step | Purpose | Deployable? |
|------|---------|-------------|
| 6.1 | Add `++` trigger detection to SearchInput (merge with existing) | Yes |
| 6.2 | Add `/` trigger for Search-scoped commands | Yes |
| 6.3 | Add Search-scoped commands (filter, sort, KG search) | Yes |
| 6.4 | Test combined slash command + KG autocomplete | Yes |

### Phase 7: Built-in Commands (KG-Enhanced)

| Step | Purpose | Deployable? |
|------|---------|-------------|
| 7.1 | Implement text formatting commands (heading, bold, list) - Chat scope | Yes |
| 7.2 | Implement `/search <query>` with KG term suggestions | Yes |
| 7.3 | Implement `/kg <term>` to explore knowledge graph | Yes |
| 7.4 | Implement context commands (/context, /clear) - Chat scope | Yes |
| 7.5 | Implement AI commands (/summarize, /explain) - Chat scope | Yes |
| 7.6 | Implement filter commands (/filter, /sort) - Search scope | Yes |

---

## 6. Testing & Verification Strategy

### Unit Tests

| Acceptance Criteria | Test Location | Test Description |
|---------------------|---------------|------------------|
| AC-2 (command filtering) | `slash_command/registry.rs` | `test_command_filtering_by_query` |
| AC-3 (keyboard nav) | `slash_command/popup.rs` | `test_keyboard_navigation` |
| AC-7 (command execution) | `slash_command/registry.rs` | `test_command_execution` |
| AC-10 (view scope) | `slash_command/registry.rs` | `test_view_scoped_commands` |
| Trigger detection | `slash_command/trigger.rs` | `test_trigger_detection` |
| Start-of-line check | `slash_command/trigger.rs` | `test_slash_start_of_line` |
| Provider suggestions | `slash_command/providers.rs` | `test_provider_suggestions` |
| KG-enhanced commands | `slash_command/kg_enhanced.rs` | `test_kg_enhanced_suggestions` |
| Debounce behavior | `slash_command/trigger.rs` | `test_debounce_timing` |

### Integration Tests

| Acceptance Criteria | Test Location | Test Description |
|---------------------|---------------|------------------|
| AC-1 (slash trigger) | `tests/slash_command_integration.rs` | `test_slash_trigger_in_chat` |
| AC-5 (KG autocomplete) | `tests/kg_autocomplete_integration.rs` | `test_kg_autocomplete_trigger` |
| AC-8 (component integration) | `tests/ui_integration_tests.rs` | `test_slash_command_with_chat_view` |
| AC-9 (KG-enhanced) | `tests/slash_command_integration.rs` | `test_kg_enhanced_search_command` |

### E2E Tests

| Acceptance Criteria | Test Location | Test Description |
|---------------------|---------------|------------------|
| AC-1, AC-6 (full flow) | `tests/e2e_user_journey.rs` | `test_slash_command_user_flow` |
| AC-4 (escape closes) | `tests/e2e_user_journey.rs` | `test_slash_command_escape` |
| KG integration flow | `tests/e2e_user_journey.rs` | `test_slash_search_with_kg_suggestions` |

---

## 7. Risk & Complexity Review

| Risk | Likelihood | Impact | Mitigation | Residual Risk |
|------|------------|--------|------------|---------------|
| GPUI inline positioning complexity | Medium | Medium | Reference SearchInput dropdown pattern, use absolute positioning with cursor tracking | May need adjustments for edge cases |
| Async KG suggestion race conditions | Medium | High | Use GPUI spawn pattern with Entity updates, cancellation tokens, debouncing | Low with proper implementation |
| Focus management conflicts | Medium | Medium | Explicit focus tracking, test with multiple input sources | Some edge cases may need iteration |
| KG-enhanced commands performance | Medium | Medium | Cache KG results, limit concurrent queries, debounce | Monitor in production |
| View scope enforcement | Low | Low | Clear scope tagging, filter at registry level | Low |
| Integration conflicts with existing autocomplete | Medium | Medium | Careful state management, unified trigger handling | Requires testing |

---

## 8. Decisions Made

Based on user input, the following decisions are finalized:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Q1: Command Scope | **View-scoped only** | Simpler implementation, clearer UX |
| Q2: Popup Positioning | **Inline below cursor** | Matches autocomplete pattern, familiar UX |
| Q3: KG Integration | **KG-enhanced from start** | Richer functionality, core differentiator |
| Q4: Feature Flag | **None needed** | Core functionality, no gating required |

---

## Summary

This design creates a modular, testable slash command system for GPUI that:

1. **Reuses existing components**: `AutocompleteEngine`, modal patterns, theme system
2. **Follows GPUI patterns**: Entity-based state, spawn for async, event subscription
3. **Integrates KG from start**: Commands like `/search` provide contextual KG suggestions
4. **View-scoped design**: Chat and Search have appropriate command sets
5. **Inline popup UX**: Matches existing autocomplete dropdown pattern

**Implementation Phases:**
- Phase 1-3 (Core + KG Providers + Triggers): ~3 days
- Phase 4 (Inline Popup UI): ~2 days
- Phase 5-6 (View Integration): ~2 days
- Phase 7 (Built-in Commands): ~2 days
- Testing: Continuous

**Total Estimated Effort: 9-10 days**

---

**Do you approve this plan as-is, or would you like to adjust any part?**
