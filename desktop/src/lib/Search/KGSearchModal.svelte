<script lang="ts">
  import { Modal, Field, Input, Button, Message } from "svelma";
  import { invoke } from "@tauri-apps/api/tauri";
  import { is_tauri, role, serverUrl, thesaurus } from "../stores";
  import { CONFIG } from "../../config";
  import { createEventDispatcher, onMount, onDestroy } from "svelte";

  export let active: boolean = false;
  export let initialQuery: string = "";
  export let conversationId: string | null = null;

  const dispatch = createEventDispatcher();

  // Search state
  let query: string = "";
  let suggestions: KGSuggestion[] = [];
  let isSearching = false;
  let searchError: string | null = null;
  let selectedSuggestion: KGSuggestion | null = null;
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  // Typeahead state (reuses autocomplete behavior from Search.svelte)
  let typeaheadSuggestions: string[] = [];
  let typeaheadIndex: number = -1;

  // Input element reference for focus management
  let searchInput: HTMLInputElement;

  interface KGSuggestion {
    term: string;
    text?: string;
    normalized_term?: string;
    url?: string;
    snippet?: string;
    score: number;
    suggestion_type?: string;
    icon?: string;
  }

  interface KGSearchResponse {
    status: string;
    suggestions: KGSuggestion[];
    error?: string;
  }

  // Access thesaurus entries
  $: thesaurusEntries = Object.entries($thesaurus);

  // Initialize query when modal opens
  $: if (active && initialQuery !== query) {
    query = initialQuery;
    if (query.trim()) {
      searchKGTerms();
    }
  }

  // Focus input when modal opens and clear any errors
  $: if (active && searchInput) {
    setTimeout(() => {
      searchInput?.focus();
      searchError = null; // Clear any previous errors when modal opens
    }, 100);
  }

  // Debounced search function
  function handleQueryChange() {
    if (searchTimeout) {
      clearTimeout(searchTimeout);
    }

    searchTimeout = setTimeout(() => {
      if (query.trim().length >= 2) {
        searchKGTerms();
      } else {
        suggestions = [];
        selectedSuggestion = null;
      }
    }, 300);
  }

  // Get KG term suggestions (thesaurus primary, API fallback)
  async function getTermSuggestions(q: string): Promise<string[]> {
    const trimmed = q.trim();
    if (!trimmed) return [];
    
    // PRIMARY: Use thesaurus matching first (same as Search component)
    const thesaurusResults = thesaurusEntries
      .filter(([key]) => key.toLowerCase().includes(trimmed.toLowerCase()))
      .map(([key]) => key)
      .slice(0, 8);
    
    // If we have good thesaurus results, return them
    if (thesaurusResults.length > 0) {
      return thesaurusResults;
    }
    
    // FALLBACK: Try API if thesaurus doesn't have results
    try {
      if ($is_tauri) {
        const resp: any = await invoke("get_autocomplete_suggestions", {
          query: trimmed,
          roleName: $role,
          limit: 8
        });
        if (resp?.status === 'success' && Array.isArray(resp.suggestions)) {
          return resp.suggestions.map((s: any) => s.term);
        }
      } else {
        const resp = await fetch(`${$serverUrl.replace('/documents/search', '')}/autocomplete/${encodeURIComponent($role)}/${encodeURIComponent(trimmed)}`);
        if (resp.ok) {
          const data = await resp.json();
          if (data?.status === 'success' && Array.isArray(data.suggestions)) {
            return data.suggestions.map((s: any) => s.term);
          }
        }
      }
    } catch (e) {
      console.warn('KG typeahead API fallback failed', e);
    }
    
    // Return empty if both thesaurus and API fail
    return [];
  }

  async function updateTypeaheadSuggestions() {
    if (query.trim().length < 1) {
      typeaheadSuggestions = [];
      typeaheadIndex = -1;
      return;
    }
    typeaheadSuggestions = await getTermSuggestions(query);
    typeaheadIndex = -1;
  }

  async function applyTypeaheadSuggestion(s: string) {
    // When a user picks an autocomplete item, immediately add the KG term
    // to the conversation context (not a document search)
    selectedSuggestion = { term: s, score: 1.0 } as KGSuggestion;
    typeaheadSuggestions = [];
    typeaheadIndex = -1;
    await addTermToContext();
  }

  function handleTypeaheadKeydown(event: KeyboardEvent) {
    if (typeaheadSuggestions.length > 0) {
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        typeaheadIndex = (typeaheadIndex + 1) % typeaheadSuggestions.length;
      } else if (event.key === 'ArrowUp') {
        event.preventDefault();
        typeaheadIndex = (typeaheadIndex - 1 + typeaheadSuggestions.length) % typeaheadSuggestions.length;
      } else if ((event.key === 'Enter' || event.key === 'Tab') && typeaheadIndex !== -1) {
        event.preventDefault();
        applyTypeaheadSuggestion(typeaheadSuggestions[typeaheadIndex]);
      } else if (event.key === 'Escape') {
        typeaheadSuggestions = [];
        typeaheadIndex = -1;
      }
    }
  }

  // Search KG terms using the new Tauri command
  async function searchKGTerms() {
    if (!query.trim() || query.trim().length < 2) {
      suggestions = [];
      return;
    }

    isSearching = true;
    searchError = null;

    try {
      if ($is_tauri) {
        const response: KGSearchResponse = await invoke("search_kg_terms", {
          request: {
            query: query.trim(),
            role_name: $role,
            limit: 20,
            min_similarity: 0.6
          }
        });

        if (response.status === 'success') {
          suggestions = response.suggestions || [];
          // Auto-select first suggestion if available
          if (suggestions.length > 0 && !selectedSuggestion) {
            selectedSuggestion = suggestions[0];
          }
        } else {
          searchError = response.error || 'Search failed';
          suggestions = [];
        }
      } else {
        // Web mode - HTTP API
        if (!conversationId) {
          searchError = 'No active conversation. Please start a conversation first.';
          suggestions = [];
          return;
        }

        const response = await fetch(`${$serverUrl.replace('/documents/search', '')}/conversations/${conversationId}/context/kg/search?query=${encodeURIComponent(query.trim())}&role=${encodeURIComponent($role)}`);

        if (response.ok) {
          const data = await response.json();
          if (data.status === 'success') {
            suggestions = data.suggestions || [];
            // Auto-select first suggestion if available
            if (suggestions.length > 0 && !selectedSuggestion) {
              selectedSuggestion = suggestions[0];
            }
          } else {
            searchError = data.error || 'Search failed';
            suggestions = [];
          }
        } else {
          searchError = `HTTP ${response.status}: ${response.statusText}`;
          suggestions = [];
        }
      }
    } catch (error) {
      console.error('KG search error:', error);
      searchError = `Search failed: ${error}`;
      suggestions = [];
    } finally {
      isSearching = false;
    }
  }

  // Handle suggestion selection
  function selectSuggestion(suggestion: KGSuggestion) {
    selectedSuggestion = suggestion;
  }

  // Add selected term to context
  async function addTermToContext() {
    if (!selectedSuggestion) return;

    if (!conversationId) {
      searchError = 'No active conversation. Please start a conversation first.';
      return;
    }

    try {
      if ($is_tauri) {
        await invoke("add_kg_term_context", {
          request: {
            conversation_id: conversationId,
            term: selectedSuggestion.term,
            role_name: $role
          }
        });

        dispatch("termAdded", {
          term: selectedSuggestion.term,
          suggestion: selectedSuggestion
        });

        // Close modal after successful addition
        handleClose();
      } else {
        // Web mode - HTTP API
        const response = await fetch(`${$serverUrl.replace('/documents/search', '')}/conversations/${conversationId}/context/kg/term`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            term: selectedSuggestion.term,
            role: $role
          })
        });

        if (response.ok) {
          const data = await response.json();
          if (data.status === 'success') {
            dispatch("termAdded", {
              term: selectedSuggestion.term,
              suggestion: selectedSuggestion
            });

            // Close modal after successful addition
            handleClose();
          } else {
            searchError = data.error || 'Failed to add term to context';
          }
        } else {
          searchError = `HTTP ${response.status}: ${response.statusText}`;
        }
      }
    } catch (error) {
      console.error('Error adding term to context:', error);
      searchError = `Failed to add term to context: ${error}`;
    }
  }

  // Add entire KG index to context
  async function addKGIndexToContext() {
    if (!conversationId) {
      searchError = 'No active conversation. Please start a conversation first.';
      return;
    }

    try {
      if ($is_tauri) {
        await invoke("add_kg_index_context", {
          request: {
            conversation_id: conversationId,
            role_name: $role
          }
        });

        dispatch("kgIndexAdded", { role: $role });

        // Close modal after successful addition
        handleClose();
      } else {
        // Web mode - HTTP API
        const response = await fetch(`${$serverUrl.replace('/documents/search', '')}/conversations/${conversationId}/context/kg/index`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            role: $role
          })
        });

        if (response.ok) {
          const data = await response.json();
          if (data.status === 'success') {
            dispatch("kgIndexAdded", { role: $role });

            // Close modal after successful addition
            handleClose();
          } else {
            searchError = data.error || 'Failed to add KG index to context';
          }
        } else {
          searchError = `HTTP ${response.status}: ${response.statusText}`;
        }
      }
    } catch (error) {
      console.error('Error adding KG index to context:', error);
      searchError = `Failed to add KG index to context: ${error}`;
    }
  }

  // Handle modal close
  function handleClose() {
    active = false;
    query = "";
    suggestions = [];
    selectedSuggestion = null;
    searchError = null;
    isSearching = false;

    if (searchTimeout) {
      clearTimeout(searchTimeout);
      searchTimeout = null;
    }
  }

  // Handle keyboard navigation
  function handleKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case 'Escape':
        handleClose();
        break;
      case 'ArrowDown':
        event.preventDefault();
        navigateSuggestions(1);
        break;
      case 'ArrowUp':
        event.preventDefault();
        navigateSuggestions(-1);
        break;
      case 'Enter':
        event.preventDefault();
        if (selectedSuggestion) {
          addTermToContext();
        }
        break;
    }
  }

  // Navigate through suggestions with keyboard
  function navigateSuggestions(direction: number) {
    if (suggestions.length === 0) return;

    const currentIndex = selectedSuggestion
      ? suggestions.findIndex(s => s.term === selectedSuggestion?.term)
      : -1;

    let newIndex = currentIndex + direction;

    if (newIndex < 0) {
      newIndex = suggestions.length - 1;
    } else if (newIndex >= suggestions.length) {
      newIndex = 0;
    }

    selectedSuggestion = suggestions[newIndex];
  }

  // Clean up timeout on component destruction
  onDestroy(() => {
    if (searchTimeout) {
      clearTimeout(searchTimeout);
    }
  });
