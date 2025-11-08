<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import Modal from '$lib/components/Modal.svelte';
import { configStore, is_tauri, role } from '$lib/stores';
import { CONFIG } from '../../config';
import type { Haystack, Role } from '../generated/types';
import type { Document } from './SearchResult';

export let active: boolean = false;
export let document: Document;

// State for the save process
let saving = false;
let _error: string | null = null;
let _success = false;
let selectedParent = '';
let customParent = '';
let useCustomParent = false;
let articleTitle = '';
let articleDescription = '';

// Available atomic servers from current role
let atomicHaystacks: Haystack[] = [];
let selectedAtomicServer = '';

// Predefined parent options
const _predefinedParents = [
	{ label: 'Root (Server Root)', value: '' },
	{ label: 'Articles Collection', value: 'articles' },
	{ label: 'Documents Collection', value: 'documents' },
	{ label: 'Knowledge Base', value: 'knowledge-base' },
	{ label: 'Research', value: 'research' },
	{ label: 'Projects', value: 'projects' },
];

// Watch for active changes to reset state and load atomic servers
$: if (active && document) {
	resetModal();
	loadAtomicServers();
}

function resetModal() {
	saving = false;
	_error = null;
	_success = false;
	selectedParent = '';
	customParent = '';
	useCustomParent = false;
	articleTitle = document?.title || '';
	articleDescription =
		document?.description || `Article saved from Terraphim search: ${document?.title}`;
	atomicHaystacks = [];
	selectedAtomicServer = '';
}

function loadAtomicServers() {
	// Find atomic server haystacks from current role configuration
	const currentRoleName = $role;
	const config = $configStore;

	if (!config?.roles || !currentRoleName) {
		console.warn('No role configuration found');
		return;
	}

	// Find the current role object - handle the complex role structure
	let currentRole: Role | null = null;

	try {
		// Handle both string keys and RoleName objects in the roles map
		for (const [roleName, roleConfig] of Object.entries(config.roles)) {
			// Cast roleConfig to Role type for proper access
			const role = roleConfig as Role;

			// Check various ways the role name might match
			const roleNameStr = typeof role.name === 'object' ? role.name.original : String(role.name);

			if (roleName === currentRoleName || roleNameStr === currentRoleName) {
				currentRole = role;
				break;
			}
		}
	} catch (error) {
		console.warn('Error checking role configuration:', error);
		return;
	}

	if (!currentRole) {
		console.warn(`Role "${currentRoleName}" not found in configuration`);
		return;
	}

	// Filter haystacks to find atomic servers
	atomicHaystacks =
		currentRole.haystacks?.filter(
			(haystack) => haystack.service === 'Atomic' && haystack.location && !haystack.read_only
		) || [];

	// Auto-select first atomic server if available
	if (atomicHaystacks.length > 0) {
		selectedAtomicServer = atomicHaystacks[0].location;
	}

	console.log('Loaded atomic servers:', atomicHaystacks);
}

function getAtomicServerSecret(): string | undefined {
	const selectedHaystack = atomicHaystacks.find((h) => h.location === selectedAtomicServer);
	return selectedHaystack?.atomic_server_secret;
}

function buildParentUrl(): string {
	const baseUrl = selectedAtomicServer.replace(/\/$/, '');

	if (useCustomParent && customParent.trim()) {
		// Custom parent - ensure it doesn't start with server URL to avoid duplication
		const parentPath = customParent.trim();
		if (parentPath.startsWith('http://') || parentPath.startsWith('https://')) {
			return parentPath; // Full URL provided
		} else {
			return `${baseUrl}/${parentPath.replace(/^\//, '')}`; // Relative path
		}
	} else if (selectedParent) {
		return `${baseUrl}/${selectedParent}`;
	} else {
		return baseUrl; // Root
	}
}

function generateArticleSlug(): string {
	// Create URL-safe slug from title
	return articleTitle
		.toLowerCase()
		.replace(/[^a-z0-9\s-]/g, '') // Remove special chars except spaces and hyphens
		.replace(/\s+/g, '-') // Replace spaces with hyphens
		.replace(/-+/g, '-') // Replace multiple hyphens with single
		.replace(/^-|-$/g, '') // Remove leading/trailing hyphens
		.substring(0, 50); // Limit length
}

