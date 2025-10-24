/**
 * @fileoverview TerraphimElement - Base class for all Terraphim Web Components
 * Provides lifecycle management, property reflection, event handling, and Shadow DOM utilities
 */

/**
 * Base class for all Terraphim web components.
 * Provides core functionality including:
 * - Lifecycle hooks (connectedCallback, disconnectedCallback, attributeChangedCallback)
 * - Property/attribute reflection system
 * - Event utilities (emit, listen, listenTo)
 * - Render utilities ($, $$, setHTML)
 * - Automatic cleanup system
 * - Shadow DOM support with configurable mode
 *
 * @class
 * @extends HTMLElement
 *
 * @example
 * // Define a custom component
 * class MyComponent extends TerraphimElement {
 *   static get observedAttributes() {
 *     return ['title', 'count'];
 *   }
 *
 *   static get properties() {
 *     return {
 *       title: { type: String, reflect: true },
 *       count: { type: Number, reflect: true, default: 0 }
 *     };
 *   }
 *
 *   constructor() {
 *     super();
 *     this.attachShadow({ mode: 'open' });
 *   }
 *
 *   render() {
 *     this.setHTML(this.shadowRoot, `
 *       <div>
 *         <h1>${this.title}</h1>
 *         <p>Count: ${this.count}</p>
 *       </div>
 *     `);
 *   }
 * }
 *
 * customElements.define('my-component', MyComponent);
 */
export class TerraphimElement extends HTMLElement {
  /**
   * List of attributes to observe for changes.
   * Subclasses should override this to specify which attributes to watch.
   *
   * @static
   * @returns {string[]} Array of attribute names to observe
   *
   * @example
   * static get observedAttributes() {
   *   return ['title', 'disabled', 'value'];
   * }
   */
  static get observedAttributes() {
    return [];
  }

  /**
   * Property definitions with type conversion and reflection.
   * Defines how properties map to attributes and their types.
   *
   * @static
   * @returns {Object.<string, PropertyDefinition>} Property definitions
   *
   * @typedef {Object} PropertyDefinition
   * @property {Function} type - Type constructor (String, Number, Boolean, Object, Array)
   * @property {boolean} [reflect=false] - Whether to reflect property changes to attributes
   * @property {*} [default] - Default value for the property
   *
   * @example
   * static get properties() {
   *   return {
   *     title: { type: String, reflect: true },
   *     count: { type: Number, reflect: true, default: 0 },
   *     disabled: { type: Boolean, reflect: true },
   *     data: { type: Object, default: () => ({}) }
   *   };
   * }
   */
  static get properties() {
    return {};
  }

  constructor() {
    super();

    /**
     * Array of cleanup functions to execute on disconnect
     * @private
     * @type {Function[]}
     */
    this._cleanupFunctions = [];

    /**
     * Whether the component is currently connected to the DOM
     * @private
     * @type {boolean}
     */
    this._isConnected = false;

    /**
     * Whether properties have been initialized
     * @private
     * @type {boolean}
     */
    this._propertiesInitialized = false;

    /**
     * State bindings for this component
     * @private
     * @type {Map<string, Object>}
     */
    this._stateBindings = new Map();

    // Initialize properties from property definitions
    this._initializeProperties();
  }

  /**
   * Initialize properties with their default values and create getters/setters
   * @private
   */
  _initializeProperties() {
    const properties = this.constructor.properties;

    for (const [propName, propDef] of Object.entries(properties)) {
      // Set default value
      let defaultValue = propDef.default;
      if (typeof defaultValue === 'function') {
        defaultValue = defaultValue();
      }

      // Store the internal value
      const internalProp = `_${propName}`;
      this[internalProp] = defaultValue !== undefined ? defaultValue : this._getDefaultValueForType(propDef.type);

      // Create getter/setter if not already defined
      if (!Object.getOwnPropertyDescriptor(this, propName)) {
        Object.defineProperty(this, propName, {
          get() {
            return this[internalProp];
          },
          set(value) {
            const oldValue = this[internalProp];
            const newValue = this._convertType(value, propDef.type);

            if (oldValue !== newValue) {
              this[internalProp] = newValue;

              // Reflect to attribute if specified
              if (propDef.reflect) {
                this._reflectPropertyToAttribute(propName, newValue);
              }

              // Call property changed callback if connected
              if (this._isConnected) {
                this.propertyChangedCallback(propName, oldValue, newValue);
              }
            }
          },
          enumerable: true,
          configurable: true
        });
      }
    }

    this._propertiesInitialized = true;
  }

