import { CONFIG } from '../../config';

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

  constructor() {
    // Use the MCP server URL - update config to point to port 8001
    this.baseUrl = 'http://localhost:8001';
    this.sessionId = `novel-${Date.now()}`;
  }

  /**
   * Build the autocomplete index for the current role
   */
  async buildAutocompleteIndex(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/message?sessionId=${this.sessionId}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'tools/call',
          params: {
            name: 'build_autocomplete_index',
            arguments: {}
          }
        })
      });

      if (response.ok) {
        const result = await response.json();
        console.log('Build index response:', result);
        if (result.result && !result.result.is_error) {
          this.autocompleteIndexBuilt = true;
          console.log('Novel autocomplete index built successfully');
          return true;
        }
      }
      
      console.warn('Failed to build Novel autocomplete index');
      return false;
    } catch (error) {
      console.error('Error building Novel autocomplete index:', error);
      return false;
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
      
      if (!lastWord || lastWord.length < 2) {
        return { text: '' };
      }

      // Get suggestions with snippets for better context
      const suggestions = await this.getSuggestionsWithSnippets(lastWord, 5);
      
      if (suggestions.length === 0) {
        return { text: '' };
      }

      // Return the best suggestion as completion text
      const bestSuggestion = suggestions[0];
      const completionText = bestSuggestion.text;
      
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
    if (!this.autocompleteIndexBuilt) {
      const built = await this.buildAutocompleteIndex();
      if (!built) {
        return [];
      }
    }

    try {
      const response = await fetch(`${this.baseUrl}/message?sessionId=${this.sessionId}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'tools/call',
          params: {
            name: 'autocomplete_terms',
            arguments: {
              query,
              limit
            }
          }
        })
      });

      if (response.ok) {
        const result = await response.json();
        console.log('Get suggestions response:', result);
        if (result.result && !result.result.is_error && result.result.content) {
          return this.parseAutocompleteContent(result.result.content);
        }
      }
      
      return [];
    } catch (error) {
      console.error('Error getting autocomplete suggestions:', error);
      return [];
    }
  }

  /**
   * Get autocomplete suggestions with snippets
   */
  async getSuggestionsWithSnippets(query: string, limit: number = 10): Promise<NovelAutocompleteSuggestion[]> {
    if (!this.autocompleteIndexBuilt) {
      const built = await this.buildAutocompleteIndex();
      if (!built) {
        return [];
      }
    }

    try {
      const response = await fetch(`${this.baseUrl}/message?sessionId=${this.sessionId}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'tools/call',
          params: {
            name: 'autocomplete_with_snippets',
            arguments: {
              query,
              limit
            }
          }
        })
      });

      if (response.ok) {
        const result = await response.json();
        console.log('Get suggestions with snippets response:', result);
        if (result.result && !result.result.is_error && result.result.content) {
          return this.parseAutocompleteWithSnippetsContent(result.result.content);
        }
      }
      
      return [];
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
  getStatus(): { ready: boolean; baseUrl: string; sessionId: string } {
    return {
      ready: this.autocompleteIndexBuilt,
      baseUrl: this.baseUrl,
      sessionId: this.sessionId
    };
  }
}

// Create and export a singleton instance
export const novelAutocompleteService = new NovelAutocompleteService();
