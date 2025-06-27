<script lang="ts">
  import { Modal } from "svelma";
  import NovelWrapper from '$lib/Editor/NovelWrapper.svelte';
  import { invoke } from '@tauri-apps/api/tauri';
  import { is_tauri } from '../stores';
  import type { Document } from "./SearchResult";
  import SvelteMarkdown from 'svelte-markdown';

  export let active: boolean = false;
  export let item: Document;
  let editing = false;
  let contentElement: HTMLElement;

  // Whenever the modal becomes active for a given item, refresh its content from persistence.
  $: if (active && item && !editing) {
    loadDocument();
  }

  $: isHtml = /<\w+/.test(item?.body ?? '');
  // Determine the original format for editing to preserve it
  $: originalFormat = isHtml ? 'html' : 'markdown';

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

  function handleDoubleClick() {
    editing = true;
  }

  function handleKeyDown(event: KeyboardEvent) {
    // Allow double-click to work by not preventing default on double-click events
    if (event.type === 'dblclick') {
      return;
    }
    
    // Enable editing with Ctrl+E or Cmd+E
    if ((event.ctrlKey || event.metaKey) && event.key === 'e') {
      event.preventDefault();
      editing = true;
    }
    
    // Save with Ctrl+S or Cmd+S when editing
    if (editing && (event.ctrlKey || event.metaKey) && event.key === 's') {
      event.preventDefault();
      saveDocument();
    }
    
    // Escape to exit edit mode
    if (editing && event.key === 'Escape') {
      event.preventDefault();
      editing = false;
    }
  }
</script>

<Modal bind:active>
  <div class="box wrapper">
    <h2>{item.title}</h2>

    {#if editing}
      <!-- Pass the article body as default content and bind back for updates -->
      <NovelWrapper bind:html={item.body} outputFormat={originalFormat}/>
      <div class="edit-controls">
        <button class="button is-primary" on:click={saveDocument}>
          Save
        </button>
        <button class="button is-light" on:click={() => editing = false}>
          Cancel
        </button>
      </div>
    {:else}
      <div 
        class="content-viewer"
        bind:this={contentElement}
        on:dblclick={handleDoubleClick}
        on:keydown={handleKeyDown}
        tabindex="0"
        role="button"
        aria-label="Double-click to edit article content"
      >
        {#if isHtml}
          <div class="prose">{@html item.body}</div>
        {:else}
          <SvelteMarkdown source={item.body} />
        {/if}
        <div class="edit-hint">
          <span class="hint-text">Double-click to edit • Ctrl+E to edit • Ctrl+S to save</span>
        </div>
      </div>
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

  .content-viewer {
    position: relative;
    cursor: pointer;
    border: 2px solid transparent;
    border-radius: 4px;
    transition: border-color 0.2s ease, background-color 0.2s ease;
    
    &:hover {
      border-color: #f0f0f0;
      background-color: #fafafa;
    }
    
    &:focus {
      outline: none;
      border-color: #3273dc;
      background-color: #f5f5f5;
    }
  }

  .edit-hint {
    margin-top: 1rem;
    padding: 0.5rem;
    background-color: #f5f5f5;
    border-radius: 4px;
    text-align: center;
    
    .hint-text {
      font-size: 0.875rem;
      color: #666;
      font-style: italic;
    }
  }

  .edit-controls {
    margin-top: 1rem;
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }

  /* Bulma hard-codes a modals width to 640px with no way to change it so we have to override it */
  :global(.modal-content) {
    width: min(100%, 95ch) !important;
  }
</style>
