import { TerraphimElement } from '../base/terraphim-element.js';

/**
 * TerraphimAtomicSaveModal Component
 *
 * Modal for saving documents/content to Atomic Server with metadata.
 *
 * @fires atomic-save - When content is saved {resource: {title, type, description, properties}}
 * @fires modal-close - When modal is closed
 *
 * @example
 * ```html
 * <terraphim-atomic-save-modal
 *   resource-type="Document"
 *   content="Content to save">
 * </terraphim-atomic-save-modal>
 * ```
 */
export class TerraphimAtomicSaveModal extends TerraphimElement {
  static get properties() {
    return {
      isOpen: { type: Boolean, default: false },
      resourceType: { type: String, default: 'Document' },
      title: { type: String, default: '' },
      description: { type: String, default: '' },
      content: { type: String, default: '' },
      tags: { type: Array, default: () => [] },
      properties: { type: Object, default: () => ({}) },
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._currentTag = '';
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
        max-width: 700px;
        max-height: 90vh;
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

      .atomic-badge {
        display: inline-flex;
        align-items: center;
        gap: var(--spacing-xs);
        padding: var(--spacing-xs) var(--spacing-sm);
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
        border-radius: var(--border-radius-sm);
        font-size: var(--font-size-xs);
        font-weight: var(--font-weight-medium);
        margin-left: var(--spacing-sm);
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
        flex: 1;
        overflow-y: auto;
        padding: var(--spacing-lg);
      }

      .form-section {
        margin-bottom: var(--spacing-lg);
      }

      .section-title {
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-semibold);
        color: var(--text-primary);
        margin-bottom: var(--spacing-sm);
        text-transform: uppercase;
        letter-spacing: 0.05em;
      }

      .form-group {
        margin-bottom: var(--spacing-md);
      }

      .form-label {
        display: block;
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-medium);
        color: var(--text-primary);
        margin-bottom: var(--spacing-xs);
      }

      .form-input,
      .form-select,
      .form-textarea {
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

      .form-textarea {
        min-height: 100px;
        resize: vertical;
        font-family: var(--font-family-mono);
      }

      .form-input:focus,
      .form-select:focus,
      .form-textarea:focus {
        outline: none;
        border-color: var(--color-primary);
      }

      .form-help {
        font-size: var(--font-size-xs);
        color: var(--text-tertiary);
        margin-top: var(--spacing-xs);
      }

      .tag-input-container {
        display: flex;
        gap: var(--spacing-xs);
      }

      .tag-input-container .form-input {
        flex: 1;
      }

      .add-tag-btn {
        padding: var(--spacing-sm) var(--spacing-md);
        background: var(--color-primary);
        color: var(--color-primary-contrast);
        border: none;
        border-radius: var(--border-radius-md);
        cursor: pointer;
        font-size: var(--font-size-sm);
        font-weight: var(--font-weight-medium);
        transition: var(--transition-base);
      }

      .add-tag-btn:hover {
        background: var(--color-primary-dark);
      }

      .tags-list {
        display: flex;
        flex-wrap: wrap;
        gap: var(--spacing-xs);
        margin-top: var(--spacing-sm);
      }

      .tag-item {
        display: flex;
        align-items: center;
        gap: var(--spacing-xs);
        padding: var(--spacing-xs) var(--spacing-sm);
        background: var(--bg-hover);
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-sm);
        font-size: var(--font-size-xs);
        color: var(--text-primary);
      }

      .tag-remove {
        background: transparent;
        border: none;
        padding: 0;
        cursor: pointer;
        color: var(--text-secondary);
        transition: var(--transition-fast);
      }

      .tag-remove:hover {
        color: var(--color-danger);
      }

      .tag-remove svg {
        width: 14px;
        height: 14px;
        fill: currentColor;
      }

      .properties-grid {
        display: grid;
        grid-template-columns: 1fr 1fr auto;
        gap: var(--spacing-xs);
        margin-top: var(--spacing-sm);
      }

      .property-row {
        display: contents;
      }

