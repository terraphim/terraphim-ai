<script lang="ts">
  import { Modal } from 'svelma';

  import type { SearchResult } from './SearchResult';
  import SvelteMarkdown from 'svelte-markdown';
  export let active: boolean = false;
  export let item: SearchResult;
  export let content = item.body;
</script>

<Modal bind:active>
  <div class="box wrapper">
    <article class="card-content content">
      <h2>{item.title}</h2>
      <!-- {@html item.body} -->

      <SvelteMarkdown source={content} />
    </article>
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
