/**
 * Debug Panel - LLM Request/Response Viewer
 * Displays detailed debug information for LLM interactions
 */

class DebugPanel {
  constructor(containerId) {
    this.containerId = containerId;
    this.container = document.getElementById(containerId);
    if (!this.container) {
      console.warn(`Debug panel container '${containerId}' not found`);
      return;
    }

    this.entries = [];
    this.maxEntries = 50;
    this.isExpanded = false;

    this.render();
  }

  render() {
    if (!this.container) return;

    this.container.innerHTML = `
      <div class="debug-panel">
        <div class="debug-header" id="debug-header">
          <span class="debug-icon">🐛</span>
          <span class="debug-title">LLM Debug Log</span>
          <span class="debug-count">(${this.entries.length})</span>
          <button class="debug-toggle" id="debug-toggle" aria-label="Toggle debug panel">
            ${this.isExpanded ? '▼' : '▲'}
          </button>
          <button class="debug-clear" id="debug-clear">Clear</button>
        </div>
        <div class="debug-content ${this.isExpanded ? 'expanded' : ''}" id="debug-content">
          <div class="debug-entries" id="debug-entries">
            ${this.entries.length === 0 ?
              '<div class="debug-empty">No debug entries yet. Make an LLM request to see logs.</div>' :
              this.entries.map((entry, i) => this.renderEntry(entry, i)).join('')
            }
          </div>
        </div>
      </div>
    `;

    // Bind events
    document.getElementById('debug-toggle')?.addEventListener('click', () => this.toggle());
    document.getElementById('debug-clear')?.addEventListener('click', () => this.clear());

    // Make entry headers clickable
    document.querySelectorAll('.debug-entry-header').forEach(header => {
      header.addEventListener('click', () => {
        header.parentElement?.classList.toggle('expanded');
      });
    });
  }

  renderEntry(entry, index) {
    const isRequest = entry.type === 'request';
    const time = new Date(entry.timestamp).toLocaleTimeString();

    return `
      <div class="debug-entry ${entry.type}" data-index="${index}">
        <div class="debug-entry-header">
          <span class="debug-entry-icon">${isRequest ? '→' : '←'}</span>
          <span class="debug-entry-type">${entry.type.toUpperCase()}</span>
          <span class="debug-entry-time">${time}</span>
          ${entry.role ? `<span class="debug-entry-role">${this.escapeHtml(entry.role)}</span>` : ''}
          ${entry.duration ? `<span class="debug-entry-duration">${entry.duration}ms</span>` : ''}
          <span class="debug-entry-expand">▼</span>
        </div>
        <div class="debug-entry-body">
          ${isRequest ? this.renderRequest(entry) : this.renderResponse(entry)}
        </div>
      </div>
    `;
  }

  renderRequest(entry) {
    return `
      <div class="debug-section">
        <strong>Endpoint:</strong> <code>${this.escapeHtml(entry.endpoint || 'N/A')}</code>
      </div>
      <div class="debug-section">
        <strong>Role:</strong> <code>${this.escapeHtml(entry.role || 'default')}</code>
      </div>
      <div class="debug-section">
        <strong>Model:</strong> <code>${this.escapeHtml(entry.model || 'default')}</code>
      </div>
      ${entry.prompt ? `
        <div class="debug-section">
          <strong>Prompt:</strong>
          <pre>${this.escapeHtml(entry.prompt)}</pre>
        </div>
      ` : ''}
      ${entry.payload ? `
        <details class="debug-details">
          <summary>Full Request Payload</summary>
          <pre>${this.escapeHtml(JSON.stringify(entry.payload, null, 2))}</pre>
        </details>
      ` : ''}
    `;
  }

  renderResponse(entry) {
    const statusClass = entry.status === 'error' ? 'status-error' : 'status-success';

    return `
      <div class="debug-section">
        <strong>Status:</strong> <span class="${statusClass}">${this.escapeHtml(entry.status || 'unknown')}</span>
      </div>
      ${entry.modelUsed ? `
        <div class="debug-section">
          <strong>Model Used:</strong> <code>${this.escapeHtml(entry.modelUsed)}</code>
        </div>
      ` : ''}
      ${entry.tokens ? `
        <div class="debug-section">
          <strong>Tokens:</strong> <code>${this.escapeHtml(JSON.stringify(entry.tokens))}</code>
        </div>
      ` : ''}
      ${entry.duration !== undefined ? `
        <div class="debug-section">
          <strong>Duration:</strong> <code>${entry.duration}ms</code>
        </div>
      ` : ''}
      ${entry.output ? `
        <div class="debug-section">
          <strong>Response Output:</strong>
          <pre>${this.escapeHtml(entry.output)}</pre>
        </div>
      ` : ''}
      ${entry.error ? `
        <div class="debug-section">
          <strong>Error:</strong>
          <pre class="debug-error">${this.escapeHtml(entry.error)}</pre>
        </div>
      ` : ''}
      ${entry.fullResponse ? `
        <details class="debug-details">
          <summary>Full Response Object</summary>
          <pre>${this.escapeHtml(JSON.stringify(entry.fullResponse, null, 2))}</pre>
        </details>
      ` : ''}
    `;
  }

  addEntry(entry) {
    if (!entry || typeof entry !== 'object') {
      console.warn('Invalid debug entry:', entry);
      return;
    }

    this.entries.unshift(entry); // Add to beginning for newest-first
    if (this.entries.length > this.maxEntries) {
      this.entries = this.entries.slice(0, this.maxEntries);
    }
    this.render();
  }

  toggle() {
    this.isExpanded = !this.isExpanded;
    this.render();
  }

  clear() {
    if (confirm('Clear all debug entries?')) {
      this.entries = [];
      this.render();
    }
  }

  escapeHtml(text) {
    if (text === null || text === undefined) return '';
    const div = document.createElement('div');
    div.textContent = String(text);
    return div.innerHTML;
  }

  show() {
    if (this.container) {
      this.container.style.display = 'block';
    }
  }

  hide() {
    if (this.container) {
      this.container.style.display = 'none';
    }
  }
}