</script>

<style lang="scss">
  .wrapper {
    position: relative;
    width: 100%;
  }
  .kg-search-container {
    position: relative;
    width: 100%;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
  }

  /* Close button positioning using Bulma's delete styling */
  .modal-close-btn {
    position: absolute !important;
    top: 1rem;
    right: 1rem;
    z-index: 10;

    /* Enhanced hover effect that respects theme */
    &:hover {
      transform: scale(1.1);
    }

    &:active {
      transform: scale(0.95);
    }
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid #e1e1e1;
  }

  .modal-title {
    flex: 1;
  }

  .modal-title h3 {
    font-size: 1.5rem;
    font-weight: 600;
    color: #363636;
    margin-bottom: 0.5rem;
  }

  .modal-title p {
    color: #757575;
    font-size: 0.875rem;
    margin: 0;
  }

  .search-section {
    margin-bottom: 1.5rem;
  }

  .suggestions-container {
    max-height: 300px;
    overflow-y: auto;
    border: 1px solid #e1e1e1;
    border-radius: 6px;
    background: #fefefe;
    margin-bottom: 1.5rem;
  }

  /* Typeahead dropdown (reused from Search.svelte) */
  .input-wrapper {
    position: relative;
  }

  /* Ensure input text is visible */
  :global(.input-wrapper .input) {
    color: #363636 !important;
  }

  /* Dark theme input text */
  @media (prefers-color-scheme: dark) {
    :global(.input-wrapper .input) {
      color: #e0e0e0 !important;
    }
  }
  .suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 5;
    list-style-type: none;
    padding: 0;
    margin: 0;
    background-color: white;
    border: 1px solid #dbdbdb;
    border-top: none;
    border-radius: 0 0 4px 4px;
    box-shadow: 0 2px 3px rgba(10, 10, 10, 0.1);
  }
  .suggestions li {
    padding: 0.5em 1em;
    cursor: pointer;
  }
  .suggestions li:hover,
  .suggestions li.active {
    background-color: #f5f5f5;
  }

  .suggestion-item {
    padding: 1rem;
    border-bottom: 1px solid #f0f0f0;
    cursor: pointer;
    transition: all 0.2s ease;
    display: block;
    text-decoration: none;
    color: inherit;
    background: none;
    border: none;
    width: 100%;
    text-align: left;

    &:hover {
      background-color: #f8f9fa;
      border-left: 3px solid #3273dc;
    }

    &.is-active {
      background-color: #e3f2fd;
      border-left: 3px solid #3273dc;
    }

    &:last-child {
      border-bottom: none;
    }
  }

  .suggestion-term {
    font-weight: 600;
    color: #363636;
    font-size: 1rem;
    margin-bottom: 0.5rem;
  }

  .suggestion-meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-bottom: 0.25rem;
  }

  .suggestion-url {
    color: #757575;
    font-style: italic;
    font-size: 0.75rem;
    margin-top: 0.25rem;
  }

  .empty-state {
    text-align: center;
    padding: 2rem;
    color: #757575;
  }

  .progress-container {
    margin-bottom: 1rem;
  }

  .progress-bar {
    width: 100%;
    height: 4px;
    background-color: #e1e1e1;
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background-color: #3273dc;
    animation: progress-animation 1.5s ease-in-out infinite;
  }

  @keyframes progress-animation {
    0% { transform: translateX(-100%); }
    50% { transform: translateX(0%); }
    100% { transform: translateX(100%); }
  }

  .modal-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 2rem;
    padding-top: 1.5rem;
    border-top: 1px solid #e1e1e1;
  }

  .action-buttons {
    display: flex;
    gap: 0.75rem;
  }

  .alternative-section {
    margin-top: 1.5rem;
    padding: 1rem;
    background: #f8f9fa;
    border-radius: 6px;
    border-left: 4px solid #3273dc;
  }

  .alternative-content {
    margin-bottom: 1rem;
  }

  .alternative-content p {
    margin: 0;
    font-size: 0.875rem;
    color: #4a4a4a;
    line-height: 1.4;
  }


  /* Responsive modal sizing (reusing ArticleModal rules) */
  :global(.modal-content) {
    width: 95vw !important;
    max-width: 1200px !important;
    max-height: calc(100vh - 2rem) !important;
    margin: 1rem auto !important;
    overflow-y: auto !important;

    /* Responsive breakpoints */
    @media (min-width: 768px) {
      width: 90vw !important;
      max-height: calc(100vh - 4rem) !important;
      margin: 2rem auto !important;
    }

    @media (min-width: 1024px) {
      width: 80vw !important;
      max-height: calc(100vh - 6rem) !important;
      margin: 3rem auto !important;
    }

    @media (min-width: 1216px) {
      width: 75vw !important;
    }

    @media (min-width: 1408px) {
      width: 70vw !important;
    }
  }

  /* Ensure modal background doesn't interfere with scrolling */
  :global(.modal) {
    padding: 0 !important;
    overflow-y: auto !important;
  }

  @media (max-width: 767px) {
    :global(.modal-content) {
      width: calc(100vw - 2rem) !important;
      max-height: calc(100vh - 1rem) !important;
      margin: 0.5rem auto !important;
    }
  }

  /* Dark theme adjustments */
  @media (prefers-color-scheme: dark) {
    .modal-title h3 {
      color: #e0e0e0;
    }

    .modal-title p {
      color: #b0b0b0;
    }

    .suggestions-container {
      background: #2a2a2a;
      border-color: #404040;
    }

    .suggestion-item {
      border-bottom-color: #404040;

      &:hover {
        background-color: #3a3a3a;
      }

      &.is-active {
        background-color: #1e3a5f;
      }
    }

    .suggestion-term {
      color: #e0e0e0;
    }

    .suggestion-url {
      color: #b0b0b0;
    }

    .alternative-section {
      background: #3a3a3a;
    }

    .alternative-content p {
      color: #d0d0d0;
    }
  }