function buildSubjectUrl(): string {
	const baseUrl = selectedAtomicServer.replace(/\/$/, '');
	const slug = generateArticleSlug();
	const timestamp = Date.now();
	return `${baseUrl}/${slug}-${timestamp}`;
}

async function _saveToAtomic() {
	if (!selectedAtomicServer || !articleTitle.trim()) {
		_error = 'Please select an atomic server and provide a title';
		return;
	}

	saving = true;
	_error = null;

	try {
		const subjectUrl = buildSubjectUrl();
		const parentUrl = buildParentUrl();
		const atomicSecret = getAtomicServerSecret();

		console.log('🔄 Saving article to atomic server:', {
			subject: subjectUrl,
			parent: parentUrl,
			server: selectedAtomicServer,
			hasSecret: !!atomicSecret,
		});

		const atomicArticle = {
			subject: subjectUrl,
			title: articleTitle.trim(),
			description: articleDescription.trim(),
			body: document.body,
			parent: parentUrl,
			shortname: generateArticleSlug(),
			// Preserve original metadata
			original_id: document.id,
			original_url: document.url,
			original_rank: document.rank,
			tags: document.tags || [],
		};

		if ($is_tauri) {
			// Use Tauri command
			await invoke('save_article_to_atomic', {
				article: atomicArticle,
				serverUrl: selectedAtomicServer,
				atomicSecret: atomicSecret,
			});
		} else {
			// Use HTTP API
			const response = await fetch(`${CONFIG.ServerURL}/atomic/save`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify({
					article: atomicArticle,
					server_url: selectedAtomicServer,
					atomic_secret: atomicSecret,
				}),
			});

			if (!response.ok) {
				const errorData = await response.json().catch(() => ({ error: response.statusText }));
				throw new Error(errorData.error || `HTTP ${response.status}: ${response.statusText}`);
			}
		}

		_success = true;
		console.log('✅ Article saved successfully to atomic server');

		// Auto-close after 2 seconds
		setTimeout(() => {
			active = false;
		}, 2000);
	} catch (err) {
		console.error('❌ Failed to save article to atomic server:', err);
		_error = (err as Error).message || 'Failed to save article to atomic server';
	} finally {
		saving = false;
	}
}

function _handleClose() {
	if (!saving) {
		active = false;
	}
}
</script>

