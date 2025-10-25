import TerraphimElement from '../base/terraphim-element.js';

/**
 * TerraphimNav - Navigation component with active link highlighting
 *
 * Features:
 * - Horizontal tab-based navigation
 * - Active link detection (exact and prefix matching)
 * - Keyboard shortcuts (Ctrl+1, Ctrl+2, etc.)
 * - Mobile responsive (icon-only on small screens)
 * - Badge notifications support
 * - ARIA accessibility
 *
 * @example
 * <terraphim-nav></terraphim-nav>
 *
 * @fires nav-item-click - When navigation item is clicked
 */
class TerraphimNav extends TerraphimElement {
  constructor() {
    super();
    this.navItems = [];
    this.currentPath = '/';
    this.mobileMenuOpen = false;
  }

  connectedCallback() {
    super.connectedCallback();
    this._loadNavItems();
    this.render();
    this._setupEventListeners();
    this._setupKeyboardShortcuts();
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    this._removeKeyboardShortcuts();
    window.removeEventListener('route-change', this._handleRouteChange);
  }

  /**
   * Load navigation items from router configuration
   * @private
   */
  _loadNavItems() {
    // Get router instance from window
    const router = window.terraphimRouter;
    if (!router || !router.routes) {
      console.warn('[TerraphimNav] Router not available');
      return;
    }

    // Filter routes that should appear in navigation
    this.navItems = router.routes
      .filter(route => route.meta && route.meta.title && route.path !== '*')
      .map(route => ({
        name: route.name,
        path: route.path,
        title: route.meta.title,
        icon: route.meta.icon || 'fas fa-circle',
        badge: route.meta.badge || null,
        exact: route.meta.exact !== false // Default to exact matching
      }));

    console.log('[TerraphimNav] Loaded nav items:', this.navItems);
  }

  /**
   * Set up event listeners
   * @private
   */
  _setupEventListeners() {
    // Listen for route changes
    this._handleRouteChange = this._handleRouteChange.bind(this);
    window.addEventListener('route-change', this._handleRouteChange);

    // Listen for nav item clicks
    const nav = this.shadowRoot.querySelector('.nav-tabs');
    if (nav) {
      nav.addEventListener('click', this._handleNavClick.bind(this));
    }
  }

  /**
   * Handle route change event
   * @private
   */
  _handleRouteChange(event) {
    const { to } = event.detail;
    this.currentPath = to.path;
    this._updateActiveLinks();
  }

  /**
   * Handle navigation item click
   * @private
   */
  _handleNavClick(event) {
    event.preventDefault();

    const link = event.target.closest('a');
    if (!link) return;

    const href = link.getAttribute('href');
    if (!href) return;

    // Get router instance
    const router = window.terraphimRouter;
    if (router) {
      router.navigate(href);
    }

    // Emit custom event
    this.emit('nav-item-click', { href });
  }

  /**
   * Update active link styles
   * @private
   */
  _updateActiveLinks() {
    const links = this.shadowRoot.querySelectorAll('.nav-tabs a');
    const router = window.terraphimRouter;

    links.forEach(link => {
      const href = link.getAttribute('href');
      const exact = link.dataset.exact === 'true';

      let isActive = false;

      if (router) {
        // Use router's isActive method
        isActive = router.isActive(href, exact);
      } else {
        // Fallback to simple path matching
        if (exact) {
          isActive = this.currentPath === href;
        } else {
          isActive = this.currentPath.startsWith(href);
        }
      }

      if (isActive) {
        link.classList.add('is-active');
        link.setAttribute('aria-current', 'page');
      } else {
        link.classList.remove('is-active');
        link.removeAttribute('aria-current');
      }
    });
  }

  /**
   * Set up keyboard shortcuts
   * @private
   */
  _setupKeyboardShortcuts() {
    this._handleKeyDown = this._handleKeyDown.bind(this);
    document.addEventListener('keydown', this._handleKeyDown);
  }

  /**
   * Remove keyboard shortcuts
   * @private
   */
  _removeKeyboardShortcuts() {
    if (this._handleKeyDown) {
      document.removeEventListener('keydown', this._handleKeyDown);
    }
  }

  /**
   * Handle keyboard shortcut
   * @private
   */
  _handleKeyDown(event) {
    // Check for Ctrl+Number (or Cmd+Number on Mac)
    if ((event.ctrlKey || event.metaKey) && event.key >= '1' && event.key <= '9') {
      event.preventDefault();

      const index = parseInt(event.key) - 1;
      if (index < this.navItems.length) {
        const item = this.navItems[index];
        const router = window.terraphimRouter;
        if (router) {
          router.navigate(item.path);
        }
      }
    }
  }

