<script lang="ts">
import { invoke } from '@tauri-apps/api/tauri';
import { Modal } from 'svelma';
// @ts-expect-error
import SvelteMarkdown from 'svelte-markdown';
import { is_tauri, role } from '$lib/stores';
import { CONFIG } from '../../config';
import NovelWrapper from '../Editor/NovelWrapper.svelte';
import type { DocumentListResponse } from '../generated/types';
import type { Document } from './SearchResult';

let {
	active = $bindable(false),
	item,
	initialEdit = false,
	kgTerm = null,
	kgRank = null,
}: {
	active?: boolean;
	item: Document;
	initialEdit?: boolean;
	kgTerm?: string | null;
	kgRank?: number | null;
} = $props();

let editing = $state(false);
let contentElement = $state<HTMLElement>();

// KG modal state (similar to ResultItem.svelte)
let _showKgModal = $state(false);
let kgDocument = $state<Document | null>(null);
let _kgTermForModal = $state<string | null>(null);
let kgRankForModal = $state<number | null>(null);
let _loadingKg = $state(false);

// Set initial edit mode when modal becomes active
$effect(() => {
	if (active && initialEdit) {
		editing = true;
	}
});

// Whenever the modal becomes active for a given item, refresh its content from persistence.
// Only load document if not in edit mode to avoid interfering with initialEdit
$effect(() => {
	if (active && item && !editing) {
		loadDocument();
	}
});

// More precise HTML detection - only treat as HTML if it looks like actual HTML document structure
let isHtml = $derived(
	item?.body
		? // Check for common HTML document patterns, not just any < tag
			/<html/i.test(item.body) ||
			/<body/i.test(item.body) ||
			/<head/i.test(item.body) ||
			// Or if it starts with HTML-like structure (not markdown)
			/^\s*<(!DOCTYPE|html|head|body|div|p|span)/i.test(item.body.trim())
		: false
);

// Determine the original format for editing to preserve it
let originalFormat = $derived(isHtml ? 'html' : 'markdown');

