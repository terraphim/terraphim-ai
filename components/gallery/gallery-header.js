/**
 * Gallery Header Component
 * Top navigation bar with logo, search, and theme toggle
 */

class GalleryHeader extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  connectedCallback() {
    this.render();
    this.setupEventListeners();
  }

  /**
   * Setup event listeners
   */
  setupEventListeners() {
    // Mobile menu toggle
    const menuToggle = this.shadowRoot.querySelector('.menu-toggle');
    if (menuToggle) {
      menuToggle.addEventListener('click', () => this.toggleMobileMenu());
    }

    // Search input
    const searchInput = this.shadowRoot.querySelector('input[type="search"]');
    if (searchInput) {
      searchInput.addEventListener('input', (e) => this.handleSearch(e.target.value));
      searchInput.addEventListener('focus', () => this.handleSearchFocus());
      searchInput.addEventListener('blur', () => this.handleSearchBlur());
    }

    // Logo click
    const logo = this.shadowRoot.querySelector('.logo');
    if (logo) {
      logo.addEventListener('click', (e) => {
        e.preventDefault();
        this.navigateHome();
      });
    }
  }

  /**
   * Toggle mobile menu
   */
  toggleMobileMenu() {
    this.dispatchEvent(new CustomEvent('menu-toggle', {
      bubbles: true,
      composed: true
    }));
  }

  /**
   * Handle search input
   * @param {string} query - Search query
   */
  handleSearch(query) {
    this.dispatchEvent(new CustomEvent('search', {
      detail: { query },
      bubbles: true,
      composed: true
    }));
  }

  /**
   * Handle search focus
   */
  handleSearchFocus() {
    this.dispatchEvent(new CustomEvent('search-focus', {
      bubbles: true,
      composed: true
    }));
  }

  /**
   * Handle search blur
   */
  handleSearchBlur() {
    this.dispatchEvent(new CustomEvent('search-blur', {
      bubbles: true,
      composed: true
    }));
  }

  /**
   * Navigate to home
   */
  navigateHome() {
    this.dispatchEvent(new CustomEvent('navigate', {
      detail: { path: '/' },
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
          position: sticky;
          top: 0;
          z-index: var(--z-sticky, 200);
          background: var(--color-surface-elevated, #ffffff);
          border-bottom: 1px solid var(--color-border, #e1e4e8);
        }

        .header-container {
          display: flex;
          align-items: center;
          gap: 1rem;
          padding: 1rem 1.5rem;
          max-width: 100%;
          margin: 0 auto;
        }

        .menu-toggle {
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 0.5rem;
          background: transparent;
          border: 1px solid var(--color-border, #e1e4e8);
          border-radius: 6px;
          color: var(--color-text-primary, #2c3e50);
          cursor: pointer;
          font-size: 1.25rem;
          transition: all 0.15s ease-in-out;
        }

        .menu-toggle:hover {
          background: var(--color-surface, #f8f9fa);
        }

        .menu-toggle:focus-visible {
          outline: 2px solid var(--color-border-focus, #3498db);
          outline-offset: 2px;
        }

        @media (min-width: 768px) {
          .menu-toggle {
            display: none;
          }
        }

        .logo {
          display: flex;
          align-items: center;
          gap: 0.75rem;
          text-decoration: none;
          color: var(--color-text-primary, #2c3e50);
          font-weight: 600;
          font-size: 1.125rem;
          cursor: pointer;
          transition: color 0.15s ease-in-out;
        }

        .logo:hover {
          color: var(--color-accent, #3498db);
        }

        .logo:focus-visible {
          outline: 2px solid var(--color-border-focus, #3498db);
          outline-offset: 2px;
          border-radius: 4px;
        }

        .logo-icon {
          font-size: 1.5rem;
          line-height: 1;
        }

        .logo-text {
          display: none;
        }

        @media (min-width: 640px) {
          .logo-text {
            display: inline;
          }
        }

        .search-container {
          flex: 1;
          max-width: 400px;
          position: relative;
        }

        .search-icon {
          position: absolute;
          left: 0.75rem;
          top: 50%;
          transform: translateY(-50%);
          color: var(--color-text-tertiary, #95a5a6);
          font-size: 1rem;
          pointer-events: none;
        }

        input[type="search"] {
          width: 100%;
          padding: 0.5rem 0.75rem 0.5rem 2.5rem;
          border: 1px solid var(--color-border, #e1e4e8);
          border-radius: 8px;
          background: var(--color-surface, #f8f9fa);
          color: var(--color-text-primary, #2c3e50);
          font-family: var(--font-family-base, system-ui, sans-serif);
          font-size: 0.875rem;
          transition: all 0.15s ease-in-out;
        }

        input[type="search"]::placeholder {
          color: var(--color-text-tertiary, #95a5a6);
        }

        input[type="search"]:focus {
          outline: none;
          border-color: var(--color-border-focus, #3498db);
          box-shadow: 0 0 0 3px rgba(52, 152, 219, 0.1);
          background: var(--color-background, #ffffff);
        }

        .header-actions {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          margin-left: auto;
        }
      </style>

      <header class="header-container">
        <button
          class="menu-toggle"
          type="button"
          aria-label="Toggle navigation menu"
          aria-expanded="false"
        >
          ☰
        </button>

        <a class="logo" href="#/" tabindex="0" role="link">
          <span class="logo-icon">🌍</span>
          <span class="logo-text">Terraphim Gallery</span>
        </a>

        <div class="search-container">
          <span class="search-icon">🔍</span>
          <input
            type="search"
            placeholder="Search components..."
            aria-label="Search components"
            autocomplete="off"
          />
        </div>

        <div class="header-actions">
          <theme-toggle></theme-toggle>
        </div>
      </header>
    `;
  }
}

customElements.define('gallery-header', GalleryHeader);
