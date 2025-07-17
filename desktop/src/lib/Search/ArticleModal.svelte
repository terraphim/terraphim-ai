<script lang="ts">
  import { Modal } from "svelma";
  import NovelWrapper from '$lib/Editor/NovelWrapper.svelte';
  import { invoke } from '@tauri-apps/api/tauri';
  import { is_tauri } from '../stores';
  import type { Document } from "./SearchResult";
  import SvelteMarkdown from 'svelte-markdown';

  export let active: boolean = false;
  export let item: Document;
  export let initialEdit: boolean = false;
  // New props for KG context
  export let kgTerm: string | null = null;
  export let kgRank: number | null = null;
  
  let editing = false;
  let contentElement: HTMLElement;

  // Set initial edit mode when modal becomes active
  $: if (active && initialEdit) {
    editing = true;
  }

  // Whenever the modal becomes active for a given item, refresh its content from persistence.
  // Only load document if not in edit mode to avoid interfering with initialEdit
  $: if (active && item && !editing) {
    loadDocument();
  }

  // More precise HTML detection - only treat as HTML if it looks like actual HTML document structure
  $: isHtml = item?.body ? (
    // Check for common HTML document patterns, not just any < tag
    (/<html/i.test(item.body) || /<body/i.test(item.body) || /<head/i.test(item.body)) ||
    // Or if it starts with HTML-like structure (not markdown)
    /^\s*<(!DOCTYPE|html|head|body|div|p|span)/i.test(item.body.trim())
  ) : false;
  
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
    
    // Escape to exit edit mode or close modal
    if (event.key === 'Escape') {
      event.preventDefault();
      if (editing) {
        editing = false;
      } else {
        active = false; // Close the modal
      }
    }
  }
</script>

<Modal bind:active>
  <div class="box wrapper">
    <!-- Close button following Bulma styling -->
    <button class="delete is-large modal-close-btn" on:click={() => active = false} aria-label="close"></button>
    
    <!-- KG Context Header -->
    {#if kgTerm && kgRank !== null}
      <div class="kg-context">
        <h3 class="subtitle is-6">
          <span class="tag is-info">Knowledge Graph</span>
          Term: <strong>{kgTerm}</strong> | Rank: <strong>{kgRank}</strong>
        </h3>
        <hr />
      </div>
    {/if}
    
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
          <div class="markdown-content">
            <SvelteMarkdown source={item.body} />
          </div>
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
    position: relative;
    width: 100%;
    height: 100%;
    /* Remove overflow from wrapper - let the global modal handle scrolling */
  }
  
  /* Close button positioning using Bulma's delete styling */
  .modal-close-btn {
    position: absolute !important;
    top: 1rem;
    right: 1rem;
    z-index: 10;
    
    /* Enhanced hover effect that respects theme */
    &:hover {
      transform: scale(1.1);
    }
    
    &:active {
      transform: scale(0.95);
    }
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

  /* Responsive modal sizing with proper height handling - override Bulma's fixed width */
  :global(.modal-content) {
    width: 95vw !important;
    max-width: 1200px !important;
    max-height: calc(100vh - 2rem) !important;
    margin: 1rem auto !important;
    overflow-y: auto !important;
    
    /* Responsive breakpoints */
    @media (min-width: 768px) {
      width: 90vw !important;
      max-height: calc(100vh - 4rem) !important;
      margin: 2rem auto !important;
    }
    
    @media (min-width: 1024px) {
      width: 80vw !important;
      max-height: calc(100vh - 6rem) !important;
      margin: 3rem auto !important;
    }
    
    @media (min-width: 1216px) {
      width: 75vw !important;
    }
    
    @media (min-width: 1408px) {
      width: 70vw !important;
    }
  }
  
  /* Ensure modal background doesn't interfere with scrolling */
  :global(.modal) {
    padding: 0 !important;
    overflow-y: auto !important;
  }
  
  @media (max-width: 767px) {
    :global(.modal-content) {
      width: calc(100vw - 2rem) !important;
      max-height: calc(100vh - 1rem) !important;
      margin: 0.5rem auto !important;
    }
  }
  
  /* Markdown content styling */
  .markdown-content {
    line-height: 1.6;
    color: #333;
  }
  
  /* Markdown element styles with global selectors */
  .markdown-content :global(h1) {
    font-size: 2em;
    margin-bottom: 0.5em;
    font-weight: bold;
  }
  
  .markdown-content :global(h2) {
    font-size: 1.5em;
    margin-bottom: 0.5em;
    font-weight: bold;
  }
  
  .markdown-content :global(h3) {
    font-size: 1.25em;
    margin-bottom: 0.5em;
    font-weight: bold;
  }
  
  .markdown-content :global(h4) {
    font-size: 1.1em;
    margin-bottom: 0.5em;
    font-weight: bold;
  }
  
  .markdown-content :global(p) {
    margin-bottom: 1em;
  }
  
  .markdown-content :global(ul), .markdown-content :global(ol) {
    margin-bottom: 1em;
    padding-left: 2em;
  }
  
  .markdown-content :global(li) {
    margin-bottom: 0.25em;
  }
  
  .markdown-content :global(blockquote) {
    border-left: 4px solid #ddd;
    margin: 0 0 1em 0;
    padding: 0.5em 1em;
    background-color: #f9f9f9;
    font-style: italic;
  }
  
  .markdown-content :global(code) {
    background-color: #f5f5f5;
    border-radius: 3px;
    padding: 0.1em 0.3em;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
    font-size: 0.9em;
  }
  
  .markdown-content :global(pre) {
    background-color: #f5f5f5;
    border-radius: 5px;
    padding: 1em;
    margin-bottom: 1em;
    overflow-x: auto;
  }
  
  .markdown-content :global(pre code) {
    background: none;
    padding: 0;
  }
  
  .markdown-content :global(a) {
    color: #3273dc;
    text-decoration: none;
  }
  
  .markdown-content :global(a:hover) {
    text-decoration: underline;
  }
  
  .markdown-content :global(table) {
    border-collapse: collapse;
    width: 100%;
    margin-bottom: 1em;
  }
  
  .markdown-content :global(th), .markdown-content :global(td) {
    border: 1px solid #ddd;
    padding: 8px;
    text-align: left;
  }
  
  .markdown-content :global(th) {
    background-color: #f2f2f2;
    font-weight: bold;
  }
  
  .markdown-content :global(hr) {
    border: none;
    border-top: 2px solid #eee;
    margin: 2em 0;
  }
</style>
