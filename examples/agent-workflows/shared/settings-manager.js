/**
 * AI Agent Workflows - Settings Manager
 * Manages user settings, server configurations, and persistence
 */

class TerraphimSettingsManager {
  constructor() {
    this.storageKey = 'terraphim-agent-workflows-settings';
    this.defaultSettings = {
      serverUrl: window.location.protocol === 'file:' ? 'http://127.0.0.1:8000' : 'http://localhost:8000',
      wsUrl: window.location.protocol === 'file:' ? 'ws://127.0.0.1:8000/ws' : 'ws://localhost:8000/ws',
      apiTimeout: 30000,
      maxRetries: 3,
      retryDelay: 1000,
      selectedRole: 'Technical Writer',
      overallRole: 'Software Development Lead',
      enableWebSocket: true,
      autoReconnect: true,
      autoDiscovery: true,
      enableDebugMode: false,
      debugLogToConsole: true,
      debugShowInUI: true,
      theme: 'auto', // 'light', 'dark', 'auto'
      discoveredServers: [],
      userProfiles: [],
      lastUsed: null
    };
    
    this.currentSettings = null;
    this.eventListeners = new Map();
    this.discoveryService = null;
    
    this.init();
  }

  // Initialize settings
  init() {
    this.loadSettings();
    this.initializeDiscoveryService();
    
    // Auto-save settings periodically
    setInterval(() => {
      if (this.hasUnsavedChanges()) {
        this.saveSettings();
      }
    }, 5000);
  }

  // Initialize server discovery service
  initializeDiscoveryService() {
    if (typeof ServerDiscoveryService !== 'undefined') {
      this.discoveryService = new ServerDiscoveryService({
        ports: [3000, 8000, 8080, 8888, 9000],
        timeout: 5000
      });
    }
  }

  // Load settings from localStorage
  loadSettings() {
    try {
      const stored = localStorage.getItem(this.storageKey);
      if (stored) {
        const parsed = JSON.parse(stored);
        this.currentSettings = { ...this.defaultSettings, ...parsed };
      } else {
        this.currentSettings = { ...this.defaultSettings };
      }
    } catch (error) {
      console.warn('Failed to load settings:', error);
      this.currentSettings = { ...this.defaultSettings };
    }

    this.emit('settingsLoaded', this.currentSettings);
  }

  // Save settings to localStorage
  saveSettings() {
    try {
      this.currentSettings.lastUsed = new Date().toISOString();
      localStorage.setItem(this.storageKey, JSON.stringify(this.currentSettings));
      this.emit('settingsSaved', this.currentSettings);
      return true;
    } catch (error) {
      console.error('Failed to save settings:', error);
      this.emit('settingsError', { action: 'save', error });
      return false;
    }
  }

  // Get current settings
  getSettings() {
    return { ...this.currentSettings };
  }

  // Update settings
  updateSettings(newSettings) {
    const oldSettings = { ...this.currentSettings };
    this.currentSettings = { ...this.currentSettings, ...newSettings };
    
    // Emit specific change events
    Object.keys(newSettings).forEach(key => {
      if (oldSettings[key] !== newSettings[key]) {
        this.emit('settingChanged', { key, oldValue: oldSettings[key], newValue: newSettings[key] });
      }
    });
    
    this.emit('settingsUpdated', this.currentSettings);
    return this.currentSettings;
  }

  // Get specific setting
  get(key) {
    return this.currentSettings[key];
  }

  // Set specific setting
  set(key, value) {
    return this.updateSettings({ [key]: value });
  }

  // Test server connection
  async testConnection(serverUrl = null) {
    const testUrl = serverUrl || this.currentSettings.serverUrl;
    
    try {
      this.emit('connectionTesting', { url: testUrl });
      
      const response = await fetch(`${testUrl}/health`, {
        method: 'GET',
        headers: { 'Accept': 'application/json' },
        signal: AbortSignal.timeout(this.currentSettings.apiTimeout)
      });

      if (response.ok) {
        let serverInfo = {};
        try {
          serverInfo = await response.json();
        } catch (e) {
          // Health endpoint might not return JSON
        }

        const result = {
          success: true,
          url: testUrl,
          responseTime: Date.now(),
          serverInfo,
          testedAt: new Date().toISOString()
        };

        this.emit('connectionSuccess', result);
        return result;
      } else {
        throw new Error(`Server responded with ${response.status}: ${response.statusText}`);
      }
    } catch (error) {
      const result = {
        success: false,
        url: testUrl,
        error: error.message,
        testedAt: new Date().toISOString()
      };

      this.emit('connectionError', result);
      return result;
    }
  }

  // Auto-discover servers
  async discoverServers(onProgress = null) {
    if (!this.discoveryService) {
      throw new Error('Server discovery service not available');
    }

    try {
      this.emit('discoveryStarted');
      
      const servers = await this.discoveryService.discoverServers(onProgress);
      
      // Update settings with discovered servers
      this.updateSettings({ discoveredServers: servers });
      
      this.emit('discoveryCompleted', { servers, count: servers.length });
      
      return servers;
    } catch (error) {
      this.emit('discoveryError', { error: error.message });
      throw error;
    }
  }

  // Get best available server
  getBestServer() {
    if (!this.discoveryService) {
      return null;
    }
    return this.discoveryService.getBestServer();
  }

