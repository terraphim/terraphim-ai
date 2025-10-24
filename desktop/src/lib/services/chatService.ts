// src/lib/services/chatService.ts
import { CONFIG } from '../../config';
import { persistentConversations, currentPersistentConversationId, contexts } from '$lib/stores';

// Define types used in this service
// Duplicating from Chat.svelte for now, can be centralized later
export type Conversation = {
	id: string;
	title: string;
	role: string;
	created_at: string;
	contexts: ContextItem[];
};

export type ContextItem = {
	id: string;
	title: string;
	content: string;
	context_type: string;
};

export async function getConversations() {
	try {
		const response = await fetch(`${CONFIG.ServerURL}/conversations`);
		if (response.ok) {
			const data = await response.json();
			const conversationList = data.conversations || [];
			persistentConversations.set(conversationList);
			if (conversationList.length > 0) {
				const firstConversationId = conversationList[0].id;
				currentPersistentConversationId.set(firstConversationId);
				// Also load contexts for the first conversation
				const contextResponse = await fetch(
					`${CONFIG.ServerURL}/conversations/${firstConversationId}`
				);
				if (contextResponse.ok) {
					const conversationData = await contextResponse.json();
					contexts.set(conversationData.conversation?.contexts || []);
				}
			}
			return conversationList; // Return the list
		} else {
			console.error('Failed to fetch conversations:', response.statusText);
		}
	} catch (error) {
		console.error('Error fetching conversations:', error);
	}
	return []; // Return empty array on failure
}

export async function createContext(conversationId: string, context: Omit<ContextItem, 'id'>) {
	try {
		const response = await fetch(`${CONFIG.ServerURL}/conversations/${conversationId}/context`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(context),
		});
		if (response.ok) {
			const data = await response.json();
			if (data.status === 'success') {
				// Refresh contexts for the active conversation
				const contextResponse = await fetch(`${CONFIG.ServerURL}/conversations/${conversationId}`);
				if (contextResponse.ok) {
					const conversationData = await contextResponse.json();
					contexts.set(conversationData.conversation?.contexts || []);
				}
			}
		} else {
			console.error('Failed to create context:', response.statusText);
		}
	} catch (error) {
		console.error('Error creating context:', error);
	}
}
