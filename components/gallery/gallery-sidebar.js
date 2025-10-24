/**
 * Gallery Sidebar Component
 * Left navigation panel with categories and component links
 */

class GallerySidebar extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this.navData = null;
    this.currentPath = '/';
  }

  connectedCallback() {
    this.render();
    this.loadNavigation();
  }

  /**
   * Load navigation data from JSON
   */
  async loadNavigation() {
    try {
      const response = await fetch('./data/nav-structure.json');
      this.navData = await response.json();
      this.renderNavigation();
    } catch (error) {
      console.error('Failed to load navigation data:', error);
      this.renderError();
    }
  }

  /**
   * Update active navigation item
   * @param {string} path - Current route path
   */
  updateActivePath(path) {
    this.currentPath = path;

    // Update all nav items
    const items = this.shadowRoot.querySelectorAll('nav-item');
    items.forEach(item => {
      if (item.path === path) {
        item.setAttribute('active', '');
      } else {
        item.removeAttribute('active');
      }
    });

    // Expand category containing active item
    this.expandActiveCategory(path);
  }

  /**
   * Expand category containing the active path
   * @param {string} path - Current route path
   */
  expandActiveCategory(path) {
    if (!this.navData) return;

    this.navData.navigation.forEach(category => {
      const hasActiveItem = category.items?.some(item => item.path === path);
      const categoryElement = this.shadowRoot.querySelector(`nav-category[label="${category.label}"]`);

      if (categoryElement && hasActiveItem) {
        categoryElement.setAttribute('expanded', '');
      }
    });
  }

  /**
   * Render navigation structure
   */
  renderNavigation() {
    if (!this.navData) return;

    const nav = this.shadowRoot.querySelector('nav');
    const navHTML = this.navData.navigation.map(category => {
      const items = category.items?.map(item => `
        <nav-item
          label="${item.label}"
          path="${item.path}"
          ${this.currentPath === item.path ? 'active' : ''}
        ></nav-item>
      `).join('') || '';

      return `
        <nav-category
          label="${category.label}"
          icon="${category.icon || ''}"
          ${category.expanded !== false ? 'expanded' : ''}
        >
          ${items}
        </nav-category>
      `;
    }).join('');

    nav.innerHTML = navHTML;
  }

  /**
   * Render error state
   */
  renderError() {
    const nav = this.shadowRoot.querySelector('nav');
    nav.innerHTML = `
      <div style="padding: 1rem; color: var(--color-error, #e74c3c); font-size: 0.875rem;">
        Failed to load navigation
      </div>
    `;
  }

  /**
   * Render component template
   */
  render() {
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: block;
          background: var(--color-surface-elevated, #ffffff);
          border-right: 1px solid var(--color-border, #e1e4e8);
          overflow-y: auto;
          height: 100%;
        }

        .sidebar-container {
          display: flex;
          flex-direction: column;
          height: 100%;
          padding: 1rem;
        }

        .sidebar-header {
          padding: 0.5rem 1rem;
          margin-bottom: 1rem;
        }

        .sidebar-title {
          font-size: 0.75rem;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          color: var(--color-text-tertiary, #95a5a6);
          margin: 0;
        }

        nav {
          flex: 1;
          overflow-y: auto;
        }

        /* Scrollbar styling for sidebar */
        nav::-webkit-scrollbar {
          width: 6px;
        }

        nav::-webkit-scrollbar-track {
          background: transparent;
        }

        nav::-webkit-scrollbar-thumb {
          background: var(--color-border, #e1e4e8);
          border-radius: 3px;
        }

        nav::-webkit-scrollbar-thumb:hover {
          background: var(--color-text-tertiary, #95a5a6);
        }

        .sidebar-footer {
          padding: 1rem;
          margin-top: auto;
          border-top: 1px solid var(--color-border, #e1e4e8);
          font-size: 0.75rem;
          color: var(--color-text-tertiary, #95a5a6);
          text-align: center;
        }

        /* Mobile styles */
        @media (max-width: 767px) {
          :host {
            position: fixed;
            top: var(--header-height, 64px);
            left: -100%;
            width: 80%;
            max-width: 320px;
            height: calc(100vh - var(--header-height, 64px));
            z-index: var(--z-fixed, 300);
            transition: left 0.25s ease-in-out;
            box-shadow: var(--shadow-xl);
          }

          :host([data-open="true"]) {
            left: 0;
          }
        }
      </style>

      <div class="sidebar-container">
        <div class="sidebar-header">
          <h2 class="sidebar-title">Components</h2>
        </div>

        <nav aria-label="Component navigation">
          <!-- Navigation items will be inserted here -->
        </nav>

        <div class="sidebar-footer">
          Terraphim Web Components
        </div>
      </div>
    `;
  }
}

customElements.define('gallery-sidebar', GallerySidebar);