      .property-input {
        padding: var(--spacing-xs) var(--spacing-sm);
        background: var(--bg-secondary);
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-sm);
        font-size: var(--font-size-xs);
        color: var(--text-primary);
      }

      .remove-property-btn {
        padding: var(--spacing-xs);
        background: transparent;
        border: 1px solid var(--border-primary);
        border-radius: var(--border-radius-sm);
        cursor: pointer;
        color: var(--text-secondary);
        transition: var(--transition-fast);
      }

      .remove-property-btn:hover {
        background: var(--color-danger);
        color: white;
        border-color: var(--color-danger);
      }

      .remove-property-btn svg {
        width: 14px;
        height: 14px;
        fill: currentColor;
      }

      .add-property-btn {
        margin-top: var(--spacing-sm);
        padding: var(--spacing-xs) var(--spacing-sm);
        background: transparent;
        border: 1px dashed var(--border-primary);
        border-radius: var(--border-radius-sm);
        cursor: pointer;
        color: var(--text-secondary);
        font-size: var(--font-size-xs);
        transition: var(--transition-base);
      }

      .add-property-btn:hover {
        border-color: var(--color-primary);
        color: var(--color-primary);
      }

      .modal-footer {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: var(--spacing-lg);
        border-top: 1px solid var(--border-primary);
      }

      .footer-info {
        font-size: var(--font-size-xs);
        color: var(--text-tertiary);
      }

      .footer-actions {
        display: flex;
        gap: var(--spacing-sm);
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
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
      }

      .modal-button.save:hover {
        opacity: 0.9;
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
    const html = `
      <style>${this.styles()}</style>
      <div class="modal-backdrop" id="backdrop"></div>
      <div class="modal-container" role="dialog" aria-modal="true" aria-labelledby="modal-title">
        <div class="modal-header">
          <div>
            <h2 class="modal-title" id="modal-title">
              Save to Atomic Server
              <span class="atomic-badge">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12,2A10,10 0 0,1 22,12A10,10 0 0,1 12,22A10,10 0 0,1 2,12A10,10 0 0,1 12,2M12,4A8,8 0 0,0 4,12A8,8 0 0,0 12,20A8,8 0 0,0 20,12A8,8 0 0,0 12,4M12,6A6,6 0 0,1 18,12A6,6 0 0,1 12,18A6,6 0 0,1 6,12A6,6 0 0,1 12,6M12,8A4,4 0 0,0 8,12A4,4 0 0,0 12,16A4,4 0 0,0 16,12A4,4 0 0,0 12,8Z"/>
                </svg>
                Atomic Data
              </span>
            </h2>
          </div>
          <button class="close-button" id="closeBtn" aria-label="Close modal">
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
              <path d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <div class="form-section">
            <h3 class="section-title">Basic Information</h3>
            <div class="form-group">
              <label class="form-label" for="resourceType">Resource Type</label>
              <select class="form-select" id="resourceType">
                <option value="Document" ${this.resourceType === 'Document' ? 'selected' : ''}>Document</option>
                <option value="Article" ${this.resourceType === 'Article' ? 'selected' : ''}>Article</option>
                <option value="Note" ${this.resourceType === 'Note' ? 'selected' : ''}>Note</option>
                <option value="Reference" ${this.resourceType === 'Reference' ? 'selected' : ''}>Reference</option>
                <option value="Collection" ${this.resourceType === 'Collection' ? 'selected' : ''}>Collection</option>
              </select>
              <p class="form-help">The type of resource being saved</p>
            </div>

            <div class="form-group">
              <label class="form-label" for="titleInput">Title *</label>
              <input
                type="text"
                class="form-input"
                id="titleInput"
                placeholder="Enter resource title"
                value="${this._escapeHTML(this.title)}"
                required>
              <p class="form-help">A descriptive title for this resource</p>
            </div>

            <div class="form-group">
              <label class="form-label" for="descriptionInput">Description</label>
              <textarea
                class="form-textarea"
                id="descriptionInput"
                placeholder="Enter a brief description">${this._escapeHTML(this.description)}</textarea>
              <p class="form-help">A short summary or description</p>
            </div>

            <div class="form-group">
              <label class="form-label" for="contentInput">Content</label>
              <textarea
                class="form-textarea"
                id="contentInput"
                placeholder="Enter content to save"
                style="min-height: 150px;">${this._escapeHTML(this.content)}</textarea>
              <p class="form-help">The main content of the resource</p>
            </div>
          </div>

          <div class="form-section">
            <h3 class="section-title">Tags</h3>
            <div class="tag-input-container">
              <input
                type="text"
                class="form-input"
                id="tagInput"
                placeholder="Add a tag"
                value="${this._escapeHTML(this._currentTag)}">
              <button class="add-tag-btn" id="addTagBtn">Add</button>
            </div>
            <div class="tags-list" id="tagsList">
              ${this.tags.map((tag, index) => `
                <div class="tag-item">
                  ${this._escapeHTML(tag)}
                  <button class="tag-remove" data-index="${index}">
                    <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                      <path d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
                    </svg>
                  </button>
                </div>
              `).join('')}
            </div>
          </div>

          <div class="form-section">
            <h3 class="section-title">Custom Properties</h3>
            <p class="form-help">Add custom key-value pairs for this resource</p>
            <div class="properties-grid" id="propertiesGrid">
              ${Object.entries(this.properties).map(([key, value], index) => `
                <input type="text" class="property-input" data-prop-key="${index}" value="${this._escapeHTML(key)}" placeholder="Key">
                <input type="text" class="property-input" data-prop-value="${index}" value="${this._escapeHTML(value)}" placeholder="Value">
                <button class="remove-property-btn" data-prop-index="${index}">
                  <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
                  </svg>
                </button>
              `).join('')}
            </div>
            <button class="add-property-btn" id="addPropertyBtn">+ Add Property</button>
          </div>
        </div>
        <div class="modal-footer">
          <div class="footer-info">
            Resource will be saved to your Atomic Server instance
          </div>
          <div class="footer-actions">
            <button class="modal-button cancel" id="cancelBtn">Cancel</button>
            <button class="modal-button save" id="saveBtn">Save to Atomic Server</button>
          </div>
        </div>
      </div>
    `;

    this.setHTML(this.shadowRoot, html);

    // Attach event listeners
    const backdrop = this.$('#backdrop');
    const closeBtn = this.$('#closeBtn');
    const cancelBtn = this.$('#cancelBtn');
    const saveBtn = this.$('#saveBtn');
    const addTagBtn = this.$('#addTagBtn');
    const tagInput = this.$('#tagInput');
    const addPropertyBtn = this.$('#addPropertyBtn');

    if (backdrop) backdrop.addEventListener('click', () => this.close());
    if (closeBtn) closeBtn.addEventListener('click', () => this.close());
    if (cancelBtn) cancelBtn.addEventListener('click', () => this.close());
    if (saveBtn) saveBtn.addEventListener('click', () => this._handleSave());
    if (addTagBtn) addTagBtn.addEventListener('click', () => this._addTag());
    if (tagInput) {
      tagInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          this._addTag();
        }
      });
      tagInput.addEventListener('input', (e) => {
        this._currentTag = e.target.value;
      });
    }
    if (addPropertyBtn) addPropertyBtn.addEventListener('click', () => this._addProperty());

    // Tag removal listeners
    this.$$('.tag-remove').forEach(btn => {
      btn.addEventListener('click', (e) => {
        const index = parseInt(e.currentTarget.dataset.index);
        this._removeTag(index);
      });
    });

    // Property removal listeners
    this.$$('.remove-property-btn').forEach(btn => {
      btn.addEventListener('click', (e) => {
        const index = parseInt(e.currentTarget.dataset.propIndex);
        this._removeProperty(index);
      });
    });
  }

  _addTag() {
    const tagInput = this.$('#tagInput');
    if (!tagInput) return;

    const tag = tagInput.value.trim();
    if (tag && !this.tags.includes(tag)) {
      this.tags = [...this.tags, tag];
      this._currentTag = '';
    }
  }

  _removeTag(index) {
    this.tags = this.tags.filter((_, i) => i !== index);
  }

  _addProperty() {
    this.properties = { ...this.properties, '': '' };
  }

  _removeProperty(index) {
    const entries = Object.entries(this.properties);
    entries.splice(index, 1);
    this.properties = Object.fromEntries(entries);
  }

  _handleSave() {
    const titleInput = this.$('#titleInput');
    const descriptionInput = this.$('#descriptionInput');
    const contentInput = this.$('#contentInput');
    const resourceType = this.$('#resourceType');

    if (!titleInput || !titleInput.value.trim()) {
      alert('Please enter a title');
      return;
    }

    // Collect properties from grid
    const props = {};
    const keys = this.$$('[data-prop-key]');
    const values = this.$$('[data-prop-value]');
    keys.forEach((keyInput, index) => {
      const key = keyInput.value.trim();
      const value = values[index].value.trim();
      if (key) {
        props[key] = value;
      }
    });

    const resource = {
      type: resourceType.value,
      title: titleInput.value.trim(),
      description: descriptionInput.value.trim(),
      content: contentInput.value.trim(),
      tags: this.tags,
      properties: props
    };

    this.dispatchEvent(new CustomEvent('atomic-save', {
      detail: { resource },
      bubbles: true,
      composed: true
    }));

    this.close();
  }

  // Public API
  open(options = {}) {
    if (options.title) this.title = options.title;
    if (options.description) this.description = options.description;
    if (options.content) this.content = options.content;
    if (options.resourceType) this.resourceType = options.resourceType;
    if (options.tags) this.tags = options.tags;
    if (options.properties) this.properties = options.properties;
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

customElements.define('terraphim-atomic-save-modal', TerraphimAtomicSaveModal);