async function loadDocument() {
	if (!$is_tauri) return;
	try {
		const resp: any = await invoke('get_document', { documentId: item.id });
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

// Handle KG link clicks (similar to ResultItem's handleTagClick)
async function handleKgClick(term: string) {
	_loadingKg = true;
	_kgTermForModal = term;

	// Add debugging information
	console.log('🔍 KG Link Click Debug Info:');
	console.log('  Term clicked:', term);
	console.log('  Current role:', $role);
	console.log('  Is Tauri mode:', $is_tauri);

	try {
		if ($is_tauri) {
			// Use Tauri command for desktop app
			console.log('  Making Tauri invoke call...');
			console.log('  Tauri command: find_documents_for_kg_term');
			console.log('  Tauri params:', { roleName: $role, term: term });

			const response: DocumentListResponse = await invoke('find_documents_for_kg_term', {
				roleName: $role,
				term: term,
			});

			console.log('  📥 Tauri response received:');
			console.log('    Status:', response.status);
			console.log('    Results count:', response.results?.length || 0);
			console.log('    Total:', response.total || 0);
			console.log('    Full response:', JSON.stringify(response, null, 2));

			if (response.status === 'success' && response.results && response.results.length > 0) {
				// Get the first (highest-ranked) document
				kgDocument = response.results[0];
				kgRankForModal = kgDocument.rank || 0;
				console.log('  ✅ Found KG document:');
				console.log('    Title:', kgDocument?.title);
				console.log('    Rank:', kgRankForModal);
				console.log('    Body length:', kgDocument?.body?.length || 0, 'characters');
				_showKgModal = true;
			} else {
				console.warn(`  ⚠️  No KG documents found for term: "${term}" in role: "${$role}"`);
				console.warn('    This could indicate:');
				console.warn('    1. Knowledge graph not built for this role');
				console.warn('    2. Term not found in knowledge graph');
				console.warn('    3. Role not configured with TerraphimGraph relevance function');
				console.warn('    Suggestion: Check server logs for KG building status');
			}
		} else {
			// Use HTTP fetch for web mode
			console.log('  Making HTTP fetch call...');
			const baseUrl = CONFIG.ServerURL;
			const encodedRole = encodeURIComponent($role);
			const encodedTerm = encodeURIComponent(term);
			const url = `${baseUrl}/roles/${encodedRole}/kg_search?term=${encodedTerm}`;

			console.log('  📤 HTTP Request details:');
			console.log('    Base URL:', baseUrl);
			console.log('    Role (encoded):', encodedRole);
			console.log('    Term (encoded):', encodedTerm);
			console.log('    Full URL:', url);

			const response = await fetch(url);

			console.log('  📥 HTTP Response received:');
			console.log('    Status code:', response.status);
			console.log('    Status text:', response.statusText);
			console.log('    Headers:', Object.fromEntries(response.headers.entries()));

			if (!response.ok) {
				throw new Error(`HTTP error! Status: ${response.status} - ${response.statusText}`);
			}

			const data = await response.json();
			console.log('  📄 Response data:');
			console.log('    Status:', data.status);
			console.log('    Results count:', data.results?.length || 0);
			console.log('    Total:', data.total || 0);
			console.log('    Full response:', JSON.stringify(data, null, 2));

			if (data.status === 'success' && data.results && data.results.length > 0) {
				// Get the first (highest-ranked) document
				kgDocument = data.results[0];
				kgRankForModal = kgDocument?.rank || 0;
				console.log('  ✅ Found KG document:');
				console.log('    Title:', kgDocument?.title);
				console.log('    Rank:', kgRankForModal);
				console.log('    Body length:', kgDocument?.body?.length || 0, 'characters');
				_showKgModal = true;
			} else {
				console.warn(`  ⚠️  No KG documents found for term: "${term}" in role: "${$role}"`);
				console.warn('    This could indicate:');
				console.warn('    1. Server not configured with Terraphim Engineer role');
				console.warn('    2. Knowledge graph not built on server');
				console.warn('    3. Term not found in knowledge graph');
				console.warn('    Suggestion: Check server logs at startup for KG building status');
				console.warn('    API URL tested:', url);
			}
		}
	} catch (error) {
		console.error('❌ Error fetching KG document:');
		console.error('  Error type:', (error as Error).constructor.name);
		console.error('  Error message:', (error as Error).message || error);
		console.error('  Request details:', {
			term,
			role: $role,
			isTauri: $is_tauri,
			timestamp: new Date().toISOString(),
		});

		if (!$is_tauri && (error as Error).message?.includes('Failed to fetch')) {
			console.error('  💡 Network error suggestions:');
			console.error('    1. Check if server is running on expected port');
			console.error('    2. Check CORS configuration');
			console.error('    3. Verify server URL in CONFIG.ServerURL');
		}

		// Graceful fallback: could show error message or do nothing
	} finally {
		_loadingKg = false;
	}
}

// Handle clicks on KG links (kg: protocol)
function _handleContentClick(event: MouseEvent) {
	const target = event.target as HTMLElement;

	// Check if the clicked element is a link with kg: protocol
	if (target.tagName === 'A') {
		const href = target.getAttribute('href');
		if (href?.startsWith('kg:')) {
			event.preventDefault();
			const term = href.substring(3); // Remove 'kg:' prefix
			handleKgClick(term);
		}
	}
}

function _handleDoubleClick() {
	editing = true;
}

function _handleKeyDown(event: KeyboardEvent) {
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
      <NovelWrapper bind:html={item.body} outputFormat={isHtml ? 'html' : 'markdown'} />
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
        on:dblclick={_handleDoubleClick}
        on:keydown={_handleKeyDown}
        on:click={_handleContentClick}
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
          <span class="hint-text">Double-click to edit • Ctrl+E to edit • Ctrl+S to save • Click KG links to explore</span>
        </div>
      </div>
    {/if}
  </div>
</Modal>

<!-- KG document modal -->
{#if kgDocument}
  <Modal bind:active={_showKgModal}>
    <div class="box wrapper">
      <!-- Close button following Bulma styling -->
      <button class="delete is-large modal-close-btn" on:click={() => _showKgModal = false} aria-label="close"></button>

      <!-- KG Context Header -->
      {#if _kgTermForModal && kgRankForModal !== null}
        <div class="kg-context">
          <h3 class="subtitle is-6">
            <span class="tag is-info">Knowledge Graph</span>
            Term: <strong>{_kgTermForModal}</strong> | Rank: <strong>{kgRankForModal}</strong>
          </h3>
          <hr />
        </div>
      {/if}

      <h2>{kgDocument?.title}</h2>

      <div
        class="content-viewer"
        on:click={_handleContentClick}
        tabindex="0"
        role="button"
        aria-label="KG document content - click KG links to explore further"
      >
        {#if kgDocument?.body && (/<html/i.test(kgDocument.body) || /<body/i.test(kgDocument.body) || /<head/i.test(kgDocument.body))}
          <div class="prose">{@html kgDocument.body}</div>
        {:else}
          <div class="markdown-content">
            <SvelteMarkdown source={kgDocument?.body || ''} />
          </div>
        {/if}
        <div class="edit-hint">
          <span class="hint-text">Knowledge Graph document • Click KG links to explore further</span>
        </div>
      </div>
    </div>
  </Modal>
{/if}

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

  /* KG context header styling */
  .kg-context {
    margin-bottom: 1rem;
    padding: 1rem;
    background-color: #f8f9fa;
    border-radius: 6px;
    border-left: 4px solid #3273dc;

    .subtitle {
      margin-bottom: 0.5rem;
    }

    .tag {
      margin-right: 0.5rem;
    }

    hr {
      margin: 0.5rem 0 0 0;
      background-color: #dee2e6;
      height: 1px;
      border: none;
    }
  }

  /* Style KG links differently from regular links */
  .markdown-content :global(a[href^="kg:"]) {
    color: #8e44ad !important;
    font-weight: 600;
    text-decoration: none;
    border-bottom: 2px solid rgba(142, 68, 173, 0.3);
    padding: 0.1rem 0.2rem;
    border-radius: 3px;
    transition: all 0.2s ease;

    &:hover {
      background-color: rgba(142, 68, 173, 0.1);
      border-bottom-color: #8e44ad;
      text-decoration: none !important;
    }

    &:before {
      content: "🔗 ";
      opacity: 0.7;
    }
  }

  .prose :global(a[href^="kg:"]) {
    color: #8e44ad !important;
    font-weight: 600;
    text-decoration: none;
    border-bottom: 2px solid rgba(142, 68, 173, 0.3);
    padding: 0.1rem 0.2rem;
    border-radius: 3px;
    transition: all 0.2s ease;

    &:hover {
      background-color: rgba(142, 68, 173, 0.1);
      border-bottom-color: #8e44ad;
      text-decoration: none !important;
    }

    &:before {
      content: "🔗 ";
      opacity: 0.7;
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
