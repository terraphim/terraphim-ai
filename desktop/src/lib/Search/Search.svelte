<script lang="ts">
  import { Field, Input } from "svelma";
  import { invoke } from "@tauri-apps/api/tauri";
  import logo from "/public/assets/terraphim_gray.png";
  import { role, is_tauri, input, serverUrl } from "../stores";
  import type { SearchResult } from "./SearchResult";
  import ResultItem from "./ResultItem.svelte";
  let result: SearchResult[] = [];

  // This gets called when the search input is changed
  // or when the user clicks on the search button or input field
  async function handleSearchInputEvent() {
    if ($is_tauri) {
      console.log("Tauri config");
      console.log($input);
      await invoke("search", {
        searchQuery: {
          search_term: $input,
          skip: 0,
          limit: 10,
          role: $role,
        },
      })
        .then((data) => {
          console.log(data);
          result = data.documents;
        })
        .catch((e) => console.error(e));
    } else {
      if ($input === "") {
        // Do nothing when the input is empty
        return;
      }

      console.log("handleSearchInputEvent triggered with input", $input);

      let json_body = JSON.stringify({
        search_term: $input,
        skip: 0,
        limit: 10,
        role: $role,
      });

      console.log(
        "Sending request to server URL: ",
        $serverUrl,
        "Search body: ",
        json_body
      );

      const response = await fetch($serverUrl, {
        method: "POST",
        headers: {
          accept: "application/json",
          "Content-Type": "application/json",
        },
        body: json_body,
      });
      const data = await response.json();

      if (!response.ok) {
        if (data.status === "error") {
          console.log("No documents found:", data);
          return;
        } else {
          console.log("Unknown response from server", data);
        }
        return;
      }

      // Valid response; update the result list
      result = data.documents;
    }
  }
</script>

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
{#if result !== null && result.length !== 0}
  {#each result as result_item}
    <ResultItem item={result_item} />
  {/each}
{:else}
  <section class="section">
    <div class="content has-text-grey has-text-centered">
      <img src={logo} alt="Terraphim Logo" />
    </div>
    <div class="content has-text-grey has-text-centered">
      <p>I am Terraphim, your personal assistant.</p>
      <p />
    </div>
  </section>
{/if}

<style>
  img {
    width: 16rem;
  }
</style>
