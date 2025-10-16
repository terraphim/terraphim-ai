/**
 * WebSocket Connection Status Component
 * Provides visual feedback for WebSocket connection state
 */

class ConnectionStatusComponent {
  constructor(containerId, apiClient) {
    this.containerId = containerId;
    this.apiClient = apiClient;
    this.isVisible = false;
    this.statusElement = null;
    this.reconnectAttempts = 0;

    this.init();
  }

  init() {
    this.createStatusElement();
    this.setupEventListeners();
    this.updateStatus();
  }

  createStatusElement() {
    const container = document.getElementById(this.containerId);
    if (!container) {
      console.warn(`Connection status container ${this.containerId} not found`);
      return;
    }

    this.statusElement = document.createElement('div');
    this.statusElement.id = 'websocket-status';
    this.statusElement.className = 'connection-status';
    this.statusElement.innerHTML = `
      <div class="status-indicator">
        <div class="status-dot"></div>
        <div class="status-text">Checking connection...</div>
        <div class="status-details"></div>
      </div>
      <div class="status-controls">
        <button class="status-toggle" title="Toggle connection status">
          <span class="toggle-icon">ℹ️</span>
        </button>
      </div>
    `;

    // Add CSS if not already present
    this.addStyles();

    // Insert at the beginning of the container
    container.insertBefore(this.statusElement, container.firstChild);

    // Set up toggle functionality
    const toggleBtn = this.statusElement.querySelector('.status-toggle');
    toggleBtn.addEventListener('click', () => {
      this.toggleDetails();
    });
  }

  addStyles() {
    if (document.getElementById('connection-status-styles')) {
      return; // Styles already added
    }

    const styles = document.createElement('style');
    styles.id = 'connection-status-styles';
    styles.textContent = `
      .connection-status {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.75rem 1rem;
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius-md);
        margin-bottom: 1rem;
        font-size: 0.875rem;
        transition: var(--transition);
      }

      .connection-status.expanded {
        flex-direction: column;
        align-items: stretch;
      }

      .status-indicator {
        display: flex;
        align-items: center;
        gap: 0.75rem;
      }

      .status-dot {
        width: 8px;
        height: 8px;
        border-radius: 50%;
        background: var(--text-muted);
        transition: var(--transition);
      }

      .status-dot.connected {
        background: var(--success);
        box-shadow: 0 0 8px rgba(16, 185, 129, 0.3);
      }

      .status-dot.connecting {
        background: var(--warning);
        animation: pulse 1.5s ease-in-out infinite;
      }

      .status-dot.disconnected {
        background: var(--danger);
      }

      .status-dot.error {
        background: var(--danger);
        animation: shake 0.5s ease-in-out;
      }

      @keyframes pulse {
        0%, 100% { opacity: 1; }
        50% { opacity: 0.5; }
      }

      @keyframes shake {
        0%, 100% { transform: translateX(0); }
        25% { transform: translateX(-2px); }
        75% { transform: translateX(2px); }
      }

      .status-text {
        color: var(--text);
        font-weight: 500;
      }

      .status-details {
        color: var(--text-muted);
        font-size: 0.75rem;
        margin-left: 1.5rem;
        display: none;
      }

      .connection-status.expanded .status-details {
        display: block;
        margin-left: 0;
        margin-top: 0.5rem;
        padding-top: 0.5rem;
        border-top: 1px solid var(--border);
      }

      .status-controls {
        display: flex;
        align-items: center;
        gap: 0.5rem;
      }

      .status-toggle {
        background: none;
        border: none;
        cursor: pointer;
        padding: 0.25rem;
        border-radius: var(--radius-sm);
        color: var(--text-muted);
        transition: var(--transition);
      }

      .status-toggle:hover {
        background: var(--surface-2);
        color: var(--text);
      }

      .retry-btn {
        background: var(--primary);
        color: white;
        border: none;
        padding: 0.25rem 0.75rem;
        border-radius: var(--radius-sm);
        cursor: pointer;
        font-size: 0.75rem;
        transition: var(--transition);
      }

      .retry-btn:hover {
        background: var(--primary-dark, #2563eb);
      }

      .retry-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }

      .status-sessions {
        margin-top: 0.5rem;
        padding: 0.5rem;
        background: var(--surface-2);
        border-radius: var(--radius-sm);
        font-size: 0.75rem;
      }

      .session-item {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.25rem 0;
        border-bottom: 1px solid var(--border);
      }

      .session-item:last-child {
        border-bottom: none;
      }

      .session-status {
        padding: 0.125rem 0.5rem;
        border-radius: var(--radius-full);
        font-size: 0.65rem;
        font-weight: 600;
        text-transform: uppercase;
      }

      .session-status.running {
        background: #fef3c7;
        color: #92400e;
      }

      .session-status.completed {
        background: #d1fae5;
        color: #065f46;
      }

      .session-status.error {
        background: #fee2e2;
        color: #991b1b;
      }
    `;

    document.head.appendChild(styles);
  }

