/**
 * @fileoverview TerraphimEvents - Global event bus and event mixin
 * Provides cross-component communication with namespace support
 */

/**
 * Global event bus for cross-component communication.
 * Singleton instance that allows components to communicate without direct coupling.
 * Supports namespaced events for better organization.
 *
 * @class
 *
 * @example
 * // Listen for events
 * TerraphimEvents.on('user:login', (data) => {
 *   console.log('User logged in:', data);
 * });
 *
 * // Emit events
 * TerraphimEvents.emit('user:login', { id: 123, name: 'Alice' });
 *
 * // One-time listeners
 * TerraphimEvents.once('app:ready', () => {
 *   console.log('App is ready!');
 * });
 *
 * // Remove listeners
 * const handler = (data) => console.log(data);
 * TerraphimEvents.on('data:update', handler);
 * TerraphimEvents.off('data:update', handler);
 */
class TerraphimEventsClass {
  constructor() {
    /**
     * Map of event names to arrays of listener objects
     * @private
     * @type {Map<string, Array<{handler: Function, once: boolean}>>}
     */
    this._listeners = new Map();
  }

  /**
   * Add an event listener.
   *
   * @param {string} eventName - Event name (supports namespacing with ':')
   * @param {Function} handler - Event handler function
   * @returns {Function} Unsubscribe function
   *
   * @example
   * const unsubscribe = TerraphimEvents.on('search:complete', (results) => {
   *   console.log('Search results:', results);
   * });
   *
   * // Later...
   * unsubscribe();
   */
  on(eventName, handler) {
    if (!this._listeners.has(eventName)) {
      this._listeners.set(eventName, []);
    }

    const listeners = this._listeners.get(eventName);
    listeners.push({ handler, once: false });

    // Return unsubscribe function
    return () => this.off(eventName, handler);
  }

  /**
   * Add a one-time event listener.
   * The listener is automatically removed after first invocation.
   *
   * @param {string} eventName - Event name
   * @param {Function} handler - Event handler function
   * @returns {Function} Unsubscribe function
   *
   * @example
   * TerraphimEvents.once('config:loaded', (config) => {
   *   console.log('Config loaded:', config);
   *   // This will only fire once
   * });
   */
  once(eventName, handler) {
    if (!this._listeners.has(eventName)) {
      this._listeners.set(eventName, []);
    }

    const listeners = this._listeners.get(eventName);
    listeners.push({ handler, once: true });

    // Return unsubscribe function
    return () => this.off(eventName, handler);
  }

  /**
   * Remove an event listener.
   *
   * @param {string} eventName - Event name
   * @param {Function} [handler] - Specific handler to remove (removes all if omitted)
   *
   * @example
   * // Remove specific handler
   * TerraphimEvents.off('data:update', myHandler);
   *
   * // Remove all handlers for event
   * TerraphimEvents.off('data:update');
   */
  off(eventName, handler = null) {
    if (!this._listeners.has(eventName)) return;

    if (handler === null) {
      // Remove all listeners for this event
      this._listeners.delete(eventName);
    } else {
      // Remove specific listener
      const listeners = this._listeners.get(eventName);
      const index = listeners.findIndex(l => l.handler === handler);
      if (index > -1) {
        listeners.splice(index, 1);
      }

      // Clean up empty listener arrays
      if (listeners.length === 0) {
        this._listeners.delete(eventName);
      }
    }
  }

  /**
   * Emit an event with optional data.
   * Notifies all registered listeners.
   *
   * @param {string} eventName - Event name
   * @param {*} [data] - Event data to pass to listeners
   *
   * @example
   * TerraphimEvents.emit('notification:show', {
   *   type: 'success',
   *   message: 'Operation completed'
   * });
   */
  emit(eventName, data = null) {
    if (!this._listeners.has(eventName)) return;

    const listeners = this._listeners.get(eventName);
    const listenersToRemove = [];

    // Call all listeners
    listeners.forEach((listener, index) => {
      try {
        listener.handler(data);

        // Mark one-time listeners for removal
        if (listener.once) {
          listenersToRemove.push(index);
        }
      } catch (e) {
        console.error(`Error in event handler for ${eventName}:`, e);
      }
    });

    // Remove one-time listeners (in reverse order to maintain indices)
    listenersToRemove.reverse().forEach(index => {
      listeners.splice(index, 1);
    });

    // Clean up empty listener arrays
    if (listeners.length === 0) {
      this._listeners.delete(eventName);
    }
  }

  /**
   * Get the number of listeners for an event.
   *
   * @param {string} eventName - Event name
   * @returns {number} Number of listeners
   *
   * @example
   * const count = TerraphimEvents.getListenerCount('search:complete');
   * console.log(`${count} listeners for search:complete`);
   */
  getListenerCount(eventName) {
    const listeners = this._listeners.get(eventName);
    return listeners ? listeners.length : 0;
  }

  /**
   * Clear all listeners for an event or all events.
   *
   * @param {string} [eventName] - Event name (clears all if omitted)
   *
   * @example
   * // Clear specific event
   * TerraphimEvents.clear('data:update');
   *
   * // Clear all events
   * TerraphimEvents.clear();
   */
  clear(eventName = null) {
    if (eventName === null) {
      this._listeners.clear();
    } else {
      this._listeners.delete(eventName);
    }
  }

