import { TerraphimElement } from './terraphim-element.js';

/**
 * TerraphimModal Component
 *
 * Base modal dialog component with overlay, animations, and accessibility features.
 *
 * @fires modal-open - When the modal is opened
 * @fires modal-close - When the modal is closed {reason: 'backdrop'|'escape'|'button'|'manual'}
 * @fires modal-confirm - When confirm action is triggered
 *
 * @example
 * ```html
 * <terraphim-modal
 *   title="Modal Title"
 *   size="medium"
 *   show-footer
 *   confirm-text="Save"
 *   cancel-text="Cancel">
 *   <div slot="content">Modal content goes here</div>
 * </terraphim-modal>
 * ```
 */
export class TerraphimModal extends TerraphimElement {
  static get properties() {
    return {
      isOpen: { type: Boolean, default: false },
      title: { type: String, default: 'Modal' },
      size: { type: String, default: 'medium' }, // 'small', 'medium', 'large', 'fullscreen'
      showFooter: { type: Boolean, reflect: true },
      showCloseButton: { type: Boolean, reflect: true, default: true },
      confirmText: { type: String, default: 'Confirm' },
      cancelText: { type: String, default: 'Cancel' },
      closeOnBackdrop: { type: Boolean, default: true },
      closeOnEscape: { type: Boolean, default: true },
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._handleEscape = this._handleEscape.bind(this);
    this._handleBackdropClick = this._handleBackdropClick.bind(this);
  }

  connectedCallback() {
    super.connectedCallback();
    if (this.isOpen) {
      this._setupEventListeners();
    }
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    this._removeEventListeners();
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
        max-height: 90vh;
        display: flex;
        flex-direction: column;
        animation: slideIn 0.3s ease-out;
        z-index: 1001;
      }

      .modal-container.size-small {
        width: 90%;
        max-width: 400px;
      }

      .modal-container.size-medium {
        width: 90%;
        max-width: 600px;
      }

      .modal-container.size-large {
        width: 90%;
        max-width: 900px;
      }

      .modal-container.size-fullscreen {
        width: 95%;
        max-width: 1400px;
        height: 90vh;
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
        display: flex;
        align-items: center;
        justify-content: center;
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
        color: var(--text-primary);
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

      .modal-button.confirm {
        background: var(--color-primary);
        color: var(--color-primary-contrast);
      }

      .modal-button.confirm:hover {
        background: var(--color-primary-dark);
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

      /* Accessibility */
      .modal-container:focus {
        outline: 2px solid var(--color-primary);
        outline-offset: 2px;
      }
    `;
  }

  render() {
    const html = `
      <style>${this.styles()}</style>
      <div class="modal-backdrop" id="backdrop"></div>
      <div class="modal-container size-${this.size}" role="dialog" aria-modal="true" aria-labelledby="modal-title" tabindex="-1">
        <div class="modal-header">
          <h2 class="modal-title" id="modal-title">${this._escapeHTML(this.title)}</h2>
          ${this.showCloseButton ? `
            <button class="close-button" id="closeBtn" aria-label="Close modal">
              <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
              </svg>
            </button>
          ` : ''}
        </div>
        <div class="modal-body">
          <slot name="content"></slot>
        </div>
        ${this.showFooter ? `
          <div class="modal-footer">
            <button class="modal-button cancel" id="cancelBtn">${this._escapeHTML(this.cancelText)}</button>
            <button class="modal-button confirm" id="confirmBtn">${this._escapeHTML(this.confirmText)}</button>
          </div>
        ` : ''}
      </div>
    `;

    this.setHTML(this.shadowRoot, html);

    // Attach event listeners
    const backdrop = this.$('#backdrop');
    const closeBtn = this.$('#closeBtn');
    const cancelBtn = this.$('#cancelBtn');
    const confirmBtn = this.$('#confirmBtn');

    if (backdrop && this.closeOnBackdrop) {
      backdrop.addEventListener('click', this._handleBackdropClick);
    }

    if (closeBtn) {
      closeBtn.addEventListener('click', () => this.close('button'));
    }

    if (cancelBtn) {
      cancelBtn.addEventListener('click', () => this.close('cancel'));
    }

    if (confirmBtn) {
      confirmBtn.addEventListener('click', () => this._handleConfirm());
    }

    // Focus management
    if (this.isOpen) {
      setTimeout(() => {
        const container = this.$('.modal-container');
        if (container) {
          container.focus();
        }
      }, 100);
    }
  }

  _handleBackdropClick(e) {
    if (e.target.id === 'backdrop') {
      this.close('backdrop');
    }
  }

  _handleEscape(e) {
    if (this.closeOnEscape && e.key === 'Escape' && this.isOpen) {
      this.close('escape');
    }
  }

  _handleConfirm() {
    this.dispatchEvent(new CustomEvent('modal-confirm', {
      bubbles: true,
      composed: true
    }));
    this.close('confirm');
  }

  _setupEventListeners() {
    document.addEventListener('keydown', this._handleEscape);
  }

  _removeEventListeners() {
    document.removeEventListener('keydown', this._handleEscape);
  }

  propertyChangedCallback(name, oldValue, newValue) {
    super.propertyChangedCallback(name, oldValue, newValue);

    if (name === 'isOpen') {
      if (newValue) {
        this._setupEventListeners();
        this.dispatchEvent(new CustomEvent('modal-open', {
          bubbles: true,
          composed: true
        }));
      } else {
        this._removeEventListeners();
      }
    }
  }

  // Public API
  open() {
    this.isOpen = true;
  }

  close(reason = 'manual') {
    this.isOpen = false;
    this.dispatchEvent(new CustomEvent('modal-close', {
      detail: { reason },
      bubbles: true,
      composed: true
    }));
  }

  toggle() {
    this.isOpen = !this.isOpen;
  }

  _escapeHTML(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
}

customElements.define('terraphim-modal', TerraphimModal);
