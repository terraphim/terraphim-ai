/**
 * @fileoverview TerraphimState - Vanilla JavaScript state management system
 * Provides reactive state management with path-based subscriptions, persistence, and debugging tools
 */

/**
 * TerraphimState - Core state management class
 * Features:
 * - Path-based get/set with dot notation (e.g., "config.haystacks.0.name")
 * - Wildcard subscriptions (e.g., "config.haystacks.*")
 * - Parent path notifications (changing "user.name" notifies "user" subscribers)
 * - Batch updates to minimize notifications
 * - localStorage persistence with debouncing
 * - Debugging tools (snapshots, time travel)
 * - Middleware/plugin system
 *
 * @class
 * @extends EventTarget
 *
 * @example
 * const state = new TerraphimState({
 *   theme: 'spacelab',
 *   user: { name: 'Alex', role: 'admin' }
 * });
 *
 * // Subscribe to changes
 * state.subscribe('user.name', (value) => {
 *   console.log('Name changed:', value);
 * });
 *
 * // Update state
 * state.set('user.name', 'Bob');
 *
 * // Batch updates
 * state.batch(() => {
 *   state.set('user.name', 'Charlie');
 *   state.set('user.role', 'user');
 * });
 */
export class TerraphimState extends EventTarget {
  /**
   * Create a new TerraphimState instance
   * @param {Object} [initialState={}] - Initial state object
   * @param {Object} [options={}] - Configuration options
   * @param {boolean} [options.persist=false] - Enable localStorage persistence
   * @param {string} [options.storagePrefix='terraphim'] - localStorage key prefix
   * @param {number} [options.persistDebounce=500] - Debounce delay for persistence (ms)
   * @param {boolean} [options.debug=false] - Enable debugging mode
   */
  constructor(initialState = {}, options = {}) {
    super();

    /**
     * Internal state storage
     * @private
     */
    this._state = this._deepClone(initialState);

    /**
     * Configuration options
     * @private
     */
    this._options = {
      persist: false,
      storagePrefix: 'terraphim',
      persistDebounce: 500,
      debug: false,
      ...options
    };

    /**
     * Map of path -> Set of subscription handlers
     * @private
     * @type {Map<string, Set<Object>>}
     */
    this._subscriptions = new Map();

    /**
     * Batch update state
     * @private
     */
    this._batching = false;
    this._batchedNotifications = new Set();

    /**
     * Persistence debounce timer
     * @private
     */
    this._persistTimer = null;

    /**
     * Middleware functions
     * @private
     * @type {Function[]}
     */
    this._middleware = [];

    /**
     * History for time travel debugging
     * @private
     * @type {Array<{state: Object, timestamp: number}>}
     */
    this._history = [];
    this._historyIndex = -1;
    this._maxHistorySize = 50;

    // Load persisted state if enabled
    if (this._options.persist) {
      this._loadFromStorage();
    }

    // Save initial snapshot for debugging
    if (this._options.debug) {
      this._saveSnapshot();
    }
  }

  /**
   * Get value at path using dot notation
   * @param {string} path - Path to value (e.g., "config.haystacks.0.name")
   * @returns {*} Value at path, or undefined if not found
   *
   * @example
   * state.get('user.name')
   * state.get('config.haystacks.0')
   */
  get(path) {
    if (!path) return this._state;

    const parts = this._parsePath(path);
    let value = this._state;

    for (const part of parts) {
      if (value === null || value === undefined) {
        return undefined;
      }
      value = value[part];
    }

    return value;
  }

