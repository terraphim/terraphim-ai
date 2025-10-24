/**
 * TerraphimEditor - Web Component orchestrator for Terraphim editing experience
 *
 * Implements three-tier progressive enhancement:
 * - Tier 1 (Core): Vanilla markdown editor (always available)
 * - Tier 2 (Enhanced): Core + autocomplete via backend (Phase 2)
 * - Tier 3 (Premium): Enhanced + Novel rich text editor (Phase 3)
 *
 * Phase 1 Implementation: Core tier only with vanilla markdown editor
 *
 * @class TerraphimEditor
 * @extends TerraphimElement
 * @fires content-changed - Emitted when content changes
 * @fires tier-detected - Emitted when editor tier is detected
 * @example
 * <terraphim-editor
 *   content="# Hello World"
 *   output-format="markdown"
 *   read-only="false">
 * </terraphim-editor>
 */
import { TerraphimElement } from '../base/terraphim-element.js';
import { VanillaMarkdownEditor } from './vanilla-markdown-editor.js';

export class TerraphimEditor extends TerraphimElement {
  static get observedAttributes() {
    return [
      'content',
      'output-format',
      'read-only',
      'show-toolbar',
      'show-preview',
      'enable-autocomplete',
      'role',
      'show-snippets',
      'suggestion-trigger',
      'max-suggestions',
      'min-query-length',
      'debounce-delay'
    ];
  }

  constructor() {
    super();

    this.attachShadow({ mode: 'open' });

    this._editorTier = 'core';
    this._vanillaEditor = null;
    this._initialized = false;
  }

  /**
   * Get current editor tier
   * @returns {string} 'core' | 'enhanced' | 'premium'
   */
  get editorTier() {
    return this._editorTier;
  }

  onConnected() {
    this._initialize();
  }

  connectedCallback() {
    super.connectedCallback();
  }

  onDisconnected() {
    this._cleanup();
  }

  disconnectedCallback() {
    super.disconnectedCallback();
  }

  onAttributeChanged(name, oldValue, newValue) {
    if (!this._initialized) {
      return;
    }

    switch (name) {
      case 'content':
        this._updateContent(newValue);
        break;
      case 'read-only':
        this._updateReadOnly(newValue === 'true');
        break;
      case 'show-toolbar':
      case 'show-preview':
        this._reinitializeEditor();
        break;
    }
  }

  attributeChangedCallback(name, oldValue, newValue) {
    super.attributeChangedCallback(name, oldValue, newValue);
  }

  /**
   * Get editor content in specified format
   * @param {string} format - Output format ('markdown' | 'html' | 'text')
   * @returns {string} Content in requested format
   */
  getContent(format = 'markdown') {
    if (!this._vanillaEditor) {
      return '';
    }

    const content = this._vanillaEditor.getContent();

    switch (format) {
      case 'markdown':
        return content;
      case 'text':
        return content;
      case 'html':
        return this._vanillaEditor._renderMarkdownPreview(content);
      default:
        return content;
    }
  }

  /**
   * Set editor content
   * @param {string} content - New content
   */
  setContent(content) {
    this.setAttribute('content', content);
  }

  /**
   * Get current editor tier
   * @returns {string} Current tier ('core' | 'enhanced' | 'premium')
   */
  getEditorTier() {
    return this._editorTier;
  }

  // Phase 2/3 methods (stubs for now)

  /**
   * Rebuild autocomplete index (Phase 2)
   * @returns {Promise<void>}
   */
  async rebuildAutocompleteIndex() {
    console.log('Autocomplete not available in core tier');
  }

  // Render method

  render() {
    if (!this.shadowRoot) {
      return;
    }

    const template = `
      <style>${this._getHostStyles()}</style>
      <div class="terraphim-editor-container">
        <div class="editor-wrapper" id="editor-wrapper"></div>
      </div>
    `;

    this.shadowRoot.innerHTML = template;

    if (!this._initialized) {
      this._detectTier();
      this._createEditor();
      this._initialized = true;
      this._emitTierDetected();
    }
  }

  _getHostStyles() {
    return `
      :host {
        display: block;
        width: 100%;
        height: 100%;
        min-height: 200px;
      }

      .terraphim-editor-container {
        width: 100%;
        height: 100%;
        display: flex;
        flex-direction: column;
      }

      .editor-wrapper {
        flex: 1;
        display: flex;
        flex-direction: column;
        overflow: hidden;
      }

      .tier-indicator {
        font-size: var(--font-size-small, 12px);
        color: var(--color-text-secondary, #666666);
        padding: 4px 8px;
        background: var(--color-bg-secondary, #f5f5f5);
        border-radius: 4px;
        margin-bottom: 8px;
        display: inline-block;
      }
    `;
  }

  // Private methods

  _initialize() {
    // Initialization is now handled in render()
  }

  _detectTier() {
    this._editorTier = 'core';
  }

  _createEditor() {
    const wrapper = this.shadowRoot.getElementById('editor-wrapper');
    if (!wrapper) {
      return;
    }

    const content = this.getAttribute('content') || '';
    const readOnly = this.getAttribute('read-only') === 'true';
    const showToolbar = this.getAttribute('show-toolbar') !== 'false';
    const showPreview = this.getAttribute('show-preview') === 'true';

    this._vanillaEditor = new VanillaMarkdownEditor({
      content,
      readOnly,
      showToolbar,
      showPreview
    });

    this._vanillaEditor.addEventListener('change', (event) => {
      this._handleContentChange(event.detail.content);
    });

    this._vanillaEditor.addEventListener('keydown', (event) => {
      this._handleKeydown(event);
    });

    const editorElement = this._vanillaEditor.render();
    wrapper.appendChild(editorElement);

    this._injectEditorStyles();
  }

