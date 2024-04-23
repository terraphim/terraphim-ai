<script lang="ts">
  import { Field, Input } from "svelma";
  import { invoke } from "@tauri-apps/api/tauri";
  import logo from "/public/assets/terraphim_gray.png";
  import { role, is_tauri, input, serverUrl } from "../stores";
  import type { SearchResult } from "./SearchResult"; // Ensure this is defined somewhere or adjust as needed
  import ResultItem from "./ResultItem.svelte";

  interface Document {
    id: string;
    url: string;
    title: string;
    body: string;
    description?: string;
    stub?: string;
    tags?: string[];
    rank?: number;
  }

  interface SearchDocumentResponse {
    status: string;
    documents: Document[];
    total: number;
  }

  let result: Document[] = [];

  async function handleSearchInputEvent() {
    console.log("handleSearchInputEvent triggered with input", $input);

    if ($is_tauri) {
      console.log("Running search in Tauri with input:", $input);
      await invoke<SearchDocumentResponse>("search", {
        searchQuery: {
          search_term: $input,
          skip: 0,
          limit: 10,
          role: $role,
        },
      })
        .then((response) => {
          if (response.status === "success") {
            result = response.documents;
          } else {
            console.error("Search failed:", response);
          }
        })
        .catch((e) => console.error("Error in Tauri search:", e));
    } else {
      if ($input === "") return; // Skip if input is empty

      const json_body = JSON.stringify({
        search_term: $input,
        skip: 0,
        limit: 10,
        role: $role,
      });

      console.log(
        "Sending HTTP request to",
        $serverUrl,
        "with body:",
        json_body
      );

      fetch($serverUrl, {
        method: "POST",
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
        },
        body: json_body,
      })
        .then(async (response) => {
          const data: SearchDocumentResponse = await response.json();
          if (!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`);
          }
          console.log("Received data:", data);
          result = data.documents;
        })
        .catch((error) => {
          console.error("Error fetching data:", error);
        });
    }
  }
</script>

<!-- HTML and other Svelte template code -->
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
    on:keyup={(e) => e.key === "Enter" && handleSearchInputEvent()}
  />
</Field>
{#if result.length}
  {#each result as result_item}
    <ResultItem item={result_item} />
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
</style>
