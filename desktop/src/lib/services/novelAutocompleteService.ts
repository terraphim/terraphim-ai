import { invoke } from '@tauri-apps/api/tauri';
import { get } from 'svelte/store';
import { is_tauri } from '../stores';

// Helper function to check if we're in Tauri mode
function isTauriMode(): boolean {
	// Check both the store value and the global window object for reliability
	return (
		get(is_tauri) || (typeof window !== 'undefined' && (window as any).__TAURI__ !== undefined)
	);
}
export interface NovelAutocompleteSuggestion {
	text: string;
	snippet?: string;
	score?: number;
}

export interface NovelAutocompleteRequest {
	prompt: string;
	context?: string;
	maxTokens?: number;
}

export interface NovelAutocompleteResponse {
	text: string;
	usage?: {
		promptTokens: number;
		completionTokens: number;
		totalTokens: number;
	};
}

/**
 * Novel Autocomplete Service that integrates with our MCP server
 * This service provides local autocomplete suggestions instead of OpenAI
 */
export class NovelAutocompleteService {
	private baseUrl: string;
	private autocompleteIndexBuilt: boolean = false;
	private sessionId: string;
	private currentRole: string = 'Default';
	private connectionRetries: number = 0;
	private maxRetries: number = 3;
	private retryDelay: number = 1000;
	private isConnecting: boolean = false;

	constructor() {
		// Use the Terraphim API server URL - check environment or default to port 8000
		this.baseUrl =
			typeof window !== 'undefined'
				? (window.location.protocol === 'https:' ? 'https://' : 'http://') +
					window.location.hostname +
					':8000'
				: 'http://localhost:8000';
		this.sessionId = `novel-${Date.now()}`;

		// Port detection will be done when needed during connection tests
	}

	/**
	 * Set the current role for autocomplete queries
	 */
	setRole(role: string): void {
		this.currentRole = role;
		// Reset index when role changes
		this.autocompleteIndexBuilt = false;
	}

	/**
	 * Detect the correct server port by checking common ports
	 */
	private async detectServerPort(): Promise<void> {
		if (get(is_tauri)) {
			// In Tauri mode, no need for API server detection
			return;
		}

		const commonPorts = [8000, 3000, 8080, 8001];

		for (const port of commonPorts) {
			try {
				const testUrl =
					typeof window !== 'undefined'
						? (window.location.protocol === 'https:' ? 'https://' : 'http://') +
							window.location.hostname +
							':' +
							port
						: 'http://localhost:' + port;

				const response = await fetch(`${testUrl}/health`, {
					method: 'GET',
					signal: AbortSignal.timeout(2000), // 2 second timeout
				});

				if (response.ok && response.status === 200) {
					// Server is responding to health check
					this.baseUrl = testUrl;
					console.log(`NovelAutocompleteService: Detected server at ${testUrl}`);
					return;
				}
			} catch (error) {
				// Continue trying other ports
				continue;
			}
		}

		console.warn(
			'NovelAutocompleteService: Could not detect running server, using default:',
			this.baseUrl
		);
	}

	/**
	 * Test the autocomplete endpoint to verify connectivity
	 */
	async buildAutocompleteIndex(): Promise<boolean> {
		if (this.isConnecting) {
			// Wait for existing connection attempt
			await new Promise((resolve) => setTimeout(resolve, 1000));
			return this.autocompleteIndexBuilt;
		}

		this.isConnecting = true;

		try {
			if (get(is_tauri)) {
				// In Tauri mode, test the autocomplete command directly
				console.log('Using Tauri autocomplete - testing connection');
				const testResponse = (await invoke('get_autocomplete_suggestions', {
					query: 'test',
					roleName: this.currentRole,
					limit: 1,
				})) as any;

				if (testResponse && testResponse.status === 'success') {
					this.autocompleteIndexBuilt = true;
					this.connectionRetries = 0;
					console.log('Tauri autocomplete connection verified');
					return true;
				}

				console.warn('Tauri autocomplete test failed:', testResponse);
				return false;
			} else {
				// Ensure server port is detected before testing REST API
				await this.detectServerPort();

				// Test the REST API autocomplete endpoint
				const testQuery = 'test';
				const encodedRole = encodeURIComponent(this.currentRole);
				const encodedQuery = encodeURIComponent(testQuery);

				const response = await fetch(
					`${this.baseUrl}/autocomplete/${encodedRole}/${encodedQuery}`,
					{
						method: 'GET',
						signal: AbortSignal.timeout(5000), // 5 second timeout
					}
				);

				if (response.ok) {
					const result = await response.json();
					console.log('REST API autocomplete test response:', result);

					if (result.status === 'success') {
						this.autocompleteIndexBuilt = true;
						this.connectionRetries = 0;
						console.log('REST API autocomplete connection verified');
						return true;
					}
				}

				console.warn('REST API autocomplete test failed:', response.status, response.statusText);
				return false;
			}
		} catch (error) {
			console.error('Error testing Novel autocomplete endpoint:', error);
			return false;
		} finally {
			this.isConnecting = false;
		}
	}

