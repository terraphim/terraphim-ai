/**
 * AI Agent Workflows - Settings UI Controller
 * Manages settings modal UI and user interactions
 */

class TerraphimSettingsUI {
  constructor(settingsManager) {
    this.settingsManager = settingsManager;
    this.modalElement = null;
    this.overlayElement = null;
    this.isOpen = false;
    this.discoveryInProgress = false;
    this.connectionTestInProgress = false;

    this.init();
  }

  async init() {
    await this.loadModalHTML();
    this.bindEvents();
    this.updateUI();
    this.setupKeyboardShortcuts();

    // Listen for settings changes
    this.settingsManager.on('settingsUpdated', () => this.updateUI());
    this.settingsManager.on('connectionSuccess', (result) => this.updateConnectionStatus('connected', `Connected to ${result.url}`));
    this.settingsManager.on('connectionError', (result) => this.updateConnectionStatus('error', `Failed to connect: ${result.error}`));
    this.settingsManager.on('discoveryStarted', () => this.onDiscoveryStarted());
    this.settingsManager.on('discoveryCompleted', (data) => this.onDiscoveryCompleted(data));
    this.settingsManager.on('discoveryError', (data) => this.onDiscoveryError(data));
  }

  async loadModalHTML() {
    try {
      const response = await fetch('../shared/settings-modal.html');
      const html = await response.text();

      // Create container and insert HTML
      const container = document.createElement('div');
      container.innerHTML = html;

      // Add to page
      document.body.appendChild(container);

      // Get references to modal elements
      this.overlayElement = document.getElementById('settings-overlay');
      this.modalElement = this.overlayElement.querySelector('.settings-modal');

      return true;
    } catch (error) {
      console.error('Failed to load settings modal:', error);
      return false;
    }
  }

  bindEvents() {
    // Modal controls
    const toggleButton = document.getElementById('settings-toggle');
    const closeButton = document.getElementById('settings-close');
    const saveButton = document.getElementById('save-settings');

    toggleButton?.addEventListener('click', () => this.toggle());
    closeButton?.addEventListener('click', () => this.close());
    saveButton?.addEventListener('click', () => this.saveSettings());

    // Click outside to close
    this.overlayElement?.addEventListener('click', (e) => {
      if (e.target === this.overlayElement) {
        this.close();
      }
    });

    // Server configuration
    const testButton = document.getElementById('test-connection');
    const discoverButton = document.getElementById('discover-servers');
    const serverUrlInput = document.getElementById('server-url');

    testButton?.addEventListener('click', () => this.testConnection());
    discoverButton?.addEventListener('click', () => this.discoverServers());
    serverUrlInput?.addEventListener('change', () => this.onServerUrlChange());

    // Settings inputs
    this.bindSettingsInputs();

    // Profile management
    this.bindProfileEvents();

    // Import/Export
    this.bindImportExportEvents();

    // Reset
    document.getElementById('reset-settings')?.addEventListener('click', () => this.resetSettings());
  }

  bindSettingsInputs() {
    const inputs = [
      'server-url', 'api-timeout', 'max-retries',
      'selected-role', 'overall-role',
      'enable-websocket', 'auto-reconnect', 'enable-debug-mode'
    ];

    inputs.forEach(id => {
      const element = document.getElementById(id);
      if (element) {
        element.addEventListener('change', () => this.onSettingChange(id, this.getInputValue(element)));
      }
    });
  }