  setupEventListeners() {
    if (!this.apiClient) return;

    // Subscribe to WebSocket events
    this.apiClient.subscribeToWorkflowEvents('connected', () => {
      this.updateStatus('connected');
    });

    this.apiClient.subscribeToWorkflowEvents('disconnected', (data) => {
      this.updateStatus('disconnected', data);
    });

    this.apiClient.subscribeToWorkflowEvents('error', (data) => {
      this.updateStatus('error', data);
    });

    this.apiClient.subscribeToWorkflowEvents('connection_failed', (data) => {
      this.updateStatus('connection_failed', data);
    });

    // Update status periodically
    setInterval(() => {
      this.updateStatus();
    }, 5000);
  }

  updateStatus(eventType = null, eventData = null) {
    if (!this.statusElement) return;

    const dot = this.statusElement.querySelector('.status-dot');
    const text = this.statusElement.querySelector('.status-text');
    const details = this.statusElement.querySelector('.status-details');

    if (!this.apiClient) {
      // No WebSocket support
      dot.className = 'status-dot disconnected';
      text.textContent = 'WebSocket not available';
      details.textContent = 'Real-time updates disabled';
      return;
    }

    const wsStatus = this.apiClient.getWebSocketStatus();
    const isConnected = this.apiClient.isWebSocketEnabled();

    switch (eventType) {
      case 'connected':
        dot.className = 'status-dot connected';
        text.textContent = 'Connected to terraphim server';
        this.reconnectAttempts = 0;
        break;

      case 'disconnected':
        dot.className = 'status-dot disconnected';
        text.textContent = 'Disconnected from server';
        this.showRetryButton();
        break;

      case 'error':
        dot.className = 'status-dot error';
        text.textContent = 'Connection error';
        this.showRetryButton();
        break;

      case 'connection_failed':
        dot.className = 'status-dot disconnected';
        text.textContent = 'Connection failed';
        this.reconnectAttempts = eventData?.attempts || 0;
        this.showRetryButton();
        break;

      default:
        // Default status check
        if (isConnected) {
          dot.className = 'status-dot connected';
          text.textContent = 'Real-time updates active';
        } else if (wsStatus.reconnectAttempts > 0) {
          dot.className = 'status-dot connecting';
          text.textContent = `Reconnecting... (${wsStatus.reconnectAttempts})`;
        } else {
          dot.className = 'status-dot disconnected';
          text.textContent = 'Offline mode';
          this.showRetryButton();
        }
        break;
    }

    // Update details
    this.updateDetails(wsStatus);
  }

  updateDetails(wsStatus) {
    const details = this.statusElement.querySelector('.status-details');
    if (!details) return;

    const activeSessions = this.apiClient.getActiveWorkflowSessions();
    const sessionCount = activeSessions.length;

    let detailsHTML = `
      <div><strong>Connection Status:</strong> ${wsStatus.connected ? 'Connected' : 'Disconnected'}</div>
      <div><strong>Active Sessions:</strong> ${sessionCount}</div>
    `;

    if (wsStatus.reconnectAttempts > 0) {
      detailsHTML += `<div><strong>Reconnect Attempts:</strong> ${wsStatus.reconnectAttempts}</div>`;
    }

    if (sessionCount > 0) {
      detailsHTML += `<div class="status-sessions">`;
      activeSessions.forEach(session => {
        detailsHTML += `
          <div class="session-item">
            <span>${session.workflowId}</span>
            <span class="session-status ${session.status}">${session.status}</span>
          </div>
        `;
      });
      detailsHTML += `</div>`;
    }

    details.innerHTML = detailsHTML;
  }

  showRetryButton() {
    const controls = this.statusElement.querySelector('.status-controls');
    if (controls.querySelector('.retry-btn')) return; // Already shown

    const retryBtn = document.createElement('button');
    retryBtn.className = 'retry-btn';
    retryBtn.textContent = 'Retry';
    retryBtn.addEventListener('click', () => {
      this.retryConnection();
    });

    controls.insertBefore(retryBtn, controls.firstChild);
  }

  hideRetryButton() {
    const retryBtn = this.statusElement.querySelector('.retry-btn');
    if (retryBtn) {
      retryBtn.remove();
    }
  }

  retryConnection() {
    if (!this.apiClient || !this.apiClient.wsClient) return;

    const retryBtn = this.statusElement.querySelector('.retry-btn');
    if (retryBtn) {
      retryBtn.disabled = true;
      retryBtn.textContent = 'Connecting...';
    }

    // Attempt to reconnect
    this.apiClient.wsClient.connect();

    // Reset button after delay
    setTimeout(() => {
      if (retryBtn) {
        retryBtn.disabled = false;
        retryBtn.textContent = 'Retry';
      }
    }, 3000);
  }

  toggleDetails() {
    this.isVisible = !this.isVisible;
    this.statusElement.classList.toggle('expanded', this.isVisible);

    const toggleIcon = this.statusElement.querySelector('.toggle-icon');
    toggleIcon.textContent = this.isVisible ? '▼' : 'ℹ️';
  }

  show() {
    if (this.statusElement) {
      this.statusElement.style.display = 'flex';
    }
  }

  hide() {
    if (this.statusElement) {
      this.statusElement.style.display = 'none';
    }
  }

  destroy() {
    if (this.statusElement) {
      this.statusElement.remove();
      this.statusElement = null;
    }
  }
}

// Export for use in examples
window.ConnectionStatusComponent = ConnectionStatusComponent;
