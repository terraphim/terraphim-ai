<script lang="ts">
import { invoke } from '@tauri-apps/api/tauri';
import { fade } from 'svelte/transition';
// @ts-expect-error
import SvelteMarkdown from 'svelte-markdown';
import { router } from 'tinro';
import { is_tauri, role, configStore as roleConfigStore } from '$lib/stores';
import { CONFIG } from '../../config';
import type { DocumentListResponse, Role } from '../generated/types';
import configStore from '../ThemeSwitcher.svelte';
import ArticleModal from './ArticleModal.svelte';
import AtomicSaveModal from './AtomicSaveModal.svelte';
import type { Document } from './SearchResult';

// API Response interfaces
interface ConversationsResponse {
	conversations: Array<{
		id: string;
		title?: string;
		created_at: string;
	}>;
}

interface CreateConversationResponse {
	status: string;
	conversation_id?: string;
	error?: string;
}

interface ConversationResponse {
	status: string;
	conversation?: {
		id: string;
		title?: string;
	};
	error?: string;
}

interface EnhancedDocument {
	source_type: string;
	document_id: string;
	url?: string;
	tags?: string[];
	rank?: number;
	[key: string]: any;
}

interface MenuItem {
	id: string;
	label: string;
	icon: string;
	action?: () => void;
	visible: boolean;
	title: string;
	className: string;
	disabled?: boolean;
	testId?: string;
	isLink?: boolean;
	href?: string;
}

export let item: Document;
let _showModal = false;
let _showKgModal = false;
let _showAtomicSaveModal = false;
let kgDocument: Document | null = null;
let _kgTerm: string | null = null;
let kgRank: number | null = null;
let _loadingKg = false;

// Summarization state
let aiSummary: string | null = null;
let summaryLoading = false;
let summaryError: string | null = null;
let _showAiSummary = false;
let summaryFromCache = false;

// Context addition state
let addingToContext = false;
let contextAdded = false;
let contextError: string | null = null;

// Chat with document state
let chattingWithDocument = false;
let chatStarted = false;

// Check if current role has atomic server configuration
$: hasAtomicServer = checkAtomicServerAvailable();

// Data-driven menu configuration
$: menuItems = generateMenuItems();

function generateMenuItems(): MenuItem[] {
	const items: MenuItem[] = [];

	// Always show download to markdown - downloads file only
	items.push({
		id: 'download-markdown',
		label: 'Download to Markdown',
		icon: 'fas fa-download',
		action: () => downloadToMarkdown(),
		visible: true,
		title: 'Download document as markdown file',
		className: '',
	});

	// Show atomic save only if configured
	if (hasAtomicServer) {
		items.push({
			id: 'save-atomic',
			label: 'Save to Atomic Server',
			icon: 'fas fa-cloud-upload-alt',
			action: () => onAtomicSaveClick(),
			visible: true,
			title: 'Save article to Atomic Server',
			className: 'has-text-primary',
		});
	}

	// Show external URL if available - opens URL in new tab and article modal
	if (item.url) {
		items.push({
			id: 'external-url',
			label: 'Open URL',
			icon: 'fas fa-link',
			action: () => openUrlAndModal(),
			visible: true,
			title: 'Open original URL in new tab',
			className: '',
		});
	}

	// VSCode integration removed as requested

	// Add to context for LLM conversation
	items.push({
		id: 'add-context',
		label: contextAdded ? 'Added to Context ✓' : addingToContext ? 'Adding...' : 'Add to Context',
		icon: contextAdded
			? 'fas fa-check-circle'
			: addingToContext
				? 'fas fa-spinner fa-spin'
				: 'fas fa-plus-circle',
		action: () => addToContext(),
		visible: true,
		title: contextAdded
			? 'Document successfully added to chat context. Go to Chat tab to see it.'
			: 'Add document to LLM conversation context',
		disabled: addingToContext || contextAdded,
		className: contextAdded ? 'has-text-success' : contextError ? 'has-text-danger' : '',
		testId: 'add-to-context-button',
	});

	// Chat with document (add to context + navigate to chat)
	items.push({
		id: 'chat-with-document',
		label: chatStarted
			? 'Opening Chat...'
			: chattingWithDocument
				? 'Adding to Chat...'
				: 'Chat with Document',
		icon: chatStarted
			? 'fas fa-external-link-alt'
			: chattingWithDocument
				? 'fas fa-spinner fa-spin'
				: 'fas fa-comment-dots',
		action: () => addToContextAndChat(),
		visible: true,
		title: chatStarted
			? 'Opening chat with this document'
			: 'Add document to context and open chat',
		disabled: chattingWithDocument || chatStarted || addingToContext,
		className: chatStarted
			? 'has-text-info'
			: contextError
				? 'has-text-danger'
				: 'has-text-primary',
		testId: 'chat-with-document-button',
	});

	return items;
}