  _reinitializeEditor() {
    this._cleanup();
    this._createEditor();
  }

  _cleanup() {
    if (this._vanillaEditor) {
      this._vanillaEditor.destroy();
      this._vanillaEditor = null;
    }

    const wrapper = this.shadowRoot.getElementById('editor-wrapper');
    if (wrapper) {
      wrapper.innerHTML = '';
    }
  }

  _updateContent(content) {
    if (this._vanillaEditor) {
      this._vanillaEditor.setContent(content);
    }
  }

  _updateReadOnly(readOnly) {
    this._reinitializeEditor();
  }

  _handleContentChange(content) {
    const event = new CustomEvent('content-changed', {
      detail: {
        content,
        format: this.getAttribute('output-format') || 'markdown'
      },
      bubbles: true,
      composed: true
    });
    this.dispatchEvent(event);
  }

  _handleKeydown(event) {
    const customEvent = new CustomEvent('editor-keydown', {
      detail: event.detail,
      bubbles: true,
      composed: true
    });
    this.dispatchEvent(customEvent);
  }

  _emitTierDetected() {
    const event = new CustomEvent('tier-detected', {
      detail: {
        tier: this._editorTier
      },
      bubbles: true,
      composed: true
    });
    this.dispatchEvent(event);
  }

  _injectEditorStyles() {
    const styleElement = document.createElement('style');
    styleElement.textContent = this._getVanillaEditorStyles();
    this.shadowRoot.appendChild(styleElement);
  }

  _getVanillaEditorStyles() {
    return `
      .vanilla-markdown-editor {
        display: flex;
        flex-direction: column;
        height: 100%;
        border: 1px solid var(--color-border, #dddddd);
        border-radius: var(--border-radius, 4px);
        background: var(--color-bg-primary, #ffffff);
        overflow: hidden;
      }

      .editor-toolbar {
        display: flex;
        gap: 4px;
        padding: 8px;
        background: var(--color-bg-secondary, #f5f5f5);
        border-bottom: 1px solid var(--color-border, #dddddd);
      }

      .toolbar-button {
        padding: 6px 12px;
        border: 1px solid var(--color-border, #cccccc);
        background: var(--color-bg-primary, #ffffff);
        color: var(--color-text-primary, #333333);
        cursor: pointer;
        border-radius: var(--border-radius, 4px);
        font-size: var(--font-size-small, 12px);
        font-weight: 600;
        transition: all 0.2s ease;
      }

      .toolbar-button:hover {
        background: var(--color-bg-hover, #e8e8e8);
        border-color: var(--color-primary, #0066cc);
      }

      .toolbar-button:active {
        transform: translateY(1px);
      }

      .toolbar-button-bold {
        font-weight: bold;
      }

      .toolbar-button-italic {
        font-style: italic;
      }

      .toolbar-button-code {
        font-family: var(--font-family-mono, 'Courier New', monospace);
      }

      .editor-container {
        display: flex;
        flex: 1;
        overflow: hidden;
      }

      .editor-content {
        flex: 1;
        padding: 16px;
        overflow-y: auto;
        font-family: var(--font-family-base, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif);
        font-size: var(--font-size-base, 14px);
        line-height: var(--line-height-base, 1.6);
        color: var(--color-text-primary, #333333);
        outline: none;
      }

      .editor-content:focus {
        background: var(--color-bg-focus, #fafafa);
      }

      .editor-content[aria-readonly="true"] {
        background: var(--color-bg-disabled, #f5f5f5);
        cursor: not-allowed;
      }

      .editor-preview {
        flex: 1;
        padding: 16px;
        overflow-y: auto;
        border-left: 1px solid var(--color-border, #dddddd);
        background: var(--color-bg-secondary, #fafafa);
        font-family: var(--font-family-base, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif);
        font-size: var(--font-size-base, 14px);
        line-height: var(--line-height-base, 1.6);
        color: var(--color-text-primary, #333333);
      }

      .editor-preview h1 {
        font-size: 2em;
        font-weight: bold;
        margin: 0.67em 0;
      }

      .editor-preview h2 {
        font-size: 1.5em;
        font-weight: bold;
        margin: 0.75em 0;
      }

      .editor-preview h3 {
        font-size: 1.17em;
        font-weight: bold;
        margin: 0.83em 0;
      }

      .editor-preview strong {
        font-weight: bold;
      }

      .editor-preview em {
        font-style: italic;
      }

      .editor-preview code {
        font-family: var(--font-family-mono, 'Courier New', monospace);
        background: var(--color-bg-code, #f0f0f0);
        padding: 2px 6px;
        border-radius: 3px;
        font-size: 0.9em;
      }

      .editor-preview a {
        color: var(--color-primary, #0066cc);
        text-decoration: underline;
      }

      .editor-preview ul {
        margin: 1em 0;
        padding-left: 2em;
      }

      .editor-preview li {
        margin: 0.5em 0;
      }
    `;
  }
}

customElements.define('terraphim-editor', TerraphimEditor);