</style>

<Modal bind:active on:close={handleClose}>
  <div class="box wrapper" data-testid="kg-search-modal">
    <div class="kg-search-container" on:keydown={handleKeydown}>
      <!-- Close button following Bulma styling -->
      <button class="delete is-large modal-close-btn" on:click={handleClose} aria-label="close"></button>

      <div class="modal-header">
        <div class="modal-title">
          <h3>Knowledge Graph Search</h3>
          <p>Search and add terms from the knowledge graph to your context</p>
        </div>
      </div>

      <div class="search-section">
        <Field>
          <div class="input-wrapper">
            <Input
              bind:element={searchInput}
              bind:value={query}
              on:input={() => { handleQueryChange(); updateTypeaheadSuggestions(); }}
              on:keydown={handleTypeaheadKeydown}
              placeholder="Search knowledge graph terms..."
              type="search"
              disabled={isSearching}
              icon="search"
              expanded
              autofocus
              data-testid="kg-search-input"
            />
            {#if typeaheadSuggestions.length > 0}
              <ul class="suggestions" data-testid="kg-typeahead-list">
                {#each typeaheadSuggestions as s, index}
                  <li
                    class:active={index === typeaheadIndex}
                    on:click={() => applyTypeaheadSuggestion(s)}
                    on:keydown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        applyTypeaheadSuggestion(s);
                      }
                    }}
                    tabindex="0"
                    role="option"
                    aria-selected={index === typeaheadIndex}
                    aria-label={`Apply suggestion: ${s}`}
                    data-testid="kg-typeahead-item"
                  >
                    {s}
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        </Field>
      </div>

      {#if searchError}
        <Message type="is-danger" data-testid="kg-search-error">
          {searchError}
        </Message>
      {/if}

      {#if isSearching}
        <div class="empty-state" data-testid="kg-search-loading">
          <div class="progress-container">
            <div class="progress-bar">
              <div class="progress-fill"></div>
            </div>
          </div>
          <p>Searching knowledge graph...</p>
        </div>
      {:else if suggestions.length > 0}
        <div class="suggestions-container" data-testid="kg-suggestions-list">
          {#each suggestions as suggestion}
            <button
              class="suggestion-item {selectedSuggestion?.term === suggestion.term ? 'is-active' : ''}"
              on:click={() => selectSuggestion(suggestion)}
              on:keydown={(e) => e.key === 'Enter' && selectSuggestion(suggestion)}
              type="button"
              data-testid="kg-suggestion-item"
            >
              <div class="suggestion-term">
                {suggestion.term}
              </div>
              <div class="suggestion-meta">
                <span class="tag is-info is-small">
                  {(suggestion.score * 100).toFixed(0)}%
                </span>
                {#if suggestion.normalized_term && suggestion.normalized_term !== suggestion.term}
                  <span class="has-text-grey">→ {suggestion.normalized_term}</span>
                {/if}
                {#if suggestion.suggestion_type}
                  <span class="tag is-small is-light">{suggestion.suggestion_type}</span>
                {/if}
              </div>
              {#if suggestion.url}
                <div class="suggestion-url">
                  {suggestion.url}
                </div>
              {/if}
            </button>
          {/each}
        </div>
      {:else if query.trim().length >= 2}
        <div class="notification is-light" data-testid="kg-search-empty">
          <p class="has-text-centered">No knowledge graph terms found for "<strong>{query}</strong>"</p>
          <p class="has-text-centered is-size-7 has-text-grey mt-2">Try different keywords or check if the role "{$role}" has a knowledge graph enabled.</p>
        </div>
      {:else}
        <div class="notification is-info is-light">
          <p class="has-text-centered">Enter at least 2 characters to search the knowledge graph</p>
          <p class="has-text-centered is-size-7 mt-2">This will search terms from the knowledge graph for role "{$role}"</p>
        </div>
      {/if}

      <div class="modal-actions">
        <div class="action-buttons">
          <Button on:click={handleClose}>
            Cancel
          </Button>

          {#if selectedSuggestion}
            <Button
              type="is-primary"
              on:click={addTermToContext}
              disabled={!selectedSuggestion}
              data-testid="kg-add-term-button"
            >
              Add "{selectedSuggestion.term}" to Context
            </Button>
          {/if}
        </div>
      </div>

      <div class="alternative-section">
        <div class="alternative-content">
          <p><strong>Alternative:</strong> Add the complete thesaurus for role "{$role}". This includes all domain-specific terms and their normalized mappings in JSON format for comprehensive vocabulary context.</p>
        </div>
        <Button
          type="is-link"
          size="is-small"
          style="width: 100%;"
          on:click={addKGIndexToContext}
          data-testid="kg-add-index-button"
        >
          Add Complete Thesaurus to Context
        </Button>
      </div>
    </div>
  </div>
</Modal>