	/**
	 * Get autocomplete suggestions for Novel editor
	 * This mimics the OpenAI completion API that Novel expects
	 */
	async getCompletion(request: NovelAutocompleteRequest): Promise<NovelAutocompleteResponse> {
		if (!this.autocompleteIndexBuilt) {
			const built = await this.buildAutocompleteIndex();
			if (!built) {
				// Return empty completion if index building fails
				return { text: '' };
			}
		}

		try {
			// Extract the last word or phrase from the prompt for autocomplete
			const lastWord = this.extractLastWord(request.prompt);

			if (!lastWord || lastWord.length < 1) {
				return { text: '' };
			}

			// Get suggestions with snippets for better context
			const suggestions = await this.getSuggestionsWithSnippets(lastWord, 5);

			if (suggestions.length === 0) {
				return { text: '' };
			}

			// Return the best suggestion as completion text
			const bestSuggestion = suggestions[0];
			let completionText = bestSuggestion.text;

			// Remove the query prefix if the suggestion starts with it
			if (completionText.toLowerCase().startsWith(lastWord.toLowerCase())) {
				completionText = completionText.substring(lastWord.length);
			}

			// Calculate token usage (approximate)
			const promptTokens = request.prompt.length / 4; // Rough estimate
			const completionTokens = completionText.length / 4;

			return {
				text: completionText,
				usage: {
					promptTokens: Math.round(promptTokens),
					completionTokens: Math.round(completionTokens),
					totalTokens: Math.round(promptTokens + completionTokens),
				},
			};
		} catch (error) {
			console.error('Error getting Novel autocomplete completion:', error);
			return { text: '' };
		}
	}

	/**
	 * Get basic autocomplete suggestions
	 */
	async getSuggestions(query: string, limit: number = 10): Promise<NovelAutocompleteSuggestion[]> {
		// Early return for empty queries
		if (!query || query.trim().length === 0) {
			return [];
		}

		if (!this.autocompleteIndexBuilt) {
			const built = await this.buildAutocompleteIndex();
			if (!built) {
				// Return empty array instead of mock suggestions
				console.warn('Autocomplete endpoint not ready, returning empty suggestions');
				return [];
			}
		}

		try {
			if (get(is_tauri)) {
				return await this.getTauriSuggestions(query, limit);
			} else {
				return await this.getRestApiSuggestions(query, limit);
			}
		} catch (error) {
			console.error('Error getting autocomplete suggestions:', error);
			return [];
		}
	}

	/**
	 * Get suggestions from Tauri backend
	 */
	private async getTauriSuggestions(
		query: string,
		limit: number
	): Promise<NovelAutocompleteSuggestion[]> {
		const response = (await invoke('get_autocomplete_suggestions', {
			query: query.trim(),
			roleName: this.currentRole,
			limit: limit,
		})) as any;

		console.log('Tauri autocomplete response:', response);

		if (response && response.status === 'success' && response.suggestions) {
			return response.suggestions
				.map((suggestion: any) => ({
					text: suggestion.term || suggestion.text || '',
					snippet: suggestion.url || suggestion.snippet || '',
					score: suggestion.score || 1.0,
				}))
				.filter((s: NovelAutocompleteSuggestion) => s.text.length > 0);
		}

		if (response && response.error) {
			console.error('Tauri autocomplete error:', response.error);
		}

		return [];
	}

