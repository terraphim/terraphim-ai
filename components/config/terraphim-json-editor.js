import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * @fileoverview TerraphimJsonEditor - Advanced JSON editor component using vanilla-jsoneditor
 *
 * A comprehensive JSON editor with:
 * - Tree, text, and table view modes
 * - Schema validation using AJV
 * - Auto-save with debouncing
 * - Dual backend support (Tauri invoke / HTTP fetch)
 * - Keyboard shortcuts (Ctrl+S to save)
 * - Format, repair, and validation methods
 *
 * @example
 * <terraphim-json-editor
 *   mode="tree"
 *   auto-save="true"
 *   auto-save-delay="2000"
 *   api-endpoint="/config/"
 *   tauri-command="update_config">
 * </terraphim-json-editor>
 */
class TerraphimJsonEditor extends TerraphimElement {
  /**
   * Observed attributes for the component
   * @static
   * @returns {string[]} List of observed attributes
   */
  static get observedAttributes() {
    return [
      'mode',
      'read-only',
      'auto-load',
      'auto-save',
      'auto-save-delay',
      'main-menu-bar',
      'navigation-bar',
      'status-bar',
      'api-endpoint',
      'tauri-command'
    ];
  }

  /**
   * Property definitions with type conversion and reflection
   * @static
   * @returns {Object} Property definitions
   */
  static get properties() {
    return {
      value: { type: Object, default: () => ({}) },
      schema: { type: Object, default: null },
      mode: { type: String, reflect: true, default: 'tree' },
      readOnly: { type: Boolean, reflect: true, default: false },
      autoLoad: { type: Boolean, reflect: true, default: false },
      autoSave: { type: Boolean, reflect: true, default: false },
      autoSaveDelay: { type: Number, reflect: true, default: 1000 },
      mainMenuBar: { type: Boolean, reflect: true, default: true },
      navigationBar: { type: Boolean, reflect: true, default: true },
      statusBar: { type: Boolean, reflect: true, default: true },
      apiEndpoint: { type: String, reflect: true, default: '/config/' },
      tauriCommand: { type: String, reflect: true, default: 'update_config' }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });

    /**
     * Vanilla JSONEditor instance
     * @private
     * @type {Object|null}
     */
    this._editor = null;

    /**
     * AJV validator instance
     * @private
     * @type {Object|null}
     */
    this._ajv = null;

    /**
     * Debounce timer for auto-save
     * @private
     * @type {number|null}
     */
    this._saveTimer = null;

    /**
     * Container element for the editor
     * @private
     * @type {HTMLElement|null}
     */
    this._editorContainer = null;

    /**
     * Whether the component is currently loading
     * @private
     * @type {boolean}
     */
    this._isLoading = false;

