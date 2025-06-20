<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { Field, Input } from "svelma";
  import { input, is_tauri, role, roles, serverUrl } from "../stores";
  import ResultItem from "./ResultItem.svelte";
  import type { Document, SearchResponse } from "./SearchResult";
  import logo from "/assets/terraphim_gray.png";
  import { thesaurus,typeahead } from "../stores";

  let results: Document[] = [];
  let error: string | null = null;
  let suggestions: string[] = [];
  let suggestionIndex = -1;

  $: thesaurusEntries = Object.entries($thesaurus);

  function getSuggestions(value: string) {
    const inputValue = value.trim().toLowerCase();
    const inputLength = inputValue.length;
    
    return inputLength === 0
      ? []
      : thesaurusEntries
          .filter(([key]) => key.toLowerCase().includes(inputValue))
          .map(([key]) => key)
          .slice(0, 5);
  }

  function updateSuggestions(event: Event) {
    const inputElement = event.target as HTMLInputElement | null;
    if (!inputElement || inputElement.selectionStart == null) {
      return;
    }
    const cursorPosition = inputElement.selectionStart;
    const textBeforeCursor = $input.slice(0, cursorPosition);
    const words = textBeforeCursor.split(/\s+/);
    const currentWord = words[words.length - 1];

    suggestions = getSuggestions(currentWord);
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
    words[words.length - 1] = suggestion;
    
    $input = [...words, textAfterCursor].join(" ");
    inputElement?.setSelectionRange?.(cursorPosition + suggestion.length, cursorPosition + suggestion.length);
    suggestions = [];
    suggestionIndex = -1;
  }

  async function handleSearchInputEvent() {
    error = null; // Clear previous errors

    if ($is_tauri) {
      try {
        const response: SearchResponse = await invoke("search", {
          searchQuery: {
            search_term: $input,
            skip: 0,
            limit: 10,
            role: $role,
          },
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

      const json_body = JSON.stringify({
        search_term: $input,
        skip: 0,
        limit: 10,
        role: $role,
      });

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

<form on:submit|preventDefault={handleSearchInputEvent}>
  <Field>
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
            >
              {suggestion}
            </li>
          {/each}
        </ul>
      {/if}
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
  .input-wrapper {
    position: relative;
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
</style>