function checkAtomicServerAvailable(): boolean {
	const currentRoleName = $role;
	const config = $roleConfigStore;

	if (!config?.roles || !currentRoleName) {
		return false;
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
		return false;
	}

	if (!currentRole) {
		return false;
	}

	// Check if role has any writable atomic server haystacks
	const atomicHaystacks =
		currentRole.haystacks?.filter(
			(haystack) => haystack.service === 'Atomic' && haystack.location && !haystack.read_only
		) || [];

	return atomicHaystacks.length > 0;
}

const onTitleClick = () => {
	_showModal = true;
};

const onAtomicSaveClick = () => {
	console.log('🔄 Opening atomic save modal for document:', item.title);
	_showAtomicSaveModal = true;
};

async function _handleTagClick(tag: string) {
	_loadingKg = true;
	_kgTerm = tag;

	// Add debugging information
	console.log('🔍 KG Search Debug Info:');
	console.log('  Tag clicked:', tag);
	console.log('  Current role:', $role);
	console.log('  Is Tauri mode:', $is_tauri);

	try {
		if ($is_tauri) {
			// Use Tauri command for desktop app
			console.log('  Making Tauri invoke call...');
			console.log('  Tauri command: find_documents_for_kg_term');
			console.log('  Tauri params:', { roleName: $role, term: tag });

			const response: DocumentListResponse = await invoke('find_documents_for_kg_term', {
				roleName: $role,
				term: tag,
			});

			console.log('  📥 Tauri response received:');
			console.log('    Status:', response.status);
			console.log('    Results count:', response.results?.length || 0);
			console.log('    Total:', response.total || 0);
			console.log('    Full response:', JSON.stringify(response, null, 2));

			if (response.status === 'success' && response.results && response.results.length > 0) {
				// Get the first (highest-ranked) document
				kgDocument = response.results[0];
				kgRank = kgDocument.rank || 0;
				console.log('  ✅ Found KG document:');
				console.log('    Title:', kgDocument.title);
				console.log('    Rank:', kgRank);
				console.log('    Body length:', kgDocument.body?.length || 0, 'characters');
				_showKgModal = true;
			} else {
				console.warn(`  ⚠️  No KG documents found for term: "${tag}" in role: "${$role}"`);
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
			const encodedTerm = encodeURIComponent(tag);
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
				if (kgDocument) {
					kgRank = kgDocument.rank || 0;
					console.log('  ✅ Found KG document:');
					console.log('    Title:', kgDocument.title);
					console.log('    Rank:', kgRank);
					console.log('    Body length:', kgDocument.body?.length || 0, 'characters');
					_showKgModal = true;
				}
			} else {
				console.warn(`  ⚠️  No KG documents found for term: "${tag}" in role: "${$role}"`);
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
		console.error('  Error type:', error instanceof Error ? error.constructor.name : 'Unknown');
		console.error('  Error message:', error instanceof Error ? error.message : String(error));
		console.error('  Request details:', {
			tag,
			role: $role,
			isTauri: $is_tauri,
			timestamp: new Date().toISOString(),
		});

		if (!$is_tauri && error instanceof Error && error.message?.includes('Failed to fetch')) {
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

async function _generateSummary() {
	if (summaryLoading || !item.id || !$role) return;

	summaryLoading = true;
	summaryError = null;

	console.log('🤖 AI Summary Debug Info:');
	console.log('  Document ID:', item.id);
	console.log('  Current role:', $role);
	console.log('  Is Tauri mode:', $is_tauri);

	try {
		const requestBody = {
			document_id: item.id,
			role: $role,
			max_length: 250,
			force_regenerate: false,
		};

		console.log('  📤 Summarization request:', requestBody);

		let response: unknown;

		if ($is_tauri) {
			// For Tauri mode, we'll make an HTTP request directly
			// as we don't have a Tauri command for summarization yet
			const baseUrl = CONFIG.ServerURL;
			const url = `${baseUrl}/documents/summarize`;

			response = await fetch(url, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify(requestBody),
			});
		} else {
			// Web mode - direct HTTP request
			const baseUrl = CONFIG.ServerURL;
			const url = `${baseUrl}/documents/summarize`;

			response = await fetch(url, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify(requestBody),
			});
		}

		console.log('  📥 Summary response received:');
		console.log('    Status code:', response.status);
		console.log('    Status text:', response.statusText);

		if (!response.ok) {
			throw new Error(`HTTP error! Status: ${response.status} - ${response.statusText}`);
		}

		const data = await response.json();
		console.log('  📄 Summary response data:', data);

		if (data.status === 'success' && data.summary) {
			aiSummary = data.summary;
			summaryFromCache = data.from_cache || false;
			_showAiSummary = true;
			console.log('  ✅ Summary generated successfully');
			console.log('    Summary length:', aiSummary?.length || 0, 'characters');
			console.log('    From cache:', summaryFromCache);
			console.log('    Model used:', data.model_used);
		} else {
			summaryError = data.error || 'Failed to generate summary';
			console.error('  ❌ Summary generation failed:', summaryError);
		}
	} catch (error) {
		console.error('❌ Error generating summary:');
		console.error('  Error type:', error instanceof Error ? error.constructor.name : 'Unknown');
		console.error('  Error message:', error instanceof Error ? error.message : String(error));
		console.error('  Request details:', {
			document_id: item.id,
			role: $role,
			isTauri: $is_tauri,
			timestamp: new Date().toISOString(),
		});

		summaryError = error instanceof Error ? error.message : 'Network error occurred';

		if (error instanceof Error && error.message?.includes('Failed to fetch')) {
			console.error('  💡 Network error suggestions:');
			console.error('    1. Check if server is running on expected port');
			console.error('    2. Verify OpenRouter is enabled for this role');
			console.error('    3. Check OPENROUTER_KEY environment variable');
			console.error('    4. Verify server URL in CONFIG.ServerURL');
		}
	} finally {
		summaryLoading = false;
	}
}

async function downloadToMarkdown() {
	console.log('📥 Downloading document as markdown:', item.title);
	console.log('📄 Document data:', {
		title: item.title,
		bodyLength: item.body?.length,
		tags: item.tags,
	});
	console.log('🖥️ Environment check - is_tauri:', $is_tauri);

	// Create markdown content
	let markdownContent = `# ${item.title}\n\n`;

	// Add metadata
	markdownContent += `**Source:** Terraphim Search\n`;
	markdownContent += `**Rank:** ${item.rank || 'N/A'}\n`;
	if (item.url) {
		markdownContent += `**URL:** ${item.url}\n`;
	}
	if (item.tags && item.tags.length > 0) {
		markdownContent += `**Tags:** ${item.tags.join(', ')}\n`;
	}
	markdownContent += `**Downloaded:** ${new Date().toISOString()}\n\n`;

	// Add description if available
	if (item.description) {
		markdownContent += `## Description\n\n${item.description}\n\n`;
	}

	// Add main content
	markdownContent += `## Content\n\n${item.body}\n`;

	// Create filename
	const filename = `${item.title.replace(/[^a-z0-9]/gi, '_').toLowerCase()}_${Date.now()}.md`;

	// Use the proven saveAsMarkdown implementation from Chat component
	try {
		if ($is_tauri) {
			// Import Tauri APIs dynamically
			const { save } = await import('@tauri-apps/api/dialog');
			const { writeTextFile } = await import('@tauri-apps/api/fs');

			console.log('💾 Using Tauri save dialog...');
			const savePath = await save({
				filters: [{ name: 'Markdown', extensions: ['md'] }],
				defaultPath: filename,
			});

			if (savePath) {
				await writeTextFile(savePath as string, markdownContent);
				console.log('✅ File saved via Tauri:', savePath);
			} else {
				console.log('❌ Save dialog cancelled');
			}
		} else {
			// Browser fallback: trigger download
			console.log('🌐 Using browser download fallback...');
			const blob = new Blob([markdownContent], { type: 'text/markdown;charset=utf-8' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = filename;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
			console.log('✅ File downloaded via browser:', filename);
		}
	} catch (error) {
		console.error('❌ Download failed:', error);

		// Fallback to browser download even in Tauri if the above fails
		console.log('🔄 Falling back to browser download...');
		const blob = new Blob([markdownContent], { type: 'text/markdown;charset=utf-8' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = filename;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);
		console.log('✅ Fallback download completed:', filename);
	}
}

function _downloadToMarkdownAndOpenModal() {
	console.log('🔄 Starting download and modal process...');
	// First download the markdown file
	downloadToMarkdown();
	// Add a small delay before opening modal to ensure download starts
	setTimeout(() => {
		console.log('📖 Opening article modal...');
		onTitleClick();
	}, 100);
}

function openUrlAndModal() {
	console.log('🔄 Opening URL in new tab and article modal...');
	// First open the URL in a new tab
	if (item.url) {
		console.log('🔗 Opening URL in new tab:', item.url);
		window.open(item.url, '_blank');
	}
	// Add a small delay before opening modal
	setTimeout(() => {
		console.log('📖 Opening article modal...');
		onTitleClick();
	}, 100);
}

function _openInVSCode() {
	const vscodeUrl = `vscode://${encodeURIComponent(item.title)}.md?${encodeURIComponent(item.body)}`;
	window.open(vscodeUrl, '_blank');
}

async function addToContext() {
	console.log('📝 Adding document to LLM context:', item.title);

	// Reset state and show loading
	addingToContext = true;
	contextAdded = false;
	contextError = null;

	try {
		let conversationId = null;

		if ($is_tauri) {
			// First, try to get or create a conversation
			try {
				const conversations = (await invoke('list_conversations')) as ConversationsResponse;
				console.log('📋 Available conversations:', conversations);

				// Find an existing conversation or use the first one
				if (conversations?.conversations && conversations.conversations.length > 0) {
					conversationId = conversations.conversations[0].id;
					console.log('🎯 Using existing conversation:', conversationId);
				} else {
					// Create a new conversation
					const newConv = (await invoke('create_conversation', {
						title: 'Search Context',
						role: $role || 'default',
					})) as CreateConversationResponse;
					if (newConv.status === 'success' && newConv.conversation_id) {
						conversationId = newConv.conversation_id;
						console.log('🆕 Created new conversation:', conversationId);
					} else {
						throw new Error(`Failed to create conversation: ${newConv.error || 'Unknown error'}`);
					}
				}
			} catch (convError) {
				console.error('❌ Failed to manage conversations:', convError);
				const errorMessage = convError instanceof Error ? convError.message : String(convError);
				throw new Error(`Could not create or find conversation: ${errorMessage}`);
			}

			// Use Tauri command for desktop app
			const enhancedItem = item as unknown as EnhancedDocument;
			const metadata: Record<string, any> = {
				source_type: 'document',
				document_id: item.id,
			};

			if (enhancedItem.url) metadata.url = enhancedItem.url;
			if (enhancedItem.tags && enhancedItem.tags.length > 0)
				metadata.tags = enhancedItem.tags.join(', ');
			if (enhancedItem.rank !== undefined) metadata.rank = enhancedItem.rank.toString();

			const contextResult = await invoke('add_context_to_conversation', {
				conversationId: conversationId,
				contextType: 'document',
				title: item.title,
				content: item.body,
				metadata: metadata,
			});

			console.log('✅ Document added to context via Tauri:', contextResult);
		} else {
			// Web mode - use HTTP API
			const baseUrl = CONFIG.ServerURL;

			// First, try to get or create a conversation
			try {
				const conversationsResponse = await fetch(`${baseUrl}/conversations`);
				if (conversationsResponse.ok) {
					const conversationsData = await conversationsResponse.json();
					if (conversationsData.conversations && conversationsData.conversations.length > 0) {
						conversationId = conversationsData.conversations[0].id;
						console.log('🎯 Using existing conversation:', conversationId);
					} else {
						// Create a new conversation
						const newConvResponse = await fetch(`${baseUrl}/conversations`, {
							method: 'POST',
							headers: { 'Content-Type': 'application/json' },
							body: JSON.stringify({
								title: 'Search Context',
								role: $role || 'default',
							}),
						});
						if (newConvResponse.ok) {
							const newConvData = await newConvResponse.json();
							if (newConvData.status === 'success' && newConvData.conversation_id) {
								conversationId = newConvData.conversation_id;
								console.log('🆕 Created new conversation:', conversationId);
							} else {
								throw new Error(
									`Failed to create conversation: ${newConvData.error || 'Unknown error'}`
								);
							}
						} else {
							throw new Error(
								`Failed to create conversation: ${newConvResponse.status} ${newConvResponse.statusText}`
							);
						}
					}
				} else {
					throw new Error(
						`Failed to list conversations: ${conversationsResponse.status} ${conversationsResponse.statusText}`
					);
				}
			} catch (convError) {
				console.error('❌ Failed to manage conversations:', convError);
				const errorMessage = convError instanceof Error ? convError.message : String(convError);
				throw new Error(`Could not create or find conversation: ${errorMessage}`);
			}

			// Add document context to conversation
			const url = `${baseUrl}/conversations/${conversationId}/context`;
			const response = await fetch(url, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					context_type: 'document',
					title: item.title,
					content: item.body,
					metadata: {
						source_type: 'document',
						document_id: item.id,
						url: item.url || '',
						tags: item.tags ? item.tags.join(', ') : '',
						rank: item.rank ? item.rank.toString() : '0',
					},
				}),
			});

			if (!response.ok) {
				throw new Error(`HTTP error! Status: ${response.status} - ${response.statusText}`);
			}

			const data = await response.json();
			console.log('✅ Document added to context via HTTP:', data);
		}

		console.log('✅ Successfully added document to LLM context');

		// Show success state
		contextAdded = true;

		// Show success notification with navigation hint
		const notification = window.document.createElement('div');
		notification.className = 'notification is-success is-light';
		notification.innerHTML = `
        <button class="delete" onclick="this.parentElement.remove()"></button>
        <strong>✓ Added to Chat Context</strong><br>
        <small>Document added successfully. <a href="/chat" class="has-text-success has-text-weight-bold">Go to Chat →</a> to see it in the context panel.</small>
      `;
		notification.style.cssText =
			'position: fixed; top: 20px; right: 20px; z-index: 1000; max-width: 350px;';
		window.document.body.appendChild(notification);

		// Auto-remove notification after 8 seconds
		setTimeout(() => {
			notification.remove();
		}, 8000);

		// Reset success state after a delay to allow re-adding if needed
		setTimeout(() => {
			contextAdded = false;
		}, 5000);
	} catch (error) {
		console.error('❌ Error adding document to context:', error);

		// Show error state
		contextError = error instanceof Error ? error.message : 'Failed to add document to context';

		// Show error notification
		const notification = window.document.createElement('div');
		notification.className = 'notification is-danger is-light';
		notification.innerHTML = `
        <button class="delete" onclick="this.parentElement.remove()"></button>
        <strong>✗ Failed to Add Context</strong><br>
        <small>${contextError}</small>
      `;
		notification.style.cssText =
			'position: fixed; top: 20px; right: 20px; z-index: 1000; max-width: 350px;';
		window.document.body.appendChild(notification);

		// Auto-remove notification after 8 seconds
		setTimeout(() => {
			notification.remove();
		}, 8000);

		// Clear error after a delay
		setTimeout(() => {
			contextError = null;
		}, 5000);
	} finally {
		addingToContext = false;
	}
}

async function addToContextAndChat() {
	console.log('💬 Adding document to context and opening chat:', item.title);

	// Reset state and show loading
	chattingWithDocument = true;
	chatStarted = false;
	contextError = null;

	try {
		let conversationId = null;

		if ($is_tauri) {
			// First, try to get or create a conversation
			try {
				const conversations = (await invoke('list_conversations')) as ConversationsResponse;
				console.log('📋 Available conversations:', conversations);

				// Find an existing conversation or use the first one
				if (conversations?.conversations && conversations.conversations.length > 0) {
					conversationId = conversations.conversations[0].id;
					console.log('🎯 Using existing conversation:', conversationId);
				} else {
					// Create a new conversation
					const newConv = (await invoke('create_conversation', {
						title: 'Chat with Documents',
						role: $role || 'default',
					})) as CreateConversationResponse;
					if (newConv.status === 'success' && newConv.conversation_id) {
						conversationId = newConv.conversation_id;
						console.log('🆕 Created new conversation:', conversationId);
					} else {
						throw new Error(`Failed to create conversation: ${newConv.error || 'Unknown error'}`);
					}
				}
			} catch (convError) {
				console.error('❌ Failed to manage conversations:', convError);
				const errorMessage = convError instanceof Error ? convError.message : String(convError);
				throw new Error(`Could not create or find conversation: ${errorMessage}`);
			}

			// Use Tauri command for desktop app
			const enhancedItem = item as unknown as EnhancedDocument;
			const metadata: Record<string, any> = {
				source_type: 'document',
				document_id: item.id,
			};

			if (enhancedItem.url) metadata.url = enhancedItem.url;
			if (enhancedItem.tags && enhancedItem.tags.length > 0)
				metadata.tags = enhancedItem.tags.join(', ');
			if (enhancedItem.rank !== undefined) metadata.rank = enhancedItem.rank.toString();

			const contextResult = await invoke('add_context_to_conversation', {
				conversationId: conversationId,
				contextType: 'document',
				title: item.title,
				content: item.body,
				metadata: metadata,
			});

			console.log('✅ Document added to context via Tauri:', contextResult);
		} else {
			// Web mode - use HTTP API
			const baseUrl = CONFIG.ServerURL;

			// First, try to get or create a conversation
			try {
				const conversationsResponse = await fetch(`${baseUrl}/conversations`);
				if (conversationsResponse.ok) {
					const conversationsData = await conversationsResponse.json();
					if (conversationsData.conversations && conversationsData.conversations.length > 0) {
						conversationId = conversationsData.conversations[0].id;
						console.log('🎯 Using existing conversation:', conversationId);
					} else {
						// Create a new conversation
						const newConvResponse = await fetch(`${baseUrl}/conversations`, {
							method: 'POST',
							headers: { 'Content-Type': 'application/json' },
							body: JSON.stringify({
								title: 'Chat with Documents',
								role: $role || 'default',
							}),
						});
						if (newConvResponse.ok) {
							const newConvData = await newConvResponse.json();
							if (newConvData.status === 'success' && newConvData.conversation_id) {
								conversationId = newConvData.conversation_id;
								console.log('🆕 Created new conversation:', conversationId);
							} else {
								throw new Error(
									`Failed to create conversation: ${newConvData.error || 'Unknown error'}`
								);
							}
						} else {
							throw new Error(
								`Failed to create conversation: ${newConvResponse.status} ${newConvResponse.statusText}`
							);
						}
					}
				} else {
					throw new Error(
						`Failed to list conversations: ${conversationsResponse.status} ${conversationsResponse.statusText}`
					);
				}
			} catch (convError) {
				console.error('❌ Failed to manage conversations:', convError);
				const errorMessage = convError instanceof Error ? convError.message : String(convError);
				throw new Error(`Could not create or find conversation: ${errorMessage}`);
			}

			// Add document context to conversation
			const url = `${baseUrl}/conversations/${conversationId}/context`;
			const response = await fetch(url, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					context_type: 'document',
					title: item.title,
					content: item.body,
					metadata: {
						source_type: 'document',
						document_id: item.id,
						url: item.url || '',
						tags: item.tags ? item.tags.join(', ') : '',
						rank: item.rank ? item.rank.toString() : '0',
					},
				}),
			});

			if (!response.ok) {
				throw new Error(`HTTP error! Status: ${response.status} - ${response.statusText}`);
			}

			const data = await response.json();
			console.log('✅ Document added to context via HTTP:', data);
		}

		console.log('✅ Successfully added document to chat context, navigating to chat...');

		// Show success state briefly
		chatStarted = true;

		// Show brief success notification before navigation
		const notification = window.document.createElement('div');
		notification.className = 'notification is-success is-light';
		notification.innerHTML = `
        <strong>💬 Opening Chat with Document</strong><br>
        <small>Context added successfully. Redirecting to chat...</small>
      `;
		notification.style.cssText =
			'position: fixed; top: 20px; right: 20px; z-index: 1000; max-width: 350px;';
		window.document.body.appendChild(notification);

		// Navigate to chat after a brief delay
		setTimeout(() => {
			notification.remove();
			router.goto('/chat');
		}, 1500);

		// Reset states after navigation
		setTimeout(() => {
			chatStarted = false;
		}, 2000);
	} catch (error) {
		console.error('❌ Error adding document to context and opening chat:', error);

		// Show error state
		contextError = error instanceof Error ? error.message : 'Failed to add document to context';

		// Show error notification
		const notification = window.document.createElement('div');
		notification.className = 'notification is-danger is-light';
		notification.innerHTML = `
        <button class="delete" onclick="this.parentElement.remove()"></button>
        <strong>✗ Failed to Open Chat with Document</strong><br>
        <small>${contextError}</small>
      `;
		notification.style.cssText =
			'position: fixed; top: 20px; right: 20px; z-index: 1000; max-width: 350px;';
		window.document.body.appendChild(notification);

		// Auto-remove notification after 8 seconds
		setTimeout(() => {
			notification.remove();
		}, 8000);

		// Clear error after a delay
		setTimeout(() => {
			contextError = null;
		}, 5000);
	} finally {
		chattingWithDocument = false;
	}
}

