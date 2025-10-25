import TerraphimElement from '../base/terraphim-element.js';
import TerraphimRouter from './terraphim-router.js';
import './terraphim-nav.js';
import './router-outlet.js';

/**
 * TerraphimApp - Main application container
 *
 * Features:
 * - Application shell with header, main content, footer
 * - Router integration and initialization
 * - Route change handling
 * - Lazy component loading
 * - Mobile menu management
 * - Loading states during transitions
 * - Page title updates
 *
 * @example
 * <terraphim-app routes='[...]'></terraphim-app>
 *
 * @fires app-ready - When application is initialized
 * @fires route-changed - When route changes
 */
class TerraphimApp extends TerraphimElement {
  constructor() {
    super();
    this.router = null;
    this.routes = [];
    this.routerMode = 'auto'; // 'auto', 'history', or 'hash'
    this.mobileMenuOpen = false;
  }

  static get observedAttributes() {
    return ['routes', 'router-mode'];
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (name === 'routes' && newValue) {
      try {
        this.routes = JSON.parse(newValue);
      } catch (error) {
        console.error('[TerraphimApp] Failed to parse routes:', error);
      }
    } else if (name === 'router-mode') {
      this.routerMode = newValue || 'auto';
    }
  }

  connectedCallback() {
    super.connectedCallback();
    this.render();
    this._initializeRouter();
    this._setupEventListeners();
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    if (this.router) {
      this.router.destroy();
    }
  }

  /**
   * Initialize router with routes
   * @private
   */
  _initializeRouter() {
    if (this.routes.length === 0) {
      console.warn('[TerraphimApp] No routes configured');
      return;
    }

    // Determine router mode
    let mode = this.routerMode;
    if (mode === 'auto') {
      // Auto-detect based on environment
      mode = window.__TAURI__ || window.__TAURI_METADATA__ ? 'hash' : 'history';
    }

    // Create router instance
    this.router = new TerraphimRouter({
      mode,
      routes: this.routes,
      beforeEach: this._beforeRouteChange.bind(this),
      afterEach: this._afterRouteChange.bind(this)
    });

    // Make router globally available
    window.terraphimRouter = this.router;

    // Initialize router
    this.router.init();

    console.log('[TerraphimApp] Router initialized in', mode, 'mode');

    // Emit app-ready event
    this.emit('app-ready', { router: this.router });
  }

  /**
   * Before route change guard
   * @private
   */
  async _beforeRouteChange(to, from) {
    console.log('[TerraphimApp] Before route change:', from?.name, '->', to.name);

    // Close mobile menu if open
    if (this.mobileMenuOpen) {
      this._toggleMobileMenu();
    }

    // Could add authentication checks here
    // if (to.meta.requiresAuth && !isAuthenticated()) {
    //   router.navigate('/login');
    //   return false;
    // }

    return true; // Allow navigation
  }

  /**
   * After route change hook
   * @private
   */
  async _afterRouteChange(to, from) {
    console.log('[TerraphimApp] After route change:', from?.name, '->', to.name);

    // Update page title
    const title = to.meta.title ? `${to.meta.title} - Terraphim AI` : 'Terraphim AI';
    document.title = title;

    // Scroll to top
    window.scrollTo(0, 0);

    // Emit custom event
    this.emit('route-changed', { to, from });

    // Track analytics (if configured)
    if (window.plausible) {
      window.plausible('pageview', { props: { path: to.path } });
    }
  }

  /**
   * Set up event listeners
   * @private
   */
  _setupEventListeners() {
    // Mobile menu toggle
    const menuButton = this.shadowRoot.querySelector('.mobile-menu-button');
    if (menuButton) {
      menuButton.addEventListener('click', this._toggleMobileMenu.bind(this));
    }

    // Logo/back button
    const logoButton = this.shadowRoot.querySelector('.logo-back-button');
    if (logoButton) {
      logoButton.addEventListener('click', this._handleLogoClick.bind(this));
    }
  }

  /**
   * Toggle mobile menu
   * @private
   */
  _toggleMobileMenu() {
    this.mobileMenuOpen = !this.mobileMenuOpen;
    const nav = this.shadowRoot.querySelector('terraphim-nav');
    if (nav) {
      nav.style.display = this.mobileMenuOpen ? 'block' : '';
    }
    const menuButton = this.shadowRoot.querySelector('.mobile-menu-button');
    if (menuButton) {
      menuButton.setAttribute('aria-expanded', this.mobileMenuOpen);
      const icon = menuButton.querySelector('i');
      if (icon) {
        icon.className = this.mobileMenuOpen ? 'fas fa-times' : 'fas fa-bars';
      }
    }
  }

  /**
   * Handle logo/back button click
   * @private
   */
  _handleLogoClick(event) {
    event.preventDefault();
    if (this.router) {
      this.router.navigate('/');
    }
  }

  /**
   * Set routes programmatically
   * @param {Array} routes - Route definitions
   */
  setRoutes(routes) {
    this.routes = routes;
    if (this.router) {
      this.router.destroy();
    }
    this._initializeRouter();
  }

  /**
   * Get current route
   * @returns {Object} Current route object
   */
  getCurrentRoute() {
    return this.router ? this.router.currentRoute : null;
  }