  /**
   * Set value at path using dot notation
   * @param {string} path - Path to value (e.g., "user.name")
   * @param {*} value - New value
   * @param {boolean} [silent=false] - If true, skip notifications
   *
   * @example
   * state.set('user.name', 'Alice')
   * state.set('config.haystacks.0.name', 'GitHub')
   */
  set(path, value, silent = false) {
    // Apply middleware
    const middlewareResult = this._applyMiddleware('set', { path, value });
    if (middlewareResult === false) {
      return; // Middleware cancelled the operation
    }

    const oldValue = this.get(path);

    // Perform the update
    if (!path) {
      this._state = this._deepClone(value);
    } else {
      const parts = this._parsePath(path);
      const lastPart = parts.pop();
      let target = this._state;

      // Navigate to parent object, creating intermediate objects as needed
      for (const part of parts) {
        if (!(part in target) || typeof target[part] !== 'object') {
          target[part] = {};
        }
        target = target[part];
      }

      target[lastPart] = value;
    }

    // Save snapshot for debugging
    if (this._options.debug) {
      this._saveSnapshot();
    }

    // Notify subscribers
    if (!silent) {
      this._notify(path, value, oldValue);
    }

    // Persist to storage
    if (this._options.persist) {
      this._debouncePersist();
    }
  }

  /**
   * Subscribe to changes at path (supports wildcards)
   * @param {string} path - Path to watch (e.g., "user.name" or "haystacks.*")
   * @param {Function} callback - Function called when value changes
   * @param {Object} [options={}] - Subscription options
   * @param {boolean} [options.immediate=false] - Call immediately with current value
   * @param {boolean} [options.deep=false] - Watch nested changes
   * @param {boolean} [options.once=false] - Auto-unsubscribe after first call
   * @param {number} [options.debounce=0] - Debounce delay in ms
   * @param {Function} [options.compare] - Custom equality check (a, b) => boolean
   * @param {boolean} [options.useRAF=false] - Defer to requestAnimationFrame
   * @returns {Function} Unsubscribe function
   *
   * @example
   * const unsubscribe = state.subscribe('user.name', (value, oldValue) => {
   *   console.log('Name changed:', value);
   * }, { immediate: true });
   *
   * // Wildcard subscription
   * state.subscribe('config.haystacks.*', (value, oldValue, fullPath) => {
   *   console.log('Haystack changed:', fullPath, value);
   * });
   *
   * // Unsubscribe
   * unsubscribe();
   */
  subscribe(path, callback, options = {}) {
    const subscription = {
      callback,
      options: {
        immediate: false,
        deep: false,
        once: false,
        debounce: 0,
        compare: null,
        useRAF: false,
        ...options
      },
      timer: null,
      rafId: null
    };

    // Add to subscriptions map
    if (!this._subscriptions.has(path)) {
      this._subscriptions.set(path, new Set());
    }
    this._subscriptions.get(path).add(subscription);

    // Call immediately if requested
    if (subscription.options.immediate) {
      const currentValue = this.get(path);
      this._callSubscription(subscription, currentValue, undefined, path);
    }

    // Return unsubscribe function
    return () => {
      if (subscription.timer) {
        clearTimeout(subscription.timer);
      }
      if (subscription.rafId) {
        cancelAnimationFrame(subscription.rafId);
      }
      this._subscriptions.get(path)?.delete(subscription);
      if (this._subscriptions.get(path)?.size === 0) {
        this._subscriptions.delete(path);
      }
    };
  }

  /**
   * Batch multiple updates into a single notification cycle
   * @param {Function} fn - Function containing multiple set() calls
   *
   * @example
   * state.batch(() => {
   *   state.set('user.name', 'Alice');
   *   state.set('user.role', 'admin');
   *   state.set('user.email', 'alice@example.com');
   * }); // Only one notification sent
   */
  batch(fn) {
    this._batching = true;
    this._batchedNotifications.clear();

    try {
      fn();
    } finally {
      this._batching = false;
      this._flushBatchedNotifications();
    }
  }

