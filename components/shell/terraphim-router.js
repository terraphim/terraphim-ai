/**
 * TerraphimRouter - Vanilla JavaScript routing system with dual-mode support
 *
 * Features:
 * - History API mode for web browsers
 * - Hash mode for Tauri desktop apps
 * - Route pattern matching with parameters (:param, :param?)
 * - Query string parsing
 * - Navigation guards (beforeEach, afterEach, beforeEnter, afterEnter)
 * - Lazy loading support
 * - Browser back/forward button support
 * - Programmatic navigation API
 * - Event-driven architecture
 *
 * @example
 * const router = new TerraphimRouter({
 *   mode: 'history', // 'history' or 'hash'
 *   routes: [
 *     { name: 'home', path: '/', component: 'home-page', lazy: () => import('./home.js') },
 *     { name: 'user', path: '/user/:id', component: 'user-page', lazy: () => import('./user.js') }
 *   ]
 * });
 */

class TerraphimRouter {
  /**
   * @param {Object} options - Router configuration
   * @param {string} options.mode - 'history' or 'hash'
   * @param {Array} options.routes - Route definitions
   * @param {Function} options.beforeEach - Global before navigation guard
   * @param {Function} options.afterEach - Global after navigation guard
   */
  constructor(options = {}) {
    this.mode = options.mode || this._detectMode();
    this.routes = options.routes || [];
    this.currentRoute = null;
    this.guards = {
      beforeEach: options.beforeEach || null,
      afterEach: options.afterEach || null
    };
    this.componentCache = new Map();
    this.initialized = false;

    // Bind methods to preserve context
    this._handlePopState = this._handlePopState.bind(this);
    this._handleHashChange = this._handleHashChange.bind(this);
    this._handleLinkClick = this._handleLinkClick.bind(this);
  }

  /**
   * Detect routing mode based on environment
   * @private
   * @returns {string} 'history' or 'hash'
   */
  _detectMode() {
    // Auto-detect Tauri environment
    if (window.__TAURI__ || window.__TAURI_METADATA__) {
      return 'hash';
    }
    // Use history API if available
    return window.history && window.history.pushState ? 'history' : 'hash';
  }

  /**
   * Initialize router and start listening to navigation events
   */
  init() {
    if (this.initialized) {
      console.warn('[TerraphimRouter] Already initialized');
      return;
    }

    // Set up event listeners
    if (this.mode === 'history') {
      window.addEventListener('popstate', this._handlePopState);
    } else {
      window.addEventListener('hashchange', this._handleHashChange);
    }

    // Intercept link clicks for client-side navigation
    document.addEventListener('click', this._handleLinkClick);

    this.initialized = true;

    // Navigate to current location
    const path = this._getCurrentPath();
    this.navigate(path, { replace: true });

    console.log(`[TerraphimRouter] Initialized in ${this.mode} mode`);
  }

  /**
   * Clean up event listeners
   */
  destroy() {
    if (this.mode === 'history') {
      window.removeEventListener('popstate', this._handlePopState);
    } else {
      window.removeEventListener('hashchange', this._handleHashChange);
    }
    document.removeEventListener('click', this._handleLinkClick);
    this.initialized = false;
    console.log('[TerraphimRouter] Destroyed');
  }

  /**
   * Get current path from URL
   * @private
   * @returns {string} Current path
   */
  _getCurrentPath() {
    if (this.mode === 'history') {
      return window.location.pathname + window.location.search;
    } else {
      const hash = window.location.hash;
      return hash ? hash.slice(1) : '/';
    }
  }

  /**
   * Handle browser back/forward button in history mode
   * @private
   */
  _handlePopState(event) {
    const path = this._getCurrentPath();
    this._resolveRoute(path);
  }

  /**
   * Handle hash change in hash mode
   * @private
   */
  _handleHashChange(event) {
    const path = this._getCurrentPath();
    this._resolveRoute(path);
  }

  /**
   * Intercept link clicks for client-side navigation
   * @private
   */
  _handleLinkClick(event) {
    const link = event.target.closest('a[href]');
    if (!link) return;

    const href = link.getAttribute('href');

    // Ignore external links and special protocols
    if (!href || href.startsWith('http') || href.startsWith('//') || href.startsWith('mailto:')) {
      return;
    }

    // Ignore if modifier keys are pressed (open in new tab)
    if (event.ctrlKey || event.metaKey || event.shiftKey) {
      return;
    }

    // Ignore if target="_blank"
    if (link.hasAttribute('target')) {
      return;
    }

    event.preventDefault();

    // Handle hash mode links
    if (this.mode === 'hash' && !href.startsWith('#')) {
      this.navigate('#' + href);
    } else {
      this.navigate(href);
    }
  }

