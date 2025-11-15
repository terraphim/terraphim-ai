<script lang="ts">
import { Editor } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';
import { Markdown } from 'tiptap-markdown';
import { is_tauri, role } from '$lib/stores';
import { novelAutocompleteService } from '../services/novelAutocompleteService';
import { TerraphimSuggestion, terraphimSuggestionStyles } from './TerraphimSuggestion';
import { SlashCommand, slashCommandStyles } from './SlashCommand';

// Svelte 5: Migrate props using $props() rune
let {
	html = $bindable(''), // $bindable allows parent to bind to this prop
	readOnly = false,
	outputFormat = 'html' as 'html' | 'markdown',
	enableAutocomplete = true,
	showSnippets = true,
	suggestionTrigger = '++',
	maxSuggestions = 8,
	minQueryLength = 1,
	debounceDelay = 300,
} = $props();

// Svelte 5: Use $state rune for reactive local state
let editor: unknown = $state(null);
let _autocompleteStatus = $state('⏳ Initializing...');
let autocompleteReady = $state(false);
let connectionTested = $state(false);
let styleElement: HTMLStyleElement | null = $state(null);
let editorInstance: Editor | null = $state(null);
let editorElement: HTMLDivElement | null = $state(null);
let isInitializing = $state(false);

// Svelte 5: Replace onMount/onDestroy with $effect for initialization and cleanup
$effect(() => {
	// Initialize autocomplete if enabled
	if (enableAutocomplete) {
		initializeAutocomplete();
	}

	// Inject CSS styles for suggestions
	if (typeof document !== 'undefined') {
		const style = document.createElement('style');
		style.textContent = `${terraphimSuggestionStyles}\n${slashCommandStyles}`;
		document.head.appendChild(style);
		styleElement = style;
	}

	// Initialize TipTap editor
	if (typeof document !== 'undefined' && editorElement) {
		const instance = new Editor({
			element: editorElement as HTMLElement,
			extensions: [
				StarterKit,
				Markdown.configure({ html: true }),
				SlashCommand.configure({
					trigger: '/',
				}),
				...(enableAutocomplete
					? [
							TerraphimSuggestion.configure({
								trigger: suggestionTrigger,
								allowSpaces: false,
								limit: maxSuggestions,
								minLength: minQueryLength,
								debounce: debounceDelay,
							}),
						]
					: []),
			],
			content: html,
			editable: !readOnly,
			onUpdate: ({ editor }) => {
				_handleUpdate(editor as any);
			},
		});
		editorInstance = instance;
		editor = instance as unknown;
	}

	// Cleanup function (replaces onDestroy)
	return () => {
		if (styleElement?.parentNode) {
			styleElement.parentNode.removeChild(styleElement);
		}
		if (editorInstance) {
			editorInstance.destroy();
			editorInstance = null;
		}
	};
});

// Svelte 5: Replace reactive statement with $effect for role changes
$effect(() => {
	if ($role && enableAutocomplete) {
		// Only update the role on change; avoid re-triggering full initialization here
		novelAutocompleteService.setRole($role);
	}
});

async function initializeAutocomplete() {
	if (isInitializing) {
		return;
	}
	isInitializing = true;
	_autocompleteStatus = '⏳ Initializing autocomplete...';
	autocompleteReady = false;
	connectionTested = false;

	try {
		// Set the current role in the autocomplete service
		novelAutocompleteService.setRole($role);

		// Test connection first
		_autocompleteStatus = '🔗 Testing connection...';
		const connectionOk = await novelAutocompleteService.testConnection();
		connectionTested = true;

		if (connectionOk) {
			// Defer heavy index building until first suggestion request to avoid race loops.
			if ($is_tauri) {
				_autocompleteStatus = '✅ Ready - Using Tauri backend';
			} else {
				_autocompleteStatus = '✅ Ready - Using MCP server backend';
			}
			autocompleteReady = true;
		} else {
			if ($is_tauri) {
				_autocompleteStatus = '❌ Tauri backend not available';
			} else {
				_autocompleteStatus = '❌ MCP server not responding';
			}
		}
	} catch (error) {
		console.error('Error initializing autocomplete:', error);
		_autocompleteStatus = '❌ Autocomplete initialization error';
	} finally {
		isInitializing = false;
	}
}

