/**
 * Navigation Category Component
 * Collapsible category group in sidebar navigation
 */

class NavCategory extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this.expanded = this.hasAttribute('expanded');
  }

  static get observedAttributes() {
    return ['label', 'icon', 'expanded'];
  }

  connectedCallback() {
    this.render();
    this.setupEventListeners();
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (oldValue !== newValue) {
      if (name === 'expanded') {
        this.expanded = this.hasAttribute('expanded');
      }
      this.render();
    }
  }

  /**
   * Get component properties
   */
  get label() {
    return this.getAttribute('label') || '';
  }

  get icon() {
    return this.getAttribute('icon') || '';
  }

  /**
   * Toggle category expansion
   */
  toggle() {
    this.expanded = !this.expanded;

    if (this.expanded) {
      this.setAttribute('expanded', '');
    } else {
      this.removeAttribute('expanded');
    }

    this.updateUI();
    this.dispatchToggle();
  }

  /**
   * Update UI based on expanded state
   */
  updateUI() {
    const content = this.shadowRoot.querySelector('.category-content');
    const button = this.shadowRoot.querySelector('.category-header');
    const chevron = this.shadowRoot.querySelector('.chevron');

    if (this.expanded) {
      content.style.display = 'block';
      button.setAttribute('aria-expanded', 'true');
      chevron.textContent = '▼';
    } else {
      content.style.display = 'none';
      button.setAttribute('aria-expanded', 'false');
      chevron.textContent = '▶';
    }
  }

  /**
   * Dispatch toggle event
   */
  dispatchToggle() {
    this.dispatchEvent(new CustomEvent('category-toggle', {
      detail: { expanded: this.expanded, label: this.label },
      bubbles: true,
      composed: true
    }));
  }

  /**
   * Setup event listeners
   */
  setupEventListeners() {
    const button = this.shadowRoot.querySelector('.category-header');
    button.addEventListener('click', () => this.toggle());
  }

  /**
   * Render component template
   */
  render() {
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: block;
          margin-bottom: 0.5rem;
        }

        .category-header {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          width: 100%;
          padding: 0.625rem 1rem;
          background: transparent;
          border: none;
          color: var(--color-text-primary, #2c3e50);
          font-family: var(--font-family-base, system-ui, sans-serif);
          font-size: 0.875rem;
          font-weight: 600;
          text-align: left;
          cursor: pointer;
          border-radius: 6px;
          transition: all 0.15s ease-in-out;
        }

        .category-header:hover {
          background: var(--color-surface, #f8f9fa);
        }

        .category-header:focus-visible {
          outline: 2px solid var(--color-border-focus, #3498db);
          outline-offset: -2px;
        }

        .chevron {
          font-size: 0.625rem;
          color: var(--color-text-tertiary, #95a5a6);
          transition: transform 0.15s ease-in-out;
          width: 0.875rem;
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .icon {
          font-size: 1rem;
          line-height: 1;
        }

        .label {
          flex: 1;
        }

        .category-content {
          display: ${this.expanded ? 'block' : 'none'};
          padding: 0.25rem 0;
        }

        ::slotted(*) {
          display: block;
        }
      </style>

      <button
        class="category-header"
        type="button"
        aria-expanded="${this.expanded}"
        aria-controls="category-content"
      >
        <span class="chevron">${this.expanded ? '▼' : '▶'}</span>
        ${this.icon ? `<span class="icon">${this.icon}</span>` : ''}
        <span class="label">${this.label}</span>
      </button>

      <div
        class="category-content"
        id="category-content"
        role="region"
      >
        <slot></slot>
      </div>
    `;
  }
}

customElements.define('nav-category', NavCategory);
