<script context="module" lang="ts">
export type ContextItem = {
	id: string;
	title: string;
	summary?: string;
	content: string;
	context_type: string;
	created_at: string;
	relevance_score?: number;
	metadata?: { [key: string]: string };
	// KG-specific fields
	kg_term_definition?: {
		term: string;
		normalized_term: string;
		id: number;
		definition?: string;
		synonyms: string[];
		related_terms: string[];
		usage_examples: string[];
		url?: string;
		metadata: Record<string, string>;
		relevance_score?: number;
	};
};
</script>

<script lang="ts">
import { invoke } from '@tauri-apps/api/tauri';
import { get } from 'svelte/store';
import { CONFIG } from '../../config';
import {
	configStore,
	currentPersistentConversationId,
	is_tauri,
	role,
	showSessionList,
} from '../stores';
import SessionList from './SessionList.svelte';
import ContextEditModal from './ContextEditModal.svelte';
import KGContextItem from '../Search/KGContextItem.svelte';
import KGSearchModal from '../Search/KGSearchModal.svelte';
import ArticleModal from '../Search/ArticleModal.svelte';
// @ts-ignore
import Markdown from 'svelte-markdown';

// Tauri APIs for saving files (only used in desktop)
let tauriDialog: any = $state(null);
let tauriFs: any = $state(null);

type ChatMessage = { role: 'system' | 'user' | 'assistant'; content: string };
type ChatResponse = { status: string; message?: string; model_used?: string; error?: string };

// API response types
type ConversationsResponse = {
	conversations: Array<{ id: string; created_at: string; title?: string }>;
};
type CreateConversationResponse = {
	status: string;
	conversation_id?: string;
	error?: string;
};
type ConversationResponse = {
	status: string;
	conversation?: {
		global_context: ContextItem[];
	};
	error?: string;
};
type Conversation = {
	id: string;
	title: string;
	role: string;
	messages: any[];
	global_context: ContextItem[];
	created_at: string;
	updated_at: string;
};

// Svelte 5: Use $state rune for reactive local state
let messages: ChatMessage[] = $state([]);
let input: string = $state('');
let sending = $state(false);
let error: string | null = $state(null);
let modelUsed: string | null = $state(null);
let _providerHint: string = $state('');
let renderMarkdown: boolean = $state(false);

// Debug state
let _debugMode: boolean = $state(false);
let _lastRequest: any = $state(null);
let _lastResponse: any = $state(null);
let _showDebugRequest: boolean = $state(false);
let _showDebugResponse: boolean = $state(false);

// Conversation and context management
let conversationId: string | null = $state(null);
let contextItems: ContextItem[] = $state([]);
let _loadingContext = $state(false);
const _showContextPanel = false;

// Manual context addition
let showAddContextForm = $state(false);
let newContextTitle = $state('');
let newContextContent = $state('');
let newContextType = $state('document');
let _savingContext = $state(false);

// Context editing
let _showContextEditModal = $state(false);
let _editingContext: ContextItem | null = $state(null);
let _contextEditMode: 'create' | 'edit' = $state('edit' as 'create' | 'edit');
let deletingContextId: string | null = $state(null);

// KG search modal
let _showKGSearchModal = $state(false);

// KG document modal (for viewing KG term documents)
let _showKgModal = $state(false);
let kgDocument: any = $state(null);
let _kgTerm: string | null = $state(null);
let _kgRank: number | null = $state(null);

// --- Persistence helpers ---
function chatStateKey(): string {
	const r = get(role) as string;
	return `terraphim:chatState:${r}`;
}

function loadChatState() {
	try {
		if (typeof window === 'undefined') return;
		const raw = localStorage.getItem(chatStateKey());
		if (!raw) return;
		const data = JSON.parse(raw);
		if (Array.isArray(data.messages)) {
			messages = data.messages;
		}
		if (typeof data.conversationId === 'string') {
			conversationId = data.conversationId;
		}
	} catch (e) {
		console.warn('Failed to load chat state:', e);
	}
}

function getRoleDisplay() {
	const currentRole = get(role);
	if (typeof currentRole === 'object' && currentRole && 'original' in currentRole) {
		return (currentRole as any).original;
	}
	return String(currentRole);
}

function saveChatState() {
	try {
		if (typeof window === 'undefined') return;
		const data = { messages, conversationId };
		localStorage.setItem(chatStateKey(), JSON.stringify(data));
	} catch (e) {
		console.warn('Failed to save chat state:', e);
	}
}

// Persist markdown toggle preference
function mdPrefKey(): string {
	return 'terraphim:chatMarkdown';
}
function loadMdPref() {
	try {
		const v = localStorage.getItem(mdPrefKey());
		if (v != null) renderMarkdown = v === 'true';
	} catch {}
}
function _saveMdPref() {
	try {
		localStorage.setItem(mdPrefKey(), renderMarkdown ? 'true' : 'false');
	} catch {}
}

function addUserMessage(text: string) {
	messages = [...messages, { role: 'user', content: text }];
	saveChatState();
}

// Load or create a conversation
async function initializeConversation() {
	try {
		if ($is_tauri) {
			// Try to get existing conversations
			const result = await invoke('list_conversations') as ConversationsResponse;
			if (result?.conversations && result.conversations.length > 0) {
				// Use the most recent conversation
				conversationId = result.conversations[0].id;
				console.log('🎯 Using existing conversation:', conversationId);
				await loadConversationContext();
			} else {
				// Create a new conversation
				await createNewConversation();
			}
		} else {
			// Web mode - HTTP API
			const response = await fetch(`${CONFIG.ServerURL}/conversations`);
			if (response.ok) {
				const data = await response.json();
				if (data.conversations && data.conversations.length > 0) {
					conversationId = data.conversations[0].id;
					console.log('🎯 Using existing conversation:', conversationId);
					await loadConversationContext();
				} else {
					await createNewConversation();
				}
			} else {
				await createNewConversation();
			}
		}
	} catch (error) {
		console.error('❌ Error initializing conversation:', error);
	}
}