<Modal bind:active on:close={_handleClose}>
  <div class="box">
    <!-- Header -->
    <div class="level">
      <div class="level-left">
        <div class="level-item">
          <h3 class="title is-4">
            <span class="icon">
              <i class="fas fa-save"></i>
            </span>
            Save to Atomic Server
          </h3>
        </div>
      </div>
      <div class="level-right">
        <div class="level-item">
          <button
            class="delete is-large"
            on:click={_handleClose}
            disabled={saving}
            aria-label="close"
          ></button>
        </div>
      </div>
    </div>

    <!-- Success Message -->
    {#if _success}
      <div class="message is-success">
        <div class="message-body">
          <p><strong>Success!</strong> Article saved to atomic server successfully.</p>
          <p>The modal will close automatically...</p>
        </div>
      </div>
    {/if}

    <!-- Error Message -->
    {#if _error}
      <div class="message is-danger">
        <div class="message-body">
          <p><strong>Error:</strong> {_error}</p>
        </div>
      </div>
    {/if}

    <!-- Main Content -->
    {#if !_success}
      <!-- Document Preview -->
      <div class="field">
        <div class="label">Document to Save</div>
        <div class="box document-preview">
          <h5 class="title is-6">{document?.title || 'Untitled'}</h5>
          {#if document?.description}
            <p class="content">{document.description}</p>
          {/if}
          <div class="tags">
            <span class="tag is-info">Rank: {document?.rank || 0}</span>
            {#if document?.tags}
              {#each document.tags as tag}
                <span class="tag">{tag}</span>
              {/each}
            {/if}
          </div>
        </div>
      </div>

      <!-- Atomic Server Selection -->
      <div class="field">
        <label class="label" for="atomic-server-select">Atomic Server</label>
        <div class="control">
          {#if atomicHaystacks.length > 0}
            <div class="select is-fullwidth">
              <select
                id="atomic-server-select"
                bind:value={selectedAtomicServer}
                disabled={saving}
              >
                {#each atomicHaystacks as haystack}
                  <option value={haystack.location}>
                    {haystack.location}{haystack.atomic_server_secret ? ' 🔐' : ' ⚠️ No Auth'}
                  </option>
                {/each}
              </select>
            </div>
            <p class="help">
              Select the atomic server from your current role configuration.
              🔐 = Authenticated, ⚠️ = Anonymous access
            </p>
          {:else}
            <div class="notification is-warning">
              <p><strong>No atomic servers available</strong></p>
              <p>Your current role "{$role}" doesn't have any writable atomic server configurations.</p>
              <p>Please configure an atomic server haystack in your role settings.</p>
            </div>
          {/if}
        </div>
      </div>

      {#if atomicHaystacks.length > 0}
        <!-- Article Details -->
        <div class="field">
          <label class="label" for="article-title">Article Title</label>
          <div class="control">
            <input
              class="input"
              type="text"
              id="article-title"
              bind:value={articleTitle}
              placeholder="Enter article title"
              disabled={saving}
              required
            />
          </div>
        </div>

        <div class="field">
          <label class="label" for="article-description">Description</label>
          <div class="control">
            <textarea
              id="article-description"
              class="textarea"
              bind:value={articleDescription}
              placeholder="Brief description of the article"
              disabled={saving}
              rows="3"
            ></textarea>
          </div>
        </div>

        <!-- Parent Selection -->
        <div class="field">
          <div class="label">Parent Collection</div>
          <div class="control">
            <label class="radio">
              <input
                type="radio"
                bind:group={useCustomParent}
                value={false}
                disabled={saving}
              />
              Use predefined collection
            </label>
            <label class="radio">
              <input
                type="radio"
                bind:group={useCustomParent}
                value={true}
                disabled={saving}
              />
              Custom parent URL/path
            </label>
          </div>
        </div>

        {#if !useCustomParent}
          <div class="field">
            <div class="control">
              <div class="select is-fullwidth">
                <select bind:value={selectedParent} disabled={saving}>
                  {#each _predefinedParents as parent}
                    <option value={parent.value}>{parent.label}</option>
                  {/each}
                </select>
              </div>
            </div>
          </div>
        {:else}
          <div class="field">
            <div class="control">
              <input
                class="input"
                type="text"
                bind:value={customParent}
                placeholder="e.g., my-collection or http://server/custom-parent"
                disabled={saving}
              />
            </div>
            <p class="help">
              Enter a collection name (e.g., "my-articles") or full URL.
              If the collection doesn't exist, it will be created.
            </p>
          </div>
        {/if}

        <!-- Action Buttons -->
        <div class="field is-grouped">
          <div class="control">
            <button
              class="button is-primary {saving ? 'is-loading' : ''}"
              disabled={saving || !articleTitle.trim() || !selectedAtomicServer}
              on:click={_saveToAtomic}
            >
              <span class="icon">
                <i class="fas fa-cloud-upload-alt"></i>
              </span>
              <span>Save to Atomic Server</span>
            </button>
          </div>
          <div class="control">
            <button
              class="button is-light"
              disabled={saving}
              on:click={_handleClose}
            >
              Cancel
            </button>
          </div>
        </div>

        <!-- Preview Info -->
        <div class="field">
          <div class="notification is-info is-light">
            <p><strong>What will be saved:</strong></p>
            <ul>
              <li><strong>Title:</strong> {articleTitle || 'Untitled'}</li>
              <li><strong>Body:</strong> Original document content ({document?.body?.length || 0} characters)</li>
              <li><strong>Parent:</strong> {buildParentUrl()}</li>
              <li><strong>Subject URL:</strong> {buildSubjectUrl()}</li>
            </ul>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</Modal>

<style>
  .document-preview {
    background-color: #f9f9f9;
    max-height: 150px;
    overflow-y: auto;
  }

  .tags {
    margin-top: 0.5rem;
  }

  .tags .tag {
    margin-right: 0.25rem;
    margin-bottom: 0.25rem;
  }

  .radio {
    margin-right: 1rem;
  }

  .help {
    margin-top: 0.25rem;
  }

  .notification ul {
    margin-left: 1rem;
  }

  .notification ul li {
    margin-bottom: 0.25rem;
  }

  .level {
    margin-bottom: 1rem;
  }
</style>
