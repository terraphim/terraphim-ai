export type ChatMessage = {
	role: 'system' | 'user' | 'assistant';
	content: string;
};

export type ContextMetadata = Record<string, string>;

export interface KgTermDefinition {
	term: string;
	normalized_term: string;
	id: number;
	definition?: string;
	synonyms: string[];
	related_terms: string[];
	usage_examples: string[];
	url?: string;
	metadata: ContextMetadata;
	relevance_score?: number;
}

export interface ContextItem {
	id: string;
	title: string;
	summary?: string;
	content: string;
	context_type: string;
	created_at: string;
	relevance_score?: number;
	metadata?: ContextMetadata;
	kg_term_definition?: KgTermDefinition;
}

export interface Conversation {
	id: string;
	title: string;
	role: string;
	messages: ChatMessage[];
	global_context: ContextItem[];
	created_at: string;
	updated_at: string;
}

export interface ChatResponse {
	status: string;
	message?: string;
	model_used?: string;
	error?: string;
}

export interface ConversationResponse {
	status: string;
	conversation?: Conversation;
	conversation_id?: string;
	error?: string;
}

export interface ConversationListResponse {
	status: string;
	conversations?: Conversation[];
	error?: string;
}

export interface ContextMutationResponse {
	status: string;
	conversation?: Conversation;
	conversation_id?: string;
	context_id?: string;
	error?: string;
}

export interface UpdateConversationResponse {
	status: string;
	error?: string;
}
