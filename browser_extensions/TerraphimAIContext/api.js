/**
 * API utility module for TerraphimAIContext extension
 * Handles server communication and configuration
 */

class TerraphimContextAPI {
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

            this.config = await response.json();

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
     * Search documents using the Terraphim API
     */
    async searchDocuments(query, role = null) {
        const searchRole = role || this.currentRole;
        if (!searchRole) {
            throw new Error('No role specified for search');
        }

        if (!this.serverUrl) {
            throw new Error('Server URL not configured');
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
     * Add document to the server
     */
    async addDocument(title, body, url) {
        if (!this.serverUrl) {
            throw new Error('Server URL not configured');
        }

        const document = {
            id: this.generateDocumentId(url),
            title: title || 'Untitled',
            body: body || '',
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
     * Build search URL for the current server/role
     */
    getSearchUrl(query) {
        if (!this.serverUrl) {
            return null;
        }
        return `${this.serverUrl}/search?q=${encodeURIComponent(query)}`;
    }

    /**
     * Build Logseq quick capture URL
     */
    getLogseqUrl(text, pageUrl) {
        return `logseq://x-callback-url/quickCapture?page="TODAY"&content="${encodeURIComponent(text)}"&url="${encodeURIComponent(pageUrl)}"`;
    }

    /**
     * Build Atomic Server add URL
     */
    getAtomicServerUrl(text) {
        const role = this.getCurrentRole();
        if (role && role.extra && role.extra.atomic_server_url) {
            return `${role.extra.atomic_server_url}/add?text=${encodeURIComponent(text)}`;
        }
        return `https://atomicserver.com/add?text=${encodeURIComponent(text)}`;
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
        if (!content) return null;

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
        return !!(this.serverUrl && this.config);
    }
}

// Create singleton instance
const terraphimContextAPI = new TerraphimContextAPI();

// Auto-initialize when script loads
terraphimContextAPI.initialize().catch(error => {
    console.error('Failed to initialize TerraphimContextAPI:', error);
});