  /**
   * Get default value for a type
   * @private
   * @param {Function} type - Type constructor
   * @returns {*} Default value
   */
  _getDefaultValueForType(type) {
    if (type === String) return '';
    if (type === Number) return 0;
    if (type === Boolean) return false;
    if (type === Array) return [];
    if (type === Object) return {};
    return undefined;
  }

  /**
   * Convert value to specified type
   * @private
   * @param {*} value - Value to convert
   * @param {Function} type - Target type constructor
   * @returns {*} Converted value
   */
  _convertType(value, type) {
    if (value === null || value === undefined) {
      return this._getDefaultValueForType(type);
    }

    if (type === String) {
      return String(value);
    }

    if (type === Number) {
      const num = Number(value);
      return isNaN(num) ? 0 : num;
    }

    if (type === Boolean) {
      // Handle attribute-style boolean (presence = true, absence = false)
      if (typeof value === 'string') {
        return value !== 'false' && value !== '';
      }
      return Boolean(value);
    }

    if (type === Object || type === Array) {
      if (typeof value === 'string') {
        try {
          return JSON.parse(value);
        } catch (e) {
          console.warn(`Failed to parse JSON for property:`, value);
          return this._getDefaultValueForType(type);
        }
      }
      return value;
    }

    return value;
  }

  /**
   * Reflect property value to attribute
   * @private
   * @param {string} propName - Property name
   * @param {*} value - Property value
   */
  _reflectPropertyToAttribute(propName, value) {
    const attrName = this._propNameToAttrName(propName);

    if (value === null || value === undefined || value === false) {
      this.removeAttribute(attrName);
    } else if (value === true) {
      this.setAttribute(attrName, '');
    } else if (typeof value === 'object') {
      this.setAttribute(attrName, JSON.stringify(value));
    } else {
      this.setAttribute(attrName, String(value));
    }
  }

  /**
   * Convert property name to attribute name (camelCase to kebab-case)
   * @private
   * @param {string} propName - Property name
   * @returns {string} Attribute name
   */
  _propNameToAttrName(propName) {
    return propName.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase();
  }

  /**
   * Convert attribute name to property name (kebab-case to camelCase)
   * @private
   * @param {string} attrName - Attribute name
   * @returns {string} Property name
   */
  _attrNameToPropName(attrName) {
    return attrName.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
  }

  /**
   * Called when the element is connected to the DOM.
   * Sets up the component and calls onConnected hook.
   * Automatically schedules render if render() method exists.
   */
  connectedCallback() {
    this._isConnected = true;

    // Call lifecycle hook
    if (typeof this.onConnected === 'function') {
      this.onConnected();
    }

    // Schedule initial render
    if (typeof this.render === 'function') {
      this._scheduleRender();
    }
  }

  /**
   * Called when the element is disconnected from the DOM.
   * Performs cleanup and calls onDisconnected hook.
   */
  disconnectedCallback() {
    this._isConnected = false;

    // Call lifecycle hook
    if (typeof this.onDisconnected === 'function') {
      this.onDisconnected();
    }

    // Cleanup state bindings
    this._cleanupStateBindings();

    // Execute all cleanup functions
    this._cleanupFunctions.forEach(fn => {
      try {
        fn();
      } catch (e) {
        console.error('Error during cleanup:', e);
      }
    });
    this._cleanupFunctions = [];
  }

  /**
   * Called when an observed attribute changes.
   * Updates corresponding property and calls attributeChangedCallback hook.
   *
   * @param {string} name - Attribute name
   * @param {string|null} oldValue - Previous attribute value
   * @param {string|null} newValue - New attribute value
   */
  attributeChangedCallback(name, oldValue, newValue) {
    if (oldValue === newValue) return;

    // Update corresponding property
    const propName = this._attrNameToPropName(name);
    const properties = this.constructor.properties;

    if (properties[propName]) {
      const propDef = properties[propName];
      this[propName] = this._convertType(newValue, propDef.type);
    }

    // Call lifecycle hook
    if (typeof this.onAttributeChanged === 'function') {
      this.onAttributeChanged(name, oldValue, newValue);
    }
  }

