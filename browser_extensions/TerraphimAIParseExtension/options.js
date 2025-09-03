/**
 * Options page JavaScript for TerraphimAIParseExtension
 * Handles configuration UI and server communication
 */

class OptionsPage {
    constructor() {
        this.api = window.terraphimAPI || new TerraphimAPI();
        this.elements = {};
        this.init();
    }

    async init() {
        this.bindElements();
        this.bindEvents();
        await this.loadCurrentSettings();
    }

    bindElements() {
        this.elements = {
            serverUrl: document.getElementById('serverUrl'),
            testConnection: document.getElementById('testConnection'),
            autoDiscover: document.getElementById('autoDiscover'),
            connectionStatus: document.getElementById('connectionStatus'),
            serverInfo: document.getElementById('serverInfo'),
            serverStatus: document.getElementById('serverStatus'),
            roleCount: document.getElementById('roleCount'),
            currentRole: document.getElementById('currentRole'),
            rolesList: document.getElementById('rolesList'),
            saveSettings: document.getElementById('saveSettings'),
            resetSettings: document.getElementById('resetSettings'),
            saveStatus: document.getElementById('saveStatus'),
            wikiLinkModes: document.querySelectorAll('input[name="wikiLinkMode"]')
        };
    }

    bindEvents() {
        this.elements.testConnection.addEventListener('click', () => this.testConnection());
        this.elements.autoDiscover.addEventListener('click', () => this.autoDiscover());
        this.elements.saveSettings.addEventListener('click', () => this.saveSettings());
        this.elements.resetSettings.addEventListener('click', () => this.resetSettings());
        this.elements.currentRole.addEventListener('change', () => this.onRoleChanged());

        // Auto-test connection when URL changes
        this.elements.serverUrl.addEventListener('blur', () => {
            if (this.elements.serverUrl.value) {
                this.testConnection();
            }
        });
    }

    async loadCurrentSettings() {
        try {
            // Load from Chrome storage
            const stored = await chrome.storage.sync.get([
                'serverUrl',
                'currentRole',
                'wikiLinkMode'
            ]);

            // Set form values
            if (stored.serverUrl) {
                this.elements.serverUrl.value = stored.serverUrl;
            }

            // Set wiki link mode (default to 0)
            const wikiMode = stored.wikiLinkMode || '0';
            const modeRadio = document.querySelector(`input[name="wikiLinkMode"][value="${wikiMode}"]`);
            if (modeRadio) {
                modeRadio.checked = true;
            }

            // Initialize API and load roles
            await this.api.initialize();

            if (this.api.isConfigured()) {
                await this.updateServerInfo();
                await this.loadRoles();

                if (stored.currentRole) {
                    this.elements.currentRole.value = stored.currentRole;
                }
            }
        } catch (error) {
            console.error('Failed to load settings:', error);
            this.showStatus('error', 'Failed to load current settings: ' + error.message);
        }
    }

    async testConnection() {
        const url = this.elements.serverUrl.value.trim();
        if (!url) {
            this.showConnectionStatus('error', 'Please enter a server URL');
            return;
        }

        this.showConnectionStatus('info', 'Testing connection...');
        this.elements.testConnection.disabled = true;

        try {
            // Test connection
            await this.api.setServerUrl(url);

            this.showConnectionStatus('success', 'Connection successful!');
            await this.updateServerInfo();
            await this.loadRoles();
        } catch (error) {
            console.error('Connection test failed:', error);
            this.showConnectionStatus('error', 'Connection failed: ' + error.message);
            this.elements.serverInfo.style.display = 'none';
        } finally {
            this.elements.testConnection.disabled = false;
        }
    }

    async autoDiscover() {
        this.showConnectionStatus('info', 'Discovering server...');
        this.elements.autoDiscover.disabled = true;

        try {
            const discovered = await this.api.discoverServer();

            if (discovered) {
                this.elements.serverUrl.value = this.api.serverUrl;
                this.showConnectionStatus('success', `Server discovered at ${this.api.serverUrl}`);
                await this.updateServerInfo();
                await this.loadRoles();
            } else {
                this.showConnectionStatus('error', 'No server found on common ports (3000, 8000, 8080, 3001)');
            }
        } catch (error) {
            console.error('Auto-discovery failed:', error);
            this.showConnectionStatus('error', 'Auto-discovery failed: ' + error.message);
        } finally {
            this.elements.autoDiscover.disabled = false;
        }
    }

    async updateServerInfo() {
        if (!this.api.isConfigured()) {
            this.elements.serverInfo.style.display = 'none';
            return;
        }

        try {
            const config = this.api.config;
            const roleCount = config && config.roles ? Object.keys(config.roles).length : 0;

            this.elements.serverStatus.textContent = 'Connected';
            this.elements.roleCount.textContent = roleCount;
            this.elements.serverInfo.style.display = 'block';
        } catch (error) {
            console.error('Failed to update server info:', error);
            this.elements.serverInfo.style.display = 'none';
        }
    }

