<script lang="ts">
  import { Modal } from "svelma";
  import NovelWrapper from '$lib/Editor/NovelWrapper.svelte';
  import { invoke } from '@tauri-apps/api/tauri';
  import { is_tauri } from '../stores';
  import type { Document } from "./SearchResult";

  export let active: boolean = false;
  export let item: Document;
  let editing = false;

  // Whenever the modal becomes active for a given item, refresh its content from persistence.
  $: if (active && item && !editing) {
    loadDocument();
  }

  async function loadDocument() {
    if (!$is_tauri) return;
    try {
      const resp: any = await invoke('get_document', { document_id: item.id });
      if (resp?.document) {
        item = resp.document;
      }
    } catch (e) {
      console.error('Failed to load document', e);
    }
  }

  async function saveDocument() {
    if (!$is_tauri) {
      editing = false;
      return;
    }
    try {
      await invoke('create_document', { document: item });
      editing = false;
    } catch (e) {
      console.error('Failed to save document', e);
    }
  }
</script>

<Modal bind:active>
  <div class="box wrapper">
    <h2>{item.title}</h2>

    {#if editing}
      <!-- Pass the article body as default content and bind back for updates -->
      <NovelWrapper bind:html={item.body}/>
      <button class="button is-primary" on:click={saveDocument}>
        Save
      </button>
    {:else}
      <NovelWrapper html={item.body} readOnly={true}/>
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
