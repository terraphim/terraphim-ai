<script lang="ts">
  import { Modal } from "svelma";
  import SvelteMarkdown from "svelte-markdown";
  import type { Document } from "./SearchResult";
  import NovelWrapper from '$lib/Editor/NovelWrapper.svelte';

  export let active: boolean = false;
  export let item: Document;
  let editing = false;
</script>

<Modal bind:active>
  <div class="box wrapper">
    <h2>{item.title}</h2>

    {#if editing}
      <!-- Pass the article body as default content and bind back for updates -->
      <NovelWrapper bind:html={item.body}/>
      <button class="button is-primary" on:click={() => editing = false}>
        Save
      </button>
    {:else}
      <SvelteMarkdown source={item.body} />
      <button class="button is-light" on:click={() => editing = true}>
        Edit
      </button>
    {/if}
  </div>
</Modal>

<style lang="scss">
  h2 {
    font-size: 1.5rem;
    font-weight: bold;
    margin-bottom: 2rem;
  }
  .wrapper {
    max-height: calc(100vh - 40px);
    overflow-y: auto;
  }

  /* Bulma hard-codes a modals width to 640px with no way to change it so we have to override it */
  :global(.modal-content) {
    width: min(100%, 95ch) !important;
  }
</style>
