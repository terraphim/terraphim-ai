/**
 * Navigation Item Component
 * Individual navigation link in the sidebar
 */

class NavItem extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  static get observedAttributes() {
    return ['label', 'path', 'active'];
  }

  connectedCallback() {
    this.render();
    this.setupEventListeners();
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (oldValue !== newValue) {
      this.render();
    }
  }

  /**
   * Get component properties
   */
  get label() {
    return this.getAttribute('label') || '';
  }

  get path() {
    return this.getAttribute('path') || '';
  }

  get active() {
    return this.hasAttribute('active');
  }

  set active(value) {
    if (value) {
      this.setAttribute('active', '');
    } else {
      this.removeAttribute('active');
    }
  }

  /**
   * Setup event listeners
   */
  setupEventListeners() {
    const link = this.shadowRoot.querySelector('a');
    link.addEventListener('click', (e) => {
      e.preventDefault();
      this.dispatchNavigation();
    });
  }

  /**
   * Dispatch navigation event
   */
  dispatchNavigation() {
    this.dispatchEvent(new CustomEvent('navigate', {
      detail: { path: this.path },
      bubbles: true,
      composed: true
    }));
  }

  /**
   * Render component template
   */
  render() {
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: block;
        }

        a {
          display: flex;
          align-items: center;
          padding: 0.5rem 1rem;
          padding-left: 2rem;
          color: var(--color-text-secondary, #7f8c8d);
          text-decoration: none;
          font-size: 0.875rem;
          font-weight: 400;
          border-radius: 6px;
          transition: all 0.15s ease-in-out;
          position: relative;
        }

        a:hover {
          background: var(--color-surface, #f8f9fa);
          color: var(--color-text-primary, #2c3e50);
        }

        a:focus-visible {
          outline: 2px solid var(--color-border-focus, #3498db);
          outline-offset: -2px;
        }

        :host([active]) a {
          background: var(--color-surface, #f8f9fa);
          color: var(--color-accent, #3498db);
          font-weight: 500;
        }

        :host([active]) a::before {
          content: '';
          position: absolute;
          left: 0.5rem;
          top: 50%;
          transform: translateY(-50%);
          width: 3px;
          height: 1rem;
          background: var(--color-accent, #3498db);
          border-radius: 2px;
        }
      </style>

      <a href="#${this.path}">
        ${this.label}
      </a>
    `;
  }
}

customElements.define('nav-item', NavItem);