  /**
   * Use a middleware function
   * Middleware signature: (action, payload) => boolean | void
   * Return false to cancel the operation
   *
   * @param {Function} middleware - Middleware function
   * @returns {Function} Remove middleware function
   *
   * @example
   * state.use((action, payload) => {
   *   console.log('Action:', action, payload);
   *   if (action === 'set' && payload.path === 'admin') {
   *     return false; // Cancel admin changes
   *   }
   * });
   */
  use(middleware) {
    this._middleware.push(middleware);
    return () => {
      const index = this._middleware.indexOf(middleware);
      if (index > -1) {
        this._middleware.splice(index, 1);
      }
    };
  }

  /**
   * Get a snapshot of current state
   * @returns {Object} Deep clone of current state
   */
  getSnapshot() {
    return this._deepClone(this._state);
  }

  /**
   * Restore state from a snapshot
   * @param {Object} snapshot - State snapshot to restore
   */
  restoreSnapshot(snapshot) {
    const oldState = this._state;
    this._state = this._deepClone(snapshot);

    // Notify all subscribers
    this._notifyAll(this._state, oldState);

    if (this._options.persist) {
      this._debouncePersist();
    }
  }

  /**
   * Time travel to previous state (debug mode only)
   * @param {number} [steps=1] - Number of steps to go back
   */
  undo(steps = 1) {
    if (!this._options.debug) {
      console.warn('Time travel requires debug mode enabled');
      return;
    }

    const newIndex = Math.max(0, this._historyIndex - steps);
    if (newIndex !== this._historyIndex) {
      this._historyIndex = newIndex;
      const snapshot = this._history[this._historyIndex];
      if (snapshot) {
        this.restoreSnapshot(snapshot.state);
      }
    }
  }

  /**
   * Time travel to next state (debug mode only)
   * @param {number} [steps=1] - Number of steps to go forward
   */
  redo(steps = 1) {
    if (!this._options.debug) {
      console.warn('Time travel requires debug mode enabled');
      return;
    }

    const newIndex = Math.min(this._history.length - 1, this._historyIndex + steps);
    if (newIndex !== this._historyIndex) {
      this._historyIndex = newIndex;
      const snapshot = this._history[this._historyIndex];
      if (snapshot) {
        this.restoreSnapshot(snapshot.state);
      }
    }
  }

  /**
   * Get state history (debug mode only)
   * @returns {Array<{state: Object, timestamp: number}>}
   */
  getHistory() {
    return this._history;
  }

  /**
   * Clear all state (reset to empty object)
   */
  clear() {
    this.restoreSnapshot({});
  }

  /**
   * Reset state to initial values
   * @param {Object} initialState - New initial state
   */
  reset(initialState = {}) {
    this.restoreSnapshot(initialState);
  }

  /**
   * Parse path string into array of parts
   * Handles dot notation and array indices
   * @private
   * @param {string} path - Path string
   * @returns {Array<string|number>} Array of path parts
   */
  _parsePath(path) {
    if (!path) return [];

    // Cache parsed paths for performance
    if (!this._pathCache) {
      this._pathCache = new Map();
    }

    if (this._pathCache.has(path)) {
      return this._pathCache.get(path);
    }

    const parts = path.split('.').map(part => {
      // Convert array indices to numbers
      const num = parseInt(part, 10);
      return isNaN(num) ? part : num;
    });

    this._pathCache.set(path, parts);
    return parts;
  }

  /**
   * Check if a path matches a subscription pattern (with wildcards)
   * @private
   * @param {string} changedPath - Actual changed path
   * @param {string} subscriptionPath - Subscription pattern (may contain wildcards)
   * @returns {boolean}
   */
  _pathMatches(changedPath, subscriptionPath) {
    if (changedPath === subscriptionPath) return true;

    const changedParts = this._parsePath(changedPath);
    const subscriptionParts = this._parsePath(subscriptionPath);

    // Check if subscription is a parent path
    if (subscriptionParts.length < changedParts.length) {
      return changedParts.slice(0, subscriptionParts.length).every((part, i) => {
        return subscriptionParts[i] === '*' || subscriptionParts[i] === part;
      });
    }

    // Check if subscription has wildcards
    if (subscriptionPath.includes('*')) {
      if (subscriptionParts.length !== changedParts.length) return false;
      return subscriptionParts.every((part, i) => {
        return part === '*' || part === changedParts[i];
      });
    }

    return false;
  }