/** Handler called by Novel editor on every update; we translate it to the
 *  wrapper's `html` variable so the parent can bind to it. */
const _handleUpdate = (editorInstance: any) => {
	editor = editorInstance;

	// Choose output format based on the outputFormat prop
	// For markdown content, use getMarkdown() to preserve markdown syntax
	// For HTML content, use getHTML() to preserve rich formatting
	if (outputFormat === 'markdown') {
		html = editorInstance.storage?.markdown?.getMarkdown?.() || '';
	} else {
		html = editorInstance.getHTML?.() || '';
	}
};

// Function to manually test autocomplete
const _testAutocomplete = async () => {
	if (!connectionTested) {
		alert('Please wait for connection test to complete');
		return;
	}

	if (!autocompleteReady) {
		alert('Autocomplete service not ready. Check the status above.');
		return;
	}

	try {
		_autocompleteStatus = '🧪 Testing autocomplete...';

		const testQuery = 'terraphim';
		const suggestions = await novelAutocompleteService.getSuggestions(testQuery, 5);

		console.log('Autocomplete test results:', suggestions);

		if (suggestions.length > 0) {
			const suggestionText = suggestions
				.map((s, i) => `${i + 1}. ${s.text}${s.snippet ? ` (${s.snippet})` : ''}`)
				.join('\n');

			alert(`✅ Found ${suggestions.length} suggestions for '${testQuery}':\n\n${suggestionText}`);

			if ($is_tauri) {
				_autocompleteStatus = '✅ Ready - Using Tauri backend';
			} else {
				_autocompleteStatus = '✅ Ready - Using MCP server backend';
			}
		} else {
			alert(
				`⚠️ No suggestions found for '${testQuery}'. This might be normal if the term isn't in your knowledge graph.`
			);
		}
	} catch (error) {
		console.error('Autocomplete test failed:', error);
		alert(`❌ Autocomplete test failed: ${(error as Error).message}`);
		_autocompleteStatus = '❌ Test failed - check console for details';
	}
};

// Function to rebuild autocomplete index
const _rebuildIndex = async () => {
	_autocompleteStatus = '⏳ Rebuilding index...';
	autocompleteReady = false;

	try {
		const success = await novelAutocompleteService.refreshIndex();

		if (success) {
			if ($is_tauri) {
				_autocompleteStatus = '✅ Ready - Tauri index rebuilt successfully';
			} else {
				_autocompleteStatus = '✅ Ready - MCP server index rebuilt successfully';
			}
			autocompleteReady = true;
		} else {
			_autocompleteStatus = '❌ Failed to rebuild index';
		}
	} catch (error) {
		console.error('Error rebuilding index:', error);
		_autocompleteStatus = '❌ Index rebuild failed - check console for details';
	}
};

// Function to demonstrate autocomplete in action
const _demonstrateAutocomplete = () => {
	if (!editor) {
		alert('Editor not ready yet');
		return;
	}

	// Insert demo text that explains the new autocomplete system
	const demoText = `# Terraphim Autocomplete Demo

This is a demonstration of the integrated Terraphim autocomplete system.

## How to Use:
1. Type "${suggestionTrigger}" to trigger autocomplete
2. Start typing any term (e.g., "${suggestionTrigger}terraphim", "${suggestionTrigger}graph")
3. Use ↑↓ arrows to navigate suggestions
4. Press Tab or Enter to select
5. Press Esc to cancel

## Try these queries:
- ${suggestionTrigger}terraphim
- ${suggestionTrigger}graph
- ${suggestionTrigger}service
- ${suggestionTrigger}automata
- ${suggestionTrigger}role

The autocomplete system uses your local knowledge graph to provide intelligent suggestions based on your selected role: **${$role}**.

---

Start typing below:`;

	(editor as any).commands?.setContent?.(demoText);

	// Focus the editor and position cursor at the end
	setTimeout(() => {
		(editor as any).commands?.focus?.('end');
	}, 100);

	alert(
		`Demo content inserted!\n\nType "${suggestionTrigger}" followed by any term to see autocomplete suggestions.\n\nExample: "${suggestionTrigger}terraphim"`
	);
};
</script>

