import { CONFIG } from '../../config';
import { invoke } from '@tauri-apps/api/tauri';
import { is_tauri } from '../stores';
import { get } from 'svelte/store';

// Helper function to check if we're in Tauri mode
function isTauriMode(): boolean {
  // Check both the store value and the global window object for reliability
  return get(is_tauri) || (typeof window !== 'undefined' && (window as any).__TAURI__ !== undefined);
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
    // Use the MCP server URL - check environment or default to port 8001
    this.baseUrl = typeof window !== 'undefined'
      ? (window.location.protocol === 'https:' ? 'https://' : 'http://') + window.location.hostname + ':8001'
      : 'http://localhost:8001';
    this.sessionId = `novel-${Date.now()}`;

    // Only try to detect server port if not in Tauri mode and health checks are needed
    if (typeof window !== 'undefined' && !isTauriMode() && this.shouldPerformHealthCheck()) {
      this.detectServerPort();
    } else if (typeof window !== 'undefined' && !isTauriMode()) {
      console.log('NovelAutocompleteService: Skipping server detection - not needed for current page');
    }
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
    if (isTauriMode()) {
      // In Tauri mode, no need for MCP server detection
      return;
    }

    // Skip health checks entirely if we're not using autocomplete features
    // This prevents unnecessary connection attempts
    const shouldSkipHealthCheck = !this.shouldPerformHealthCheck();
    if (shouldSkipHealthCheck) {
      console.log('NovelAutocompleteService: Skipping health check - service not needed');
      return;
    }

    const commonPorts = [8001, 3000]; // Reduced from 4 to 2 most likely ports

    for (const port of commonPorts) {
      try {
        const testUrl = typeof window !== 'undefined'
          ? (window.location.protocol === 'https:' ? 'https://' : 'http://') + window.location.hostname + ':' + port
          : 'http://localhost:' + port;

        const response = await fetch(`${testUrl}/message?sessionId=health-check`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            jsonrpc: '2.0',
            id: 0,
            method: 'ping',
            params: {}
          }),
          signal: AbortSignal.timeout(1000) // Reduced from 2s to 1s timeout
        });

        if (response.ok || response.status === 404) {
          // Server is responding, even if endpoint doesn't exist
          this.baseUrl = testUrl;
          console.log(`NovelAutocompleteService: Detected server at ${testUrl}`);
          return;
        }
      } catch (error) {
        // Silently continue to next port to reduce console spam
        continue;
      }
    }

    console.warn('NovelAutocompleteService: Could not detect running server, using default:', this.baseUrl);
  }

  /**
   * Determine if health checks should be performed based on current context
   */
  private shouldPerformHealthCheck(): boolean {
    // Skip health checks if we're on specific pages that don't need autocomplete
    if (typeof window !== 'undefined') {
      const path = window.location.pathname;
      const skipPaths = ['/config', '/graph', '/fetch'];
      if (skipPaths.some(skipPath => path.startsWith(skipPath))) {
        return false;
      }
    }

    // Always perform health check if we might need autocomplete
    return true;
  }

  /**
   * Build the autocomplete index for the current role with retry logic
   */
  async buildAutocompleteIndex(): Promise<boolean> {
    if (this.isConnecting) {
      // Wait for existing connection attempt
      await new Promise(resolve => setTimeout(resolve, 1000));
      return this.autocompleteIndexBuilt;
    }

    this.isConnecting = true;

    try {
      if (isTauriMode()) {
        // In Tauri mode, no index building needed - the backend has the thesaurus
        console.log('Using Tauri backend - no index building required');
        this.autocompleteIndexBuilt = true;
        this.connectionRetries = 0;
        console.log('Tauri autocomplete ready');
        return true;
      } else {
        return await this.buildMCPIndex();
      }
    } catch (error) {
      console.error('Error building Novel autocomplete index:', error);
      return false;
    } finally {
      this.isConnecting = false;
    }
  }

  /**
   * Build MCP index with retry logic
   */
  private async buildMCPIndex(): Promise<boolean> {
    for (let attempt = 0; attempt <= this.maxRetries; attempt++) {
      try {
        const response = await fetch(`${this.baseUrl}/message?sessionId=${this.sessionId}`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            jsonrpc: '2.0',
            id: Date.now(),
            method: 'tools/call',
            params: {
              name: 'build_autocomplete_index',
              arguments: {}
            }
          }),
          signal: AbortSignal.timeout(10000) // 10 second timeout
        });

        if (response.ok) {
          const result = await response.json();
          console.log(`MCP build index response (attempt ${attempt + 1}):`, result);

          if (result.result && !result.result.is_error) {
            this.autocompleteIndexBuilt = true;
            this.connectionRetries = 0;
            console.log('MCP autocomplete index built successfully');
            return true;
          }
        } else if (response.status >= 500 && attempt < this.maxRetries) {
          // Server error, retry
          console.warn(`MCP server error ${response.status}, retrying in ${this.retryDelay}ms...`);
          await new Promise(resolve => setTimeout(resolve, this.retryDelay * (attempt + 1)));
          continue;
        }

        console.warn(`MCP build index failed (attempt ${attempt + 1}):`, response.status, response.statusText);

      } catch (error) {
        if (error instanceof Error && error.name === 'AbortError') {
          console.warn(`MCP request timeout (attempt ${attempt + 1})`);
        } else {
          console.warn(`MCP connection error (attempt ${attempt + 1}):`, error);
        }

        if (attempt < this.maxRetries) {
          await new Promise(resolve => setTimeout(resolve, this.retryDelay * (attempt + 1)));
        }
      }
    }

    console.error('Failed to build MCP autocomplete index after all retries');
    return false;
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
          totalTokens: Math.round(promptTokens + completionTokens)
        }
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
        console.warn('Autocomplete index not built, returning empty suggestions');
        return [];
      }
    }

    try {
      if (isTauriMode()) {
        return await this.getTauriSuggestions(query, limit);
      } else {
        return await this.getMCPSuggestions(query, limit, 'autocomplete_terms');
      }
    } catch (error) {
      console.error('Error getting autocomplete suggestions:', error);
      return [];
    }
  }

  /**
   * Get suggestions from Tauri backend
   */
  private async getTauriSuggestions(query: string, limit: number): Promise<NovelAutocompleteSuggestion[]> {
    const response = await invoke('get_autocomplete_suggestions', {
      query: query.trim(),
      role_name: this.currentRole,
      limit: limit
    }) as any;

    console.log('Tauri autocomplete response:', response);

    if (response && response.status === 'success' && response.suggestions) {
      return response.suggestions.map((suggestion: any) => ({
        text: suggestion.term || suggestion.text || '',
        snippet: suggestion.url || suggestion.snippet || '',
        score: suggestion.score || 1.0
      })).filter((s: NovelAutocompleteSuggestion) => s.text.length > 0);
    }

    if (response && response.error) {
      console.error('Tauri autocomplete error:', response.error);
    }

    return [];
  }

  /**
   * Get suggestions from MCP server
   */
  private async getMCPSuggestions(query: string, limit: number, method: string): Promise<NovelAutocompleteSuggestion[]> {
    const requestId = Date.now();

    const response = await fetch(`${this.baseUrl}/message?sessionId=${this.sessionId}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: requestId,
        method: 'tools/call',
        params: {
          name: method,
          arguments: {
            query: query.trim(),
            limit,
            role: this.currentRole
          }
        }
      }),
      signal: AbortSignal.timeout(5000) // 5 second timeout
    });

    if (!response.ok) {
      console.error(`MCP ${method} request failed:`, response.status, response.statusText);
      return [];
    }

    const result = await response.json();
    console.log(`MCP ${method} response:`, result);

    if (result.result && !result.result.is_error && result.result.content) {
      if (method === 'autocomplete_with_snippets') {
        return this.parseAutocompleteWithSnippetsContent(result.result.content);
      } else {
        return this.parseAutocompleteContent(result.result.content);
      }
    }

    if (result.error) {
      console.error(`MCP ${method} error:`, result.error);
    }

    return [];
  }

  /**
   * Get autocomplete suggestions with snippets
   */
  async getSuggestionsWithSnippets(query: string, limit: number = 10): Promise<NovelAutocompleteSuggestion[]> {
    // Early return for empty queries
    if (!query || query.trim().length === 0) {
      return [];
    }

    if (!this.autocompleteIndexBuilt) {
      const built = await this.buildAutocompleteIndex();
      if (!built) {
        console.warn('Autocomplete index not built, returning empty suggestions with snippets');
        return [];
      }
    }

    try {
      if (isTauriMode()) {
        // Tauri doesn't have separate snippets endpoint, use regular suggestions
        return await this.getTauriSuggestions(query, limit);
      } else {
        return await this.getMCPSuggestions(query, limit, 'autocomplete_with_snippets');
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
   * Parse autocomplete content from MCP response
   */
  private parseAutocompleteContent(content: any[]): NovelAutocompleteSuggestion[] {
    const suggestions: NovelAutocompleteSuggestion[] = [];

    for (const item of content) {
      if (item.type === 'text' && item.text) {
        // Skip the summary line (e.g., "Found X suggestions")
        if (!item.text.startsWith('Found') && !item.text.startsWith('•')) {
          suggestions.push({ text: item.text.trim() });
        } else if (item.text.startsWith('•')) {
          // Extract term from bullet point format
          const term = item.text.replace('•', '').trim();
          if (term) {
            suggestions.push({ text: term });
          }
        }
      }
    }

    return suggestions;
  }

  /**
   * Parse autocomplete with snippets content from MCP response
   */
  private parseAutocompleteWithSnippetsContent(content: any[]): NovelAutocompleteSuggestion[] {
    const suggestions: NovelAutocompleteSuggestion[] = [];

    for (const item of content) {
      if (item.type === 'text' && item.text) {
        // Skip the summary line
        if (!item.text.startsWith('Found') && !item.text.startsWith('•')) {
          // Check if it's in the format "term — snippet"
          const parts = item.text.split(' — ');
          if (parts.length === 2) {
            suggestions.push({
              text: parts[0].trim(),
              snippet: parts[1].trim()
            });
          } else {
            suggestions.push({ text: item.text.trim() });
          }
        } else if (item.text.startsWith('•')) {
          // Extract term from bullet point format
          const term = item.text.replace('•', '').trim();
          if (term) {
            suggestions.push({ text: term });
          }
        }
      }
    }

    return suggestions;
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
      usingTauri: isTauriMode(),
      currentRole: this.currentRole,
      connectionRetries: this.connectionRetries,
      isConnecting: this.isConnecting
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
      if (isTauriMode()) {
        // In Tauri mode, test the autocomplete command directly
        const response = await invoke('get_autocomplete_suggestions', {
          query: 'test',
          role_name: this.currentRole,
          limit: 1
        }) as any;

        // Check if we got a valid response structure (success or error)
        return response && (response.status === 'success' || response.status === 'error');
      } else {
        const response = await fetch(`${this.baseUrl}/message?sessionId=${this.sessionId}`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            jsonrpc: '2.0',
            id: Date.now(),
            method: 'tools/list',
            params: {}
          }),
          signal: AbortSignal.timeout(3000)
        });
        return response.ok;
      }
    } catch (error) {
      console.warn('Connection test failed:', error);
      return false;
    }
  }
}

// Create and export a singleton instance
export const novelAutocompleteService = new NovelAutocompleteService();
