<script lang="ts">
<<<<<<< HEAD
  import { Modal } from "svelma";
  import { createEventDispatcher } from 'svelte';
  import type { ContextItem, ContextMetadata } from './types';
=======
import { createEventDispatcher } from 'svelte';
import { Modal } from 'svelma';
import type { ContextItem } from './Chat.svelte';
>>>>>>> origin/main

export let active: boolean = false;
export let context: ContextItem | null = null;
export let mode: 'create' | 'edit' = 'edit';

const dispatch = createEventDispatcher();

<<<<<<< HEAD
  // Form data
  type EditableContext = ContextItem & { metadata: ContextMetadata };

  let editingContext: EditableContext | null = null;
  
  // Form state variables
  let contextType = 'UserInput';
  let title = '';
  let summary = '';
  let content = '';
  
  // Update form values when editingContext changes
  $: if (editingContext) {
    contextType = editingContext.context_type || 'UserInput';
    title = editingContext.title || '';
    summary = editingContext.summary || '';
    content = editingContext.content || '';
  }
  
  // Event handlers to update editingContext
  function updateContextType(event: Event) {
    const target = event.target as HTMLSelectElement;
    contextType = target.value;
    if (editingContext) {
      editingContext.context_type = contextType;
    }
  }
  
  function updateTitle(event: Event) {
    const target = event.target as HTMLInputElement;
    title = target.value;
    if (editingContext) {
      editingContext.title = title;
    }
  }
  
  function updateSummary(event: Event) {
    const target = event.target as HTMLTextAreaElement;
    summary = target.value;
    if (editingContext) {
      editingContext.summary = summary;
    }
  }
  
  function updateContent(event: Event) {
    const target = event.target as HTMLTextAreaElement;
    content = target.value;
    if (editingContext) {
      editingContext.content = content;
    }
  }
  let contextTypeOptions = [
    { value: 'Document', label: 'Document' },
    { value: 'SearchResult', label: 'Search Result' },
    { value: 'UserInput', label: 'User Input' },
    { value: 'System', label: 'System' },
    { value: 'External', label: 'External' }
  ];

  // Initialize form data when modal becomes active
  $: if (active) {
    if (context) {
      editingContext = {
        ...context,
        metadata: { ...(context.metadata ?? {}) }
      };
    } else if (mode === 'create') {
      editingContext = {
        id: '',
        context_type: 'UserInput',
        title: '',
        summary: '',
        content: '',
        metadata: {},
        created_at: new Date().toISOString(),
        relevance_score: undefined
      };
    }
  } else {
    editingContext = null;
  }

  // Validation
  $: isValid = editingContext &&
    title.trim() !== '' &&
    content.trim() !== '';
=======
// Form data
let editingContext: ContextItem | null = null;
const _contextTypeOptions = [
	{ value: 'Document', label: 'Document' },
	{ value: 'SearchResult', label: 'Search Result' },
	{ value: 'UserInput', label: 'User Input' },
	{ value: 'System', label: 'System' },
	{ value: 'External', label: 'External' },
];

// Initialize form data when modal becomes active
$: if (active && context) {
	editingContext = {
		...context,
		// Ensure we have a proper copy
		metadata: { ...context.metadata },
	};
} else if (active && mode === 'create') {
	// Initialize new context item
	editingContext = {
		id: '',
		context_type: 'UserInput',
		title: '',
		summary: '',
		content: '',
		metadata: {},
		created_at: new Date().toISOString(),
		relevance_score: undefined,
	};
}

// Validation
$: isValid =
	editingContext && editingContext.title.trim() !== '' && editingContext.content.trim() !== '';
>>>>>>> origin/main

function handleClose() {
	active = false;
	editingContext = null;
	dispatch('close');
}

function handleSave() {
	if (!isValid || !editingContext) return;

	if (mode === 'edit') {
		dispatch('update', editingContext);
	} else {
		dispatch('create', editingContext);
	}

	handleClose();
}