<div class="novel-editor" bind:this={editorElement}></div>

<!-- Autocomplete Status and Controls -->
{#if enableAutocomplete}
  <div style="margin-top: 10px; padding: 12px; background: #f8f9fa; border-radius: 6px; border: 1px solid #e9ecef;">
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
      <strong style="color: #495057;">Local Autocomplete Status:</strong>
      <div style="display: flex; gap: 8px;">
        <button
          on:click={_testAutocomplete}
          style="
            padding: 4px 8px;
            background: #007bff;
            color: white;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 12px;
          "
          disabled={!autocompleteReady}
        >
          Test
        </button>
        <button
          on:click={_rebuildIndex}
          style="
            padding: 4px 8px;
            background: #28a745;
            color: white;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 12px;
          "
        >
          Rebuild Index
        </button>
        <button
          on:click={_demonstrateAutocomplete}
          style="
            padding: 4px 8px;
            background: #ffc107;
            color: #212529;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 12px;
          "
          disabled={!autocompleteReady}
        >
          Demo
        </button>
      </div>
    </div>

    <div style="font-size: 13px; color: #6c757d; margin-bottom: 8px; font-family: monospace;">
      {_autocompleteStatus}
    </div>

    {#if connectionTested && !autocompleteReady}
      <div style="font-size: 12px; color: #dc3545; margin-bottom: 8px; padding: 6px; background: #f8d7da; border-radius: 4px;">
        <strong>⚠️ Autocomplete Not Available</strong><br>
        {#if $is_tauri}
          Tauri backend connection failed. Ensure the application has proper permissions.
        {:else}
          MCP server not responding. Ensure the server is running on {novelAutocompleteService.getStatus().baseUrl}
        {/if}
      </div>
    {/if}

    <div style="font-size: 12px; color: #6c757d;">
      <strong>Configuration:</strong>
      <br>• <strong>Backend:</strong> {$is_tauri ? 'Tauri (native)' : `MCP Server (${novelAutocompleteService.getStatus().baseUrl})`}
      <br>• <strong>Role:</strong> {$role}
      <br>• <strong>Trigger:</strong> "{suggestionTrigger}" + text
      <br>• <strong>Min Length:</strong> {minQueryLength} character{minQueryLength !== 1 ? 's' : ''}
      <br>• <strong>Max Results:</strong> {maxSuggestions}
      <br>• <strong>Debounce:</strong> {debounceDelay}ms
      <br>• <strong>Snippets:</strong> {showSnippets ? 'Enabled' : 'Disabled'}
    </div>

    {#if autocompleteReady}
      <div style="margin-top: 8px; padding: 8px; background: #d1edff; border: 1px solid #b3d9ff; border-radius: 4px;">
        <strong>🎯 Autocomplete Active</strong>
        <div style="font-size: 11px; margin-top: 4px; color: #0056b3;">
          Type <code>{suggestionTrigger}</code> in the editor above to trigger suggestions.<br>
          Example: <code>{suggestionTrigger}terraphim</code> or <code>{suggestionTrigger}graph</code>
        </div>
      </div>
    {:else if connectionTested}
      <div style="margin-top: 8px; padding: 8px; background: #f8d7da; border: 1px solid #f5c6cb; border-radius: 4px;">
        <strong>❌ Autocomplete Unavailable</strong>
        <div style="font-size: 11px; margin-top: 4px; color: #721c24;">
          Click "Rebuild Index" to retry or check server/backend status.
        </div>
      </div>
    {/if}
  </div>
{/if}