  /**
   * Get all registered event names.
   *
   * @returns {string[]} Array of event names
   *
   * @example
   * const events = TerraphimEvents.getEventNames();
   * console.log('Registered events:', events);
   */
  getEventNames() {
    return Array.from(this._listeners.keys());
  }

  /**
   * Check if an event has any listeners.
   *
   * @param {string} eventName - Event name
   * @returns {boolean} True if event has listeners
   *
   * @example
   * if (TerraphimEvents.hasListeners('data:update')) {
   *   console.log('Someone is listening to data updates');
   * }
   */
  hasListeners(eventName) {
    return this._listeners.has(eventName) && this._listeners.get(eventName).length > 0;
  }
}

/**
 * Global singleton instance of the event bus.
 * Use this for cross-component communication.
 */
export const TerraphimEvents = new TerraphimEventsClass();

/**
 * Mixin that adds global event bus integration to a class.
 * Automatically manages event subscriptions with cleanup on disconnect.
 *
 * @param {Class} BaseClass - Base class to extend (typically TerraphimElement)
 * @returns {Class} Extended class with event bus functionality
 *
 * @example
 * class MyComponent extends TerraphimEventBus(TerraphimElement) {
 *   onConnected() {
 *     // Subscribe to global events
 *     this.onGlobal('theme:changed', (theme) => {
 *       this.updateTheme(theme);
 *     });
 *
 *     this.onGlobal('user:logout', () => {
 *       this.reset();
 *     });
 *   }
 *
 *   handleClick() {
 *     // Emit global events
 *     this.emitGlobal('button:clicked', { id: this.id });
 *   }
 * }
 */
export function TerraphimEventBus(BaseClass) {
  return class extends BaseClass {
    constructor() {
      super();

      /**
       * Array of global event unsubscribe functions
       * @private
       * @type {Function[]}
       */
      this._globalEventCleanups = [];
    }

    /**
     * Subscribe to a global event with automatic cleanup.
     *
     * @param {string} eventName - Event name
     * @param {Function} handler - Event handler function
     * @returns {Function} Unsubscribe function
     *
     * @example
     * this.onGlobal('search:start', () => {
     *   this.showLoadingSpinner();
     * });
     *
     * this.onGlobal('search:complete', (results) => {
     *   this.displayResults(results);
     * });
     */
    onGlobal(eventName, handler) {
      const unsubscribe = TerraphimEvents.on(eventName, handler);
      this._globalEventCleanups.push(unsubscribe);

      // Also add to element's cleanup functions if available
      if (typeof this.addCleanup === 'function') {
        this.addCleanup(unsubscribe);
      }

      return unsubscribe;
    }

    /**
     * Subscribe to a global event once with automatic cleanup.
     *
     * @param {string} eventName - Event name
     * @param {Function} handler - Event handler function
     * @returns {Function} Unsubscribe function
     *
     * @example
     * this.onceGlobal('app:ready', () => {
     *   console.log('App is ready, initialize component');
     * });
     */
    onceGlobal(eventName, handler) {
      const unsubscribe = TerraphimEvents.once(eventName, handler);
      this._globalEventCleanups.push(unsubscribe);

      // Also add to element's cleanup functions if available
      if (typeof this.addCleanup === 'function') {
        this.addCleanup(unsubscribe);
      }

      return unsubscribe;
    }

    /**
     * Emit a global event.
     *
     * @param {string} eventName - Event name
     * @param {*} [data] - Event data
     *
     * @example
     * this.emitGlobal('notification:show', {
     *   type: 'error',
     *   message: 'Something went wrong'
     * });
     */
    emitGlobal(eventName, data = null) {
      TerraphimEvents.emit(eventName, data);
    }

    /**
     * Remove a global event listener.
     *
     * @param {string} eventName - Event name
     * @param {Function} [handler] - Specific handler to remove
     *
     * @example
     * this.offGlobal('search:complete', this.handleSearchComplete);
     */
    offGlobal(eventName, handler = null) {
      TerraphimEvents.off(eventName, handler);
    }

    /**
     * Override disconnectedCallback to cleanup global event listeners.
     * @private
     */
    disconnectedCallback() {
      // Clean up global event listeners
      this._globalEventCleanups.forEach(cleanup => {
        try {
          cleanup();
        } catch (e) {
          console.error('Error cleaning up global event listener:', e);
        }
      });
      this._globalEventCleanups = [];

      // Call parent disconnectedCallback if it exists
      if (super.disconnectedCallback) {
        super.disconnectedCallback();
      }
    }
  };
}

/**
 * Create a CustomEvent that works properly across Shadow DOM boundaries.
 * Sets composed: true by default for event propagation.
 *
 * @param {string} eventName - Event name
 * @param {*} [detail=null] - Event detail data
 * @param {Object} [options={}] - Additional event options
 * @returns {CustomEvent} CustomEvent instance
 *
 * @example
 * const event = createEvent('item-selected', { id: 123 });
 * element.dispatchEvent(event);
 *
 * const cancelableEvent = createEvent('delete-item', { id: 456 }, {
 *   cancelable: true,
 *   bubbles: false
 * });
 */
export function createEvent(eventName, detail = null, options = {}) {
  return new CustomEvent(eventName, {
    detail,
    bubbles: true,
    composed: true,
    cancelable: false,
    ...options
  });
}