  /**
   * Get all parent paths for a given path
   * @private
   * @param {string} path - Path to get parents for
   * @returns {string[]} Array of parent paths
   */
  _getParentPaths(path) {
    const parts = this._parsePath(path);
    const parents = [];

    for (let i = parts.length - 1; i > 0; i--) {
      parents.push(parts.slice(0, i).join('.'));
    }

    return parents;
  }

  /**
   * Notify subscribers of a change
   * @private
   * @param {string} path - Changed path
   * @param {*} newValue - New value
   * @param {*} oldValue - Old value
   */
  _notify(path, newValue, oldValue) {
    if (this._batching) {
      this._batchedNotifications.add(path);
      return;
    }

    // Find all matching subscriptions
    const toNotify = [];

    for (const [subscriptionPath, subscriptions] of this._subscriptions) {
      if (this._pathMatches(path, subscriptionPath)) {
        subscriptions.forEach(subscription => {
          toNotify.push({ subscription, path: subscriptionPath, actualPath: path });
        });
      }
    }

    // Also notify parent paths
    const parentPaths = this._getParentPaths(path);
    for (const parentPath of parentPaths) {
      const subscriptions = this._subscriptions.get(parentPath);
      if (subscriptions) {
        subscriptions.forEach(subscription => {
          if (subscription.options.deep) {
            toNotify.push({ subscription, path: parentPath, actualPath: path });
          }
        });
      }
    }

    // Call all matching subscriptions
    toNotify.forEach(({ subscription, path: subscriptionPath, actualPath }) => {
      this._callSubscription(subscription, newValue, oldValue, actualPath);

      // Handle once option
      if (subscription.options.once) {
        this._subscriptions.get(subscriptionPath)?.delete(subscription);
      }
    });

    // Dispatch custom event
    this.dispatchEvent(new CustomEvent('state-changed', {
      detail: { path, newValue, oldValue }
    }));
  }

  /**
   * Call a subscription with options handling
   * @private
   * @param {Object} subscription - Subscription object
   * @param {*} newValue - New value
   * @param {*} oldValue - Old value
   * @param {string} path - Changed path
   */
  _callSubscription(subscription, newValue, oldValue, path) {
    const { callback, options } = subscription;

    // Check custom compare function
    if (options.compare && options.compare(newValue, oldValue)) {
      return; // Values are considered equal, skip notification
    }

    const doCall = () => {
      try {
        callback(newValue, oldValue, path);
      } catch (error) {
        console.error('Error in subscription callback:', error);
      }
    };

    // Handle debounce
    if (options.debounce > 0) {
      if (subscription.timer) {
        clearTimeout(subscription.timer);
      }
      subscription.timer = setTimeout(() => {
        subscription.timer = null;
        doCall();
      }, options.debounce);
      return;
    }

    // Handle requestAnimationFrame
    if (options.useRAF) {
      if (subscription.rafId) {
        cancelAnimationFrame(subscription.rafId);
      }
      subscription.rafId = requestAnimationFrame(() => {
        subscription.rafId = null;
        doCall();
      });
      return;
    }

    // Default: call immediately
    doCall();
  }

  /**
   * Flush batched notifications
   * @private
   */
  _flushBatchedNotifications() {
    const paths = Array.from(this._batchedNotifications);
    this._batchedNotifications.clear();

    paths.forEach(path => {
      const newValue = this.get(path);
      this._notify(path, newValue, undefined);
    });
  }

