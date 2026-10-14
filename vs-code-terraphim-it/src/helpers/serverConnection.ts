// src/helpers/serverConnection.ts
import * as vscode from 'vscode';

export interface TerraphimServerConfig {
  serverUrl: string;
  atomicServerUrl: string;
  enableLocalServer: boolean;
  selectedRole: string;
}

export interface SearchRequest {
  search_term: string;
  skip?: number;
  limit?: number;
  role: string;
}

export interface SearchResponse {
  status: string;
  results: Array<{
    id: string;
    title: string;
    body: string;
    url: string;
    description?: string;
    tags?: string[];
    rank?: number;
  }>;
}

export interface AutocompleteResponse {
  status: string;
  suggestions: Array<{
    term: string;
    score: number;
  }>;
}

export class TerraphimServerConnection {
  private config: TerraphimServerConfig;
  private configurationRevision = 0;

  constructor() {
    this.config = this.loadConfiguration();
  }

  private loadConfiguration(): TerraphimServerConfig {
    const config = vscode.workspace.getConfiguration('terraphimIt');

    return {
      serverUrl: config.get<string>('serverUrl') || 'http://localhost:8000',
      atomicServerUrl: config.get<string>('atomicServerUrl') || 'https://common.terraphim.io/drive/h6grD0ID',
      enableLocalServer: config.get<boolean>('enableLocalServer') !== false, // default true
      selectedRole: config.get<string>('selectedRole') || 'Terraphim Engineer'
    };
  }

  public updateConfiguration(): void {
    const next = this.loadConfiguration();
    if (JSON.stringify(next) !== JSON.stringify(this.config)) {
      this.config = next;
      this.configurationRevision += 1;
    }
  }

  public getConfigurationRevision(): number {
    return this.configurationRevision;
  }

  public getServerUrl(): string {
    const url = new URL(this.config.serverUrl);
    if (!['http:', 'https:'].includes(url.protocol)) {
      throw new Error('Set terraphimIt.serverUrl to an HTTP or HTTPS Terraphim server.');
    }
    const hostname = url.hostname.toLowerCase();
    const loopback = hostname === 'localhost' || hostname.endsWith('.localhost') ||
      hostname.startsWith('127.') || ['0.0.0.0', '[::]', '[::1]'].includes(hostname) ||
      /^\[::ffff:(?:7f[0-9a-f]{2}:|0:0\])/.test(hostname);
    if (!this.config.enableLocalServer && loopback) {
      throw new Error('Local Terraphim server is disabled. Set terraphimIt.serverUrl to your remote server or enable terraphimIt.enableLocalServer.');
    }
    return url.href.replace(/\/$/, '');
  }

  public getAtomicServerUrl(): string {
    return this.config.atomicServerUrl;
  }

  public getSelectedRole(): string {
    return this.config.selectedRole;
  }

  public isLocalServerEnabled(): boolean {
    return this.config.enableLocalServer;
  }

  /**
   * Check if the Terraphim server is healthy
   */
  public async healthCheck(): Promise<{ healthy: boolean; message: string; version?: string }> {
    let timeoutId: ReturnType<typeof setTimeout> | undefined;
    try {
      const healthUrl = `${this.getServerUrl()}/health`;
      console.log(`Checking server health at: ${healthUrl}`);

      const controller = new AbortController();
      timeoutId = setTimeout(() => controller.abort(), 5000); // 5 second timeout

      const response = await fetch(healthUrl, {
        method: 'GET',
        signal: controller.signal,
        headers: {
          'Accept': 'application/json',
          'Content-Type': 'application/json'
        }
      });

      if (response.ok) {
        const healthData = await response.text();
        return {
          healthy: true,
          message: `Server is healthy: ${healthData}`,
          version: response.headers.get('X-Terraphim-Version') || undefined
        };
      } else {
        return {
          healthy: false,
          message: `Server responded with status: ${response.status} ${response.statusText}`
        };
      }
    } catch (error: any) {
      let message = 'Unknown error occurred';

      if (error.name === 'AbortError') {
        message = 'Health check timed out after 5 seconds';
      } else if (error.code === 'ECONNREFUSED') {
        message = 'Connection refused - server may not be running';
      } else if (error instanceof TypeError && error.message.includes('fetch')) {
        message = 'Network error - cannot reach server';
      } else {
        message = `Health check failed: ${error.message}`;
      }

      return {
        healthy: false,
        message: message
      };
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Search documents using the Terraphim server
   */
  public async searchDocuments(query: string, limit: number = 10): Promise<SearchResponse> {
    try {
      const searchUrl = `${this.getServerUrl()}/documents/search`;
      const searchRequest: SearchRequest = {
        search_term: query,
        skip: 0,
        limit: limit,
        role: this.config.selectedRole
      };

      console.log(`Searching documents at: ${searchUrl}`, searchRequest);

      const response = await fetch(searchUrl, {
        method: 'POST',
        signal: AbortSignal.timeout(5000),
        headers: {
          'Accept': 'application/json',
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(searchRequest)
      });

      if (!response.ok) {
        throw new Error(`Search request failed: ${response.status} ${response.statusText}`);
      }

      const searchResponse: SearchResponse = await response.json();
      return searchResponse;
    } catch (error: any) {
      console.error('Search documents error:', error);
      // Return empty results on error
      return {
        status: 'error',
        results: []
      };
    }
  }

  /**
   * Get autocomplete suggestions using the Terraphim server
   */
  public async getAutocompleteSuggestions(query: string, limit: number = 10): Promise<AutocompleteResponse> {
    try {
      const autocompleteUrl = `${this.getServerUrl()}/autocomplete/${encodeURIComponent(this.config.selectedRole)}/${encodeURIComponent(query)}`;

      console.log(`Getting autocomplete suggestions at: ${autocompleteUrl}`);

      const response = await fetch(autocompleteUrl, {
        method: 'GET',
        signal: AbortSignal.timeout(5000),
        headers: {
          'Accept': 'application/json',
          'Content-Type': 'application/json'
        }
      });

      if (!response.ok) {
        throw new Error(`Autocomplete request failed: ${response.status} ${response.statusText}`);
      }

      const autocompleteResponse: AutocompleteResponse = await response.json();
      return autocompleteResponse;
    } catch (error: any) {
      console.error('Autocomplete error:', error);
      // Return empty suggestions on error
      return {
        status: 'error',
        suggestions: []
      };
    }
  }

  /**
   * Test the connection to both servers
   */
  public async testConnection(): Promise<{
    terraphim: { healthy: boolean; message: string };
    atomic: { healthy: boolean; message: string };
  }> {
    const terraphimResult = await this.healthCheck();

    // Test Atomic server (simple ping)
    let atomicResult = { healthy: false, message: 'Test not implemented' };
    try {
      const response = await fetch(this.config.atomicServerUrl, {
        method: 'HEAD',
        signal: AbortSignal.timeout(5000)
      });
      atomicResult = {
        healthy: response.ok,
        message: response.ok ? 'Atomic server is accessible' : `Atomic server returned: ${response.status}`
      };
    } catch (error: any) {
      atomicResult = {
        healthy: false,
        message: `Cannot reach Atomic server: ${error.message}`
      };
    }

    return {
      terraphim: terraphimResult,
      atomic: atomicResult
    };
  }
}

// Export a singleton instance
export const serverConnection = new TerraphimServerConnection();