  /**
   * Navigate to path
   * @param {string} path - Target path
   * @param {Object} options - Navigation options
   */
  navigate(path, options = {}) {
    if (this.router) {
      this.router.navigate(path, options);
    }
  }

  /**
   * Render component template
   */
  render() {
    this.shadowRoot.innerHTML = `
      ${this.getStyles()}
      <div class="app-shell">
        <header class="app-header">
          <div class="header-left">
            <button class="logo-back-button" aria-label="Home" title="Home">
              <span class="icon">
                <i class="fas fa-brain"></i>
              </span>
              <span class="logo-text">Terraphim AI</span>
            </button>
          </div>
          <div class="header-center">
            <terraphim-nav></terraphim-nav>
          </div>
          <div class="header-right">
            <button class="mobile-menu-button" aria-label="Toggle menu" aria-expanded="false">
              <span class="icon">
                <i class="fas fa-bars"></i>
              </span>
            </button>
          </div>
        </header>

        <main class="app-main">
          <router-outlet></router-outlet>
        </main>

        <footer class="app-footer">
          <nav class="footer-nav">
            <span class="footer-text">Terraphim AI - Privacy-first AI Assistant</span>
            <span class="footer-links">
              <a href="https://github.com/terraphim/terraphim-ai" target="_blank" rel="noopener">
                <i class="fab fa-github"></i> GitHub
              </a>
            </span>
          </nav>
        </footer>
      </div>
    `;
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
          height: 100vh;
          overflow: hidden;
        }

        .app-shell {
          display: flex;
          flex-direction: column;
          height: 100%;
          background: var(--app-background, #f5f5f5);
        }

        .app-header {
          display: flex;
          align-items: center;
          background: var(--header-background, #fff);
          border-bottom: 1px solid var(--border-color, #dbdbdb);
          padding: 0.5rem 1rem;
          flex-shrink: 0;
          position: relative;
          z-index: 100;
        }

        .header-left {
          flex-shrink: 0;
        }

        .header-center {
          flex-grow: 1;
          display: flex;
          justify-content: center;
          overflow: hidden;
        }

        .header-right {
          flex-shrink: 0;
          display: none; /* Hidden by default, shown on mobile */
        }

        .logo-back-button {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          background: none;
          border: none;
          cursor: pointer;
          padding: 0.5rem;
          color: var(--primary-color, #3273dc);
          font-size: 1.25rem;
          font-weight: 600;
          transition: opacity 0.2s;
        }

        .logo-back-button:hover {
          opacity: 0.8;
        }

        .logo-back-button .icon {
          font-size: 1.5rem;
        }

        .mobile-menu-button {
          display: flex;
          align-items: center;
          justify-content: center;
          background: none;
          border: none;
          cursor: pointer;
          padding: 0.5rem;
          color: var(--text-color, #4a4a4a);
          font-size: 1.25rem;
        }

        .app-main {
          flex: 1;
          overflow-y: auto;
          overflow-x: hidden;
          position: relative;
        }

        .app-footer {
          flex-shrink: 0;
          background: var(--footer-background, #fff);
          border-top: 1px solid var(--border-color, #dbdbdb);
          padding: 1rem;
        }

        .footer-nav {
          display: flex;
          justify-content: space-between;
          align-items: center;
          max-width: 1200px;
          margin: 0 auto;
          font-size: 0.875rem;
          color: var(--text-muted, #7a7a7a);
        }

        .footer-text {
          display: flex;
          align-items: center;
        }

        .footer-links {
          display: flex;
          gap: 1rem;
        }

        .footer-links a {
          color: var(--text-muted, #7a7a7a);
          text-decoration: none;
          transition: color 0.2s;
        }

        .footer-links a:hover {
          color: var(--primary-color, #3273dc);
        }

        .icon {
          display: inline-flex;
          align-items: center;
          justify-content: center;
        }

        /* Mobile responsive */
        @media screen and (max-width: 768px) {
          .app-header {
            flex-wrap: wrap;
          }

          .header-right {
            display: block;
          }

          .header-center {
            order: 3;
            width: 100%;
            margin-top: 0.5rem;
            display: none; /* Hidden by default on mobile */
          }

          .header-center[data-mobile-open="true"] {
            display: flex;
          }

          .logo-text {
            display: none;
          }

          .footer-nav {
            flex-direction: column;
            gap: 0.5rem;
            text-align: center;
          }
        }

        /* Loading overlay */
        .loading-overlay {
          position: absolute;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          background: rgba(255, 255, 255, 0.9);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 1000;
        }

        .loading-overlay .spinner {
          border: 4px solid rgba(0, 0, 0, 0.1);
          border-left-color: #3273dc;
          border-radius: 50%;
          width: 40px;
          height: 40px;
          animation: spin 1s linear infinite;
        }

        @keyframes spin {
          to { transform: rotate(360deg); }
        }

        /* Dark theme support */
        @media (prefers-color-scheme: dark) {
          .app-shell {
            --app-background: #1a1a1a;
            --header-background: #2a2a2a;
            --footer-background: #2a2a2a;
            --border-color: #3a3a3a;
            --text-color: #e0e0e0;
            --text-muted: #a0a0a0;
          }
        }
      </style>
    `;
  }
}

customElements.define('terraphim-app', TerraphimApp);

export default TerraphimApp;