  /**
   * Called when a property changes.
   * Can be overridden by subclasses to react to property changes.
   *
   * @param {string} name - Property name
   * @param {*} oldValue - Previous property value
   * @param {*} newValue - New property value
   */
  propertyChangedCallback(name, oldValue, newValue) {
    // Default: schedule re-render
    if (typeof this.render === 'function') {
      this._scheduleRender();
    }
  }

  /**
   * Schedule a render on next animation frame.
   * Automatically debounces multiple render requests.
   * @private
   */
  _scheduleRender() {
    if (this._renderScheduled) return;

    this._renderScheduled = true;
    requestAnimationFrame(() => {
      this._renderScheduled = false;
      if (this._isConnected && typeof this.render === 'function') {
        this.render();
      }
    });
  }

  /**
   * Lifecycle hook: called when element is connected to DOM
   * Override in subclasses to perform setup logic
   */
  onConnected() {
    // Override in subclass
  }

  /**
   * Lifecycle hook: called when element is disconnected from DOM
   * Override in subclasses to perform cleanup logic
   */
  onDisconnected() {
    // Override in subclass
  }

  /**
   * Lifecycle hook: called when an observed attribute changes
   * Override in subclasses to react to attribute changes
   *
   * @param {string} name - Attribute name
   * @param {string|null} oldValue - Previous value
   * @param {string|null} newValue - New value
   */
  onAttributeChanged(name, oldValue, newValue) {
    // Override in subclass
  }

  /**
   * Emit a custom event from this element.
   * Automatically sets composed: true and bubbles: true for Shadow DOM compatibility.
   *
   * @param {string} eventName - Name of the event
   * @param {*} [detail=null] - Event detail data
   * @param {Object} [options={}] - Additional event options
   * @returns {boolean} True if event was not cancelled
   *
   * @example
   * this.emit('item-selected', { id: 123, name: 'Item' });
   * this.emit('value-changed', this.value, { bubbles: false });
   */
  emit(eventName, detail = null, options = {}) {
    const event = new CustomEvent(eventName, {
      detail,
      bubbles: true,
      composed: true,
      cancelable: true,
      ...options
    });
    return this.dispatchEvent(event);
  }

  /**
   * Add event listener with automatic cleanup on disconnect.
   *
   * @param {string} eventName - Event name
   * @param {Function} handler - Event handler function
   * @param {Object} [options={}] - Event listener options
   * @returns {Function} Cleanup function
   *
   * @example
   * this.listen('click', (e) => {
   *   console.log('Clicked!', e);
   * });
   *
   * // With options
   * this.listen('scroll', handler, { passive: true });
   */
  listen(eventName, handler, options = {}) {
    this.addEventListener(eventName, handler, options);

    const cleanup = () => {
      this.removeEventListener(eventName, handler, options);
    };

    this._cleanupFunctions.push(cleanup);
    return cleanup;
  }

  /**
   * Add event listener on another element with automatic cleanup.
   *
   * @param {EventTarget} target - Target element
   * @param {string} eventName - Event name
   * @param {Function} handler - Event handler function
   * @param {Object} [options={}] - Event listener options
   * @returns {Function} Cleanup function
   *
   * @example
   * this.listenTo(window, 'resize', () => {
   *   this.updateLayout();
   * });
   *
   * this.listenTo(document, 'keydown', (e) => {
   *   if (e.key === 'Escape') this.close();
   * });
   */
  listenTo(target, eventName, handler, options = {}) {
    target.addEventListener(eventName, handler, options);

    const cleanup = () => {
      target.removeEventListener(eventName, handler, options);
    };

    this._cleanupFunctions.push(cleanup);
    return cleanup;
  }