// Create a new conversation
async function createNewConversation() {
	try {
		const currentRole = get(role) as string;

		if ($is_tauri) {
			const result = await invoke('create_conversation', {
				title: 'Chat Conversation',
				role: currentRole,
			}) as CreateConversationResponse;
			if (result.status === 'success' && result.conversation_id) {
				conversationId = result.conversation_id;
				console.log('🆕 Created new conversation:', conversationId);
				saveChatState();
			}
		} else {
			const response = await fetch(`${CONFIG.ServerURL}/conversations`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					title: 'Chat Conversation',
					role: currentRole,
				}),
			});
			if (response.ok) {
				const data = await response.json();
				if (data.status === 'Success' && data.conversation_id) {
					conversationId = data.conversation_id;
					console.log('🆕 Created new conversation:', conversationId);
					saveChatState();
				}
			}
		}
	} catch (error) {
		console.error('❌ Error creating conversation:', error);
	}
}

// Load context for the current conversation
async function loadConversationContext() {
	if (!conversationId) {
		console.warn('⚠️ Cannot load context: no conversation ID available');
		return;
	}

	_loadingContext = true;
	console.log('🔄 Loading conversation context for:', conversationId);

	try {
		if ($is_tauri) {
			console.log('📱 Loading context via Tauri...');

			// Check if this is a persistent conversation
			const isPersistent = $currentPersistentConversationId === conversationId;
			const command = isPersistent ? 'get_persistent_conversation' : 'get_conversation';

			console.log(`Using ${command} for ${isPersistent ? 'persistent' : 'in-memory'} conversation`);

			const result = await invoke(command, {
				conversationId: isPersistent ? conversationId : conversationId,
			}) as ConversationResponse;

			console.log('📥 Tauri response:', result);

			if (result.status === 'success' && result.conversation) {
				const newContextItems = result.conversation.global_context || [];
				contextItems = newContextItems;
				console.log(
					`✅ Loaded ${newContextItems.length} context items via Tauri (${isPersistent ? 'persistent' : 'in-memory'})`
				);
			} else {
				console.error(
					`❌ Failed to get conversation via Tauri (${command}):`,
					result.error || 'Unknown error'
				);
				contextItems = [];
			}
		} else {
			console.log('🌐 Loading context via HTTP...');
			const response = await fetch(`${CONFIG.ServerURL}/conversations/${conversationId}`);

			console.log('📥 HTTP response status:', response.status, response.statusText);

			if (response.ok) {
				const data = await response.json();
				console.log('📄 HTTP response data:', data);

				if (data.status === 'success' && data.conversation) {
					const newContextItems = data.conversation.global_context || [];
					contextItems = newContextItems;
					console.log(`✅ Loaded ${newContextItems.length} context items via HTTP`);
				} else {
					console.error('❌ Failed to get conversation via HTTP:', data.error || 'Unknown error');
					contextItems = [];
				}
			} else {
				console.error('❌ HTTP request failed:', response.status, response.statusText);
				contextItems = [];
			}
		}
	} catch (error) {
		console.error('❌ Error loading conversation context:', {
			error: error instanceof Error ? error.message : String(error),
			conversationId,
			isTauri: $is_tauri,
			timestamp: new Date().toISOString(),
		});
		contextItems = [];
	} finally {
		_loadingContext = false;
		console.log('🏁 Context loading completed. Items count:', contextItems.length);
	}
}

// Toggle manual context form
function toggleAddContextForm() {
	showAddContextForm = !showAddContextForm;
	if (!showAddContextForm) {
		// Reset form
		newContextTitle = '';
		newContextContent = '';
		newContextType = 'document';
	}
}

// Add manual context
async function _addManualContext() {
	if (!conversationId || !newContextTitle.trim() || !newContextContent.trim()) return;

	_savingContext = true;
	try {
		const contextData = {
			title: newContextTitle.trim(),
			summary: null,
			content: newContextContent.trim(),
			context_type: newContextType,
		};

		if ($is_tauri) {
			const result = await invoke('add_context_to_conversation', {
				conversationId,
				contextData,
			}) as { status: string; error?: string };
			if (result.status === 'success') {
				await loadConversationContext();
				toggleAddContextForm();

				// Show success notification
				console.log('✅ Context added successfully via Tauri');
			} else {
				console.error('❌ Failed to add context via Tauri:', result.error);
			}
		} else {
			const response = await fetch(`${CONFIG.ServerURL}/conversations/${conversationId}/context`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(contextData),
			});
			if (response.ok) {
				const data = await response.json();
				if (data.status === 'success') {
					await loadConversationContext();
					toggleAddContextForm();

					// Show success notification
					console.log('✅ Context added successfully via HTTP');
				} else {
					console.error('❌ Failed to add context via HTTP:', data.error);
				}
			} else {
				console.error('❌ HTTP request failed:', response.status, response.statusText);
			}
		}
	} catch (error) {
		console.error('❌ Error adding manual context:', error);
	} finally {
		_savingContext = false;
	}
}

// Edit context functionality
async function _editContext(context: ContextItem, termFromEvent?: string) {
	// For KG context items, use find_documents_for_kg_term to show related documents
	if (context.context_type === 'KGTermDefinition') {
		// Extract term from different possible sources
		let term: string | null = termFromEvent || null;

		// Try to get term from kg_term_definition object (if available)
		if (context.kg_term_definition?.term) {
			term = context.kg_term_definition.term;
		}
		// Fallback: extract from title (format: "KG Term: {term}")
		else if (context.title.startsWith('KG Term: ')) {
			term = context.title.replace('KG Term: ', '');
		}
		// Fallback: try normalized_term from metadata
		else if (context.metadata?.normalized_term) {
			term = context.metadata.normalized_term;
		}

		if (term) {
			await showKGDocumentsForTerm(term);
		} else {
			console.warn('Could not extract term from KG context item:', context);
			// Fall back to edit modal
			_editingContext = context;
			_contextEditMode = 'edit';
			_showContextEditModal = true;
		}
	} else {
		// For regular context items, use the edit modal
		_editingContext = context;
		_contextEditMode = 'edit';
		_showContextEditModal = true;
	}
}

