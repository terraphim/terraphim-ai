/**
 * API utility module for communicating with Terraphim server
 * Handles configuration, server discovery, and API calls
 */

class TerraphimAPI {
    constructor() {
        this.serverUrl = null;
        this.config = null;
        this.currentRole = null;
    }

    /**
     * Initialize API with stored configuration
     */
    async initialize() {
        const stored = await chrome.storage.sync.get(['serverUrl', 'currentRole']);
        this.serverUrl = stored.serverUrl;
        this.currentRole = stored.currentRole;

        if (!this.serverUrl) {
            await this.discoverServer();
        }

        if (this.serverUrl) {
            await this.loadConfig();
        }
    }

    /**
     * Try to discover local Terraphim server
     */
    async discoverServer() {
        const commonPorts = [3000, 8000, 8080, 3001];

        for (const port of commonPorts) {
            const testUrl = `http://localhost:${port}`;
            try {
                const response = await fetch(`${testUrl}/health`, {
                    method: 'GET',
                    signal: AbortSignal.timeout(3000) // 3 second timeout
                });

                if (response.ok) {
                    this.serverUrl = testUrl;
                    await chrome.storage.sync.set({ serverUrl: testUrl });
                    console.log(`Discovered Terraphim server at ${testUrl}`);
                    return true;
                }
            } catch (error) {
                console.log(`Server not found on port ${port}`);
            }
        }

        console.warn('Could not discover Terraphim server. Please configure manually.');
        return false;
    }

    /**
     * Load server configuration and available roles
     */
    async loadConfig() {
        if (!this.serverUrl) {
            throw new Error('Server URL not set');
        }

        try {
            const response = await fetch(`${this.serverUrl}/config`);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const data = await response.json();

            // Extract the nested config from the server response
            if (data.status === 'success' && data.config) {
                this.config = data.config;
            } else {
                throw new Error('Invalid config response from server: ' + (data.error || 'Missing config data'));
            }

            // Set default role if none selected
            if (!this.currentRole && this.config.roles) {
                const roleNames = Object.keys(this.config.roles);
                if (roleNames.length > 0) {
                    this.currentRole = roleNames[0];
                    await chrome.storage.sync.set({ currentRole: this.currentRole });
                }
            }

            return this.config;
        } catch (error) {
            console.error('Failed to load config:', error);
            throw error;
        }
    }

    /**
     * Get current role configuration
     */
    getCurrentRole() {
        if (!this.config || !this.currentRole) {
            return null;
        }
        return this.config.roles[this.currentRole];
    }

    /**
     * Get thesaurus for current role
     */
    async getThesaurus(roleName = null) {
        const role = roleName || this.currentRole;
        if (!role) {
            throw new Error('No role specified');
        }

        try {
            const response = await fetch(`${this.serverUrl}/thesaurus/${encodeURIComponent(role)}`);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const data = await response.json();
            if (data.status === 'success') {
                return data.thesaurus || {};
            } else {
                throw new Error(data.error || 'Failed to fetch thesaurus');
            }
        } catch (error) {
            console.error('Failed to fetch thesaurus:', error);
            throw error;
        }
    }

    /**
     * Add document to the server
     */
    async addDocument(title, body, url) {
        if (!this.serverUrl) {
            throw new Error('Server URL not set');
        }

        const document = {
            id: this.generateDocumentId(url),
            title,
            body,
            url,
            description: this.extractDescription(body),
            tags: []
        };

        try {
            const response = await fetch(`${this.serverUrl}/documents`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(document)
            });

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const result = await response.json();
            if (result.status === 'success') {
                return result;
            } else {
                throw new Error(result.error || 'Failed to add document');
            }
        } catch (error) {
            console.error('Failed to add document:', error);
            throw error;
        }
    }

    /**
     * Search documents
     */
    async searchDocuments(query, role = null) {
        const searchRole = role || this.currentRole;
        if (!searchRole) {
            throw new Error('No role specified for search');
        }

        try {
            const response = await fetch(`${this.serverUrl}/documents/search`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    query,
                    role: searchRole,
                    limit: 10
                })
            });

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const result = await response.json();
            if (result.status === 'success') {
                return result.results || [];
            } else {
                throw new Error(result.error || 'Search failed');
            }
        } catch (error) {
            console.error('Search failed:', error);
            throw error;
        }
    }

    /**
     * Get available roles
     */
    getAvailableRoles() {
        if (!this.config || !this.config.roles) {
            return [];
        }
        return Object.keys(this.config.roles);
    }

    /**
     * Set current role
     */
    async setCurrentRole(roleName) {
        if (!this.config || !this.config.roles[roleName]) {
            throw new Error(`Role ${roleName} not found`);
        }

        this.currentRole = roleName;
        await chrome.storage.sync.set({ currentRole: roleName });
    }

    /**
     * Set server URL
     */
    async setServerUrl(url) {
        // Remove trailing slash
        url = url.replace(/\/$/, '');

        // Test connection
        try {
            const response = await fetch(`${url}/health`);
            if (!response.ok) {
                throw new Error('Server health check failed');
            }
        } catch (error) {
            throw new Error(`Cannot connect to server at ${url}: ${error.message}`);
        }

        this.serverUrl = url;
        await chrome.storage.sync.set({ serverUrl: url });

        // Reload configuration
        await this.loadConfig();
    }

    /**
     * Get knowledge graph domain for current role
     */
    getKnowledgeGraphDomain() {
        const role = this.getCurrentRole();
        if (!role || !role.extra) {
            return null;
        }

        // Look for KG domain in role configuration
        return role.extra.kg_domain || role.extra.logseq_graph || null;
    }

    /**
     * Generate document ID from URL
     */
    generateDocumentId(url) {
        return btoa(url).replace(/[+/=]/g, '').substring(0, 32);
    }

    /**
     * Extract description from document content
     */
    extractDescription(content) {
        const lines = content.split('\n').filter(line => line.trim());
        const meaningfulLines = lines.filter(line =>
            line.length > 20 &&
            !line.startsWith('#') &&
            !line.match(/^[\s\-\*\>]+$/)
        );

        if (meaningfulLines.length === 0) {
            return lines[0]?.substring(0, 200) || null;
        }

        return meaningfulLines[0].substring(0, 200);
    }

    /**
     * Check if API is properly configured
     */
    isConfigured() {
        return !!(this.serverUrl && this.config && this.currentRole);
    }
}

// Create singleton instance
const terraphimAPI = new TerraphimAPI();

// Auto-initialize when script loads
terraphimAPI.initialize().catch(error => {
    console.error('Failed to initialize TerraphimAPI:', error);
});