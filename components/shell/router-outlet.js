import TerraphimElement from '../base/terraphim-element.js';

/**
 * RouterOutlet - Dynamic component container for route rendering
 *
 * Features:
 * - Dynamic component loading based on route
 * - Loading states with spinner
 * - Error states with retry button
 * - Component cleanup on route change
 * - Pass route params/query to components
 * - Keep-alive component caching (optional)
 *
 * @example
 * <router-outlet></router-outlet>
 */
class RouterOutlet extends TerraphimElement {
  constructor() {
    super();
    this.currentComponent = null;
    this.keepAlive = false;
    this.componentCache = new Map();
  }

  static get observedAttributes() {
    return ['keep-alive'];
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (name === 'keep-alive') {
      this.keepAlive = newValue !== null;
    }
  }

  connectedCallback() {
    super.connectedCallback();
    this.render();
    this._setupRouteListener();
  }

  disconnectedCallback() {
    super.disconnectedCallback();
    this._cleanupCurrentComponent();
    window.removeEventListener('route-change-start', this._handleRouteChangeStart);
    window.removeEventListener('route-change', this._handleRouteChange);
    window.removeEventListener('route-change-error', this._handleRouteError);
  }

  /**
   * Set up route change listeners
   * @private
   */
  _setupRouteListener() {
    this._handleRouteChangeStart = this._handleRouteChangeStart.bind(this);
    this._handleRouteChange = this._handleRouteChange.bind(this);
    this._handleRouteError = this._handleRouteError.bind(this);

    window.addEventListener('route-change-start', this._handleRouteChangeStart);
    window.addEventListener('route-change', this._handleRouteChange);
    window.addEventListener('route-change-error', this._handleRouteError);
  }

  /**
   * Handle route change start (show loading)
   * @private
   */
  _handleRouteChangeStart(event) {
    const { to, from } = event.detail;
    console.log('[RouterOutlet] Route changing from', from?.name, 'to', to.name);
    this._showLoading();
  }

  /**
   * Handle route change (render component)
   * @private
   */
  _handleRouteChange(event) {
    const { to, from } = event.detail;
    this._renderComponent(to);
  }

  /**
   * Handle route change error
   * @private
   */
  _handleRouteError(event) {
    const { to, from, error } = event.detail;
    console.error('[RouterOutlet] Route error:', error);
    this._showError(error, to);
  }

  /**
   * Clean up current component
   * @private
   */
  _cleanupCurrentComponent() {
    if (this.currentComponent) {
      const outlet = this.shadowRoot.querySelector('.outlet-content');
      if (outlet && !this.keepAlive) {
        // Remove component from DOM
        while (outlet.firstChild) {
          outlet.removeChild(outlet.firstChild);
        }
      }
      this.currentComponent = null;
    }
  }

  /**
   * Show loading state
   * @private
   */
  _showLoading() {
    const outlet = this.shadowRoot.querySelector('.outlet-content');
    if (!outlet) return;

    outlet.innerHTML = `
      <div class="outlet-loading">
        <div class="spinner"></div>
        <p>Loading...</p>
      </div>
    `;
  }

  /**
   * Show error state
   * @private
   */
  _showError(error, route) {
    const outlet = this.shadowRoot.querySelector('.outlet-content');
    if (!outlet) return;

    outlet.innerHTML = `
      <div class="outlet-error">
        <div class="error-icon">⚠️</div>
        <h2>Failed to load component</h2>
        <p>${error.message || 'Unknown error'}</p>
        <button class="retry-button" data-route="${route.name}">
          Retry
        </button>
      </div>
    `;

    const retryButton = outlet.querySelector('.retry-button');
    retryButton.addEventListener('click', () => {
      window.location.reload();
    });
  }

  /**
   * Render component for route
   * @private
   */
  _renderComponent(route) {
    const outlet = this.shadowRoot.querySelector('.outlet-content');
    if (!outlet) return;

    // Check cache if keep-alive enabled
    if (this.keepAlive && this.componentCache.has(route.component)) {
      const cachedElement = this.componentCache.get(route.component);
      outlet.innerHTML = '';
      outlet.appendChild(cachedElement);
      this.currentComponent = route.component;
      this._updateComponentProps(cachedElement, route);
      return;
    }

    // Clean up previous component
    this._cleanupCurrentComponent();

    // Create new component element
    const componentElement = document.createElement(route.component);

    // Set route data as properties
    this._updateComponentProps(componentElement, route);

    // Render component
    outlet.innerHTML = '';
    outlet.appendChild(componentElement);

    // Cache if keep-alive enabled
    if (this.keepAlive) {
      this.componentCache.set(route.component, componentElement);
    }

    this.currentComponent = route.component;

    console.log('[RouterOutlet] Rendered component:', route.component);
  }

  /**
   * Update component properties with route data
   * @private
   */
  _updateComponentProps(element, route) {
    // Set route params
    if (route.params && Object.keys(route.params).length > 0) {
      element.routeParams = route.params;
      if (typeof element.setAttribute === 'function') {
        element.setAttribute('route-params', JSON.stringify(route.params));
      }
    }

    // Set query params
    if (route.query && Object.keys(route.query).length > 0) {
      element.queryParams = route.query;
      if (typeof element.setAttribute === 'function') {
        element.setAttribute('query-params', JSON.stringify(route.query));
      }
    }

    // Set route meta
    if (route.meta) {
      element.routeMeta = route.meta;
    }
  }

  /**
   * Render component template
   */
  render() {
    this.shadowRoot.innerHTML = `
      ${this.getStyles()}
      <div class="router-outlet">
        <div class="outlet-content">
          <!-- Route component will be rendered here -->
        </div>
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
          height: 100%;
        }

        .router-outlet {
          width: 100%;
          height: 100%;
          position: relative;
        }

        .outlet-content {
          width: 100%;
          height: 100%;
        }

        .outlet-loading,
        .outlet-error {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          height: 100%;
          padding: 2rem;
          text-align: center;
        }

        .spinner {
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

        .outlet-loading p {
          margin-top: 1rem;
          color: #666;
          font-size: 0.9rem;
        }

        .error-icon {
          font-size: 3rem;
          margin-bottom: 1rem;
        }

        .outlet-error h2 {
          margin: 0 0 0.5rem 0;
          color: #f14668;
          font-size: 1.5rem;
        }

        .outlet-error p {
          margin: 0 0 1.5rem 0;
          color: #666;
        }

        .retry-button {
          padding: 0.75rem 1.5rem;
          background: #3273dc;
          color: white;
          border: none;
          border-radius: 4px;
          cursor: pointer;
          font-size: 1rem;
          transition: background 0.2s;
        }

        .retry-button:hover {
          background: #2366d1;
        }

        .retry-button:active {
          transform: translateY(1px);
        }
      </style>
    `;
  }
}

customElements.define('router-outlet', RouterOutlet);

export default RouterOutlet;