// Show KG documents for a term using find_documents_for_kg_term API
async function showKGDocumentsForTerm(term: string) {
	console.log(`🔍 Finding KG documents for term: "${term}" in role: "${$role}"`);

	try {
		if ($is_tauri) {
			// Use Tauri command for desktop app
			console.log('  Making Tauri invoke call...');
			console.log('  Tauri command: find_documents_for_kg_term');
			console.log('  Tauri params:', { roleName: $role, term: term });

			const response: any = await invoke('find_documents_for_kg_term', {
				roleName: $role,
				term: term,
			});

			console.log('  📥 Tauri response received:');
			console.log('    Status:', response.status);
			console.log('    Results count:', response.results?.length || 0);
			console.log('    Total:', response.total || 0);

			if (response.status === 'success' && response.results && response.results.length > 0) {
				// Show the first (highest-ranked) document in a modal
				kgDocument = response.results[0];
				_kgRank = kgDocument.rank || 0;
				_kgTerm = term;
				console.log('  ✅ Found KG document:', kgDocument.title);
				console.log('  📄 Document content preview:', `${kgDocument.body?.substring(0, 200)}...`);
				_showKgModal = true;
			} else {
				console.warn(`  ⚠️  No KG documents found for term: "${term}" in role: "${$role}"`);
			}
		} else {
			// Use HTTP fetch for web mode
			console.log('  Making HTTP fetch call...');
			const baseUrl = CONFIG.ServerURL;
			const encodedRole = encodeURIComponent($role);
			const encodedTerm = encodeURIComponent(term);
			const url = `${baseUrl}/roles/${encodedRole}/kg_search?term=${encodedTerm}`;

			const response = await fetch(url);

			if (!response.ok) {
				throw new Error(`HTTP error! Status: ${response.status} - ${response.statusText}`);
			}

			const data = await response.json();
			console.log('  📄 Response data:', data.status, 'Results:', data.results?.length || 0);

			if (data.status === 'success' && data.results && data.results.length > 0) {
				// Show the first (highest-ranked) document
				kgDocument = data.results[0];
				_kgRank = kgDocument.rank || 0;
				_kgTerm = term;
				console.log('  ✅ Found KG document:', kgDocument.title);
				console.log('  📄 Document content preview:', `${kgDocument.body?.substring(0, 200)}...`);
				_showKgModal = true;
			} else {
				console.warn(`  ⚠️  No KG documents found for term: "${term}" in role: "${$role}"`);
			}
		}
	} catch (error) {
		console.error('❌ Error fetching KG document:', error);
	}
}

// Delete context with confirmation
function _confirmDeleteContext(context: ContextItem) {
	if (confirm(`Are you sure you want to delete "${context.title}"?`)) {
		deleteContext(context.id);
	}
}

// Delete context
async function deleteContext(contextId: string) {
	if (!conversationId || deletingContextId) return;

	deletingContextId = contextId;
	console.log('🗑️ Deleting context:', contextId);

	try {
		if ($is_tauri) {
			const result = await invoke('delete_context', {
				conversationId,
				contextId,
			}) as { status?: string; error?: string };
			if (result?.status === 'success') {
				console.log('✅ Context deleted successfully via Tauri');
				await loadConversationContext();
			} else {
				console.error('❌ Failed to delete context via Tauri:', result?.error);
			}
		} else {
			const response = await fetch(
				`${CONFIG.ServerURL}/conversations/${conversationId}/context/${contextId}`,
				{
					method: 'DELETE',
					headers: { 'Content-Type': 'application/json' },
				}
			);
			if (response.ok) {
				const data = await response.json();
				if (data.status === 'success') {
					console.log('✅ Context deleted successfully via HTTP');
					await loadConversationContext();
				} else {
					console.error('❌ Failed to delete context via HTTP:', data.error);
				}
			} else {
				console.error('❌ HTTP delete request failed:', response.status);
			}
		}
	} catch (error) {
		console.error('❌ Error deleting context:', error);
	} finally {
		deletingContextId = null;
	}
}

// Update context
async function _updateContext(updatedContext: ContextItem) {
	if (!conversationId) return;

	console.log('📝 Updating context:', updatedContext.id);

	try {
		const updatePayload = {
			context_type: updatedContext.context_type,
			title: updatedContext.title,
			summary: updatedContext.summary,
			content: updatedContext.content,
			metadata: updatedContext.metadata,
		};

		if ($is_tauri) {
			const result = await invoke('update_context', {
				conversationId,
				contextId: updatedContext.id,
				request: updatePayload,
			}) as { status?: string; error?: string };
			if (result?.status === 'success') {
				console.log('✅ Context updated successfully via Tauri');
				await loadConversationContext();
			} else {
				console.error('❌ Failed to update context via Tauri:', result?.error);
			}
		} else {
			const response = await fetch(
				`${CONFIG.ServerURL}/conversations/${conversationId}/context/${updatedContext.id}`,
				{
					method: 'PUT',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify(updatePayload),
				}
			);
			if (response.ok) {
				const data = await response.json();
				if (data.status === 'success') {
					console.log('✅ Context updated successfully via HTTP');
					await loadConversationContext();
				} else {
					console.error('❌ Failed to update context via HTTP:', data.error);
				}
			} else {
				console.error('❌ HTTP update request failed:', response.status);
			}
		}
	} catch (error) {
		console.error('❌ Error updating context:', error);
	}
}