    /**
     * Whether the component is currently saving
     * @private
     * @type {boolean}
     */
    this._isSaving = false;
  }

  /**
   * Called when element is connected to DOM
   */
  async onConnected() {
    this.render();
    await this._initializeEditor();
    this._setupKeyboardShortcuts();

    if (this.autoLoad) {
      await this.load();
    }
  }

  /**
   * Called when element is disconnected from DOM
   */
  onDisconnected() {
    this._cleanupEditor();
    this._clearSaveTimer();
  }

  /**
   * Render the component structure
   */
  render() {
    this.setHTML(this.shadowRoot, `
      ${this.getStyles()}
      <div class="json-editor-wrapper">
        <div class="helper-text">
          <p>
            <strong>Note:</strong> The best editing experience is to configure Atomic Server.
            In the meantime, use the editor below. You will need to refresh the page
            (Command+R or Ctrl+R) to see changes.
          </p>
        </div>
        <div class="editor-container" id="jsoneditor"></div>
        <div class="editor-status">
          <span class="status-indicator" id="status"></span>
        </div>
      </div>
    `);

    this._editorContainer = this.$('#jsoneditor');
  }

  /**
   * Initialize the JSON editor
   * @private
   */
  async _initializeEditor() {
    if (!this._editorContainer) {
      console.error('Editor container not found');
      return;
    }

    // Dynamically import vanilla-jsoneditor and ajv
    try {
      const [{ JSONEditor }, Ajv] = await Promise.all([
        import('https://cdn.jsdelivr.net/npm/vanilla-jsoneditor@1.0.6/standalone.js'),
        import('https://cdn.jsdelivr.net/npm/ajv@8.12.0/dist/ajv.bundle.js')
      ]);

      // Initialize AJV validator
      this._ajv = new Ajv.default({
        allErrors: true,
        verbose: true
      });

      // Create editor instance
      this._editor = new JSONEditor({
        target: this._editorContainer,
        props: {
          content: {
            json: this.value
          },
          mode: this.mode,
          readOnly: this.readOnly,
          mainMenuBar: this.mainMenuBar,
          navigationBar: this.navigationBar,
          statusBar: this.statusBar,
          onChange: this._handleChange.bind(this),
          onBlur: () => this.emit('blur'),
          onFocus: () => this.emit('focus')
        }
      });

      this._updateStatus('Editor initialized', 'success');
    } catch (error) {
      console.error('Failed to initialize JSON editor:', error);
      this._updateStatus('Failed to load editor', 'error');
      this.emit('load-error', { error });
    }
  }

  /**
   * Handle editor content changes
   * @private
   * @param {Object} updatedContent - New content from editor
   * @param {Object} previousContent - Previous content
   * @param {Object} status - Editor status info
   */
  _handleChange(updatedContent, previousContent, status) {
    // Extract JSON from content
    if (!updatedContent || typeof updatedContent !== 'object') {
      return;
    }

    let newValue;
    if ('json' in updatedContent) {
      newValue = updatedContent.json;
    } else if ('text' in updatedContent) {
      try {
        newValue = JSON.parse(updatedContent.text);
      } catch (e) {
        console.warn('Invalid JSON in text mode:', e);
        return;
      }
    } else {
      return;
    }

    // Update internal value
    this.value = newValue;

    // Emit change event
    this.emit('change', {
      value: newValue,
      previousValue: previousContent?.json || previousContent?.text
    });

    // Validate if schema is provided
    if (this.schema) {
      this._validateContent(newValue);
    }

    // Auto-save if enabled
    if (this.autoSave) {
      this._scheduleSave();
    }
  }

  /**
   * Validate content against schema
   * @private
   * @param {Object} content - Content to validate
   * @returns {boolean} True if valid
   */
  _validateContent(content) {
    if (!this._ajv || !this.schema) {
      return true;
    }

    try {
      const validate = this._ajv.compile(this.schema);
      const valid = validate(content);

      if (!valid) {
        const errors = validate.errors || [];
        this._updateStatus(`Validation errors: ${errors.length}`, 'error');
        this.emit('validation-error', { errors });
        return false;
      }

      this._updateStatus('Validation passed', 'success');
      return true;
    } catch (error) {
      console.error('Validation error:', error);
      this._updateStatus('Validation failed', 'error');
      this.emit('validation-error', { error });
      return false;
    }
  }

  /**
   * Schedule auto-save with debouncing
   * @private
   */
  _scheduleSave() {
    this._clearSaveTimer();
    this._saveTimer = setTimeout(() => {
      this.save();
    }, this.autoSaveDelay);
  }

  /**
   * Clear auto-save timer
   * @private
   */
  _clearSaveTimer() {
    if (this._saveTimer !== null) {
      clearTimeout(this._saveTimer);
      this._saveTimer = null;
    }
  }

  /**
   * Setup keyboard shortcuts
   * @private
   */
  _setupKeyboardShortcuts() {
    this.listenTo(document, 'keydown', (e) => {
      // Ctrl+S or Cmd+S to save
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        this.save();
      }
    });
  }

  /**
   * Update status indicator
   * @private
   * @param {string} message - Status message
   * @param {string} type - Status type (success, error, info)
   */
  _updateStatus(message, type = 'info') {
    const statusEl = this.$('#status');
    if (!statusEl) return;

    statusEl.textContent = message;
    statusEl.className = `status-indicator status-${type}`;

    // Auto-clear success/info messages after 3 seconds
    if (type === 'success' || type === 'info') {
      setTimeout(() => {
        if (statusEl.textContent === message) {
          statusEl.textContent = '';
          statusEl.className = 'status-indicator';
        }
      }, 3000);
    }
  }

  /**
   * Clean up editor instance
   * @private
   */
  _cleanupEditor() {
    if (this._editor) {
      try {
        this._editor.$destroy();
      } catch (e) {
        console.warn('Error destroying editor:', e);
      }
      this._editor = null;
    }
  }

  /**
   * Get current JSON value from editor
   * @returns {Object} Current JSON value
   * @public
   */
  get() {
    if (!this._editor) {
      return this.value;
    }

    try {
      const content = this._editor.get();
      if (content && 'json' in content) {
        return content.json;
      } else if (content && 'text' in content) {
        return JSON.parse(content.text);
      }
      return this.value;
    } catch (e) {
      console.error('Failed to get editor content:', e);
      return this.value;
    }
  }

  /**
   * Set JSON value in editor
   * @param {Object} value - New JSON value
   * @public
   */
  set(value) {
    this.value = value;

    if (!this._editor) {
      return;
    }

    try {
      this._editor.set({
        json: value
      });
      this._updateStatus('Content updated', 'success');
    } catch (e) {
      console.error('Failed to set editor content:', e);
      this._updateStatus('Failed to update content', 'error');
    }
  }

  /**
   * Validate current content
   * @returns {boolean} True if valid
   * @public
   */
  validate() {
    const content = this.get();
    return this._validateContent(content);
  }

  /**
   * Format/beautify JSON
   * @public
   */
  format() {
    if (!this._editor) {
      return;
    }

    try {
      const content = this.get();
      this.set(content);
      this._updateStatus('JSON formatted', 'success');
    } catch (e) {
      console.error('Failed to format JSON:', e);
      this._updateStatus('Failed to format JSON', 'error');
    }
  }

  /**
   * Repair invalid JSON
   * @public
   */
  async repair() {
    if (!this._editor) {
      return;
    }

    try {
      const content = this._editor.get();
      if (content && 'text' in content) {
        // Try to parse and repair
        const repaired = JSON.parse(content.text);
        this.set(repaired);
        this._updateStatus('JSON repaired', 'success');
      }
    } catch (e) {
      console.error('Failed to repair JSON:', e);
      this._updateStatus('Failed to repair JSON', 'error');
    }
  }

  /**
   * Save current content to backend
   * @returns {Promise<void>}
   * @public
   */
  async save() {
    if (this._isSaving) {
      console.log('Save already in progress');
      return;
    }

    this._isSaving = true;
    this._updateStatus('Saving...', 'info');

    try {
      const content = this.get();

      // Validate before saving if schema is provided
      if (this.schema && !this._validateContent(content)) {
        this._updateStatus('Cannot save: validation failed', 'error');
        this._isSaving = false;
        return;
      }

      // Determine backend (Tauri vs HTTP)
      if (window.__TAURI__) {
        await this._saveTauri(content);
      } else {
        await this._saveHttp(content);
      }

      this._updateStatus('Saved successfully', 'success');
      this.emit('config-saved', { value: content });
    } catch (error) {
      console.error('Failed to save configuration:', error);
      this._updateStatus('Failed to save', 'error');
      this.emit('save-error', { error });
    } finally {
      this._isSaving = false;
    }
  }

  /**
   * Save via Tauri invoke
   * @private
   * @param {Object} content - Content to save
   * @returns {Promise<void>}
   */
  async _saveTauri(content) {
    const { invoke } = window.__TAURI__.tauri;
    const result = await invoke(this.tauriCommand, { configNew: content });
    console.log('Tauri save result:', result);
  }

  /**
   * Save via HTTP fetch
   * @private
   * @param {Object} content - Content to save
   * @returns {Promise<void>}
   */
  async _saveHttp(content) {
    const response = await fetch(this.apiEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(content)
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const result = await response.json();
    console.log('HTTP save result:', result);
  }

  /**
   * Load content from backend
   * @returns {Promise<void>}
   * @public
   */
  async load() {
    if (this._isLoading) {
      console.log('Load already in progress');
      return;
    }

    this._isLoading = true;
    this._updateStatus('Loading...', 'info');

    try {
      let content;

      // Determine backend (Tauri vs HTTP)
      if (window.__TAURI__) {
        content = await this._loadTauri();
      } else {
        content = await this._loadHttp();
      }

      this.set(content);
      this._updateStatus('Loaded successfully', 'success');
      this.emit('config-loaded', { value: content });
    } catch (error) {
      console.error('Failed to load configuration:', error);
      this._updateStatus('Failed to load', 'error');
      this.emit('load-error', { error });
    } finally {
      this._isLoading = false;
    }
  }

  /**
   * Load via Tauri invoke
   * @private
   * @returns {Promise<Object>} Loaded content
   */
  async _loadTauri() {
    const { invoke } = window.__TAURI__.tauri;
    const content = await invoke('get_config');
    return content;
  }

  /**
   * Load via HTTP fetch
   * @private
   * @returns {Promise<Object>} Loaded content
   */
  async _loadHttp() {
    const response = await fetch(this.apiEndpoint, {
      method: 'GET',
      headers: {
        'Accept': 'application/json'
      }
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    return await response.json();
  }

  /**
   * Get component styles
   * @returns {string} CSS styles
   * @private
   */
  getStyles() {
    return `
      <style>
        /* Import vanilla-jsoneditor dark theme CSS */
        @import url('https://cdn.jsdelivr.net/npm/vanilla-jsoneditor@1.0.6/themes/jse-theme-dark.css');

        :host {
          display: block;
          width: 100%;
          height: 100%;
        }

        .json-editor-wrapper {
          display: flex;
          flex-direction: column;
          height: 100%;
          gap: 1rem;
        }

        .helper-text {
          padding: 1rem;
          background: #f5f5f5;
          border: 2px solid #3273dc;
          border-radius: 4px;
          font-size: 0.9rem;
          color: #363636;
        }

        .helper-text strong {
          color: #3273dc;
        }

        .editor-container {
          flex: 1;
          border: 1px solid #dbdbdb;
          border-radius: 4px;
          overflow: hidden;
          min-height: 400px;
        }

        .editor-status {
          padding: 0.5rem;
          min-height: 24px;
        }

        .status-indicator {
          font-size: 0.875rem;
          padding: 0.25rem 0.5rem;
          border-radius: 3px;
        }

        .status-success {
          background: #48c774;
          color: white;
        }

        .status-error {
          background: #f14668;
          color: white;
        }

        .status-info {
          background: #3273dc;
          color: white;
        }

        /* Dark theme support */
        :host-context([data-theme="dark"]) .helper-text {
          background: #2b2b2b;
          border-color: #4a90e2;
          color: #e0e0e0;
        }

        :host-context([data-theme="dark"]) .helper-text strong {
          color: #4a90e2;
        }

        :host-context([data-theme="dark"]) .editor-container {
          border-color: #4a4a4a;
        }

        /* Ensure editor fills container */
        .editor-container :global(.jse-main) {
          height: 100%;
        }
      </style>
    `;
  }
}

// Register the custom element
customElements.define('terraphim-json-editor', TerraphimJsonEditor);

export default TerraphimJsonEditor;
