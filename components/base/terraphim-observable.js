/**
 * @fileoverview TerraphimObservable - Reactive state management mixin
 * Provides proxy-based reactivity with path-based subscriptions and batch updates
 */

/**
 * Makes an object deeply observable using Proxy.
 * Tracks changes and notifies subscribers.
 *
 * @param {Object} target - Object to make observable
 * @param {Function} onChange - Callback when any property changes
 * @param {string} [path=''] - Current property path (for nested objects)
 * @returns {Proxy} Observable proxy of the target object
 *
 * @private
 */
function createObservableProxy(target, onChange, path = '') {
  return new Proxy(target, {
    get(obj, prop) {
      const value = obj[prop];

      // Return primitives and functions as-is
      if (value === null || value === undefined) {
        return value;
      }

      if (typeof value === 'function') {
        return value.bind(obj);
      }

      // Recursively proxy nested objects and arrays
      if (typeof value === 'object' && !value.__isProxy) {
        const newPath = path ? `${path}.${prop}` : prop;
        return createObservableProxy(value, onChange, newPath);
      }

      return value;
    },

    set(obj, prop, value) {
      const oldValue = obj[prop];

      if (oldValue !== value) {
        obj[prop] = value;
        const changePath = path ? `${path}.${prop}` : prop;
        onChange(changePath, oldValue, value);
      }

      return true;
    },

    deleteProperty(obj, prop) {
      if (prop in obj) {
        const oldValue = obj[prop];
        delete obj[prop];
        const changePath = path ? `${path}.${prop}` : prop;
        onChange(changePath, oldValue, undefined);
      }
      return true;
    }
  });
}

/**
 * Mixin that adds observable/reactive state management to a class.
 * Provides automatic change detection and subscription system.
 *
 * @param {Class} BaseClass - Base class to extend (typically TerraphimElement)
 * @returns {Class} Extended class with observable functionality
 *
 * @example
 * class MyComponent extends TerraphimObservable(TerraphimElement) {
 *   constructor() {
 *     super();
 *     this.state = this.observe({
 *       count: 0,
 *       user: { name: 'Alice', age: 30 }
 *     });
 *
 *     // Subscribe to all changes
 *     this.subscribe('*', () => {
 *       this.render();
 *     });
 *
 *     // Subscribe to specific path
 *     this.subscribe('user.name', (path, oldVal, newVal) => {
 *       console.log(`Name changed from ${oldVal} to ${newVal}`);
 *     });
 *   }
 *
 *   increment() {
 *     this.state.count++; // Automatically triggers subscribers
 *   }
 * }
 */
