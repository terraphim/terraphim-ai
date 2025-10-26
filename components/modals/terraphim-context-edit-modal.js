import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * TerraphimContextEditModal Component
 *
 * Modal for adding or editing context items (files, URLs, documents).
 *
 * @fires context-save - When context item is saved {item: {type, title, path, id?}}
 * @fires modal-close - When modal is closed
 *
 * @example
 * ```html
 * <terraphim-context-edit-modal
 *   mode="add"
 *   item-type="file">
 * </terraphim-context-edit-modal>
 * ```
 */
export class TerraphimContextEditModal extends TerraphimElement {
  static get properties() {
    return {
      isOpen: { type: Boolean, default: false },
      mode: { type: String, default: 'add' }, // 'add' or 'edit'
      itemType: { type: String, default: 'file' }, // 'file', 'url', 'document'
      itemTitle: { type: String, default: '' },
      itemPath: { type: String, default: '' },
      itemId: { type: String, default: null },
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._formData = {
      type: 'file',
      title: '',
      path: ''
    };
  }

  styles() {
    return `
      :host {
        display: none;
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        z-index: 1000;
      }

      :host([is-open]) {
        display: flex;
        align-items: center;
        justify-content: center;
      }

      .modal-backdrop {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.5);
        animation: fadeIn 0.2s ease-out;
      }

      .modal-container {
        position: relative;
        background: var(--bg-elevated);
        border-radius: var(--border-radius-lg);
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
        width: 90%;
        max-width: 500px;
        display: flex;
        flex-direction: column;
        animation: slideIn 0.3s ease-out;
        z-index: 1001;
      }

      .modal-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: var(--spacing-lg);
        border-bottom: 1px solid var(--border-primary);
      }

      .modal-title {
        font-size: var(--font-size-xl);
        font-weight: var(--font-weight-bold);
        margin: 0;
        color: var(--text-primary);
      }

      .close-button {
        background: transparent;
        border: none;
        padding: var(--spacing-xs);
        cursor: pointer;
        border-radius: var(--border-radius-sm);
        color: var(--text-secondary);
        transition: var(--transition-base);
      }

      .close-button:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
      }

      .close-button svg {
        width: 24px;
        height: 24px;
        fill: currentColor;
      }

      .modal-body {
        padding: var(--spacing-lg);
      }

      .form-group {
        margin-bottom: var(--spacing-md);
      }

      .form-label {
        display: block;
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-semibold);
        color: var(--text-primary);
        margin-bottom: var(--spacing-xs);
      }

      .form-input,
      .form-select {
        width: 100%;
        padding: var(--spacing-sm);
        background: var(--bg-secondary);
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-md);
        font-family: var(--font-family-sans);
        font-size: var(--font-size-sm);
        color: var(--text-primary);
        transition: var(--transition-base);
      }

      .form-input:focus,
      .form-select:focus {
        outline: none;
        border-color: var(--color-primary);
      }

      .form-help {
        font-size: var(--font-size-xs);
        color: var(--text-tertiary);
        margin-top: var(--spacing-xs);
      }

      .form-error {
        font-size: var(--font-size-xs);
        color: var(--color-danger);
        margin-top: var(--spacing-xs);
      }

      .modal-footer {
        display: flex;
        justify-content: flex-end;
        gap: var(--spacing-sm);
        padding: var(--spacing-lg);
        border-top: 1px solid var(--border-primary);
      }

      .modal-button {
        padding: var(--spacing-sm) var(--spacing-lg);
        border: none;
        border-radius: var(--border-radius-md);
        font-family: var(--font-family-sans);
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-medium);
        cursor: pointer;
        transition: var(--transition-base);
      }

      .modal-button.cancel {
        background: var(--bg-elevated);
        color: var(--text-primary);
        border: 1px solid var(--border-primary);
      }

      .modal-button.cancel:hover {
        background: var(--bg-hover);
      }

      .modal-button.save {
        background: var(--color-primary);
        color: var(--color-primary-contrast);
      }

      .modal-button.save:hover {
        background: var(--color-primary-dark);
      }

      .modal-button:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }

      @keyframes fadeIn {
        from { opacity: 0; }
        to { opacity: 1; }
      }

      @keyframes slideIn {
        from {
          opacity: 0;
          transform: translateY(-20px);
        }
        to {
          opacity: 1;
          transform: translateY(0);
        }
      }
    `;
  }

  render() {
    const title = this.mode === 'add' ? 'Add Context Item' : 'Edit Context Item';
    const saveText = this.mode === 'add' ? 'Add' : 'Save';

    const html = `
      <style>${this.styles()}</style>
      <div class="modal-backdrop" id="backdrop"></div>
      <div class="modal-container" role="dialog" aria-modal="true" aria-labelledby="modal-title">
        <div class="modal-header">
          <h2 class="modal-title" id="modal-title">${this._escapeHTML(title)}</h2>
          <button class="close-button" id="closeBtn" aria-label="Close modal">
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
              <path d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <form id="contextForm">
            <div class="form-group">
              <label class="form-label" for="typeSelect">Type</label>
              <select class="form-select" id="typeSelect" required>
                <option value="file" ${this.itemType === 'file' ? 'selected' : ''}>File</option>
                <option value="url" ${this.itemType === 'url' ? 'selected' : ''}>URL</option>
                <option value="document" ${this.itemType === 'document' ? 'selected' : ''}>Document</option>
              </select>
              <p class="form-help">The type of context item you're adding</p>
            </div>

            <div class="form-group">
              <label class="form-label" for="titleInput">Title</label>
              <input
                type="text"
                class="form-input"
                id="titleInput"
                placeholder="Enter a descriptive title"
                value="${this._escapeHTML(this.itemTitle)}"
                required>
              <p class="form-help">A friendly name for this context item</p>
              <p class="form-error" id="titleError" style="display: none;"></p>
            </div>

            <div class="form-group">
              <label class="form-label" for="pathInput">Path / URL</label>
              <input
                type="text"
                class="form-input"
                id="pathInput"
                placeholder="Enter file path or URL"
                value="${this._escapeHTML(this.itemPath)}"
                required>
              <p class="form-help" id="pathHelp">The location of this context item</p>
              <p class="form-error" id="pathError" style="display: none;"></p>
            </div>
          </form>
        </div>
        <div class="modal-footer">
          <button class="modal-button cancel" id="cancelBtn">Cancel</button>
          <button class="modal-button save" id="saveBtn">${this._escapeHTML(saveText)}</button>
        </div>
      </div>
    `;

    this.setHTML(this.shadowRoot, html);

    // Attach event listeners
    const backdrop = this.$('#backdrop');
    const closeBtn = this.$('#closeBtn');
    const cancelBtn = this.$('#cancelBtn');
    const saveBtn = this.$('#saveBtn');
    const form = this.$('#contextForm');
    const typeSelect = this.$('#typeSelect');
    const titleInput = this.$('#titleInput');
    const pathInput = this.$('#pathInput');
    const pathHelp = this.$('#pathHelp');

    if (backdrop) {
      backdrop.addEventListener('click', () => this.close());
    }

    if (closeBtn) {
      closeBtn.addEventListener('click', () => this.close());
    }

    if (cancelBtn) {
      cancelBtn.addEventListener('click', () => this.close());
    }

    if (saveBtn) {
      saveBtn.addEventListener('click', (e) => {
        e.preventDefault();
        this._handleSave();
      });
    }

    if (form) {
      form.addEventListener('submit', (e) => {
        e.preventDefault();
        this._handleSave();
      });
    }

    if (typeSelect) {
      typeSelect.addEventListener('change', (e) => {
        this._formData.type = e.target.value;
        this._updatePathHelp(e.target.value);
      });
    }

    if (titleInput) {
      titleInput.addEventListener('input', (e) => {
        this._formData.title = e.target.value;
        this._clearError('titleError');
      });
    }

    if (pathInput) {
      pathInput.addEventListener('input', (e) => {
        this._formData.path = e.target.value;
        this._clearError('pathError');
      });
    }

    // Initialize form data
    this._formData = {
      type: this.itemType || 'file',
      title: this.itemTitle || '',
      path: this.itemPath || ''
    };

    this._updatePathHelp(this._formData.type);
  }

  _updatePathHelp(type) {
    const pathHelp = this.$('#pathHelp');
    if (!pathHelp) return;

    const helpText = {
      file: 'Full path to the file (e.g., /Users/name/document.pdf)',
      url: 'Full URL including protocol (e.g., https://example.com)',
      document: 'Relative path or identifier for the document'
    };

    pathHelp.textContent = helpText[type] || 'The location of this context item';
  }

  _validateForm() {
    let isValid = true;

    // Validate title
    if (!this._formData.title || this._formData.title.trim() === '') {
      this._showError('titleError', 'Title is required');
      isValid = false;
    }

    // Validate path
    if (!this._formData.path || this._formData.path.trim() === '') {
      this._showError('pathError', 'Path/URL is required');
      isValid = false;
    } else if (this._formData.type === 'url') {
      try {
        new URL(this._formData.path);
      } catch (e) {
        this._showError('pathError', 'Please enter a valid URL');
        isValid = false;
      }
    }

    return isValid;
  }

  _showError(elementId, message) {
    const errorEl = this.$(`#${elementId}`);
    if (errorEl) {
      errorEl.textContent = message;
      errorEl.style.display = 'block';
    }
  }

  _clearError(elementId) {
    const errorEl = this.$(`#${elementId}`);
    if (errorEl) {
      errorEl.textContent = '';
      errorEl.style.display = 'none';
    }
  }

  _handleSave() {
    if (!this._validateForm()) {
      return;
    }

    const item = {
      type: this._formData.type,
      title: this._formData.title.trim(),
      path: this._formData.path.trim(),
    };

    if (this.mode === 'edit' && this.itemId) {
      item.id = this.itemId;
    }

    this.dispatchEvent(new CustomEvent('context-save', {
      detail: { item },
      bubbles: true,
      composed: true
    }));

    this.close();
  }

  // Public API
  open(options = {}) {
    if (options.mode) this.mode = options.mode;
    if (options.item) {
      this.itemType = options.item.type || 'file';
      this.itemTitle = options.item.title || '';
      this.itemPath = options.item.path || '';
      this.itemId = options.item.id || null;
    } else {
      this.itemType = 'file';
      this.itemTitle = '';
      this.itemPath = '';
      this.itemId = null;
    }
    this.isOpen = true;
  }

  close() {
    this.isOpen = false;
    this.dispatchEvent(new CustomEvent('modal-close', {
      bubbles: true,
      composed: true
    }));
  }

  _escapeHTML(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
}

customElements.define('terraphim-context-edit-modal', TerraphimContextEditModal);
