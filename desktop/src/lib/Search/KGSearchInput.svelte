<script lang="ts">
import { invoke } from '@tauri-apps/api/tauri';
import { Field, Input } from 'svelma';
import { is_tauri } from '$lib/stores';
import { CONFIG } from '../../config';

let {
	roleName,
	placeholder = 'Search over Knowledge graph...',
	autofocus = false,
	onSelect,
	initialValue = '',
	onInputChange = null,
}: {
	roleName: string;
	placeholder?: string;
	autofocus?: boolean;
	onSelect: (term: string) => void;
	initialValue?: string;
	onInputChange?: ((value: string) => void) | null;
} = $props();

// Input value - sync with parent
let query = $state(initialValue);
let searchInput = $state<HTMLInputElement>();

// Sync initialValue when it changes externally (e.g., role change)
$effect(() => {
	if (initialValue !== undefined && initialValue !== query) {
		query = initialValue;
	}
});

// Autocomplete state
let autocompleteSuggestions = $state<string[]>([]);
let suggestionIndex = $state(-1);
let searchTimeout = $state<ReturnType<typeof setTimeout> | null>(null);
let _autocompleteError = $state<string | null>(null);

// Get KG term suggestions (autocomplete)
async function getTermSuggestions(q: string): Promise<string[]> {
	const trimmed = q.trim();
	if (!trimmed || trimmed.length < 2) return [];
	try {
		if ($is_tauri) {
			const resp: any = await invoke('get_autocomplete_suggestions', {
				query: trimmed,
				role_name: roleName,
				limit: 8,
			});
			if (resp?.status === 'success' && Array.isArray(resp.suggestions)) {
				_autocompleteError = null;
				return resp.suggestions.map((s: any) => s.term);
			}
		} else {
			const resp = await fetch(
				`${CONFIG.ServerURL}/autocomplete/${encodeURIComponent(roleName)}/${encodeURIComponent(trimmed)}`
			);
			if (resp.ok) {
				const data = await resp.json();
				if (data?.status === 'success' && Array.isArray(data.suggestions)) {
					_autocompleteError = null;
					return data.suggestions.map((s: any) => s.term);
				}
			}
		}
	} catch (e) {
		console.warn('KG autocomplete failed', e);
		_autocompleteError = 'KG autocomplete unavailable';
	}
	return [];
}

// Update autocomplete suggestions
async function updateAutocompleteSuggestions() {
	const inputValue = query.trim();
	if (inputValue.length < 2) {
		autocompleteSuggestions = [];
		suggestionIndex = -1;
		_autocompleteError = null;
		return;
	}

	try {
		const suggestions = await getTermSuggestions(inputValue);
		autocompleteSuggestions = suggestions;
		suggestionIndex = -1;
	} catch (error) {
		console.warn('Failed to get autocomplete suggestions:', error);
		autocompleteSuggestions = [];
		suggestionIndex = -1;
		_autocompleteError = 'Failed to load suggestions';
	}
}

// Apply autocomplete suggestion
function applySuggestion(suggestion: string) {
	query = suggestion;
	autocompleteSuggestions = [];
	suggestionIndex = -1;
	_autocompleteError = null;
	// Call parent callback
	onSelect(suggestion);
}

// Handle input changes with debouncing
async function _handleInput(_event: Event) {
	// Sync input value to parent store as user types (query is updated by bind:value)
	if (onInputChange) {
		onInputChange(query);
	}

	if (searchTimeout) {
		clearTimeout(searchTimeout);
	}
	searchTimeout = setTimeout(() => {
		updateAutocompleteSuggestions();
		searchTimeout = null;
	}, 300);
}

// Handle keyboard navigation
function _handleKeydown(event: KeyboardEvent) {
	if (autocompleteSuggestions.length > 0) {
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			suggestionIndex = (suggestionIndex + 1) % autocompleteSuggestions.length;
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			suggestionIndex =
				(suggestionIndex - 1 + autocompleteSuggestions.length) % autocompleteSuggestions.length;
		} else if ((event.key === 'Enter' || event.key === 'Tab') && suggestionIndex !== -1) {
			event.preventDefault();
			applySuggestion(autocompleteSuggestions[suggestionIndex]);
		} else if (event.key === 'Escape') {
			event.preventDefault();
			autocompleteSuggestions = [];
			suggestionIndex = -1;
		}
	} else if (event.key === 'Enter') {
		// If no suggestions, trigger search with current query
		if (query.trim().length >= 2) {
			onSelect(query.trim());
		}
	}
}

// Clean up timeout on component destruction
$effect(() => {
	return () => {
		if (searchTimeout) {
			clearTimeout(searchTimeout);
		}
	};
});
</script>

<Field>
	<div class="input-wrapper">
		<Input
			bind:element={searchInput}
			bind:value={query}
			on:input={_handleInput}
			on:keydown={_handleKeydown}
			{placeholder}
			type="search"
			icon="search"
			expanded
			{autofocus}
			data-testid="kg-search-input"
		/>
		{#if autocompleteSuggestions.length > 0}
			<ul class="suggestions" data-testid="kg-autocomplete-list">
				{#each autocompleteSuggestions as suggestion, index}
					<li
						class:active={index === suggestionIndex}
						on:click={() => applySuggestion(suggestion)}
						on:keydown={(e) => {
							if (e.key === 'Enter' || e.key === ' ') {
								e.preventDefault();
								applySuggestion(suggestion);
							}
						}}
						tabindex="0"
						role="option"
						aria-selected={index === suggestionIndex}
						aria-label={`Apply suggestion: ${suggestion}`}
						data-testid="kg-autocomplete-item"
					>
						{suggestion}
					</li>
				{/each}
			</ul>
		{/if}
		{#if _autocompleteError}
			<div class="autocomplete-error" data-testid="kg-autocomplete-error">
				{_autocompleteError}
			</div>
		{/if}
	</div>
</Field>

<style>
	.input-wrapper {
		position: relative;
		width: 100%;
	}

	.suggestions {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		z-index: 5;
		list-style-type: none;
		padding: 0;
		margin: 0;
		background-color: white;
		border: 1px solid #dbdbdb;
		border-top: none;
		border-radius: 0 0 4px 4px;
		box-shadow: 0 2px 3px rgba(10, 10, 10, 0.1);
	}

	.suggestions li {
		padding: 0.5em 1em;
		cursor: pointer;
	}

	.suggestions li:hover,
	.suggestions li.active {
		background-color: #f5f5f5;
	}

	.autocomplete-error {
		margin-top: 0.25rem;
		font-size: 0.75rem;
		color: #dc3545;
		padding: 0.25rem 0.5rem;
		background-color: #f8d7da;
		border-radius: 3px;
		border: 1px solid #f5c6cb;
	}

	@media (prefers-color-scheme: dark) {
		.suggestions {
			background-color: #2a2a2a;
			border-color: #404040;
		}

		.suggestions li:hover,
		.suggestions li.active {
			background-color: #3a3a3a;
		}

		.autocomplete-error {
			background-color: #3a2a2a;
			border-color: #5a3a3a;
			color: #ff6b6b;
		}
	}
</style>