if (configStore[$role as keyof typeof configStore] !== undefined) {
	console.log('Have attribute', configStore[$role as keyof typeof configStore]);
	if (Object.hasOwn(configStore[$role as keyof typeof configStore], 'enableLogseq')) {
		console.log('enable logseq True');
	} else {
		console.log("Didn't make it");
	}
}
</script>

<div class="box">
  <article class="media">
    <div class="media-content">
      <div class="content">
        <div class="level-right">
          {#if item.tags}
          <div class="tags">
              {#each item.tags as tag}
                <button
                  class="tag-button"
                  on:click={() => _handleTagClick(tag)}
                  disabled={_loadingKg}
                  title="Click to view knowledge graph document"
                >
                  <span class="tag is-rounded">{tag}</span>
                </button>
              {/each}
          </div>
          {/if}
        </div>
          <div class="level-right">
          <div class="tags">
            <span class="tag is-rounded">Rank {item.rank || 0}</span>
          </div>
        </div>
        <div transition:fade>
          <button on:click={onTitleClick}>
            <h2 class="title">
              {item.title}
            </h2>
          </button>
          <div class="description">
            <small class="description-label">Description:</small>
            <div class="description-content">
              {#if item.description}
                <SvelteMarkdown source={item.description} />
              {:else}
                <small class="no-description">No description available</small>
              {/if}
            </div>
          </div>

          <!-- AI Summary Section -->
          <div class="ai-summary-section">
            {#if !_showAiSummary && !summaryLoading && !summaryError}
              <button
                class="button is-small is-info is-outlined ai-summary-button"
                on:click={_generateSummary}
                disabled={summaryLoading}
                title="Generate AI-powered summary using OpenRouter"
              >
                <span class="icon is-small">
                  <i class="fas fa-magic" aria-hidden="true"></i>
                </span>
                <span>AI Summary</span>
              </button>
            {/if}

            {#if summaryLoading}
              <div class="ai-summary-loading">
                <span class="icon">
                  <i class="fas fa-spinner fa-spin" aria-hidden="true"></i>
                </span>
                <small>Generating AI summary...</small>
              </div>
            {/if}

            {#if summaryError}
              <div class="ai-summary-error">
                <span class="icon has-text-danger">
                  <i class="fas fa-exclamation-triangle" aria-hidden="true"></i>
                </span>
                <small class="has-text-danger">Summary error: {summaryError}</small>
                <button
                  class="button is-small is-text"
                  on:click={() => { summaryError = null; _generateSummary(); }}
                  title="Retry generating summary"
                >
                  Retry
                </button>
              </div>
            {/if}

            {#if _showAiSummary && aiSummary}
              <div class="ai-summary" transition:fade>
                <div class="ai-summary-header">
                  <small class="ai-summary-label">
                    <span class="icon is-small">
                      <i class="fas fa-robot" aria-hidden="true"></i>
                    </span>
                    AI Summary
                    {#if summaryFromCache}
                      <span class="tag is-small is-light">cached</span>
                    {:else}
                      <span class="tag is-small is-success">fresh</span>
                    {/if}
                  </small>
                  <button
                    class="button is-small is-text"
                    on:click={() => _showAiSummary = false}
                    title="Hide AI summary"
                  >
                    <span class="icon is-small">
                      <i class="fas fa-times" aria-hidden="true"></i>
                    </span>
                  </button>
                </div>
                <div class="ai-summary-content">
                  <SvelteMarkdown source={aiSummary} />
                </div>
                <div class="ai-summary-actions">
                  <button
                    class="button is-small is-text"
                    on:click={() => { _generateSummary(); }}
                    disabled={summaryLoading}
                    title="Regenerate summary"
                  >
                    <span class="icon is-small">
                      <i class="fas fa-redo" aria-hidden="true"></i>
                    </span>
                    <span>Regenerate</span>
                  </button>
                </div>
              </div>
            {/if}
          </div>

          <br />
        </div>
      </div>
      <div class="level-right">
        <nav class="level is-mobile" transition:fade>
          <div class="level-right">
            {#each menuItems as item}
              {#if item.visible}
                {#if item.disabled}
                  <button
                    type="button"
                    class="level-item button is-ghost"
                    aria-label={item.title}
                    title={item.title}
                    disabled={true}
                  >
                    <span class="icon is-medium" class:has-text-primary={item.className}>
                      <i class={item.icon} />
                    </span>
                  </button>
                {:else if item.isLink}
                  <a
                    href={item.href}
                    target="_blank"
                    class="level-item"
                    aria-label={item.title}
                    title={item.title}
                  >
                    <span class="icon is-medium" class:has-text-primary={item.className}>
                      <i class={item.icon} />
                    </span>
                  </a>
                {:else}
                  <button
                    type="button"
                    class="level-item button is-ghost"
                    aria-label={item.title}
                    on:click={item.action}
                    title={item.title}
                    data-testid={item.testId || ''}
                  >
                    <span class="icon is-medium" class:has-text-primary={item.className?.includes('primary')} class:has-text-success={item.className?.includes('success')} class:has-text-danger={item.className?.includes('danger')}>
                      <i class={item.icon} />
                    </span>
                  </button>
                {/if}
              {/if}
            {/each}
          </div>
        </nav>
      </div>
    </div>
  </article>

  <!-- Context addition feedback -->
  {#if contextError}
    <div class="notification is-danger is-light mt-2">
      <button class="delete" on:click={() => contextError = null}></button>
      {contextError}
    </div>
  {/if}
</div>

<!-- Original document modal -->
<ArticleModal bind:active={_showModal} item={item} />

<!-- KG document modal -->
{#if kgDocument}
  <ArticleModal
    bind:active={_showKgModal}
    item={kgDocument}
    kgTerm={_kgTerm}
    kgRank={kgRank}
  />
{/if}

<!-- Atomic Save Modal -->
{#if hasAtomicServer}
  <AtomicSaveModal
    bind:active={_showAtomicSaveModal}
    document={item}
  />
{/if}

<style lang="scss">
  button {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    cursor: pointer;
    outline: inherit;
    display: block;
  }

  .tag-button {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    outline: inherit;
    display: inline-block;

    &:hover {
      opacity: 0.8;
    }

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }

  .title {
    font-size: 1.3em;
    margin-bottom: 0px;

    &:hover,
    &:focus {
      text-decoration: underline;
    }
  }

  .description {
    margin-top: 0.5rem;
  }

  .description-label {
    font-weight: 600;
    color: #666;
    margin-right: 0.5rem;
  }

  .description-content {
    display: inline;

    /* Style markdown content within description */
    :global(p) {
      display: inline;
      margin: 0;
    }

    :global(strong) {
      font-weight: 600;
    }

    :global(em) {
      font-style: italic;
    }

    :global(code) {
      background-color: #f5f5f5;
      padding: 0.1rem 0.3rem;
      border-radius: 3px;
      font-family: monospace;
      font-size: 0.9em;
    }

    :global(a) {
      color: #3273dc;
      text-decoration: none;

      &:hover {
        text-decoration: underline;
      }
    }
  }

  .no-description {
    color: #999;
    font-style: italic;
  }

  /* AI Summary Styling */
  .ai-summary-section {
    margin-top: 0.75rem;
  }

  .ai-summary-button {
    margin-top: 0.5rem;
  }

  .ai-summary-loading {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
    color: #3273dc;
  }

  .ai-summary-error {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .ai-summary {
    margin-top: 0.75rem;
    padding: 0.75rem;
    background-color: #f8f9fa;
    border-left: 4px solid #3273dc;
    border-radius: 4px;
  }

  .ai-summary-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .ai-summary-label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-weight: 600;
    color: #3273dc;
  }

  .ai-summary-content {
    margin-bottom: 0.5rem;

    /* Style markdown content within AI summary */
    :global(p) {
      margin: 0 0 0.5rem 0;
      line-height: 1.4;
    }

    :global(p:last-child) {
      margin-bottom: 0;
    }

    :global(strong) {
      font-weight: 600;
    }

    :global(em) {
      font-style: italic;
    }

    :global(code) {
      background-color: #e8e8e8;
      padding: 0.1rem 0.3rem;
      border-radius: 3px;
      font-family: monospace;
      font-size: 0.9em;
    }

    :global(a) {
      color: #3273dc;
      text-decoration: none;

      &:hover {
        text-decoration: underline;
      }
    }
  }

  /* Button spacing for result item menu */
  .level.is-mobile .level-right {
    gap: 0.5rem;
  }

  .level-item.button {
    margin-left: 0.5rem;
    margin-right: 0;
  }

  .level-item.button:first-child {
    margin-left: 0;
  }

  .ai-summary-actions {
    display: flex;
    justify-content: flex-end;
  }
</style>