<<<<<<< HEAD
  function handleMetadataInput(key: string, value: string) {
    if (!editingContext) return;
    const metadata: ContextMetadata = { ...editingContext.metadata };
    metadata[key] = value;
    editingContext = { ...editingContext, metadata };
  }

  function handleMetadataKeyInput(event: Event, originalKey: string, currentValue: string) {
    if (!editingContext) return;
    const target = event.target as HTMLInputElement | null;
    if (!target) return;
    const newKey = target.value;
    const metadata: ContextMetadata = { ...editingContext.metadata };
    delete metadata[originalKey];
    metadata[newKey] = currentValue;
    editingContext = { ...editingContext, metadata };
  }

  function handleMetadataValueInput(event: Event, key: string) {
    if (!editingContext) return;
    const target = event.target as HTMLInputElement | null;
    if (!target) return;
    handleMetadataInput(key, target.value);
  }

  function removeMetadataEntry(key: string) {
    if (!editingContext) return;
    const metadata: ContextMetadata = { ...editingContext.metadata };
    delete metadata[key];
    editingContext = { ...editingContext, metadata };
  }

  function addMetadataEntry() {
    if (!editingContext) return;
    editingContext = {
      ...editingContext,
      metadata: {
        ...editingContext.metadata,
        [`key_${Date.now()}`]: ''
      }
    };
  }

  function handleDelete() {
    if (mode === 'edit' && context) {
      dispatch('delete', context.id);
      handleClose();
    }
  }
=======
function _handleDelete() {
	if (mode === 'edit' && context) {
		dispatch('delete', context.id);
		handleClose();
	}
}
>>>>>>> origin/main

// Handle keyboard shortcuts
function _handleKeydown(event: KeyboardEvent) {
	if (event.key === 'Escape') {
		handleClose();
	} else if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
		handleSave();
	}
}
</script>

<svelte:window on:keydown={_handleKeydown} />