  bindProfileEvents() {
    const saveProfileButton = document.getElementById('save-profile');
    const profileNameInput = document.getElementById('profile-name');

    saveProfileButton?.addEventListener('click', () => this.saveCurrentProfile());
    profileNameInput?.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        this.saveCurrentProfile();
      }
    });
  }

  bindImportExportEvents() {
    const exportButton = document.getElementById('export-settings');
    const importButton = document.getElementById('import-settings');
    const importFile = document.getElementById('import-file');

    exportButton?.addEventListener('click', () => this.exportSettings());
    importButton?.addEventListener('click', () => importFile?.click());
    importFile?.addEventListener('change', (e) => this.importSettings(e));
  }

  setupKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
      if (e.ctrlKey && e.key === ',') {
        e.preventDefault();
        this.toggle();
      }
      if (e.key === 'Escape' && this.isOpen) {
        e.preventDefault();
        this.close();
      }
    });
  }

  // Modal control methods
  open() {
    if (!this.overlayElement) return;

    this.isOpen = true;
    this.overlayElement.classList.add('active');
    document.body.style.overflow = 'hidden';

    // Focus first input
    const firstInput = this.modalElement?.querySelector('input, select, button');
    firstInput?.focus();

    this.updateUI();
  }

  close() {
    if (!this.overlayElement) return;

    this.isOpen = false;
    this.overlayElement.classList.remove('active');
    document.body.style.overflow = '';
  }

  toggle() {
    if (this.isOpen) {
      this.close();
    } else {
      this.open();
    }
  }

  // UI update methods
  updateUI() {
    const settings = this.settingsManager.getSettings();

    // Update form fields
    this.setInputValue('server-url', settings.serverUrl);
    this.setInputValue('api-timeout', settings.apiTimeout);
    this.setInputValue('max-retries', settings.maxRetries);
    this.setInputValue('selected-role', settings.selectedRole);
    this.setInputValue('overall-role', settings.overallRole);
    this.setInputValue('enable-websocket', settings.enableWebSocket);
    this.setInputValue('auto-reconnect', settings.autoReconnect);
    this.setInputValue('enable-debug-mode', settings.enableDebugMode);

    // Update discovered servers
    this.updateDiscoveredServers(settings.discoveredServers);

    // Update profiles
    this.updateProfilesList(settings.userProfiles);

    // Update connection status
    this.updateConnectionToggle();
  }

  updateConnectionStatus(status, message) {
    const statusElement = document.getElementById('connection-status');
    const messageElement = document.getElementById('connection-message');
    const indicatorElement = statusElement?.querySelector('.connection-indicator');

    if (statusElement && messageElement && indicatorElement) {
      statusElement.className = `connection-status ${status}`;
      indicatorElement.className = `connection-indicator ${status}`;
      messageElement.textContent = message;
    }
  }

  updateConnectionToggle() {
    const toggleButton = document.getElementById('settings-toggle');
    if (!toggleButton) return;

    toggleButton.className = 'settings-toggle';

    // Test connection status asynchronously
    if (this.settingsManager.get('serverUrl')) {
      this.settingsManager.testConnection().then(result => {
        if (result.success) {
          toggleButton.classList.add('connected');
          toggleButton.title = `Connected to ${result.url} - Settings (Ctrl+,)`;
        } else {
          toggleButton.classList.add('error');
          toggleButton.title = `Connection failed - Settings (Ctrl+,)`;
        }
      }).catch(() => {
        toggleButton.classList.add('error');
        toggleButton.title = `Connection failed - Settings (Ctrl+,)`;
      });
    }
  }

  updateDiscoveredServers(servers) {
    const container = document.getElementById('discovered-servers-container');
    const serversElement = document.getElementById('discovered-servers');

    if (!container || !serversElement) return;

    if (servers && servers.length > 0) {
      container.style.display = 'block';
      serversElement.innerHTML = '';

      servers.forEach(server => {
        const serverItem = this.createServerItem(server);
        serversElement.appendChild(serverItem);
      });
    } else {
      container.style.display = 'none';
    }
  }

  createServerItem(server) {
    const item = document.createElement('div');
    item.className = 'server-item';
    if (server.url === this.settingsManager.get('serverUrl')) {
      item.classList.add('selected');
    }

    item.innerHTML = `
      <div class="server-info">
        <div class="server-url">${server.url}</div>
        <div class="server-meta">
          ${server.wsAvailable ? '🟢' : '🔴'} WebSocket
          | ${server.workflowEndpoints.length} endpoints
          | v${server.version}
        </div>
      </div>
      <div class="server-response-time">${server.responseTime}ms</div>
    `;

    item.addEventListener('click', () => {
      this.selectServer(server);
    });

    return item;
  }

  selectServer(server) {
    this.settingsManager.switchToServer(server.url);
    this.updateUI();
  }

  updateProfilesList(profiles) {
    const profilesList = document.getElementById('profiles-list');
    if (!profilesList) return;

    profilesList.innerHTML = '';

    if (profiles && profiles.length > 0) {
      profiles.forEach(profile => {
        const profileItem = this.createProfileItem(profile);
        profilesList.appendChild(profileItem);
      });
    } else {
      profilesList.innerHTML = '<div class="settings-help">No saved profiles</div>';
    }
  }

  createProfileItem(profile) {
    const item = document.createElement('div');
    item.className = 'profile-item';

    const createdDate = new Date(profile.createdAt).toLocaleDateString();

    item.innerHTML = `
      <div class="profile-info">
        <div class="profile-name">${profile.name}</div>
        <div class="profile-meta">Created ${createdDate} | ${profile.settings.serverUrl}</div>
      </div>
      <div class="profile-actions">
        <button class="profile-action" title="Load profile" data-action="load" data-profile="${profile.id}">
          <svg class="settings-icon" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M3 17a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zM6.293 6.707a1 1 0 010-1.414l3-3a1 1 0 011.414 0l3 3a1 1 0 11-1.414 1.414L11 5.414V13a1 1 0 11-2 0V5.414L7.707 6.707a1 1 0 01-1.414 0z"/>
          </svg>
        </button>
        <button class="profile-action" title="Delete profile" data-action="delete" data-profile="${profile.id}">
          <svg class="settings-icon" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M9 2a1 1 0 000 2h2a1 1 0 100-2H9z"/>
            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"/>
          </svg>
        </button>
      </div>
    `;

    // Bind profile actions
    item.addEventListener('click', (e) => {
      const action = e.target.closest('[data-action]')?.dataset.action;
      const profileId = e.target.closest('[data-action]')?.dataset.profile;

      if (action && profileId) {
        if (action === 'load') {
          this.loadProfile(profileId);
        } else if (action === 'delete') {
          this.deleteProfile(profileId);
        }
      }
    });

    return item;
  }

  // Event handlers
  onSettingChange(key, value) {
    const mappedKey = this.mapInputKeyToSetting(key);
    this.settingsManager.set(mappedKey, value);
  }

  onServerUrlChange() {
    const url = document.getElementById('server-url')?.value;
    if (url) {
      const wsUrl = this.settingsManager.getWebSocketUrl(url);
      this.settingsManager.updateSettings({ serverUrl: url, wsUrl });
    }
  }

  async testConnection() {
    if (this.connectionTestInProgress) return;

    const testButton = document.getElementById('test-connection');
    const serverUrl = document.getElementById('server-url')?.value;

    if (!serverUrl) {
      this.updateConnectionStatus('error', 'Please enter a server URL');
      return;
    }

    this.connectionTestInProgress = true;
    if (testButton) testButton.disabled = true;

    this.updateConnectionStatus('connecting', 'Testing connection...');

    try {
      const result = await this.settingsManager.testConnection(serverUrl);
      // Status will be updated via event listener
    } finally {
      this.connectionTestInProgress = false;
      if (testButton) testButton.disabled = false;
    }
  }

  async discoverServers() {
    if (this.discoveryInProgress) return;

    const discoverButton = document.getElementById('discover-servers');

    this.discoveryInProgress = true;
    if (discoverButton) discoverButton.disabled = true;

    try {
      await this.settingsManager.discoverServers((progress) => {
        this.updateDiscoveryProgress(progress);
      });
    } finally {
      this.discoveryInProgress = false;
      if (discoverButton) discoverButton.disabled = false;
    }
  }

  onDiscoveryStarted() {
    const progressElement = document.getElementById('discovery-progress');
    const statusElement = document.getElementById('discovery-status');

    if (progressElement) progressElement.style.display = 'block';
    if (statusElement) {
      statusElement.style.display = 'block';
      statusElement.textContent = 'Scanning for servers...';
    }
  }

  onDiscoveryCompleted(data) {
    const progressElement = document.getElementById('discovery-progress');
    const statusElement = document.getElementById('discovery-status');

    if (progressElement) progressElement.style.display = 'none';
    if (statusElement) {
      statusElement.textContent = `Discovery complete. Found ${data.count} server(s).`;
      setTimeout(() => {
        if (statusElement) statusElement.style.display = 'none';
      }, 3000);
    }
  }

  onDiscoveryError(data) {
    const progressElement = document.getElementById('discovery-progress');
    const statusElement = document.getElementById('discovery-status');

    if (progressElement) progressElement.style.display = 'none';
    if (statusElement) {
      statusElement.textContent = `Discovery failed: ${data.error}`;
      statusElement.style.color = '#ef4444';
      setTimeout(() => {
        if (statusElement) {
          statusElement.style.display = 'none';
          statusElement.style.color = '';
        }
      }, 5000);
    }
  }

  updateDiscoveryProgress(progress) {
    const progressBar = document.getElementById('discovery-progress-bar');
    const statusElement = document.getElementById('discovery-status');

    if (progressBar) {
      progressBar.style.width = `${progress.percentage}%`;
    }

    if (statusElement) {
      statusElement.textContent = `Scanning ${progress.currentUrl} (${progress.completed}/${progress.total})`;
    }
  }

  saveSettings() {
    this.settingsManager.saveSettings();
    this.close();
  }

  saveCurrentProfile() {
    const profileNameInput = document.getElementById('profile-name');
    const name = profileNameInput?.value?.trim();

    if (!name) {
      alert('Please enter a profile name');
      return;
    }

    try {
      this.settingsManager.saveProfile(name);
      if (profileNameInput) profileNameInput.value = '';
      this.updateUI();
    } catch (error) {
      alert(`Failed to save profile: ${error.message}`);
    }
  }

  loadProfile(profileId) {
    try {
      this.settingsManager.loadProfile(profileId);
      this.updateUI();
    } catch (error) {
      alert(`Failed to load profile: ${error.message}`);
    }
  }

  deleteProfile(profileId) {
    if (confirm('Are you sure you want to delete this profile?')) {
      try {
        this.settingsManager.deleteProfile(profileId);
        this.updateUI();
      } catch (error) {
        alert(`Failed to delete profile: ${error.message}`);
      }
    }
  }

  exportSettings() {
    const data = this.settingsManager.exportSettings();
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);

    const a = document.createElement('a');
    a.href = url;
    a.download = `terraphim-settings-${new Date().toISOString().split('T')[0]}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);

    URL.revokeObjectURL(url);
  }

  importSettings(event) {
    const file = event.target.files[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const data = JSON.parse(e.target.result);
        this.settingsManager.importSettings(data);
        this.updateUI();
        alert('Settings imported successfully');
      } catch (error) {
        alert(`Failed to import settings: ${error.message}`);
      }
    };
    reader.readAsText(file);

    // Reset file input
    event.target.value = '';
  }

  resetSettings() {
    if (confirm('Are you sure you want to reset all settings to defaults?')) {
      this.settingsManager.resetToDefaults();
      this.updateUI();
    }
  }

  // Utility methods
  mapInputKeyToSetting(inputKey) {
    const mapping = {
      'server-url': 'serverUrl',
      'api-timeout': 'apiTimeout',
      'max-retries': 'maxRetries',
      'selected-role': 'selectedRole',
      'overall-role': 'overallRole',
      'enable-websocket': 'enableWebSocket',
      'auto-reconnect': 'autoReconnect'
    };
    return mapping[inputKey] || inputKey;
  }

  getInputValue(element) {
    if (element.type === 'checkbox') {
      return element.checked;
    } else if (element.type === 'number') {
      return parseInt(element.value, 10);
    }
    return element.value;
  }

  setInputValue(id, value) {
    const element = document.getElementById(id);
    if (!element) return;

    if (element.type === 'checkbox') {
      element.checked = Boolean(value);
    } else {
      element.value = value;
    }
  }

  // Public API
  isModalOpen() {
    return this.isOpen;
  }

  getSettings() {
    return this.settingsManager.getSettings();
  }

  // Cleanup
  destroy() {
    if (this.overlayElement) {
      this.overlayElement.remove();
    }
  }
}

// Export for use in examples
window.TerraphimSettingsUI = TerraphimSettingsUI;
