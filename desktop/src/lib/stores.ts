import { writable } from 'svelte/store';
import { CONFIG } from '../config';
// Import generated types instead of manual definitions
import type { Config, Role, RoleName } from './generated/types';

// Custom interface for thesaurus (not in generated types)
interface NormalisedThesaurus {
	id: string;
	term: string;
}

// writable key value store for thesaurus, where value is id and normalised term
const thesaurus = writable<Array<Record<string, NormalisedThesaurus>>>([]);

// Default empty configuration - updated to match generated Config type
const defaultConfig: Config = {
	id: 'Desktop' as const,
	global_shortcut: '',
	roles: {} as Record<string, Role>,
	default_role: { original: '', lowercase: '' } as RoleName,
	selected_role: { original: '', lowercase: '' } as RoleName,
};

const theme = writable<string>('spacelab');
const role = writable<string>('selected'); // Updated to be empty by default, set upon config load
const is_tauri = writable<boolean>(false);
const _atomic_configured = writable<boolean>(false);
const serverUrl = writable<string>(`${CONFIG.ServerURL}/documents/search`);
const configStore = writable<Config>(defaultConfig); // Store the whole config object
const isInitialSetupComplete = writable<boolean>(false);

// Roles should be an array of Role objects - using generated Role type
const roles = writable<Role[]>([]);

const input = writable<string>('');
const typeahead = writable<boolean>(false);

// Conversation management stores
export type ConversationSummary = {
	id: string;
	title: string;
	role: string;
	message_count: number;
	preview: string | null;
	created_at: string;
	updated_at: string;
};

export type ConversationStatistics = {
	total_conversations: number;
	total_messages: number;
	conversations_by_role: Record<string, number>;
};

export type ContextItem = {
	id: string;
	title: string;
	content: string;
	context_type: string;
	// Add other fields from your type definition if necessary
};

// Store for persistent conversations list
const persistentConversations = writable<ConversationSummary[]>([]);

// Store for current persistent conversation ID
const currentPersistentConversationId = writable<string | null>(null);

// Store for conversation statistics
const conversationStatistics = writable<ConversationStatistics>({
	total_conversations: 0,
	total_messages: 0,
	conversations_by_role: {},
});

// Store for showing/hiding session list panel
const showSessionList = writable<boolean>(false);

// Store for the contexts of the currently active conversation
const contexts = writable<ContextItem[]>([]);

export {
	configStore,
	input,
	is_tauri,
	role,
	roles,
	serverUrl,
	theme,
	typeahead,
	thesaurus,
	isInitialSetupComplete,
	persistentConversations,
	currentPersistentConversationId,
	conversationStatistics,
	showSessionList,
	contexts,
};