async function sendMessage() {
	if (!input.trim() || sending) return;
	error = null;
	const currentRole = get(role) as string;
	const userText = input.trim();
	input = '';

	// Ensure we have a conversation
	if (!conversationId) {
		await initializeConversation();
	}

	addUserMessage(userText);
	sending = true;
	try {
		const requestBody: any = { role: currentRole, messages };

		// Include conversation_id if we have one
		if (conversationId) {
			requestBody.conversation_id = conversationId;
		}

		// Capture request for debugging
		_lastRequest = {
			timestamp: new Date().toISOString(),
			method: $is_tauri ? 'TAURI_INVOKE' : 'HTTP_POST',
			endpoint: $is_tauri ? 'chat' : `${CONFIG.ServerURL}/chat`,
			body: JSON.parse(JSON.stringify(requestBody)), // Deep clone for debug
			context_items_count: contextItems.length,
			conversation_id: conversationId,
		};

		let data: ChatResponse;
		if ($is_tauri) {
			// Tauri mode - use invoke
			data = await invoke('chat', { request: requestBody });
		} else {
			// Web mode - use HTTP API
			const res = await fetch(`${CONFIG.ServerURL}/chat`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(requestBody),
			});
			if (!res.ok) {
				throw new Error(`HTTP ${res.status}`);
			}
			data = await res.json();
		}

		// Capture response for debugging
		_lastResponse = {
			timestamp: new Date().toISOString(),
			status: data.status,
			model_used: data.model_used,
			message_length: data.message?.length || 0,
			full_response: JSON.parse(JSON.stringify(data)), // Deep clone for debug
			error: data.error || null,
		};

		modelUsed = data.model_used ?? null;
		if (data.status?.toLowerCase() === 'success' && data.message) {
			messages = [...messages, { role: 'assistant', content: data.message }];
			saveChatState();
		} else {
			error = data.error || 'Chat failed';
		}
	} catch (e: any) {
		error = e?.message || String(e);
		// Capture error in response debug info
		_lastResponse = {
			timestamp: new Date().toISOString(),
			status: 'error',
			error: e?.message || String(e),
			full_response: null,
		};
	} finally {
		sending = false;
	}
}

function _handleKeydown(e: KeyboardEvent) {
	if ((e.key === 'Enter' || e.key === 'Return') && !e.shiftKey) {
		e.preventDefault();
		sendMessage();
	}
}

// KG search modal handlers
function _openKGSearch() {
	_showKGSearchModal = true;
}

function _handleKGTermAdded(event: CustomEvent) {
	console.log('✅ KG term added to context:', event.detail.term);
	// Reload context to show the new KG term
	loadConversationContext();
}

function _handleKGIndexAdded(event: CustomEvent) {
	console.log('✅ KG index added to context for role:', event.detail.role);
	// Reload context to show the new KG index
	loadConversationContext();
}

// Svelte 5: Replace onMount with $effect for initialization and cleanup
$effect(() => {
	// Load markdown preference
	loadMdPref();

	// Hydrate chat state from localStorage if present; otherwise seed greeting
	loadChatState();
	if (messages.length === 0) {
		messages = [
			{
				role: 'assistant',
				content: 'Hi! How can I help you? Ask me anything about your search results or documents.',
			},
		];
		saveChatState();
	}

	// Lazy-load Tauri modules if running in desktop
	if (get(is_tauri)) {
		import('@tauri-apps/api/dialog').then((m) => (tauriDialog = m)).catch(() => {});
		import('@tauri-apps/api/fs').then((m) => (tauriFs = m)).catch(() => {});
	}

	// Initialize conversation and load context
	initializeConversation();

	// Refresh context when navigating to chat page
	if (typeof window !== 'undefined') {
		const handleFocus = () => {
			// Refresh context when window regains focus (user comes back to chat)
			if (conversationId) {
				loadConversationContext();
			}
		};

		window.addEventListener('focus', handleFocus);

		// Cleanup function
		return () => {
			window.removeEventListener('focus', handleFocus);
		};
	}
});

// Svelte 5: Replace reactive statement with $effect
// Compute provider/model hint from actual chat response or role settings
$effect(() => {
	try {
		// If we have a model_used from the actual chat response, analyze it
		if (modelUsed) {
			// Check if modelUsed is actually a provider name (common providers)
			const commonProviders = ['ollama', 'openrouter', 'anthropic', 'openai', 'groq'];

			if (commonProviders.includes(modelUsed.toLowerCase())) {
				// modelUsed is a provider name, try to get the actual model from role config
				const cfg: any = get(configStore) as any;
				const rname = get(role) as string;
				const r: any = cfg?.roles ? cfg.roles[rname] : null;

				let actualModel = '';
				if (modelUsed.toLowerCase() === 'ollama') {
					actualModel = r?.ollama_model || r?.llm_chat_model || '';
				} else if (modelUsed.toLowerCase() === 'openrouter') {
					actualModel = r?.openrouter_chat_model || r?.openrouter_model || '';
				}

				_providerHint = `Provider: ${modelUsed}${actualModel ? ` model: ${actualModel}` : ''}`;
			} else {
				// modelUsed is likely an actual model name, show it as Model
				_providerHint = `Model: ${modelUsed}`;
			}
		} else {
			// Otherwise, fall back to role configuration for display before any chat
			const cfg: any = get(configStore) as any;
			const rname = get(role) as string;
			const r: any = cfg?.roles ? cfg.roles[rname] : null;

			// Try to determine provider from role settings
			let provider = '';
			let model = '';

			// Check for OpenRouter configuration
			if (r?.openrouter_enabled || r?.openrouter_chat_model || r?.openrouter_model) {
				provider = 'openrouter';
				model = r?.openrouter_chat_model || r?.openrouter_model || '';
			}
			// Check for Ollama configuration
			else if (r?.ollama_model || r?.llm_provider === 'ollama') {
				provider = 'ollama';
				model = r?.ollama_model || r?.llm_chat_model || '';
			}
			// Check for generic LLM provider
			else if (r?.llm_provider) {
				provider = r.llm_provider;
				model = r?.llm_chat_model || '';
			}
			// Check global defaults
			else if (cfg?.default_model_provider) {
				provider = cfg.default_model_provider;
				model = cfg?.default_chat_model || '';
			}

			// Only show hint if we have provider info
			if (provider) {
				_providerHint = `Provider: ${provider}${model ? ` model: ${model}` : ''}`;
			} else {
				_providerHint = '';
			}
		}
	} catch (_e) {
		_providerHint = modelUsed ? `Model: ${modelUsed}` : '';
	}
});

// Copy/save helpers
async function _copyAsMarkdown(content: string) {
	try {
		await navigator.clipboard.writeText(content);
	} catch (e) {
		console.warn('Clipboard write failed', e);
	}
}

