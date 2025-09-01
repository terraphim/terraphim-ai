<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { Field, Input } from "svelma";
  import { input, is_tauri, role, roles, serverUrl } from "../stores";
  import ResultItem from "./ResultItem.svelte";
  import type { Document, SearchResponse } from "./SearchResult";
  import logo from "/assets/terraphim_gray.png";
  import { thesaurus,typeahead } from "../stores";
  import BackButton from "../BackButton.svelte";
  import { parseSearchInput, buildSearchQuery } from "./searchUtils";

  let results: Document[] = [];
  let error: string | null = null;
  let suggestions: string[] = [];
  let suggestionIndex = -1;
  let selectedOperator: 'none' | 'and' | 'or' = 'none';

  $: thesaurusEntries = Object.entries($thesaurus);

  // Helper function to get term suggestions for autocomplete
  async function getTermSuggestions(query: string): Promise<string[]> {
    try {
      if ($is_tauri) {
        const response = await invoke("get_autocomplete_suggestions", {
          query: query,
          roleName: $role,
          limit: 8
        });

        if (response.status === 'success' && response.suggestions) {
          return response.suggestions.map((suggestion: any) => suggestion.term);
        }
      } else {
        const response = await fetch(`${$serverUrl.replace('/documents/search', '')}/autocomplete/${encodeURIComponent($role)}/${encodeURIComponent(query)}`);
        if (response.ok) {
          const data = await response.json();
          if (data.status === 'success' && data.suggestions) {
            return data.suggestions.map((suggestion: any) => suggestion.term);
          }
        }
      }

      // Fallback to thesaurus matching
      return thesaurusEntries
        .filter(([key]) => key.toLowerCase().includes(query.toLowerCase()))
        .map(([key]) => key)
        .slice(0, 8);
    } catch (error) {
      console.warn('Error fetching term suggestions:', error);
      return [];
    }
  }

  async function getSuggestions(value: string): Promise<string[]> {
    const inputValue = value.trim();
    const inputLength = inputValue.length;

    // Return empty suggestions for very short inputs
    if (inputLength === 0) {
      return [];
    }

    // If user has selected an operator from UI, don't suggest text operators
    if (selectedOperator !== 'none') {
      // For UI operator selection, suggest terms for any word in input
      const words = inputValue.split(/\s+/);
      const lastWord = words[words.length - 1].toLowerCase();

      if (lastWord.length < 2) {
        return [];
      }

      return getTermSuggestions(lastWord);
    }

    // Check if user is typing after a term that could be followed by AND/OR
    const words = inputValue.split(/\s+/);
    const lastWord = words[words.length - 1].toLowerCase();

    // If the last word is a partial "and" or "or", suggest these operators
    if (words.length > 1 && selectedOperator === 'none') {
      const operatorSuggestions = [];
      if ("and".startsWith(lastWord)) {
        operatorSuggestions.push("AND");
      }
      if ("or".startsWith(lastWord)) {
        operatorSuggestions.push("OR");
      }
      if (operatorSuggestions.length > 0) {
        return operatorSuggestions;
      }
    }

    // If the input ends with "AND" or "OR", suggest terms but don't include operators
    const inputLower = inputValue.toLowerCase();
    if (inputLower.includes(" and ") || inputLower.includes(" or ")) {
      // For multi-term queries, only suggest terms after the operator
      const termAfterOperator = lastWord;
      if (termAfterOperator.length < 2) {
        return [];
      }

      return getTermSuggestions(termAfterOperator);
    }

    // Regular single-term autocomplete
    try {
      const termSuggestions = await getTermSuggestions(inputValue);

      // Add operator suggestions if we have a single term and no UI operator selected
      if (words.length === 1 && words[0].length > 2 && selectedOperator === 'none') {
        return [...termSuggestions.slice(0, 6), "AND", "OR"];
      }

      return termSuggestions;
    } catch (error) {
      console.warn('Error fetching autocomplete suggestions:', error);
      // Fall back to thesaurus-based matching
      const termSuggestions = thesaurusEntries
        .filter(([key]) => key.toLowerCase().includes(inputValue.toLowerCase()))
        .map(([key]) => key)
        .slice(0, 6);

      if (words.length === 1 && words[0].length > 2 && selectedOperator === 'none') {
        return [...termSuggestions, "AND", "OR"];
      }

      return termSuggestions;
    }
  }

  async function updateSuggestions(event: Event) {
    const inputElement = event.target as HTMLInputElement | null;
    if (!inputElement || inputElement.selectionStart == null) {
      return;
    }
    const cursorPosition = inputElement.selectionStart;
    const textBeforeCursor = $input.slice(0, cursorPosition);
    const words = textBeforeCursor.split(/\s+/);
    const currentWord = words[words.length - 1];

    // Only fetch suggestions if the current word has at least 2 characters
    if (currentWord.length >= 2) {
      try {
        suggestions = await getSuggestions(currentWord);
      } catch (error) {
        console.warn('Failed to get suggestions:', error);
        suggestions = [];
      }
    } else {
      suggestions = [];
    }
    suggestionIndex = -1;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (suggestions.length === 0) return;

    if (event.key === "ArrowDown") {
      event.preventDefault();
      suggestionIndex = (suggestionIndex + 1) % suggestions.length;
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      suggestionIndex = (suggestionIndex - 1 + suggestions.length) % suggestions.length;
    } else if ((event.key === "Enter" || event.key === "Tab") && suggestionIndex !== -1) {
      event.preventDefault();
      applySuggestion(suggestions[suggestionIndex]);
    }
  }

  function applySuggestion(suggestion: string) {
    const inputElement = document.querySelector('input[type="search"]') as HTMLInputElement;
    const cursorPosition = inputElement?.selectionStart ?? 0;
    const textBeforeCursor = $input.slice(0, cursorPosition);
    const textAfterCursor = $input.slice(cursorPosition);
    const words = textBeforeCursor.split(/\s+/);

    // Handle logical operators specially
    if (suggestion === "AND" || suggestion === "OR") {
      // If the last word is being replaced by an operator, add space after
      words[words.length - 1] = suggestion;
      $input = [...words, "", textAfterCursor].join(" ");
      const newPosition = cursorPosition + suggestion.length + 1;
      inputElement?.setSelectionRange?.(newPosition, newPosition);
    } else {
      // Regular term suggestion
      words[words.length - 1] = suggestion;
      $input = [...words, textAfterCursor].join(" ");
      const newPosition = cursorPosition + suggestion.length;
      inputElement?.setSelectionRange?.(newPosition, newPosition);
    }

    suggestions = [];
    suggestionIndex = -1;
  }

  // Create SearchQuery using shared utilities and UI operator selection
  function buildSearchQueryFromInput(): any {
    const inputText = $input.trim();
    if (!inputText) return null;

    // If user has selected an operator from UI, enforce it
    if (selectedOperator !== 'none') {
      // Split on spaces to get multiple terms
      const terms = inputText.split(/\s+/).filter(term => term.length > 0);
      if (terms.length > 1) {
        return {
          search_term: terms[0],
          search_terms: terms,
          operator: selectedOperator,
          skip: 0,
          limit: 10,
          role: $role,
        };
      } else {
        // Single term with operator selected - just search single term
        return {
          search_term: inputText,
          skip: 0,
          limit: 10,
          role: $role,
        };
      }
    }

    // Use parseSearchInput to detect operators in text
    const parsed = parseSearchInput(inputText);
    const searchQuery = buildSearchQuery(parsed, $role);

    return {
      ...searchQuery,
      skip: 0,
      limit: 10,
    };
  }

  async function handleSearchInputEvent() {
    error = null; // Clear previous errors

    if ($is_tauri) {
      if (!$input.trim()) return; // Skip if input is empty

      try {
        const searchQuery = buildSearchQueryFromInput();
        if (!searchQuery) return;

        const response: SearchResponse = await invoke("search", {
          searchQuery,
        });
        if (response.status === "success") {
          results = response.results;
          console.log("Response results");
          console.log(results);
        } else {
          error = `Search failed: ${response.status}`;
          console.error("Search failed:", response);
        }
      } catch (e) {
        error = `Error in Tauri search: ${e}`;
        console.error("Error in Tauri search:", e);
      }
    } else {
      if (!$input.trim()) return; // Skip if input is empty

      const searchQuery = buildSearchQueryFromInput();
      if (!searchQuery) return;

      const json_body = JSON.stringify(searchQuery);

      try {
        const response = await fetch($serverUrl, {
          method: "POST",
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          body: json_body,
        });
        const data: SearchResponse = await response.json();
        if (!response.ok) {
          throw new Error(`HTTP error! Status: ${response.status}`);
        }
        results = data.results;
      } catch (err) {
        console.error("Error fetching data:", err);
        error = `Error fetching data: ${err}`;
      }
    }
  }
