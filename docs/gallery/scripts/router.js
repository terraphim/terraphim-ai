/**
 * Simple Hash-based Router
 * Handles navigation without page reloads
 */

class Router {
  constructor() {
    this.routes = new Map();
    this.currentRoute = null;
    this.listeners = new Set();

    // Listen for hash changes
    window.addEventListener('hashchange', () => this.handleRouteChange());

    // Handle initial route
    this.handleRouteChange();
  }

  /**
   * Register a route handler
   * @param {string} path - Route path (e.g., "/components/button")
   * @param {Function} handler - Function to call when route matches
   */
  register(path, handler) {
    this.routes.set(path, handler);
  }

  /**
   * Navigate to a specific route
   * @param {string} path - Route path to navigate to
   */
  navigate(path) {
    window.location.hash = path;
  }

  /**
   * Get current route path from hash
   * @returns {string} Current route path
   */
  getCurrentPath() {
    const hash = window.location.hash.slice(1) || '/';
    return hash;
  }

  /**
   * Handle route change event
   */
  handleRouteChange() {
    const path = this.getCurrentPath();
    const previousRoute = this.currentRoute;
    this.currentRoute = path;

    // Find matching route handler
    const handler = this.routes.get(path);

    if (handler) {
      handler(path, previousRoute);
    } else {
      // Try to find a default or fallback handler
      const defaultHandler = this.routes.get('*');
      if (defaultHandler) {
        defaultHandler(path, previousRoute);
      }
    }

    // Notify all listeners
    this.notifyListeners(path, previousRoute);
  }

  /**
   * Add a route change listener
   * @param {Function} listener - Callback function (currentPath, previousPath)
   */
  addListener(listener) {
    this.listeners.add(listener);
  }

  /**
   * Remove a route change listener
   * @param {Function} listener - Callback function to remove
   */
  removeListener(listener) {
    this.listeners.delete(listener);
  }

  /**
   * Notify all listeners of route change
   * @param {string} currentPath - Current route path
   * @param {string} previousPath - Previous route path
   */
  notifyListeners(currentPath, previousPath) {
    this.listeners.forEach(listener => {
      try {
        listener(currentPath, previousPath);
      } catch (error) {
        console.error('Error in route listener:', error);
      }
    });
  }

  /**
   * Parse route path into segments
   * @param {string} path - Route path
   * @returns {Object} Parsed route object
   */
  parsePath(path) {
    const segments = path.split('/').filter(s => s);

    return {
      full: path,
      segments,
      category: segments[0] || null,
      component: segments[1] || null,
      section: segments[2] || null
    };
  }

  /**
   * Build a route path from parts
   * @param {string} category - Category name
   * @param {string} component - Component name (optional)
   * @param {string} section - Section name (optional)
   * @returns {string} Complete route path
   */
  buildPath(category, component = null, section = null) {
    const parts = [category];
    if (component) parts.push(component);
    if (section) parts.push(section);
    return '/' + parts.join('/');
  }
}

// Export singleton instance
export const router = new Router();