    async loadRoles() {
        if (!this.api.isConfigured()) {
            this.elements.currentRole.innerHTML = '<option value="">Connect to server first</option>';
            this.elements.rolesList.innerHTML = '<p>Connect to server to see available roles...</p>';
            return;
        }

        try {
            const roles = this.api.getAvailableRoles();

            // Update role selector
            this.elements.currentRole.innerHTML = '<option value="">Select a role...</option>';
            roles.forEach(roleName => {
                const option = document.createElement('option');
                option.value = roleName;
                option.textContent = roleName;
                this.elements.currentRole.appendChild(option);
            });

            // Update roles list
            if (roles.length > 0) {
                const rolesHtml = roles.map(roleName => {
                    const role = this.api.config.roles[roleName];
                    const relevanceFunc = role.relevance_function || 'Unknown';
                    const haystackCount = role.haystacks ? role.haystacks.length : 0;

                    return `
                        <div class="role-item">
                            <strong>${roleName}</strong><br>
                            <small>Relevance: ${relevanceFunc} | Haystacks: ${haystackCount}</small>
                        </div>
                    `;
                }).join('');

                this.elements.rolesList.innerHTML = rolesHtml;
            } else {
                this.elements.rolesList.innerHTML = '<p>No roles available</p>';
            }

        } catch (error) {
            console.error('Failed to load roles:', error);
            this.elements.currentRole.innerHTML = '<option value="">Error loading roles</option>';
            this.elements.rolesList.innerHTML = '<p>Error loading roles: ' + error.message + '</p>';
        }
    }

    async onRoleChanged() {
        const selectedRole = this.elements.currentRole.value;
        if (selectedRole && this.api.isConfigured()) {
            try {
                await this.api.setCurrentRole(selectedRole);
                this.showStatus('success', `Role changed to "${selectedRole}"`);
            } catch (error) {
                console.error('Failed to change role:', error);
                this.showStatus('error', 'Failed to change role: ' + error.message);
            }
        }
    }

    async saveSettings() {
        this.elements.saveSettings.disabled = true;

        try {
            const settings = {
                serverUrl: this.elements.serverUrl.value.trim(),
                currentRole: this.elements.currentRole.value,
                wikiLinkMode: document.querySelector('input[name="wikiLinkMode"]:checked')?.value || '0'
            };

            // Save to Chrome storage
            await chrome.storage.sync.set(settings);

            // Update API if server URL changed
            if (settings.serverUrl && settings.serverUrl !== this.api.serverUrl) {
                await this.api.setServerUrl(settings.serverUrl);
            }

            // Update role if changed
            if (settings.currentRole && settings.currentRole !== this.api.currentRole) {
                await this.api.setCurrentRole(settings.currentRole);
            }

            this.showStatus('success', 'Settings saved successfully!');

            // Refresh server info
            await this.updateServerInfo();

        } catch (error) {
            console.error('Failed to save settings:', error);
            this.showStatus('error', 'Failed to save settings: ' + error.message);
        } finally {
            this.elements.saveSettings.disabled = false;
        }
    }

    async resetSettings() {
        if (confirm('Are you sure you want to reset all settings to defaults?')) {
            try {
                // Clear Chrome storage
                await chrome.storage.sync.clear();

                // Reset form
                this.elements.serverUrl.value = '';
                this.elements.currentRole.value = '';
                document.querySelector('input[name="wikiLinkMode"][value="0"]').checked = true;

                // Reset API
                this.api.serverUrl = null;
                this.api.currentRole = null;
                this.api.config = null;

                // Update UI
                this.elements.serverInfo.style.display = 'none';
                this.elements.currentRole.innerHTML = '<option value="">Connect to server first</option>';
                this.elements.rolesList.innerHTML = '<p>Connect to server to see available roles...</p>';

                this.showStatus('success', 'Settings reset to defaults');

            } catch (error) {
                console.error('Failed to reset settings:', error);
                this.showStatus('error', 'Failed to reset settings: ' + error.message);
            }
        }
    }

    showConnectionStatus(type, message) {
        this.elements.connectionStatus.className = `status ${type}`;
        this.elements.connectionStatus.textContent = message;
        this.elements.connectionStatus.style.display = 'block';

        if (type === 'success' || type === 'error') {
            setTimeout(() => {
                this.elements.connectionStatus.style.display = 'none';
            }, 5000);
        }
    }

    showStatus(type, message) {
        this.elements.saveStatus.className = `status ${type}`;
        this.elements.saveStatus.textContent = message;
        this.elements.saveStatus.style.display = 'block';

        setTimeout(() => {
            this.elements.saveStatus.style.display = 'none';
        }, 3000);
    }
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    new OptionsPage();
});