</script>

<BackButton fallbackPath="/" />

<form on:submit|preventDefault={handleSearchInputEvent}>
  <Field>
    <div class="search-row">
      <div class="input-wrapper">
        <Input
          type="search"
          bind:value={$input}
          placeholder={$typeahead ? `Search over Knowledge graph for ${$role}` : "Search"}
          icon="search"
          expanded
          autofocus
          on:click={handleSearchInputEvent}
          on:submit={handleSearchInputEvent}
          on:keydown={handleKeydown}
          on:input={updateSuggestions}
        />
      {#if suggestions.length > 0}
        <ul class="suggestions">
          {#each suggestions as suggestion, index}
            <li
              class:active={index === suggestionIndex}
              on:click={() => applySuggestion(suggestion)}
              on:keydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  applySuggestion(suggestion);
                }
              }}
              tabindex="0"
              role="option"
              aria-selected={index === suggestionIndex}
              aria-label={`Apply suggestion: ${suggestion}`}
            >
              {suggestion}
            </li>
          {/each}
        </ul>
      {/if}
      </div>

      <div class="operator-controls">
        <div class="control">
          <label class="radio">
            <input type="radio" bind:group={selectedOperator} value="none" />
            Exact
          </label>
          <label class="radio">
            <input type="radio" bind:group={selectedOperator} value="and" />
            All (AND)
          </label>
          <label class="radio">
            <input type="radio" bind:group={selectedOperator} value="or" />
            Any (OR)
          </label>
        </div>
      </div>
    </div>
  </Field>
</form>

{#if error}
  <p class="error">{error}</p>
{:else if results.length}
  {#each results as item}
    <ResultItem document={item} />
  {/each}
{:else}
  <section class="section">
    <div class="content has-text-grey has-text-centered">
      <img src={logo} alt="Terraphim Logo" />
      <p>I am Terraphim, your personal assistant.</p>
      <button class="button is-primary" data-testid="wizard-start" on:click={() => window.location.href = '/config/wizard'}>
        <span class="icon">
          <i class="fas fa-magic"></i>
        </span>
        <span>Configuration Wizard</span>
      </button>
    </div>
  </section>
{/if}

<style>
  img {
    width: 16rem;
  }
  .error {
    color: red;
  }
  .search-row {
    display: flex;
    gap: 1rem;
    align-items: flex-start;
    width: 100%;
  }

  .input-wrapper {
    position: relative;
    flex: 1;
  }

  .operator-controls {
    flex-shrink: 0;
    min-width: 200px;
  }

  .operator-controls .control {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .operator-controls .radio {
    font-size: 0.875rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .operator-controls input[type="radio"] {
    margin: 0;
  }
  .suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 1;
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
  /* Center logo and text on empty state */
  .has-text-centered {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 40vh;
  }
</style>
