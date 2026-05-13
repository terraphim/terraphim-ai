# Summary: Desktop Application Components (Svelte)

**Purpose:** Tauri-based desktop application providing GUI for Terraphim AI with Bulma CSS framework.

**Technology Stack:**
- Svelte with TypeScript
- Tauri 2.0 for desktop integration
- Bulma CSS (no Tailwind)
- Vite build tool
- Yarn package manager

**Component Structure:**

| Component | Purpose |
|-----------|---------|
| `App.svelte` | Root application component |
| `StartupScreen.svelte` | Initial loading/setup screen |
| `ConfigWizard.svelte` | Configuration setup wizard |
| `ThemeSwitcher.svelte` | Theme switching UI |
| `Shortcuts.svelte` | Keyboard shortcuts display |

**Search Module (`lib/Search/`):**
- `Search.svelte`: Main search interface
- `KGSearchModal.svelte`: Knowledge graph search modal
- `KGSearchInput.svelte`: Search input with autocomplete
- `SearchInput.svelte`: Generic search input component
- `ResultItem.svelte`: Individual search result
- `TermChip.svelte`: Term/tag chips
- `KGContextItem.svelte`: KG context display item
- `ArticleModal.svelte`: Article view/edit modal
- `AtomicSaveModal.svelte`: Atomic Server save dialog

**Chat Module (`lib/Chat/`):**
- `Chat.svelte`: Main chat interface
- `SessionList.svelte`: Conversation session list
- `ContextEditModal.svelte`: Context editing dialog

**Fetchers (`lib/Fetchers/`):**
- `FetchRole.svelte`: Role data fetching
- `FetchTabs.svelte`: Tab data fetching

**Editor (`lib/Editor/`):**
- `NovelWrapper.svelte`: Novel editor wrapper

**Other Components:**
- `ConfigJsonEditor.svelte`: JSON configuration editor
- `Cli.svelte`: CLI interface
- `Communication.svelte`: Communication settings
- `RoleGraphVisualization.svelte`: Rolegraph visualizer
- `BackButton.svelte`: Navigation back button

**Key Features:**
- Multi-role support with role switching
- KG-powered search with autocomplete
- Document editing with Novel editor
- Chat interface with conversation history
- Configuration wizard for setup
- Theme switching