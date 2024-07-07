<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { Field, Input } from "svelma";
  import { input, is_tauri, role, roles,serverUrl } from "../stores";
  import ResultItem from "./ResultItem.svelte";
  import type { Document, SearchResponse } from "./SearchResult";
  import logo from "/assets/terraphim_gray.png";
  import Typeahead from "svelte-typeahead";
  import { typeahead, thesaurus } from "../stores";
  let results: Document[] = [];
  let error: string | null = null;
  let data = [];
  // Reactively handle search input
  // $: {
  //   if ($input.trim()) {
  //     handleSearchInputEvent();
  //   } else {
  //     results = [];
  //     error = null;
  //   }
  // }
  // Enable this for typeahead component
  $: {
  if ($typeahead) {
      console.log("thesaurus", $thesaurus);
      for (const [key, value] of Object.entries($thesaurus)) {
        data.push({ label: key, value: value });
      }
      console.log("data", data);
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
      } catch (error) {
        console.error("Error fetching data:", error);
        this.error = `Error fetching data: ${error}`;
      }
    }
  }
  // const extract = (item) => item.state;
</script>
<form on:submit|preventDefault={$input}>
{#if $typeahead}
<Field>
  <Typeahead type="search" icon="search" bind:value={$input} label="Search" placeholder={`Search over Knowledge graph for ${$role}`} {data} extract={(item) => item.label}
    on:select={handleSearchInputEvent}
    on:click={handleSearchInputEvent}
    on:submit={handleSearchInputEvent}
    on:keyup={e => e.key === 'Enter' && handleSearchInputEvent()}/>
  </Field>
{:else}
<Field>
  <Input
    type="search"
    bind:value={$input}
    placeholder="Search"
    icon="search"
    expanded
    autofocus
    on:click={handleSearchInputEvent}
    on:submit={handleSearchInputEvent}
    on:keyup={e => e.key === 'Enter' && handleSearchInputEvent()}
  />
</Field>
{/if}
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

<!-- key value for thesaurus if typeahead is true -->
<!-- {#if $typeahead}
  <div>
    {#each Object.entries($thesaurus) as [key, value]}
      <div>{key}: {value.id}</div>
    {/each}
  </div>
{/if} -->
<style>
  img {
    width: 16rem;
  }
  .error {
    color: red;
  }
  :global([data-svelte-typeahead]) {

      padding-left: 2.5em;


      box-shadow: inset 0 .0625em .125em rgba(10,10,10,.05);
      width: 100%;


      max-width: 100%;


      background-color: #fff;
      border-color: #dbdbdb;
      border-radius: 4px;
      color: #363636;
  }

</style>
