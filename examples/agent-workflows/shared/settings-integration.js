/**
 * AI Agent Workflows - Settings Integration
 * Integrates settings management with examples
 */

class WorkflowSettingsIntegration {
  constructor() {
    this.settingsManager = null;
    this.settingsUI = null;
    this.apiClient = null;
    this.isInitialized = false;
  }

  async init() {
    if (this.isInitialized) return;

    try {
      // Initialize settings manager
      if (typeof TerraphimSettingsManager !== 'undefined') {
        this.settingsManager = new TerraphimSettingsManager();
      } else {
        console.warn('Settings Manager not available');
        return false;
      }

      // Initialize settings UI
      if (typeof TerraphimSettingsUI !== 'undefined') {
        this.settingsUI = new TerraphimSettingsUI(this.settingsManager);
      } else {
        console.warn('Settings UI not available');
        return false;
      }

      // Get current settings and update global API client
      const settings = this.settingsManager.getSettings();
      this.updateGlobalApiClient(settings);

      // Listen for settings changes
      this.settingsManager.on('settingsUpdated', (newSettings) => {
        this.updateGlobalApiClient(newSettings);
      });

      // Listen for server switches
      this.settingsManager.on('serverSwitched', (serverInfo) => {
        console.log('Switched to server:', serverInfo);
        this.updateGlobalApiClient(this.settingsManager.getSettings());
      });

      this.isInitialized = true;
      return true;
    } catch (error) {
      console.error('Failed to initialize settings integration:', error);
      return false;
    }
  }

  updateGlobalApiClient(settings) {
    // Update global apiClient if it exists
    if (window.apiClient && typeof window.apiClient.updateConfiguration === 'function') {
      window.apiClient.updateConfiguration({
        baseUrl: settings.serverUrl,
        timeout: settings.apiTimeout,
        maxRetries: settings.maxRetries,
        retryDelay: settings.retryDelay,
        enableWebSocket: settings.enableWebSocket,
        autoReconnect: settings.autoReconnect
      });
      
      console.log('Updated API client configuration:', window.apiClient.getConfiguration());
    }

    // Create new API client if it doesn't exist
    if (!window.apiClient && typeof TerraphimApiClient !== 'undefined') {
      window.apiClient = new TerraphimApiClient(settings.serverUrl, {
        timeout: settings.apiTimeout,
        maxRetries: settings.maxRetries,
        retryDelay: settings.retryDelay,
        enableWebSocket: settings.enableWebSocket,
        autoReconnect: settings.autoReconnect
      });
      
      console.log('Created new API client:', window.apiClient.getConfiguration());
    }
  }

  // Get current settings for workflow execution
  getWorkflowSettings() {
    if (!this.settingsManager) return {};
    
    const settings = this.settingsManager.getSettings();
    return {
      role: settings.selectedRole,
      overallRole: settings.overallRole,
      realTime: settings.enableWebSocket,
      timeout: settings.apiTimeout
    };
  }

  // Update workflow input with current role settings
  enhanceWorkflowInput(input) {
    const settings = this.getWorkflowSettings();
    
    return {
      ...input,
      role: input.role || settings.role,
      overall_role: input.overall_role || settings.overallRole
    };
  }

  // Get settings manager instance
  getSettingsManager() {
    return this.settingsManager;
  }

  // Get settings UI instance
  getSettingsUI() {
    return this.settingsUI;
  }

  // Check if settings are available
  isAvailable() {
    return this.isInitialized && this.settingsManager && this.settingsUI;
  }

  // Show settings modal
  showSettings() {
    if (this.settingsUI) {
      this.settingsUI.open();
    }
  }

  // Test current server connection
  async testConnection() {
    if (this.settingsManager) {
      return await this.settingsManager.testConnection();
    }
    return { success: false, error: 'Settings not available' };
  }

  // Quick setup for common scenarios
  async quickSetup() {
    if (!this.isAvailable()) {
      const initialized = await this.init();
      if (!initialized) {
        return false;
      }
    }

    // Test current connection
    const connectionResult = await this.testConnection();
    
    // If connection fails, try to discover servers
    if (!connectionResult.success && this.settingsManager.discoveryService) {
      console.log('Connection failed, discovering servers...');
      
      try {
        const servers = await this.settingsManager.discoverServers();
        if (servers.length > 0) {
          // Switch to the best server
          const bestServer = this.settingsManager.getBestServer();
          if (bestServer) {
            this.settingsManager.switchToServer(bestServer.url);
            console.log('Switched to discovered server:', bestServer.url);
          }
        }
      } catch (error) {
        console.warn('Server discovery failed:', error);
      }
    }

    return true;
  }
}

// Global instance
let globalSettingsIntegration = null;

// Initialize settings integration
async function initializeSettings() {
  if (!globalSettingsIntegration) {
    globalSettingsIntegration = new WorkflowSettingsIntegration();
  }
  
  return await globalSettingsIntegration.init();
}

// Get settings integration instance
function getSettingsIntegration() {
  return globalSettingsIntegration;
}

// Export to global scope
window.WorkflowSettingsIntegration = WorkflowSettingsIntegration;
window.initializeSettings = initializeSettings;
window.getSettingsIntegration = getSettingsIntegration;