export function TerraphimObservable(BaseClass) {
  return class extends BaseClass {
    constructor() {
      super();

      /**
       * Map of subscription paths to arrays of subscriber functions
       * @private
       * @type {Map<string, Function[]>}
       */
      this._subscriptions = new Map();

      /**
       * Set of changes pending in the current batch
       * @private
       * @type {Set<Object>}
       */
      this._pendingChanges = new Set();

      /**
       * Whether a batch update is scheduled
       * @private
       * @type {boolean}
       */
      this._batchScheduled = false;

      /**
       * Whether batch updates are enabled
       * @private
       * @type {boolean}
       */
      this._batchEnabled = true;
    }

    /**
     * Make an object observable.
     * Returns a proxy that tracks changes and notifies subscribers.
     *
     * @param {Object} obj - Object to observe
     * @returns {Proxy} Observable proxy
     *
     * @example
     * this.state = this.observe({
     *   items: [],
     *   selectedId: null,
     *   filters: { search: '', category: 'all' }
     * });
     */
    observe(obj) {
      return createObservableProxy(obj, (path, oldValue, newValue) => {
        this._notifyChange(path, oldValue, newValue);
      });
    }

    /**
     * Subscribe to changes at a specific path.
     * Use '*' to subscribe to all changes.
     * Supports dot notation for nested properties.
     *
     * @param {string} path - Property path to watch (e.g., 'user.name' or '*')
     * @param {Function} callback - Callback function(path, oldValue, newValue)
     * @returns {Function} Unsubscribe function
     *
     * @example
     * // Watch specific property
     * const unsubscribe = this.subscribe('count', (path, old, val) => {
     *   console.log(`Count: ${old} -> ${val}`);
     * });
     *
     * // Watch nested property
     * this.subscribe('user.name', (path, old, val) => {
     *   console.log(`Name changed: ${val}`);
     * });
     *
     * // Watch all changes
     * this.subscribe('*', (path, old, val) => {
     *   console.log(`${path} changed`);
     * });
     *
     * // Unsubscribe
     * unsubscribe();
     */
    subscribe(path, callback) {
      if (!this._subscriptions.has(path)) {
        this._subscriptions.set(path, []);
      }

      const subscribers = this._subscriptions.get(path);
      subscribers.push(callback);

      // Return unsubscribe function
      return () => {
        const index = subscribers.indexOf(callback);
        if (index > -1) {
          subscribers.splice(index, 1);
        }

        // Clean up empty subscription arrays
        if (subscribers.length === 0) {
          this._subscriptions.delete(path);
        }
      };
    }

    /**
     * Unsubscribe from changes at a specific path.
     *
     * @param {string} path - Property path
     * @param {Function} [callback] - Specific callback to remove (removes all if omitted)
     *
     * @example
     * // Remove specific callback
     * this.unsubscribe('count', myCallback);
     *
     * // Remove all callbacks for path
     * this.unsubscribe('count');
     */
    unsubscribe(path, callback = null) {
      if (!this._subscriptions.has(path)) return;

      if (callback === null) {
        // Remove all subscribers for this path
        this._subscriptions.delete(path);
      } else {
        // Remove specific subscriber
        const subscribers = this._subscriptions.get(path);
        const index = subscribers.indexOf(callback);
        if (index > -1) {
          subscribers.splice(index, 1);
        }

        // Clean up empty subscription arrays
        if (subscribers.length === 0) {
          this._subscriptions.delete(path);
        }
      }
    }

    /**
     * Notify subscribers of a change.
     * Handles batching and wildcard subscriptions.
     *
     * @private
     * @param {string} path - Property path that changed
     * @param {*} oldValue - Previous value
     * @param {*} newValue - New value
     */
    _notifyChange(path, oldValue, newValue) {
      const change = { path, oldValue, newValue };

      if (this._batchEnabled) {
        // Add to pending changes
        this._pendingChanges.add(change);

        // Schedule batch notification
        if (!this._batchScheduled) {
          this._batchScheduled = true;
          Promise.resolve().then(() => {
            this._flushChanges();
          });
        }
      } else {
        // Notify immediately
        this._notifySubscribers(change);
      }
    }

    /**
     * Flush all pending changes and notify subscribers.
     * @private
     */
    _flushChanges() {
      this._batchScheduled = false;

      const changes = Array.from(this._pendingChanges);
      this._pendingChanges.clear();

      changes.forEach(change => {
        this._notifySubscribers(change);
      });
    }

    /**
     * Notify all relevant subscribers of a change.
     * @private
     * @param {Object} change - Change object with path, oldValue, newValue
     */
    _notifySubscribers(change) {
      const { path, oldValue, newValue } = change;

      // Notify wildcard subscribers
      const wildcardSubs = this._subscriptions.get('*');
      if (wildcardSubs) {
        wildcardSubs.forEach(callback => {
          try {
            callback(path, oldValue, newValue);
          } catch (e) {
            console.error(`Error in wildcard subscriber:`, e);
          }
        });
      }

      // Notify exact path subscribers
      const exactSubs = this._subscriptions.get(path);
      if (exactSubs) {
        exactSubs.forEach(callback => {
          try {
            callback(path, oldValue, newValue);
          } catch (e) {
            console.error(`Error in subscriber for ${path}:`, e);
          }
        });
      }

      // Notify parent path subscribers
      // e.g., if 'user.name' changes, notify 'user' subscribers
      const pathParts = path.split('.');
      for (let i = pathParts.length - 1; i > 0; i--) {
        const parentPath = pathParts.slice(0, i).join('.');
        const parentSubs = this._subscriptions.get(parentPath);
        if (parentSubs) {
          parentSubs.forEach(callback => {
            try {
              callback(path, oldValue, newValue);
            } catch (e) {
              console.error(`Error in parent subscriber for ${parentPath}:`, e);
            }
          });
        }
      }
    }

    /**
     * Execute a function with batching disabled.
     * All changes will be notified immediately.
     *
     * @param {Function} fn - Function to execute
     *
     * @example
     * this.withoutBatching(() => {
     *   this.state.count = 1;  // Notifies immediately
     *   this.state.count = 2;  // Notifies immediately
     *   this.state.count = 3;  // Notifies immediately
     * });
     */
    withoutBatching(fn) {
      const wasBatchEnabled = this._batchEnabled;
      this._batchEnabled = false;

      try {
        fn();
      } finally {
        this._batchEnabled = wasBatchEnabled;
      }
    }

    /**
     * Execute a function and batch all changes into a single notification.
     * Useful for multiple related updates.
     *
     * @param {Function} fn - Function to execute
     * @returns {Promise} Promise that resolves when batch is complete
     *
     * @example
     * await this.batch(() => {
     *   this.state.count++;
     *   this.state.user.name = 'Bob';
     *   this.state.items.push(newItem);
     *   // All changes notified together after function completes
     * });
     */
    async batch(fn) {
      const wasBatchEnabled = this._batchEnabled;
      this._batchEnabled = true;

      try {
        fn();
      } finally {
        this._batchEnabled = wasBatchEnabled;
      }

      // Wait for batch to flush
      await new Promise(resolve => {
        if (this._batchScheduled) {
          Promise.resolve().then(() => {
            resolve();
          });
        } else {
          resolve();
        }
      });
    }

    /**
     * Get the number of subscribers for a path.
     *
     * @param {string} path - Property path
     * @returns {number} Number of subscribers
     *
     * @example
     * const count = this.getSubscriberCount('user.name');
     * console.log(`${count} subscribers watching user.name`);
     */
    getSubscriberCount(path) {
      const subscribers = this._subscriptions.get(path);
      return subscribers ? subscribers.length : 0;
    }

    /**
     * Clear all subscriptions.
     * Useful for cleanup or testing.
     *
     * @example
     * this.clearSubscriptions();
     */
    clearSubscriptions() {
      this._subscriptions.clear();
    }
  };
}
