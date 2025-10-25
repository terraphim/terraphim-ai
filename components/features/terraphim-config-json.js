import { TerraphimElement } from '../base/terraphim-element.js';
import '../config/terraphim-json-editor.js';

/**
 * @fileoverview TerraphimConfigJson - JSON configuration editor feature component
 *
 * Wraps the terraphim-json-editor component with app-specific configuration
 * and integration logic.
 *
 * @example
 * <terraphim-config-json></terraphim-config-json>
 */
class TerraphimConfigJson extends TerraphimElement {
  /**
   * Property definitions
   * @static
   * @returns {Object} Property definitions
   */
  static get properties() {
    return {
      config: { type: Object, default: () => ({}) },
      schema: { type: Object, default: null },
      mode: { type: String, default: 'tree' }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  /**
   * Called when element is connected to DOM
   */
  onConnected() {
    this.render();
    this._setupEditor();
  }

  /**
   * Render the component
   */
  render() {
    this.setHTML(this.shadowRoot, `
      ${this.getStyles()}
      <div class="config-json-container">
        <terraphim-json-editor
          id="editor"
          mode="${this.mode}"
          auto-save="false"
          auto-save-delay="1000"
          main-menu-bar="true"
          navigation-bar="true"
          status-bar="true"
          api-endpoint="/config/"
          tauri-command="update_config">
        </terraphim-json-editor>
      </div>
    `);
  }

  /**
   * Setup editor with initial configuration
   * @private
   */
  _setupEditor() {
    const editor = this.$('#editor');
    if (!editor) return;

    // Set initial config if provided
    if (this.config && Object.keys(this.config).length > 0) {
      editor.value = this.config;
    }

    // Set schema if provided
    if (this.schema) {
      editor.schema = this.schema;
    }

    // Forward editor events
    this._forwardEditorEvents(editor);
  }

  /**
   * Forward events from editor to parent
   * @private
   * @param {HTMLElement} editor - Editor element
   */
  _forwardEditorEvents(editor) {
    const events = [
      'change',
      'blur',
      'focus',
      'validation-error',
      'config-saved',
      'config-loaded',
      'save-error',
      'load-error'
    ];

    events.forEach(eventName => {
      this.listenTo(editor, eventName, (e) => {
        this.emit(eventName, e.detail);
      });
    });
  }

  /**
   * Get current configuration from editor
   * @returns {Object} Current configuration
   * @public
   */
  getConfig() {
    const editor = this.$('#editor');
    return editor ? editor.get() : this.config;
  }

  /**
   * Set configuration in editor
   * @param {Object} config - New configuration
   * @public
   */
  setConfig(config) {
    this.config = config;
    const editor = this.$('#editor');
    if (editor) {
      editor.set(config);
    }
  }

  /**
   * Save configuration to backend
   * @returns {Promise<void>}
   * @public
   */
  async save() {
    const editor = this.$('#editor');
    if (editor) {
      await editor.save();
    }
  }

  /**
   * Load configuration from backend
   * @returns {Promise<void>}
   * @public
   */
  async load() {
    const editor = this.$('#editor');
    if (editor) {
      await editor.load();
    }
  }

  /**
   * Validate current configuration
   * @returns {boolean} True if valid
   * @public
   */
  validate() {
    const editor = this.$('#editor');
    return editor ? editor.validate() : true;
  }

  /**
   * Get component styles
   * @returns {string} CSS styles
   * @private
   */
  getStyles() {
    return `
      <style>
        :host {
          display: block;
          width: 100%;
          height: 100%;
        }

        .config-json-container {
          width: 100%;
          height: 100%;
          display: flex;
          flex-direction: column;
        }

        terraphim-json-editor {
          flex: 1;
          min-height: 500px;
        }
      </style>
    `;
  }
}

customElements.define('terraphim-config-json', TerraphimConfigJson);

export default TerraphimConfigJson;
