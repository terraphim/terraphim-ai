<script lang="ts">
  import { Modal, Field, Input, Button, Message, Loading } from "svelma";
  import { invoke } from "@tauri-apps/api/tauri";
  import { is_tauri, role, serverUrl } from "../stores";
  import { createEventDispatcher, onMount, onDestroy } from "svelte";

  export let active: boolean = false;
  export let initialQuery: string = "";

  const dispatch = createEventDispatcher();

  // Search state
  let query: string = "";
  let suggestions: KGSuggestion[] = [];
  let isSearching = false;
  let searchError: string | null = null;
  let selectedSuggestion: KGSuggestion | null = null;
  let searchTimeout: number | null = null;

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

  // Initialize query when modal opens
  $: if (active && initialQuery !== query) {
    query = initialQuery;
    if (query.trim()) {
      searchKGTerms();
    }
  }

  // Focus input when modal opens
  $: if (active && searchInput) {
    setTimeout(() => {
      searchInput?.focus();
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
        // Web mode - would need a corresponding HTTP endpoint
        searchError = 'KG search not available in web mode';
        suggestions = [];
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

    try {
      if ($is_tauri) {
        await invoke("add_kg_term_context", {
          request: {
            conversation_id: "default", // TODO: Get actual conversation ID
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
        searchError = 'Context management not available in web mode';
      }
    } catch (error) {
      console.error('Error adding term to context:', error);
      searchError = `Failed to add term to context: ${error}`;
    }
  }

  // Add entire KG index to context
  async function addKGIndexToContext() {
    try {
      if ($is_tauri) {
        await invoke("add_kg_index_context", {
          request: {
            conversation_id: "default", // TODO: Get actual conversation ID
            role_name: $role
          }
        });

        dispatch("kgIndexAdded", { role: $role });

        // Close modal after successful addition
        handleClose();
      } else {
        searchError = 'Context management not available in web mode';
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

<style>
  .kg-search-container {
    max-height: 60vh;
    display: flex;
    flex-direction: column;
  }

  .suggestions-list {
    flex: 1;
    overflow-y: auto;
    max-height: 300px;
    border: 1px solid #e1e1e1;
    border-radius: 4px;
    margin: 0.5rem 0;
  }

  .suggestion-item {
    padding: 0.75rem;
    cursor: pointer;
    border-bottom: 1px solid #f5f5f5;
    transition: background-color 0.2s;
  }

  .suggestion-item:hover,
  .suggestion-item.selected {
    background-color: #f0f8ff;
  }

  .suggestion-item:last-child {
    border-bottom: none;
  }

  .suggestion-term {
    font-weight: 600;
    color: #363636;
    margin-bottom: 0.25rem;
  }

  .suggestion-meta {
    font-size: 0.875rem;
    color: #757575;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .suggestion-score {
    background: #e8f4fd;
    color: #1976d2;
    padding: 0.125rem 0.375rem;
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .suggestion-url {
    font-style: italic;
    font-size: 0.75rem;
    color: #9e9e9e;
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: #757575;
  }

  .search-header {
    margin-bottom: 1rem;
  }

  .search-actions {
    margin-top: 1rem;
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }

  .kg-index-action {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid #e1e1e1;
  }

  .kg-index-description {
    font-size: 0.875rem;
    color: #757575;
    margin-bottom: 0.5rem;
  }
</style>

<Modal bind:active on:close={handleClose}>
  <div class="kg-search-container" on:keydown={handleKeydown}>
    <div class="search-header">
      <h3 class="title is-4">Knowledge Graph Search</h3>
      <p class="subtitle is-6">Search and add terms from the knowledge graph to your context</p>
    </div>

    <Field>
      <Input
        bind:element={searchInput}
        bind:value={query}
        on:input={handleQueryChange}
        placeholder="Search knowledge graph terms..."
        type="search"
        disabled={isSearching}
      />
    </Field>

    {#if searchError}
      <Message type="is-danger">
        {searchError}
      </Message>
    {/if}

    {#if isSearching}
      <div class="empty-state">
        <Loading />
        <p>Searching knowledge graph...</p>
      </div>
    {:else if suggestions.length > 0}
      <div class="suggestions-list">
        {#each suggestions as suggestion}
          <div
            class="suggestion-item {selectedSuggestion?.term === suggestion.term ? 'selected' : ''}"
            on:click={() => selectSuggestion(suggestion)}
            on:keydown={(e) => e.key === 'Enter' && selectSuggestion(suggestion)}
            role="button"
            tabindex="0"
          >
            <div class="suggestion-term">
              {suggestion.term}
            </div>
            <div class="suggestion-meta">
              <span class="suggestion-score">
                Score: {(suggestion.score * 100).toFixed(0)}%
              </span>
              {#if suggestion.normalized_term && suggestion.normalized_term !== suggestion.term}
                <span>→ {suggestion.normalized_term}</span>
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
          </div>
        {/each}
      </div>
    {:else if query.trim().length >= 2}
      <div class="empty-state">
        <p>No knowledge graph terms found for "{query}"</p>
        <p class="is-size-7">Try different keywords or check if the role has a knowledge graph enabled.</p>
      </div>
    {:else}
      <div class="empty-state">
        <p>Enter at least 2 characters to search the knowledge graph</p>
      </div>
    {/if}

    <div class="search-actions">
      <Button on:click={handleClose}>
        Cancel
      </Button>

      {#if selectedSuggestion}
        <Button
          type="is-primary"
          on:click={addTermToContext}
          disabled={!selectedSuggestion}
        >
          Add "{selectedSuggestion.term}" to Context
        </Button>
      {/if}
    </div>

    <div class="kg-index-action">
      <div class="kg-index-description">
        Add the entire knowledge graph index for role "{$role}" to provide comprehensive context information.
      </div>
      <Button
        type="is-link"
        size="is-small"
        on:click={addKGIndexToContext}
      >
        Add Complete KG Index to Context
      </Button>
    </div>
  </div>
</Modal>