  // Switch to discovered server
  switchToServer(serverUrl) {
    const server = this.currentSettings.discoveredServers.find(s => s.url === serverUrl);
    
    if (server) {
      this.updateSettings({
        serverUrl: server.url,
        wsUrl: server.wsUrl
      });
      
      this.emit('serverSwitched', server);
      return server;
    }
    
    // Manual server URL
    this.updateSettings({
      serverUrl: serverUrl,
      wsUrl: this.getWebSocketUrl(serverUrl)
    });
    
    const manualServer = { url: serverUrl, wsUrl: this.getWebSocketUrl(serverUrl), manual: true };
    this.emit('serverSwitched', manualServer);
    return manualServer;
  }

  // Get WebSocket URL from HTTP URL
  getWebSocketUrl(httpUrl) {
    try {
      const url = new URL(httpUrl);
      const protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
      return `${protocol}//${url.host}/ws`;
    } catch (error) {
      console.error('Invalid URL:', httpUrl);
      return 'ws://localhost:8000/ws';
    }
  }

  // Profile management
  saveProfile(name, settings = null) {
    const profileSettings = settings || this.currentSettings;
    const profile = {
      name,
      settings: { ...profileSettings },
      createdAt: new Date().toISOString(),
      id: Date.now().toString(36) + Math.random().toString(36).substr(2)
    };

    const profiles = [...this.currentSettings.userProfiles];
    const existingIndex = profiles.findIndex(p => p.name === name);
    
    if (existingIndex >= 0) {
      profiles[existingIndex] = { ...profiles[existingIndex], ...profile, updatedAt: new Date().toISOString() };
    } else {
      profiles.push(profile);
    }

    this.updateSettings({ userProfiles: profiles });
    this.emit('profileSaved', profile);
    return profile;
  }

  // Load profile
  loadProfile(nameOrId) {
    const profile = this.currentSettings.userProfiles.find(p => 
      p.name === nameOrId || p.id === nameOrId
    );

    if (!profile) {
      throw new Error(`Profile not found: ${nameOrId}`);
    }

    const oldSettings = { ...this.currentSettings };
    this.currentSettings = { 
      ...profile.settings, 
      userProfiles: this.currentSettings.userProfiles 
    };

    this.emit('profileLoaded', { profile, oldSettings });
    return profile;
  }

  // Delete profile
  deleteProfile(nameOrId) {
    const profiles = this.currentSettings.userProfiles.filter(p => 
      p.name !== nameOrId && p.id !== nameOrId
    );

    if (profiles.length === this.currentSettings.userProfiles.length) {
      throw new Error(`Profile not found: ${nameOrId}`);
    }

    this.updateSettings({ userProfiles: profiles });
    this.emit('profileDeleted', { nameOrId });
    return true;
  }

  // Get all profiles
  getProfiles() {
    return [...this.currentSettings.userProfiles];
  }

  // Export settings
  exportSettings() {
    return {
      version: '1.0',
      exportedAt: new Date().toISOString(),
      settings: this.currentSettings,
      metadata: {
        userAgent: navigator.userAgent,
        url: window.location.href
      }
    };
  }

  // Import settings
  importSettings(data) {
    try {
      if (!data || !data.settings) {
        throw new Error('Invalid settings format');
      }

      // Validate required fields
      const requiredFields = ['serverUrl'];
      for (const field of requiredFields) {
        if (!data.settings[field]) {
          throw new Error(`Missing required field: ${field}`);
        }
      }

      // Merge with current settings
      const importedSettings = { ...this.defaultSettings, ...data.settings };
      this.currentSettings = importedSettings;
      this.saveSettings();

      this.emit('settingsImported', { data, settings: importedSettings });
      return importedSettings;
    } catch (error) {
      this.emit('settingsError', { action: 'import', error });
      throw error;
    }
  }

  // Check if debug mode is enabled
  isDebugMode() {
    return this.currentSettings.enableDebugMode === true;
  }

  // Toggle debug mode
  toggleDebugMode(enabled) {
    this.updateSettings({ enableDebugMode: enabled });
    this.emit('debugModeChanged', enabled);
    return enabled;
  }

  // Reset to defaults
  resetToDefaults() {
    const oldSettings = { ...this.currentSettings };
    this.currentSettings = { ...this.defaultSettings };
    this.saveSettings();

    this.emit('settingsReset', { oldSettings, newSettings: this.currentSettings });
    return this.currentSettings;
  }

  // Check if settings have unsaved changes
  hasUnsavedChanges() {
    try {
      const stored = localStorage.getItem(this.storageKey);
      if (!stored) return true;
      
      const storedSettings = JSON.parse(stored);
      return JSON.stringify(storedSettings) !== JSON.stringify(this.currentSettings);
    } catch (error) {
      return true;
    }
  }

  // Event system
  on(event, callback) {
    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, new Set());
    }
    this.eventListeners.get(event).add(callback);
    
    // Return unsubscribe function
    return () => {
      const listeners = this.eventListeners.get(event);
      if (listeners) {
        listeners.delete(callback);
      }
    };
  }

  off(event, callback) {
    const listeners = this.eventListeners.get(event);
    if (listeners) {
      listeners.delete(callback);
    }
  }

  emit(event, data = null) {
    const listeners = this.eventListeners.get(event);
    if (listeners) {
      listeners.forEach(callback => {
        try {
          callback(data);
        } catch (error) {
          console.error(`Error in settings event handler for ${event}:`, error);
        }
      });
    }
  }

  // Cleanup
  destroy() {
    if (this.hasUnsavedChanges()) {
      this.saveSettings();
    }
    this.eventListeners.clear();
  }
}

// Export for use in examples
window.TerraphimSettingsManager = TerraphimSettingsManager;