  /**
   * Render component template
   */
  render() {
    const navItemsHTML = this.navItems.map((item, index) => {
      const shortcut = index + 1;
      const badge = item.badge ? `<span class="tag is-info">${item.badge}</span>` : '';

      return `
        <li>
          <a
            href="${item.path}"
            data-exact="${item.exact}"
            title="${item.title} (Ctrl+${shortcut})"
            aria-label="${item.title}"
          >
            <span class="icon is-small">
              <i class="${item.icon}" aria-hidden="true"></i>
            </span>
            <span class="nav-text">${item.title}</span>
            ${badge}
          </a>
        </li>
      `;
    }).join('');

    this.shadowRoot.innerHTML = `
      ${this.getStyles()}
      <nav class="terraphim-nav" role="navigation" aria-label="Main navigation">
        <div class="tabs is-boxed">
          <ul class="nav-tabs">
            ${navItemsHTML}
          </ul>
        </div>
      </nav>
    `;

    // Update active links after render
    requestAnimationFrame(() => {
      this._updateActiveLinks();
    });
  }

  /**
   * Component styles
   */
  getStyles() {
    return `
      <style>
        :host {
          display: block;
          width: 100%;
        }

        .terraphim-nav {
          background: var(--nav-background, #fff);
          border-bottom: 1px solid var(--border-color, #dbdbdb);
        }

        .tabs {
          margin: 0;
          padding: 0;
          overflow-x: auto;
          -webkit-overflow-scrolling: touch;
        }

        .tabs.is-boxed {
          align-items: stretch;
          display: flex;
          justify-content: flex-start;
        }

        .nav-tabs {
          display: flex;
          list-style: none;
          margin: 0;
          padding: 0;
          border-bottom-color: #dbdbdb;
          border-bottom-style: solid;
          border-bottom-width: 1px;
        }

        .nav-tabs li {
          display: block;
          flex-shrink: 0;
        }

        .nav-tabs a {
          align-items: center;
          border-bottom-color: #dbdbdb;
          border-bottom-style: solid;
          border-bottom-width: 1px;
          color: #4a4a4a;
          display: flex;
          justify-content: center;
          margin-bottom: -1px;
          padding: 0.75rem 1rem;
          vertical-align: top;
          text-decoration: none;
          transition: all 0.2s ease;
          position: relative;
          white-space: nowrap;
        }

        .nav-tabs a:hover {
          background-color: #f5f5f5;
          border-bottom-color: #b5b5b5;
        }

        .nav-tabs a.is-active {
          background-color: var(--nav-active-bg, #fff);
          border-color: #dbdbdb;
          border-bottom-color: transparent;
          color: var(--nav-active-color, #3273dc);
          font-weight: 600;
        }

        .nav-tabs a.is-active::after {
          content: '';
          position: absolute;
          bottom: -1px;
          left: 0;
          right: 0;
          height: 3px;
          background: var(--nav-active-color, #3273dc);
        }

        .icon {
          align-items: center;
          display: inline-flex;
          justify-content: center;
          height: 1.5rem;
          width: 1.5rem;
        }

        .icon.is-small {
          height: 1rem;
          width: 1rem;
        }

        .nav-text {
          margin-left: 0.5rem;
        }

        .tag {
          display: inline-flex;
          align-items: center;
          background-color: #f5f5f5;
          border-radius: 4px;
          color: #4a4a4a;
          font-size: 0.75rem;
          height: 1.5rem;
          justify-content: center;
          line-height: 1.5;
          padding-left: 0.5rem;
          padding-right: 0.5rem;
          white-space: nowrap;
          margin-left: 0.5rem;
        }

        .tag.is-info {
          background-color: #3273dc;
          color: #fff;
        }

        /* Mobile responsive */
        @media screen and (max-width: 768px) {
          .nav-tabs a {
            padding: 0.75rem 0.5rem;
          }

          .nav-text {
            display: none;
          }

          .icon {
            margin: 0;
          }

          .tag {
            position: absolute;
            top: 0.25rem;
            right: 0.25rem;
            font-size: 0.625rem;
            height: 1rem;
            padding: 0 0.25rem;
            min-width: 1rem;
          }
        }

        /* Focus styles for accessibility */
        .nav-tabs a:focus {
          outline: 2px solid #3273dc;
          outline-offset: 2px;
        }

        /* Scrollbar styling for horizontal scroll */
        .tabs::-webkit-scrollbar {
          height: 4px;
        }

        .tabs::-webkit-scrollbar-track {
          background: #f1f1f1;
        }

        .tabs::-webkit-scrollbar-thumb {
          background: #888;
          border-radius: 2px;
        }

        .tabs::-webkit-scrollbar-thumb:hover {
          background: #555;
        }
      </style>
    `;
  }
}

customElements.define('terraphim-nav', TerraphimNav);

export default TerraphimNav;