  /**
   * Notify all subscribers (used after restore)
   * @private
   */
  _notifyAll(newState, oldState) {
    for (const [path, subscriptions] of this._subscriptions) {
      const newValue = this.get(path);
      const oldValue = this._getValueFromState(oldState, path);
      subscriptions.forEach(subscription => {
        this._callSubscription(subscription, newValue, oldValue, path);
      });
    }
  }

  /**
   * Get value from a state object at path
   * @private
   */
  _getValueFromState(state, path) {
    if (!path) return state;
    const parts = this._parsePath(path);
    let value = state;
    for (const part of parts) {
      if (value === null || value === undefined) return undefined;
      value = value[part];
    }
    return value;
  }

  /**
   * Apply middleware functions
   * @private
   * @param {string} action - Action name
   * @param {Object} payload - Action payload
   * @returns {boolean} False if cancelled
   */
  _applyMiddleware(action, payload) {
    for (const middleware of this._middleware) {
      const result = middleware(action, payload);
      if (result === false) {
        return false;
      }
    }
    return true;
  }

  /**
   * Save state snapshot to history (debug mode)
   * @private
   */
  _saveSnapshot() {
    // Remove any future history if we're not at the end
    if (this._historyIndex < this._history.length - 1) {
      this._history = this._history.slice(0, this._historyIndex + 1);
    }

    // Add new snapshot
    this._history.push({
      state: this._deepClone(this._state),
      timestamp: Date.now()
    });

    // Limit history size
    if (this._history.length > this._maxHistorySize) {
      this._history.shift();
    } else {
      this._historyIndex++;
    }
  }

  /**
   * Load state from localStorage
   * @private
   */
  _loadFromStorage() {
    try {
      // Load individual keys with prefix
      const keys = Object.keys(localStorage);
      const prefix = `${this._options.storagePrefix}:`;

      keys.forEach(key => {
        if (key.startsWith(prefix)) {
          const path = key.slice(prefix.length);
          const value = JSON.parse(localStorage.getItem(key));
          this.set(path, value, true); // Silent update
        }
      });
    } catch (error) {
      console.error('Failed to load state from storage:', error);
    }
  }

  /**
   * Save state to localStorage
   * @private
   */
  _saveToStorage() {
    if (!this._options.persist) return;

    try {
      // For now, save the entire state under a single key
      // In production, you might want to save individual paths
      const key = `${this._options.storagePrefix}:state`;
      localStorage.setItem(key, JSON.stringify(this._state));
    } catch (error) {
      console.error('Failed to save state to storage:', error);
    }
  }

  /**
   * Debounce persistence to localStorage
   * @private
   */
  _debouncePersist() {
    if (this._persistTimer) {
      clearTimeout(this._persistTimer);
    }

    this._persistTimer = setTimeout(() => {
      this._persistTimer = null;
      this._saveToStorage();
    }, this._options.persistDebounce);
  }

  /**
   * Deep clone an object
   * @private
   * @param {*} obj - Object to clone
   * @returns {*} Cloned object
   */
  _deepClone(obj) {
    if (obj === null || typeof obj !== 'object') {
      return obj;
    }

    if (Array.isArray(obj)) {
      return obj.map(item => this._deepClone(item));
    }

    const cloned = {};
    for (const key in obj) {
      if (obj.hasOwnProperty(key)) {
        cloned[key] = this._deepClone(obj[key]);
      }
    }
    return cloned;
  }
}

/**
 * Create a global state instance (singleton pattern)
 * @param {Object} initialState - Initial state
 * @param {Object} options - Configuration options
 * @returns {TerraphimState} Global state instance
 */
export function createGlobalState(initialState, options) {
  if (!window.__TERRAPHIM_STATE__) {
    window.__TERRAPHIM_STATE__ = new TerraphimState(initialState, options);
  }
  return window.__TERRAPHIM_STATE__;
}

/**
 * Get the global state instance
 * @returns {TerraphimState|null} Global state instance or null
 */
export function getGlobalState() {
  return window.__TERRAPHIM_STATE__ || null;
}
