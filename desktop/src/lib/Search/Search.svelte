<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { Field, Input } from "svelma";
  import { input, is_tauri, role, serverUrl } from "../stores";
  import ResultItem from "./ResultItem.svelte";
  import type { Document, SearchResponse } from "./SearchResult";
  import logo from "/assets/terraphim_gray.png";

  let results: Document[] = [];
  let error: string | null = null;

  // Reactively handle search input
  $: {
    if ($input.trim()) {
      handleSearchInputEvent();
    } else {
      results = [];
      error = null;
    }
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
      } catch (error) {
        console.error("Error fetching data:", error);
        this.error = `Error fetching data: ${error}`;
      }
    }
  }
</script>
<form on:submit|preventDefault={$input}>
<Field>
  <Input
    type="search"
    bind:value={$input}
    placeholder="Search"
    icon="search"
    expanded
    autofocus
  />
</Field>
</form>
{#if error}
  <p class="error">{error}</p>
{:else if results.length}
  {#each results as result_item}
    <ResultItem document={result_item} />
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
</style>
