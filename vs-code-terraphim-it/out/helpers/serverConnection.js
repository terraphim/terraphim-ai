"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.serverConnection = exports.TerraphimServerConnection = void 0;
// src/helpers/serverConnection.ts
const vscode = require("vscode");
class TerraphimServerConnection {
    constructor() {
        this.config = this.loadConfiguration();
    }
    loadConfiguration() {
        const config = vscode.workspace.getConfiguration('terraphimIt');
        return {
            serverUrl: config.get('serverUrl') || 'http://localhost:8000',
            atomicServerUrl: config.get('atomicServerUrl') || 'https://common.terraphim.io/drive/h6grD0ID',
            enableLocalServer: config.get('enableLocalServer') !== false, // default true
            selectedRole: config.get('selectedRole') || 'Terraphim Engineer'
        };
    }
    updateConfiguration() {
        this.config = this.loadConfiguration();
    }
    getServerUrl() {
        return this.config.serverUrl;
    }
    getAtomicServerUrl() {
        return this.config.atomicServerUrl;
    }
    getSelectedRole() {
        return this.config.selectedRole;
    }
    isLocalServerEnabled() {
        return this.config.enableLocalServer;
    }
    /**
     * Check if the Terraphim server is healthy
     */
    async healthCheck() {
        try {
            const healthUrl = `${this.config.serverUrl}/health`;
            console.log(`Checking server health at: ${healthUrl}`);
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), 5000); // 5 second timeout
            const response = await fetch(healthUrl, {
                method: 'GET',
                signal: controller.signal,
                headers: {
                    'Accept': 'application/json',
                    'Content-Type': 'application/json'
                }
            });
            clearTimeout(timeoutId);
            if (response.ok) {
                const healthData = await response.text();
                return {
                    healthy: true,
                    message: `Server is healthy: ${healthData}`,
                    version: response.headers.get('X-Terraphim-Version') || undefined
                };
            }
            else {
                return {
                    healthy: false,
                    message: `Server responded with status: ${response.status} ${response.statusText}`
                };
            }
        }
        catch (error) {
            let message = 'Unknown error occurred';
            if (error.name === 'AbortError') {
                message = 'Health check timed out after 5 seconds';
            }
            else if (error.code === 'ECONNREFUSED') {
                message = 'Connection refused - server may not be running';
            }
            else if (error instanceof TypeError && error.message.includes('fetch')) {
                message = 'Network error - cannot reach server';
            }
            else {
                message = `Health check failed: ${error.message}`;
            }
            return {
                healthy: false,
                message: message
            };
        }
    }
    /**
     * Search documents using the Terraphim server
     */
    async searchDocuments(query, limit = 10) {
        try {
            const searchUrl = `${this.config.serverUrl}/documents/search`;
            const searchRequest = {
                search_term: query,
                skip: 0,
                limit: limit,
                role: this.config.selectedRole
            };
            console.log(`Searching documents at: ${searchUrl}`, searchRequest);
            const response = await fetch(searchUrl, {
                method: 'POST',
                headers: {
                    'Accept': 'application/json',
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(searchRequest)
            });
            if (!response.ok) {
                throw new Error(`Search request failed: ${response.status} ${response.statusText}`);
            }
            const searchResponse = await response.json();
            return searchResponse;
        }
        catch (error) {
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
    async getAutocompleteSuggestions(query, limit = 10) {
        try {
            const autocompleteUrl = `${this.config.serverUrl}/autocomplete/${encodeURIComponent(this.config.selectedRole)}/${encodeURIComponent(query)}`;
            console.log(`Getting autocomplete suggestions at: ${autocompleteUrl}`);
            const response = await fetch(autocompleteUrl, {
                method: 'GET',
                headers: {
                    'Accept': 'application/json',
                    'Content-Type': 'application/json'
                }
            });
            if (!response.ok) {
                throw new Error(`Autocomplete request failed: ${response.status} ${response.statusText}`);
            }
            const autocompleteResponse = await response.json();
            return autocompleteResponse;
        }
        catch (error) {
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
    async testConnection() {
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
        }
        catch (error) {
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
exports.TerraphimServerConnection = TerraphimServerConnection;
// Export a singleton instance
exports.serverConnection = new TerraphimServerConnection();
//# sourceMappingURL=serverConnection.js.map