<Modal {active} on:close={handleClose}>
  <div class="modal-card">
    <header class="modal-card-head">
      <p class="modal-card-title">
        {mode === 'edit' ? 'Edit Context Item' : 'Add Context Item'}
      </p>
      <button class="delete" aria-label="close" on:click={handleClose}></button>
    </header>

    {#if editingContext}
      <section class="modal-card-body">
        <!-- Context Type -->
        <div class="field">
          <label class="label" for="context-type">Type</label>
          <div class="control">
            <div class="select is-fullwidth">
              <select
                id="context-type"
                bind:value={contextType}
                on:change={updateContextType}
                data-testid="context-type-select"
              >
                {#each _contextTypeOptions as option}
                  <option value={option.value}>{option.label}</option>
                {/each}
              </select>
            </div>
          </div>
        </div>

        <!-- Title -->
        <div class="field">
          <label class="label" for="context-title">Title *</label>
          <div class="control">
            <input
              id="context-title"
              class="input"
              type="text"
              placeholder="Enter title..."
              bind:value={title}
              on:input={updateTitle}
              data-testid="context-title-input"
              required
            >
          </div>
          {#if title.trim() === ''}
            <p class="help is-danger">Title is required</p>
          {/if}
        </div>

        <!-- Summary -->
        <div class="field">
          <label class="label" for="context-summary">Summary</label>
          <div class="control">
            <textarea
              id="context-summary"
              class="textarea"
              placeholder="Brief summary of the content (optional)..."
              bind:value={summary}
              on:input={updateSummary}
              data-testid="context-summary-textarea"
              rows="3"
              maxlength="500"
            ></textarea>
          </div>
          <p class="help">
            {#if summary}
              {summary.length}/500 characters
            {:else}
              Optional brief summary (up to 500 characters)
            {/if}
          </p>
        </div>

        <!-- Content -->
        <div class="field">
          <label class="label" for="context-content">Content *</label>
          <div class="control">
            <textarea
              id="context-content"
              class="textarea"
              placeholder="Enter the full content..."
              bind:value={content}
              on:input={updateContent}
              data-testid="context-content-textarea"
              rows="8"
              required
            ></textarea>
          </div>
          {#if content.trim() === ''}
            <p class="help is-danger">Content is required</p>
          {/if}
        </div>

        <!-- Metadata (optional advanced section) -->
        <details class="details">
          <summary class="summary">
            <span class="icon-text">
              <span class="icon">
                <i class="fas fa-cog"></i>
              </span>
              <span>Advanced Options</span>
            </span>
          </summary>

          <!-- Metadata editing temporarily disabled for build -->
          <div class="field mt-4">
            <p class="label">Metadata</p>
            <div class="content">
<<<<<<< HEAD
              <p class="help">Additional key-value pairs for this context item</p>

              {#if Object.keys(editingContext?.metadata || {}).length === 0}
                <p class="has-text-grey-light">No metadata defined</p>
              {:else}
                {#each Object.entries(editingContext?.metadata || {}) as [key, value], index}
                  <div class="field is-grouped">
                    <div class="control is-expanded">
                      <input
                        class="input is-small"
                        type="text"
                        placeholder="Key"
                        value={key}
                        on:input={(event) => handleMetadataKeyInput(event, key, value)}
                      >
                    </div>
                    <div class="control is-expanded">
                      <input
                        class="input is-small"
                        type="text"
                        placeholder="Value"
                        value={editingContext?.metadata?.[key] || ''}
                        on:input={(event) => handleMetadataValueInput(event, key)}
                      >
                    </div>
                    <div class="control">
                      <button
                        class="button is-small is-danger is-outlined"
                        aria-label="Remove metadata entry"
                        on:click={() => removeMetadataEntry(key)}
                      >
                        <span class="icon">
                          <i class="fas fa-times"></i>
                        </span>
                      </button>
                    </div>
                  </div>
                {/each}
              {/if}

              <button
                class="button is-small is-light"
                on:click={addMetadataEntry}
              >
                <span class="icon">
                  <i class="fas fa-plus"></i>
                </span>
                <span>Add Metadata</span>
              </button>
=======
              <p class="help">Metadata editing temporarily disabled</p>
              <pre>{JSON.stringify(editingContext?.metadata || {}, null, 2)}</pre>
>>>>>>> origin/main
            </div>
          </div>
        </details>
      </section>

      <footer class="modal-card-foot">
        <div class="field is-grouped">
          <div class="control">
            <button
              class="button is-primary"
              on:click={handleSave}
              disabled={!isValid}
              data-testid="save-context-button"
            >
              <span class="icon">
                <i class="fas fa-save"></i>
              </span>
              <span>{mode === 'edit' ? 'Save Changes' : 'Add Context'}</span>
            </button>
          </div>

          <div class="control">
            <button
              class="button is-light"
              on:click={handleClose}
              data-testid="cancel-context-button"
            >
              <span>Cancel</span>
            </button>
          </div>

          {#if mode === 'edit'}
            <div class="control is-expanded"></div>
            <div class="control">
              <button
                class="button is-danger is-outlined"
                on:click={_handleDelete}
                data-testid="delete-context-button"
              >
                <span class="icon">
                  <i class="fas fa-trash"></i>
                </span>
                <span>Delete</span>
              </button>
            </div>
          {/if}
        </div>

        <div class="help">
          <strong>Keyboard shortcuts:</strong>
          Ctrl/Cmd + Enter to save, Escape to close
        </div>
      </footer>
    {/if}
  </div>
</Modal>

<style>
  .details {
    margin-top: 1rem;
  }

  .summary {
    cursor: pointer;
    padding: 0.5rem;
    border: 1px solid #dbdbdb;
    border-radius: 4px;
    background-color: #f5f5f5;
  }

  .summary:hover {
    background-color: #eeeeee;
  }

  .textarea {
    min-height: 120px;
    resize: vertical;
  }

  .modal-card-body {
    max-height: 70vh;
    overflow-y: auto;
  }

  .help {
    font-size: 0.75rem;
    color: #666;
    margin-top: 1rem;
  }
</style>