  /**
   * Navigate to a path
   * @param {string} path - Target path
   * @param {Object} options - Navigation options
   * @param {boolean} options.replace - Replace current history entry
   */
  navigate(path, options = {}) {
    const replace = options.replace || false;

    // Normalize path
    if (this.mode === 'hash' && !path.startsWith('#')) {
      path = '#' + path;
    }

    // Update URL
    if (this.mode === 'history') {
      if (replace) {
        window.history.replaceState(null, '', path);
      } else {
        window.history.pushState(null, '', path);
      }
    } else {
      // Hash mode
      const hashPath = path.startsWith('#') ? path : '#' + path;
      if (replace) {
        window.location.replace(hashPath);
      } else {
        window.location.hash = hashPath.slice(1);
      }
    }

    // Resolve route only if not replace (hashchange will trigger for hash mode)
    if (this.mode === 'history' || replace) {
      this._resolveRoute(this._getCurrentPath());
    }
  }

  /**
   * Match path to route pattern
   * @private
   * @param {string} pattern - Route pattern (e.g., '/user/:id')
   * @param {string} path - Actual path (e.g., '/user/123')
   * @returns {Object|null} Match result with params or null
   */
  _matchRoute(pattern, path) {
    // Remove query string from path
    const [pathname] = path.split('?');

    // Exact match
    if (pattern === pathname) {
      return { params: {}, pattern };
    }

    // Wildcard match
    if (pattern === '*') {
      return { params: {}, pattern };
    }

    // Pattern match with parameters
    const patternParts = pattern.split('/');
    const pathParts = pathname.split('/');

    // Different number of parts (unless optional params)
    if (patternParts.length !== pathParts.length) {
      // Check if last part is optional param
      const lastPart = patternParts[patternParts.length - 1];
      if (lastPart && lastPart.startsWith(':') && lastPart.endsWith('?')) {
        // Optional param missing is OK
        if (patternParts.length === pathParts.length + 1) {
          patternParts.pop(); // Remove optional param
        } else {
          return null;
        }
      } else {
        return null;
      }
    }

    const params = {};

    for (let i = 0; i < patternParts.length; i++) {
      const patternPart = patternParts[i];
      const pathPart = pathParts[i];

      if (patternPart.startsWith(':')) {
        // Parameter
        const paramName = patternPart.slice(1).replace('?', '');
        params[paramName] = pathPart;
      } else if (patternPart !== pathPart) {
        // Literal doesn't match
        return null;
      }
    }

    return { params, pattern };
  }

  /**
   * Parse query string
   * @private
   * @param {string} path - Path with query string
   * @returns {Object} Query parameters
   */
  _parseQuery(path) {
    const [, queryString] = path.split('?');
    if (!queryString) return {};

    const query = {};
    queryString.split('&').forEach(param => {
      const [key, value] = param.split('=');
      query[decodeURIComponent(key)] = value ? decodeURIComponent(value) : '';
    });

    return query;
  }

