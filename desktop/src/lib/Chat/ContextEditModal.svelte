<script lang="ts">
import { Modal } from 'svelma';
import { createEventDispatcher } from 'svelte';
import type { ContextItem } from './Chat.svelte';

let {
	active = $bindable(false),
	context = null,
	mode = 'edit',
}: {
	active?: boolean;
	context?: ContextItem | null;
	mode?: 'create' | 'edit';
} = $props();

const dispatch = createEventDispatcher();

// Form data
let editingContext = $state<ContextItem | null>(null);
const _contextTypeOptions = [
	{ value: 'Document', label: 'Document' },
	{ value: 'SearchResult', label: 'Search Result' },
	{ value: 'UserInput', label: 'User Input' },
	{ value: 'System', label: 'System' },
	{ value: 'External', label: 'External' },
];

// Initialize form data when modal becomes active
$effect(() => {
	if (active && context) {
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
});

// Validation
let isValid = $derived(
	editingContext && editingContext.title.trim() !== '' && editingContext.content.trim() !== ''
);

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

function _handleDelete() {
	if (mode === 'edit' && context) {
		dispatch('delete', context.id);
		handleClose();
	}
}

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
                bind:value={editingContext.context_type}
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
              bind:value={editingContext.title}
              data-testid="context-title-input"
              required
            >
          </div>
          {#if editingContext.title.trim() === ''}
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
              bind:value={editingContext.summary}
              data-testid="context-summary-textarea"
              rows="3"
              maxlength="500"
            ></textarea>
          </div>
          <p class="help">
            {#if editingContext.summary}
              {editingContext.summary.length}/500 characters
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
              bind:value={editingContext.content}
              data-testid="context-content-textarea"
              rows="8"
              required
            ></textarea>
          </div>
          {#if editingContext.content.trim() === ''}
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
            <label class="label">Metadata</label>
            <div class="content">
              <p class="help">Metadata editing temporarily disabled</p>
              <pre>{JSON.stringify(editingContext?.metadata || {}, null, 2)}</pre>
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