async function _saveAsMarkdown(content: string) {
	try {
		if (get(is_tauri) && tauriDialog && tauriFs) {
			const savePath = await tauriDialog.save({
				filters: [{ name: 'Markdown', extensions: ['md'] }],
			});
			if (savePath) {
				await tauriFs.writeTextFile(savePath as string, content);
			}
		} else {
			// Browser fallback: trigger download
			const blob = new Blob([content], { type: 'text/markdown;charset=utf-8' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = 'chat.md';
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
		}
	} catch (e) {
		console.warn('Save markdown failed', e);
	}
}

// ============================================================================
// Persistent Conversation Management
// ============================================================================

// Load a persistent conversation
async function loadPersistentConversation(conversationIdToLoad: string) {
	try {
		if ($is_tauri) {
			const result = await invoke('get_persistent_conversation', {
				conversationId: conversationIdToLoad,
			}) as { status: string; conversation?: any; error?: string };

			if (result.status === 'success' && result.conversation) {
				const conv = result.conversation;

				// Update conversation ID
				conversationId = conv.id;
				currentPersistentConversationId.set(conv.id);

				// Load messages
				messages = conv.messages || [];

				// Load context
				contextItems = conv.global_context || [];

				// Update UI state
				error = null;

				console.log('✅ Loaded persistent conversation:', conv.title);
				saveChatState();
			} else {
				error = result.error || 'Failed to load conversation';
				console.error('❌ Failed to load persistent conversation:', error);
			}
		}
	} catch (e) {
		console.error('❌ Error loading persistent conversation:', e);
		error = String(e);
	}
}

// Save current conversation as persistent
async function _savePersistentConversation() {
	if (!conversationId) {
		console.warn('⚠️ No conversation ID to save');
		return;
	}

	try {
		if ($is_tauri) {
			// Get the current conversation
			const result = await invoke('get_conversation', { conversationId }) as { status: string; conversation?: any; error?: string };

			if (result.status === 'success' && result.conversation) {
				const conv = result.conversation;

				// Save as persistent
				const saveResult = await invoke('create_persistent_conversation', {
					title: conv.title || 'Chat Conversation',
					role: conv.role,
				}) as { status: string; conversation?: any; error?: string };

				if (saveResult.status === 'success' && saveResult.conversation) {
					const persistentConv = saveResult.conversation;

					// Update the persistent conversation with current messages and context
					const updateResult = await invoke('update_persistent_conversation', {
						conversation: {
							...persistentConv,
							messages: conv.messages,
							global_context: conv.global_context,
						},
					}) as { status: string; error?: string };

					if (updateResult.status === 'success') {
						currentPersistentConversationId.set(persistentConv.id);
						console.log('✅ Saved persistent conversation:', persistentConv.id);
					}
				}
			}
		}
	} catch (e) {
		console.error('❌ Error saving persistent conversation:', e);
	}
}

// Handle session list selection
function _handleSessionSelect(conversationIdToLoad: string) {
	loadPersistentConversation(conversationIdToLoad);
}

// Handle new conversation from session list
function _handleNewConversation() {
	// Clear current state
	messages = [];
	contextItems = [];
	conversationId = null;
	currentPersistentConversationId.set(null);
	error = null;

	// Create new conversation
	createNewConversation();

	console.log('🆕 Started new conversation');
}

// Toggle session list panel
function _toggleSessionList() {
	showSessionList.update((v) => !v);
}
</script>

<section class="section" data-testid="chat-interface">
  <div class="container">
    <div class="chat-layout-grid" class:sidebar-hidden={!$showSessionList}>
      <!-- Session List Sidebar (conditionally shown) -->
      {#if $showSessionList}
        <div class="session-list-column">
          <SessionList
            currentConversationId={$currentPersistentConversationId}
            onSelectConversation={_handleSessionSelect}
            onNewConversation={_handleNewConversation}
          />
        </div>
      {/if}

      <!-- Main Chat Area -->
      <div class="main-chat-area">
        <div class="chat-header">
          <div>
            <h2 class="title is-4">Chat</h2>
            <p class="subtitle is-6">Role: {getRoleDisplay()}</p>
            {#if conversationId}
              <p class="is-size-7 has-text-grey">Conversation ID: {conversationId}</p>
            {/if}
          </div>
          <div class="chat-header-actions">
            <button
              class="button is-small"
              on:click={_toggleSessionList}
              title={$showSessionList ? 'Hide session list' : 'Show session list'}
            >
              <span class="icon is-small">
                <i class="fas fa-{$showSessionList ? 'angle-left' : 'bars'}"></i>
              </span>
              <span>{$showSessionList ? 'Hide' : 'History'}</span>
            </button>
            {#if conversationId && !$currentPersistentConversationId}
              <button
                class="button is-small is-success"
                on:click={_savePersistentConversation}
                title="Save this conversation"
              >
                <span class="icon is-small">
                  <i class="fas fa-save"></i>
                </span>
                <span>Save</span>
              </button>
            {/if}
          </div>
        </div>

        <div class="chat-window" data-testid="chat-messages">
      <div class="chat-toolbar">
        <div class="field is-grouped">
          <div class="control">
            <label class="checkbox is-size-7">
              <input type="checkbox" bind:checked={renderMarkdown} on:change={_saveMdPref} />
              Render markdown
            </label>
          </div>
          <div class="control">
            <label class="checkbox is-size-7">
              <input type="checkbox" bind:checked={_debugMode} />
              Debug mode
            </label>
          </div>
        </div>
      </div>
      {#each messages as m, i}
        <div class={`msg ${m.role}`}>
          <div class="bubble">
            {#if m.role === 'assistant'}
              <!-- Assistant messages: show markdown or plain text based on toggle -->
              {#if renderMarkdown}
                <div class="markdown-body"><Markdown source={m.content} /></div>
              {:else}
                <pre>{m.content}</pre>
              {/if}
              <!-- Always show action buttons for assistant messages -->
              <div class="msg-actions">
                <button class="button is-small is-light" title="Copy as markdown" on:click={() => _copyAsMarkdown(m.content)}>
                  <span class="icon is-small"><i class="fas fa-copy"></i></span>
                </button>
                <button class="button is-small is-light" title="Save as markdown" on:click={() => _saveAsMarkdown(m.content)}>
                  <span class="icon is-small"><i class="fas fa-save"></i></span>
                </button>
                {#if _debugMode && i === messages.length - 1}
                  <!-- Debug buttons only for the latest assistant message -->
                  <button
                    class="button is-small is-warning"
                    title="Show debug request (sent to LLM)"
                    on:click={() => _showDebugRequest = true}
                    disabled={!_lastRequest}
                  >
                    <span class="icon is-small"><i class="fas fa-bug"></i></span>
                    <span class="is-size-7">REQ</span>
                  </button>
                  <button
                    class="button is-small is-info"
                    title="Show debug response (from LLM)"
                    on:click={() => _showDebugResponse = true}
                    disabled={!_lastResponse}
                  >
                    <span class="icon is-small"><i class="fas fa-code"></i></span>
                    <span class="is-size-7">RES</span>
                  </button>
                {/if}
              </div>
            {:else}
              <!-- User/system messages: always show as plain text -->
              <pre>{m.content}</pre>
            {/if}
          </div>
        </div>
      {/each}
      {#if sending}
        <div class="msg assistant">
          <div class="bubble loading">
            <span class="icon is-small"><i class="fas fa-spinner fa-spin"></i></span>
            <span>Thinking...</span>
          </div>
        </div>
      {/if}
    </div>

    {#if _providerHint}
      <p class="is-size-7 has-text-grey">{_providerHint}</p>
    {/if}
    {#if error}
      <p class="has-text-danger is-size-7">{error}</p>
    {/if}

        <div class="field has-addons chat-input">
          <div class="control is-expanded">
            <textarea class="textarea" rows="3" bind:value={input} on:keydown={_handleKeydown} placeholder="Type your message and press Enter..." data-testid="chat-input"></textarea>
          </div>
          <div class="control">
            <button class="button is-primary" on:click={sendMessage} disabled={sending || !input.trim()} data-testid="send-message-button">
              <span class="icon"><i class="fas fa-paper-plane"></i></span>
            </button>
          </div>
        </div>
      </div>

      <!-- Context Panel -->
      <div class="context-panel-column">
        <div class="box context-panel" data-testid="context-panel">
          <h4 class="title is-5 mb-3">Context</h4>
          <div class="level is-mobile">
            <div class="level-left">
              <div class="level-item">
                <div class="buttons has-addons">
                  <button class="button is-small is-info" data-testid="kg-search-button" on:click={_openKGSearch}>
                    <span class="icon is-small">
                      <i class="fas fa-sitemap"></i>
                    </span>
                    <span>KG Search</span>
                  </button>
                </div>
              </div>
            </div>
            <div class="level-right">
              <div class="level-item">
                <button
                  class="button is-small is-light"
                  on:click={loadConversationContext}
                  disabled={_loadingContext}
                  data-testid="refresh-context-button"
                >
                  {#if _loadingContext}
                    <span class="icon is-small">
                      <i class="fas fa-spinner fa-spin"></i>
                    </span>
                  {:else}
                    <span class="icon is-small">
                      <i class="fas fa-refresh"></i>
                    </span>
                  {/if}
                </button>
              </div>
            </div>
          </div>

          <!-- Manual Context Addition Form -->
          {#if showAddContextForm}
            <div class="box has-background-light mb-4" data-testid="add-context-form">
              <div class="field">
                <label class="label is-small">Context Type</label>
                <div class="control">
                  <div class="select is-small is-fullwidth">
                    <select bind:value={newContextType} data-testid="context-type-select">
                      <option value="document">Document</option>
                      <option value="search_result">Search Result</option>
                      <option value="user_input">User Input</option>
                      <option value="note">Note</option>
                    </select>
                  </div>
                </div>
              </div>

              <div class="field">
                <label class="label is-small">Title</label>
                <div class="control">
                  <input class="input is-small" type="text" placeholder="Enter context title" bind:value={newContextTitle} data-testid="context-title-input" />
                </div>
              </div>

              <div class="field">
                <label class="label is-small">Content</label>
                <div class="control">
                  <textarea class="textarea is-small" rows="4" placeholder="Enter context content" bind:value={newContextContent} data-testid="context-content-textarea"></textarea>
                </div>
              </div>

              <div class="field is-grouped">
                <div class="control">
                  <button class="button is-primary is-small" on:click={_addManualContext} disabled={_savingContext || !newContextTitle.trim() || !newContextContent.trim()} data-testid="add-context-submit-button">
                    {#if _savingContext}
                      <span class="icon is-small"><i class="fas fa-spinner fa-spin"></i></span>
                    {:else}
                      <span class="icon is-small"><i class="fas fa-plus"></i></span>
                    {/if}
                  </button>
                </div>
                <div class="control">
                  <button class="button is-light is-small" on:click={toggleAddContextForm} disabled={_savingContext}>
                    <span class="icon is-small"><i class="fas fa-times"></i></span>
                    <span>Cancel</span>
                  </button>
                </div>
              </div>
            </div>
          {/if}

          {#if contextItems.length === 0}
            <div class="has-text-centered has-text-grey-light" data-testid="empty-context-message">
              <span class="icon is-large">
                <i class="fas fa-inbox fa-2x"></i>
              </span>
              <p class="is-size-6">No context items yet</p>
              <p class="is-size-7">Add documents from search results to provide context for your chat.</p>
            </div>
          {:else}
            <div class="context-items" data-testid="conversation-context">
              {#each contextItems as item, index}
                {#if item.context_type === 'KGTermDefinition' || item.context_type === 'KGIndex'}
                  <!-- Use specialized KG context item component -->
                  <KGContextItem
                    contextItem={item}
                    compact={true}
                    on:remove={e => deleteContext(e.detail.contextId)}
                    on:viewDetails={e => _editContext(e.detail.contextItem, e.detail.term)}
                  />
                {:else}
                  <!-- Use default context item rendering for non-KG items -->
                  <div class="context-item" data-context-id={item.id} data-testid={`context-item-${index}`} data-context-type={item.context_type}>
                    <div class="level is-mobile">
                      <div class="level-left">
                        <div class="level-item">
                          <span class="tag is-small {
                            item.context_type === 'Document' ? 'is-info' :
                            item.context_type === 'SearchResult' ? 'is-primary' :
                            item.context_type === 'UserInput' ? 'is-warning' : 'is-light'
                          }" data-testid={`context-type-${index}`}>
                            {item.context_type.replace(/([A-Z])/g, ' $1').trim()}
                          </span>
                        </div>
                      </div>
                      <div class="level-right">
                        <div class="level-item">
                          {#if item.relevance_score}
                            <span class="tag is-light is-small">
                              {item.relevance_score.toFixed(1)}
                            </span>
                          {/if}
                        </div>
                        <div class="level-item context-actions">
                          <div class="field is-grouped">
                            <div class="control">
                              <button
                                class="button is-small is-light"
                                on:click={() => _editContext(item)}
                                data-testid={`edit-context-${index}`}
                                title="Edit context"
                              >
                                <span class="icon is-small">
                                  <i class="fas fa-edit"></i>
                                </span>
                              </button>
                            </div>
                            <div class="control">
                              <button
                                class="button is-small is-light is-danger"
                                on:click={() => _confirmDeleteContext(item)}
                                data-testid={`delete-context-${index}`}
                                title="Delete context"
                              >
                                <span class="icon is-small">
                                  <i class="fas fa-trash"></i>
                                </span>
                              </button>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>

                    <h6 class="title is-6 has-text-dark" data-testid={`context-title-${index}`}>{item.title}</h6>

                    <div class="content is-small">
                      {#if item.summary}
                        <p class="context-summary" data-testid={`context-summary-${index}`}>
                          {item.summary}
                        </p>
                      {:else}
                        <p class="context-preview" data-testid={`context-content-${index}`}>
                          {item.content.substring(0, 150)}{item.content.length > 150 ? '...' : ''}
                        </p>
                      {/if}
                    </div>

                    <div class="is-size-7 has-text-grey">
                      Added: {new Date(item.created_at).toLocaleString()}
                    </div>
                  </div>
                {/if}

                {#if index < contextItems.length - 1}
                  <hr class="context-divider">
                {/if}
              {/each}
            </div>
          {/if}

          <div class="mt-4">
            <div class="level is-mobile">
              <div class="level-left">
                <div class="level-item">
                  <span class="tag is-light is-small" data-testid="context-summary">
                    {contextItems.length} context items
                  </span>
                </div>
              </div>
              <div class="level-right">
                <div class="level-item">
                  <span class="is-size-7 has-text-grey">
                    Context is automatically included in your chat
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</section>

<!-- Debug Request Modal -->
{#if _showDebugRequest}
  <div class="modal is-active">
    <div class="modal-background" on:click={() => _showDebugRequest = false} role="button" tabindex="0" aria-label="Close debug request modal"></div>
    <div class="modal-card">
      <header class="modal-card-head">
        <p class="modal-card-title">
          <span class="icon"><i class="fas fa-bug"></i></span>
          Debug Request (Sent to LLM)
        </p>
        <button class="delete" aria-label="close" on:click={() => _showDebugRequest = false}></button>
      </header>
      <section class="modal-card-body">
        {#if _lastRequest}
          <div class="content">
            <p class="has-text-weight-semibold">Request Details:</p>
            <div class="tags are-medium">
              <span class="tag is-info">Method: {_lastRequest.method}</span>
              <span class="tag is-primary">Time: {new Date(_lastRequest.timestamp).toLocaleTimeString()}</span>
              <span class="tag is-success">Context Items: {_lastRequest.context_items_count}</span>
            </div>
            <p class="has-text-weight-semibold mt-4">Full Request JSON:</p>
            <pre class="debug-json"><code>{JSON.stringify(_lastRequest, null, 2)}</code></pre>
          </div>
        {:else}
          <p class="has-text-grey">No request data available</p>
        {/if}
      </section>
      <footer class="modal-card-foot">
        <button class="button" on:click={() => _showDebugRequest = false}>Close</button>
        {#if _lastRequest}
          <button class="button is-primary" on:click={() => _copyAsMarkdown(JSON.stringify(_lastRequest, null, 2))}>
            <span class="icon"><i class="fas fa-copy"></i></span>
            <span>Copy JSON</span>
          </button>
        {/if}
      </footer>
    </div>
  </div>
{/if}

<!-- Debug Response Modal -->
{#if _showDebugResponse}
  <div class="modal is-active">
    <div class="modal-background" on:click={() => _showDebugResponse = false} role="button" tabindex="0" aria-label="Close debug response modal"></div>
    <div class="modal-card">
      <header class="modal-card-head">
        <p class="modal-card-title">
          <span class="icon"><i class="fas fa-code"></i></span>
          Debug Response (From LLM)
        </p>
        <button class="delete" aria-label="close" on:click={() => _showDebugResponse = false}></button>
      </header>
      <section class="modal-card-body">
        {#if _lastResponse}
          <div class="content">
            <p class="has-text-weight-semibold">Response Details:</p>
            <div class="tags are-medium">
              <span class="tag is-info">Status: {_lastResponse.status}</span>
              <span class="tag is-primary">Time: {new Date(_lastResponse.timestamp).toLocaleTimeString()}</span>
              {#if _lastResponse.model_used}
                <span class="tag is-success">Model: {_lastResponse.model_used}</span>
              {/if}
              {#if _lastResponse.message_length}
                <span class="tag is-warning">Length: {_lastResponse.message_length} chars</span>
              {/if}
            </div>
            <p class="has-text-weight-semibold mt-4">Full Response JSON:</p>
            <pre class="debug-json"><code>{JSON.stringify(_lastResponse, null, 2)}</code></pre>
          </div>
        {:else}
          <p class="has-text-grey">No response data available</p>
        {/if}
      </section>
      <footer class="modal-card-foot">
        <button class="button" on:click={() => _showDebugResponse = false}>Close</button>
        {#if _lastResponse}
          <button class="button is-primary" on:click={() => _copyAsMarkdown(JSON.stringify(_lastResponse, null, 2))}>
            <span class="icon"><i class="fas fa-copy"></i></span>
            <span>Copy JSON</span>
          </button>
        {/if}
      </footer>
    </div>
  </div>
{/if}

<!-- Context Edit Modal -->
<ContextEditModal
  bind:active={_showContextEditModal}
  context={_editingContext}
  mode={_contextEditMode}
  on:update={e => _updateContext(e.detail)}
  on:delete={e => deleteContext(e.detail)}
  on:close={() => {
    _showContextEditModal = false;
    _editingContext = null;
  }}
/>

<!-- KG Search Modal -->
<KGSearchModal
  bind:active={_showKGSearchModal}
  conversationId={conversationId}
  on:termAdded={_handleKGTermAdded}
  on:kgIndexAdded={_handleKGIndexAdded}
/>

<!-- KG Document Modal -->
{#if kgDocument}
  <ArticleModal
    bind:active={_showKgModal}
    item={kgDocument}
    kgTerm={_kgTerm}
    kgRank={_kgRank}
  />
{/if}

<style>
  /* CSS Grid Layout for Chat Interface */
  .chat-layout-grid {
    display: grid;
    grid-template-columns: 1fr minmax(300px, 400px);
    gap: 0;
    min-height: calc(100vh - 200px);
    transition: grid-template-columns 0.3s ease;
  }

  .chat-layout-grid:not(.sidebar-hidden) {
    grid-template-columns: minmax(280px, 350px) 1fr minmax(300px, 400px);
  }

  .session-list-column {
    border-right: 1px solid var(--bs-border-color);
    height: 100%;
    overflow: hidden;
    background: var(--bs-body-bg);
  }

  .main-chat-area {
    display: flex;
    flex-direction: column;
    min-width: 0; /* Prevents flex item from overflowing */
    height: 100%;
  }

  .context-panel-column {
    border-left: 1px solid var(--bs-border-color);
    height: 100%;
    overflow: hidden;
    background: var(--bs-body-bg);
    padding: 0;
  }

  .chat-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  .chat-header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .chat-window {
    border: 1px solid #ececec;
    border-radius: 6px;
    padding: 0.75rem;
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: #fff;
    margin-bottom: 0.75rem;
    display: flex;
    flex-direction: column;
  }
  .chat-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }
  .markdown-body :global(pre), .markdown-body :global(code) {
    white-space: pre-wrap;
    word-break: break-word;
  }
  .msg-actions {
    margin-top: 0.25rem;
    display: flex;
    gap: 0.25rem;
  }
  .msg { display: flex; margin-bottom: 0.5rem; }
  .msg.user { justify-content: flex-end; }
  .msg.assistant { justify-content: flex-start; }
  .bubble { max-width: 70ch; padding: 0.5rem 0.75rem; border-radius: 12px; }
  .user .bubble { background: #3273dc; color: #fff; }
  .assistant .bubble { background: #f5f5f5; color: #333; }
  .bubble pre { white-space: pre-wrap; word-wrap: break-word; margin: 0; font-family: inherit; }
  .loading { display: inline-flex; gap: 0.5rem; align-items: center; }
  .chat-input {
    align-items: flex-end;
    flex-shrink: 0;
    margin-top: auto;
  }

  .chat-input .control.is-expanded {
    flex: 1;
    min-width: 0;
  }

  .chat-input .control.is-expanded .textarea {
    resize: vertical;
    min-height: 3rem;
    max-height: 8rem;
    width: 100%;
  }

  .chat-input .control:not(.is-expanded) {
    flex-shrink: 0;
  }

  /* Context Panel Styles */
  .context-panel {
    height: 100%;
    overflow-y: auto;
    background: #fafafa;
    margin: 0;
  }

  .context-items {
    max-height: 50vh;
    overflow-y: auto;
  }

  .context-item {
    padding: 0.75rem 0;
    transition: background-color 0.2s ease;
  }

  .context-item:hover {
    background-color: rgba(0, 0, 0, 0.02);
    border-radius: 6px;
    padding: 0.75rem;
    margin: 0 -0.75rem;
  }

  .context-preview {
    line-height: 1.4;
    color: #666;
    margin-bottom: 0.5rem;
  }

  .context-summary {
    line-height: 1.4;
    color: #333;
    font-weight: 500;
    margin-bottom: 0.5rem;
    font-style: italic;
  }

  .context-actions {
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .context-item:hover .context-actions {
    opacity: 1;
  }

  .context-divider {
    margin: 0.5rem 0;
    background-color: #e8e8e8;
  }

  /* Debug JSON Styling */
  .debug-json {
    background-color: #f5f5f5;
    border: 1px solid #e8e8e8;
    border-radius: 4px;
    padding: 1rem;
    font-family: 'Courier New', Consolas, monospace;
    font-size: 0.8rem;
    line-height: 1.4;
    max-height: 60vh;
    overflow: auto;
    white-space: pre-wrap;
    word-wrap: break-word;
  }

  .debug-json code {
    background: none;
    color: #333;
    font-family: inherit;
    font-size: inherit;
  }

  /* Responsive Design */
  @media screen and (max-width: 1024px) {
    .chat-layout-grid {
      grid-template-columns: 1fr minmax(280px, 350px);
    }

    .chat-layout-grid:not(.sidebar-hidden) {
      grid-template-columns: minmax(250px, 300px) 1fr minmax(280px, 350px);
    }
  }

  @media screen and (max-width: 768px) {
    .chat-layout-grid {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr auto;
      min-height: calc(100vh - 150px);
    }

    .chat-layout-grid:not(.sidebar-hidden) .session-list-column {
      max-height: 30vh;
      border-right: none;
      border-bottom: 1px solid var(--bs-border-color);
    }

    .chat-layout-grid:not(.sidebar-hidden) .main-chat-area {
      min-height: 50vh;
    }

    .context-panel-column {
      border-left: none;
      border-top: 1px solid var(--bs-border-color);
      max-height: 30vh;
    }

    .chat-header {
      flex-direction: column;
      align-items: stretch;
      gap: 0.5rem;
    }

    .chat-header-actions {
      justify-content: center;
    }

    .context-panel {
      margin-top: 1rem;
      max-height: 40vh;
    }

    .chat-input .control.is-expanded .textarea {
      min-height: 4rem;
    }
  }

  @media screen and (max-width: 480px) {
    .chat-layout-grid {
      min-height: calc(100vh - 120px);
    }

    .chat-header-actions {
      flex-direction: column;
      gap: 0.25rem;
    }

    .chat-header-actions .button {
      width: 100%;
      justify-content: center;
    }

    .bubble {
      max-width: 90%;
    }
  }
</style>