  /**
   * Resolve route for given path
   * @private
   * @param {string} path - Path to resolve
   */
  async _resolveRoute(path) {
    // Normalize path for hash mode
    if (this.mode === 'hash') {
      path = path.startsWith('#') ? path.slice(1) : path;
    }

    // Find matching route
    let matchedRoute = null;
    let matchResult = null;

    for (const route of this.routes) {
      const result = this._matchRoute(route.path, path);
      if (result) {
        matchedRoute = route;
        matchResult = result;
        break;
      }
    }

    // No match found - try wildcard
    if (!matchedRoute) {
      matchedRoute = this.routes.find(r => r.path === '*');
      matchResult = { params: {}, pattern: '*' };
    }

    if (!matchedRoute) {
      console.error('[TerraphimRouter] No route matched for path:', path);
      return;
    }

    // Parse query parameters
    const query = this._parseQuery(path);

    // Build route context
    const to = {
      name: matchedRoute.name,
      path,
      params: matchResult.params,
      query,
      meta: matchedRoute.meta || {},
      component: matchedRoute.component
    };

    const from = this.currentRoute;

    // Run global beforeEach guard
    if (this.guards.beforeEach) {
      const result = await this.guards.beforeEach(to, from);
      if (result === false) {
        console.log('[TerraphimRouter] Navigation cancelled by beforeEach guard');
        return;
      }
    }

    // Run route-specific beforeEnter guard
    if (matchedRoute.beforeEnter) {
      const result = await matchedRoute.beforeEnter(to, from);
      if (result === false) {
        console.log('[TerraphimRouter] Navigation cancelled by beforeEnter guard');
        return;
      }
    }

    // Emit route-change-start event
    this._emitEvent('route-change-start', { to, from });

    // Load component if lazy
    if (matchedRoute.lazy && !this.componentCache.has(matchedRoute.component)) {
      try {
        await matchedRoute.lazy();
        this.componentCache.set(matchedRoute.component, true);
      } catch (error) {
        console.error('[TerraphimRouter] Failed to load component:', error);
        this._emitEvent('route-change-error', { to, from, error });
        return;
      }
    }

    // Update current route
    this.currentRoute = to;

    // Emit route-change event
    this._emitEvent('route-change', { to, from });

    // Run route-specific afterEnter guard
    if (matchedRoute.afterEnter) {
      await matchedRoute.afterEnter(to, from);
    }

    // Run global afterEach guard
    if (this.guards.afterEach) {
      await this.guards.afterEach(to, from);
    }

    // Update page title
    if (to.meta.title) {
      document.title = to.meta.title;
    }

    console.log('[TerraphimRouter] Navigated to:', to.name, to.path);
  }

  /**
   * Emit custom event
   * @private
   * @param {string} eventName - Event name
   * @param {Object} detail - Event detail
   */
  _emitEvent(eventName, detail) {
    const event = new CustomEvent(eventName, {
      bubbles: true,
      composed: true,
      detail
    });
    window.dispatchEvent(event);
  }

  /**
   * Get route by name
   * @param {string} name - Route name
   * @returns {Object|null} Route definition or null
   */
  getRoute(name) {
    return this.routes.find(r => r.name === name) || null;
  }

  /**
   * Build path for named route with params
   * @param {string} name - Route name
   * @param {Object} params - Route parameters
   * @param {Object} query - Query parameters
   * @returns {string} Built path
   */
  buildPath(name, params = {}, query = {}) {
    const route = this.getRoute(name);
    if (!route) {
      console.error('[TerraphimRouter] Route not found:', name);
      return '/';
    }

    let path = route.path;

    // Replace parameters
    Object.keys(params).forEach(key => {
      path = path.replace(`:${key}`, params[key]);
      path = path.replace(`:${key}?`, params[key]);
    });

    // Remove optional parameters that weren't provided
    path = path.replace(/\/:[^/]+\?/g, '');

    // Add query string
    const queryString = Object.keys(query)
      .map(key => `${encodeURIComponent(key)}=${encodeURIComponent(query[key])}`)
      .join('&');

    if (queryString) {
      path += '?' + queryString;
    }

    return path;
  }

  /**
   * Navigate to named route
   * @param {string} name - Route name
   * @param {Object} params - Route parameters
   * @param {Object} query - Query parameters
   * @param {Object} options - Navigation options
   */
  push(name, params = {}, query = {}, options = {}) {
    const path = this.buildPath(name, params, query);
    this.navigate(path, options);
  }

  /**
   * Replace current route with named route
   * @param {string} name - Route name
   * @param {Object} params - Route parameters
   * @param {Object} query - Query parameters
   */
  replace(name, params = {}, query = {}) {
    this.push(name, params, query, { replace: true });
  }

  /**
   * Go back in history
   */
  back() {
    window.history.back();
  }

  /**
   * Go forward in history
   */
  forward() {
    window.history.forward();
  }

  /**
   * Check if path matches current route
   * @param {string} path - Path to check
   * @param {boolean} exact - Exact match or prefix match
   * @returns {boolean} True if matches
   */
  isActive(path, exact = false) {
    if (!this.currentRoute) return false;

    const currentPath = this.currentRoute.path;

    if (exact) {
      return currentPath === path;
    } else {
      return currentPath.startsWith(path);
    }
  }
}

// Export as global singleton
window.TerraphimRouter = TerraphimRouter;

export default TerraphimRouter;