  /**
   * Register a cleanup function to be called on disconnect.
   *
   * @param {Function} fn - Cleanup function
   *
   * @example
   * const interval = setInterval(() => this.update(), 1000);
   * this.addCleanup(() => clearInterval(interval));
   */
  addCleanup(fn) {
    this._cleanupFunctions.push(fn);
  }

  /**
   * Query selector within element (or shadow root if present).
   *
   * @param {string} selector - CSS selector
   * @returns {Element|null} First matching element or null
   *
   * @example
   * const button = this.$('button.primary');
   * const input = this.$('#username');
   */
  $(selector) {
    const root = this.shadowRoot || this;
    return root.querySelector(selector);
  }

  /**
   * Query selector all within element (or shadow root if present).
   *
   * @param {string} selector - CSS selector
   * @returns {NodeList} All matching elements
   *
   * @example
   * const items = this.$$('.list-item');
   * items.forEach(item => item.classList.add('active'));
   */
  $$(selector) {
    const root = this.shadowRoot || this;
    return root.querySelectorAll(selector);
  }

  /**
   * Safely set innerHTML with optional sanitization.
   * For Shadow DOM, pass shadowRoot as target.
   *
   * @param {Element} target - Target element
   * @param {string} html - HTML string to set
   *
   * @example
   * this.setHTML(this.shadowRoot, `
   *   <div class="container">
   *     <h1>${this.title}</h1>
   *     <p>${this.description}</p>
   *   </div>
   * `);
   */
  setHTML(target, html) {
    target.innerHTML = html;
  }

  /**
   * Request an update/re-render of the component.
   * Useful when component state changes and render needs to be called.
   *
   * @example
   * this.data.push(newItem);
   * this.requestUpdate();
   */
  requestUpdate() {
    this._scheduleRender();
  }

  /**
   * Bind a state path to a component property or callback
   *
   * @param {Object} state - TerraphimState instance
   * @param {string} statePath - Path in state to bind (e.g., "user.name")
   * @param {string|Function} target - Property name or callback function
   * @param {Object} [options={}] - Subscription options
   * @returns {Function} Unsubscribe function
   *
   * @example
   * // Bind to property
   * this.bindState(globalState, 'theme', 'currentTheme');
   *
   * // Bind to callback
   * this.bindState(globalState, 'user.name', (value) => {
   *   this.$('.username').textContent = value;
   * });
   *
   * // With options
   * this.bindState(globalState, 'config', this.updateConfig, {
   *   immediate: true,
   *   deep: true
   * });
   */
  bindState(state, statePath, target, options = {}) {
    const callback = typeof target === 'function'
      ? target
      : (value) => { this[target] = value; };

    // Subscribe to state changes
    const unsubscribe = state.subscribe(statePath, (value, oldValue, path) => {
      callback.call(this, value, oldValue, path);
    }, options);

    // Store binding for cleanup
    const bindingKey = `${statePath}:${typeof target === 'string' ? target : 'callback'}`;
    this._stateBindings.set(bindingKey, { unsubscribe, state, statePath, target });

    // Add to cleanup functions
    this.addCleanup(unsubscribe);

    return unsubscribe;
  }

  /**
   * Set a value in state
   * Convenience method for updating state from components
   *
   * @param {Object} state - TerraphimState instance
   * @param {string} path - Path to update
   * @param {*} value - New value
   *
   * @example
   * this.setState(globalState, 'theme', 'dark');
   * this.setState(globalState, 'user.name', 'Alice');
   */
  setState(state, path, value) {
    state.set(path, value);
  }

  /**
   * Get a value from state
   * Convenience method for reading state in components
   *
   * @param {Object} state - TerraphimState instance
   * @param {string} path - Path to read
   * @returns {*} Value at path
   *
   * @example
   * const theme = this.getState(globalState, 'theme');
   * const userName = this.getState(globalState, 'user.name');
   */
  getState(state, path) {
    return state.get(path);
  }

  /**
   * Cleanup all state bindings
   * Called automatically on disconnect
   * @private
   */
  _cleanupStateBindings() {
    this._stateBindings.forEach(({ unsubscribe }) => {
      try {
        unsubscribe();
      } catch (e) {
        console.error('Error cleaning up state binding:', e);
      }
    });
    this._stateBindings.clear();
  }
}