	/**
	 * Get suggestions from REST API
	 */
	private async getRestApiSuggestions(
		query: string,
		limit: number
	): Promise<NovelAutocompleteSuggestion[]> {
		// Ensure server port is detected before making REST API requests
		await this.detectServerPort();

		const encodedRole = encodeURIComponent(this.currentRole);
		const encodedQuery = encodeURIComponent(query.trim());

		const response = await fetch(`${this.baseUrl}/autocomplete/${encodedRole}/${encodedQuery}`, {
			method: 'GET',
			signal: AbortSignal.timeout(5000), // 5 second timeout
		});

		if (!response.ok) {
			console.error(`REST API autocomplete request failed:`, response.status, response.statusText);
			return [];
		}

		const result = await response.json();
		console.log(`REST API autocomplete response for "${query}":`, result);

		if (result.status === 'success' && result.suggestions) {
			// Convert API response format to NovelAutocompleteSuggestion format
			return result.suggestions
				.slice(0, limit) // Limit results to requested amount
				.map((suggestion: any) => ({
					text: suggestion.text || suggestion.term || '',
					snippet: suggestion.snippet || suggestion.url || '',
					score: suggestion.score || 1.0,
				}))
				.filter((s: NovelAutocompleteSuggestion) => s.text.length > 0);
		}

		if (result.error) {
			console.error(`REST API autocomplete error:`, result.error);
		}

		return [];
	}

	/**
	 * Get autocomplete suggestions with snippets
	 */
	async getSuggestionsWithSnippets(
		query: string,
		limit: number = 10
	): Promise<NovelAutocompleteSuggestion[]> {
		// Early return for empty queries
		if (!query || query.trim().length === 0) {
			return [];
		}

		if (!this.autocompleteIndexBuilt) {
			const built = await this.buildAutocompleteIndex();
			if (!built) {
				console.warn('Autocomplete endpoint not ready, returning empty suggestions with snippets');
				return [];
			}
		}

		try {
			if (get(is_tauri)) {
				// Tauri doesn't have separate snippets endpoint, use regular suggestions
				return await this.getTauriSuggestions(query, limit);
			} else {
				// REST API already returns snippets if available
				return await this.getRestApiSuggestions(query, limit);
			}
		} catch (error) {
			console.error('Error getting autocomplete suggestions with snippets:', error);
			return [];
		}
	}

	/**
	 * Extract the last word from the prompt for autocomplete
	 */
	private extractLastWord(prompt: string): string {
		const words = prompt.trim().split(/\s+/);
		return words[words.length - 1] || '';
	}

	/**
	 * Check if the service is ready
	 */
	isReady(): boolean {
		return this.autocompleteIndexBuilt;
	}

	/**
	 * Get service status for debugging
	 */
	getStatus(): {
		ready: boolean;
		baseUrl: string;
		sessionId: string;
		usingTauri: boolean;
		currentRole: string;
		connectionRetries: number;
		isConnecting: boolean;
	} {
		return {
			ready: this.autocompleteIndexBuilt,
			baseUrl: this.baseUrl,
			sessionId: this.sessionId,
			usingTauri: get(is_tauri),
			currentRole: this.currentRole,
			connectionRetries: this.connectionRetries,
			isConnecting: this.isConnecting,
		};
	}

	/**
	 * Force refresh the autocomplete index
	 */
	async refreshIndex(): Promise<boolean> {
		this.autocompleteIndexBuilt = false;
		this.connectionRetries = 0;
		return await this.buildAutocompleteIndex();
	}

	/**
	 * Test the connection without building index
	 */
	async testConnection(): Promise<boolean> {
		try {
			if (get(is_tauri)) {
				const response = (await invoke('get_config')) as any;
				return response && response.status === 'success';
			} else {
				// Ensure server port is detected before testing connection
				await this.detectServerPort();

				const response = await fetch(`${this.baseUrl}/health`, {
					method: 'GET',
					signal: AbortSignal.timeout(3000),
				});
				return response.ok && response.status === 200;
			}
		} catch (error) {
			console.warn('Connection test failed:', error);
			return false;
		}
	}
}

// Create and export a singleton instance
export const novelAutocompleteService = new NovelAutocompleteService();
