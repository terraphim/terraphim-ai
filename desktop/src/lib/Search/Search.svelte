<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { Field, Input, Taglist, Tag } from "svelma";
  import { input, is_tauri, role, roles, serverUrl } from "../stores";
  import ResultItem from "./ResultItem.svelte";
  import type { Document, SearchResponse } from "./SearchResult";
  import logo from "/assets/terraphim_gray.png";
  import { thesaurus,typeahead } from "../stores";
  import BackButton from "../BackButton.svelte";
  import { parseSearchInput, buildSearchQuery } from "./searchUtils";
  import TermChip from "./TermChip.svelte";

  let results: Document[] = [];
  let error: string | null = null;
  let suggestions: string[] = [];
  let suggestionIndex = -1;
  let selectedOperator: 'none' | 'and' | 'or' = 'none';

  // Term chips state
  interface SelectedTerm {
    value: string;
    isFromKG: boolean;
  }
  let selectedTerms: SelectedTerm[] = [];
  let currentLogicalOperator: 'AND' | 'OR' | null = null;

  $: thesaurusEntries = Object.entries($thesaurus);

  // State to prevent circular updates
  let isUpdatingFromChips = false;

  // Reactive statement to parse input and update chips when user types
  // Only parse when input contains operators to avoid constant parsing
  $: if ($input && !isUpdatingFromChips && ($input.includes(' AND ') || $input.includes(' OR ') || $input.includes(' and ') || $input.includes(' or '))) {
    parseAndUpdateChips($input);
  }

  // Function to parse input and update chips
  function parseAndUpdateChips(inputText: string) {
    const parsed = parseSearchInput(inputText);
    
    if (parsed.hasOperator && parsed.terms.length > 1) {
      const newSelectedTerms = parsed.terms.map(term => {
        const isFromKG = thesaurusEntries.some(([key]) => key.toLowerCase() === term.toLowerCase());
        return { value: term, isFromKG };
      });
      
      // Only update if the terms have actually changed
      const currentTermValues = selectedTerms.map(t => t.value);
      const newTermValues = newSelectedTerms.map(t => t.value);
      
      if (JSON.stringify(currentTermValues) !== JSON.stringify(newTermValues) || 
          currentLogicalOperator !== parsed.operator) {
        selectedTerms = newSelectedTerms;
        currentLogicalOperator = parsed.operator;
      }
    } else if (parsed.terms.length === 1 && selectedTerms.length > 0) {
      // Single term - clear chips if they exist
      selectedTerms = [];
      currentLogicalOperator = null;
    } else if (parsed.terms.length === 0) {
      // Empty input - clear everything
      selectedTerms = [];
      currentLogicalOperator = null;
    }
  }

  // Helper function to get term suggestions for autocomplete
  async function getTermSuggestions(query: string): Promise<string[]> {
    try {
      if ($is_tauri) {
        const response: any = await invoke("get_autocomplete_suggestions", {
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

    // Parse the input to detect operators and terms
    const parsed = parseSearchInput(inputValue);
    const words = inputValue.split(/\s+/);
    const lastWord = words[words.length - 1].toLowerCase();

    // If user has selected an operator from UI, don't suggest text operators
    if (selectedOperator !== 'none') {
      // For UI operator selection, suggest terms for any word in input
      if (lastWord.length < 2) {
        return [];
      }
      return getTermSuggestions(lastWord);
    }

    // If we have operators in the input, prioritize term suggestions after operators
    if (parsed.hasOperator && parsed.terms.length > 0) {
      // Get the last term (what user is currently typing)
      const currentTerm = parsed.terms[parsed.terms.length - 1];
      if (currentTerm && currentTerm.length >= 2) {
        return getTermSuggestions(currentTerm);
      }
      return [];
    }

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
    if (inputLower.includes(" and ") || inputLower.includes(" or ") || 
        inputLower.includes(" AND ") || inputLower.includes(" OR ")) {
      // For multi-term queries, only suggest terms after the operator
      const termAfterOperator = lastWord;
      if (termAfterOperator.length < 2) {
        return [];
      }
      return getTermSuggestions(termAfterOperator);
    }

    // Regular single-term autocomplete with enhanced operator suggestions
    try {
      const termSuggestions = await getTermSuggestions(inputValue);

      // Add operator suggestions if we have a single term and no UI operator selected
      if (words.length === 1 && words[0].length > 2 && selectedOperator === 'none') {
        // Prioritize capitalized operators
        return [...termSuggestions.slice(0, 5), "AND", "OR"];
      }

      return termSuggestions;
    } catch (error) {
      console.warn('Error fetching autocomplete suggestions:', error);
      // Fall back to thesaurus-based matching
      const termSuggestions = thesaurusEntries
        .filter(([key]) => key.toLowerCase().includes(inputValue.toLowerCase()))
        .map(([key]) => key)
        .slice(0, 5);

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

    // Check if user is typing an operator
    if (currentWord.toLowerCase() === 'a' || currentWord.toLowerCase() === 'an') {
      suggestions = ['AND'];
    } else if (currentWord.toLowerCase() === 'o' || currentWord.toLowerCase() === 'or') {
      suggestions = ['OR'];
    } else if (currentWord.length >= 2) {
      // Get term suggestions for longer words
      try {
        const termSuggestions = await getSuggestions(currentWord);
        
        // Add operator suggestions if we have existing terms
        if (words.length > 1 && !textBeforeCursor.includes(' AND ') && !textBeforeCursor.includes(' OR ')) {
          suggestions = [...termSuggestions, 'AND', 'OR'];
        } else {
          suggestions = termSuggestions;
        }
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
    if (suggestions.length > 0) {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        suggestionIndex = (suggestionIndex + 1) % suggestions.length;
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        suggestionIndex = (suggestionIndex - 1 + suggestions.length) % suggestions.length;
      } else if ((event.key === "Enter" || event.key === "Tab") && suggestionIndex !== -1) {
        event.preventDefault();
        applySuggestion(suggestions[suggestionIndex]);
      } else if (event.key === "Escape") {
        event.preventDefault();
        suggestions = [];
        suggestionIndex = -1;
      }
    } else if (event.key === "Enter") {
      event.preventDefault();
      parseAndSearch();
    }
  }

  // Combined function to parse input into chips and then search
  function parseAndSearch() {
    // Force parsing of the current input
    parseAndUpdateChips($input);
    // Then perform search
    handleSearchInputEvent();
  }

  function applySuggestion(suggestion: string) {
    if (suggestion === 'AND' || suggestion === 'OR') {
      // If it's an operator, set it for the next term
      currentLogicalOperator = suggestion as 'AND' | 'OR';

      // Parse current input to add current term if not already added
      const parsed = parseSearchInput($input);
      if (parsed.terms.length > 0 && !selectedTerms.some(t => t.value === parsed.terms[parsed.terms.length - 1])) {
        addSelectedTerm(parsed.terms[parsed.terms.length - 1]);
      }

      $input = $input + ` ${suggestion} `;
    } else {
      // It's a term suggestion - replace the current partial term
      const words = $input.trim().split(/\s+/);
      const lastWord = words[words.length - 1];
      
      // If the last word is a partial match for the suggestion, replace it
      if (suggestion.toLowerCase().startsWith(lastWord.toLowerCase())) {
        // Replace the last word with the full suggestion
        words[words.length - 1] = suggestion;
        $input = words.join(' ');
        
        // Trigger parsing to update chips with the new input
        parseAndUpdateChips($input);
      } else {
        // If no partial match, add as new term
        addSelectedTerm(suggestion, currentLogicalOperator);
      }
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
      // First parse the input to remove any text operators and get clean terms
      const parsed = parseSearchInput(inputText);
      
      // If parsing found operators, use those terms; otherwise split on spaces
      const terms = parsed.hasOperator ? parsed.terms : inputText.split(/\s+/).filter(term => term.length > 0);
      
      if (terms.length > 1) {
        // Use shared utility with UI operator override
        const fakeParser = {
          hasOperator: true,
          operator: (selectedOperator === 'and' ? 'AND' : 'OR') as 'AND' | 'OR',
          terms: terms,
          originalQuery: inputText,
        };
        const searchQuery = buildSearchQuery(fakeParser, $role);
        return {
          ...searchQuery,
          skip: 0,
          limit: 10,
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

  // Term chips management
  function addSelectedTerm(term: string, operator: 'AND' | 'OR' | null = null) {
    // Check if term is from thesaurus (KG)
    const isFromKG = thesaurusEntries.some(([key]) => key.toLowerCase() === term.toLowerCase());

    // Don't add if already selected
    if (selectedTerms.some(t => t.value.toLowerCase() === term.toLowerCase())) {
      return;
    }

    selectedTerms = [...selectedTerms, { value: term, isFromKG }];

    if (operator && selectedTerms.length > 1) {
      currentLogicalOperator = operator;
    }

    // Update the input to show the structured query
    updateInputFromSelectedTerms();
  }

  function removeSelectedTerm(term: string) {
    selectedTerms = selectedTerms.filter(t => t.value !== term);
    updateInputFromSelectedTerms();
  }

  function updateInputFromSelectedTerms() {
    isUpdatingFromChips = true;
    
    if (selectedTerms.length === 0) {
      $input = '';
      currentLogicalOperator = null;
    } else if (selectedTerms.length === 1) {
      $input = selectedTerms[0].value;
      currentLogicalOperator = null;
    } else {
      const operator = currentLogicalOperator || 'AND';
      $input = selectedTerms.map(t => t.value).join(` ${operator} `);
    }
    
    // Reset the flag after a brief delay to allow reactivity to settle
    setTimeout(() => {
      isUpdatingFromChips = false;
    }, 10);
  }

  function clearSelectedTerms() {
    selectedTerms = [];
    currentLogicalOperator = null;
    $input = '';
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

      <!-- Selected terms display -->
      {#if selectedTerms.length > 0}
        <div class="selected-terms-section">
          <Taglist>
            {#each selectedTerms as term, index}
              <div class="term-tag-wrapper" class:from-kg={term.isFromKG}>
                <Tag 
                  rounded 
                  on:click={() => removeSelectedTerm(term.value)}
                  title="Click to remove term"
                >
                  {term.value}
                  <button 
                    class="remove-tag-btn" 
                    on:click|stopPropagation={() => removeSelectedTerm(term.value)}
                    aria-label={`Remove term: ${term.value}`}
                  >
                    ×
                  </button>
                </Tag>
              </div>
              {#if index < selectedTerms.length - 1}
                <div class="operator-tag-wrapper">
                  <Tag class="operator-tag" rounded>
                    {currentLogicalOperator || 'AND'}
                  </Tag>
                </div>
              {/if}
            {/each}
          </Taglist>
          <button type="button" class="clear-terms-btn" on:click={clearSelectedTerms}>
            Clear all
          </button>
        </div>
      {/if}

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
        
        <!-- Parse button for manual parsing -->
        {#if $input && ($input.includes(' AND ') || $input.includes(' OR ')) && selectedTerms.length === 0}
          <button type="button" class="button is-small is-info" on:click={() => parseAndUpdateChips($input)}>
            Parse Terms
          </button>
        {/if}
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

  /* Selected terms section */
  .selected-terms-section {
    margin-top: 0.5rem;
    padding: 0.5rem;
    background: rgba(0, 0, 0, 0.02);
    border-radius: 4px;
    border: 1px solid #e0e0e0;
  }

  .term-tag-wrapper {
    cursor: pointer;
    transition: all 0.2s ease;
    position: relative;
    display: inline-block;
  }

  .term-tag-wrapper:hover {
    opacity: 0.8;
    transform: scale(1.02);
  }

  .term-tag-wrapper.from-kg :global(.tag) {
    background-color: #3273dc;
    color: white;
  }

  .term-tag-wrapper.from-kg:hover :global(.tag) {
    background-color: #2366d1;
  }

  .remove-tag-btn {
    position: absolute;
    right: 0.25rem;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: inherit;
    font-size: 0.8rem;
    font-weight: bold;
    cursor: pointer;
    padding: 0;
    width: 1rem;
    height: 1rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    transition: background-color 0.2s ease;
  }

  .remove-tag-btn:hover {
    background-color: rgba(0, 0, 0, 0.1);
  }

  .operator-tag-wrapper :global(.tag) {
    background-color: #f5f5f5;
    color: #666;
    font-weight: 600;
    cursor: default;
  }

  .clear-terms-btn {
    margin-top: 0.5rem;
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    background: #f5f5f5;
    border: 1px solid #ddd;
    border-radius: 3px;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }

  .clear-terms-btn:hover {
    background: #e0e0e0;
  }
</style>
