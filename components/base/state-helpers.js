/**
 * @fileoverview State Helper Utilities
 * Provides computed values, action creators, validators, and derived stores
 */

import { TerraphimState } from './terraphim-state.js';

/**
 * Create a computed value that derives from state paths
 * The compute function is called whenever any dependency changes
 *
 * @param {TerraphimState} state - State instance
 * @param {string[]} dependencies - Array of paths to watch
 * @param {Function} computeFn - Function to compute derived value
 * @param {Object} [options={}] - Subscription options
 * @returns {Object} Computed value object with { value, unsubscribe() }
 *
 * @example
 * const fullName = computed(
 *   state,
 *   ['user.firstName', 'user.lastName'],
 *   (firstName, lastName) => `${firstName} ${lastName}`
 * );
 *
 * console.log(fullName.value); // "John Doe"
 *
 * // Cleanup
 * fullName.unsubscribe();
 */
export function computed(state, dependencies, computeFn, options = {}) {
  let currentValue;
  const unsubscribers = [];

  const recalculate = () => {
    const values = dependencies.map(dep => state.get(dep));
    currentValue = computeFn(...values);
  };

  // Initial calculation
  recalculate();

  // Subscribe to all dependencies
  dependencies.forEach(dep => {
    const unsub = state.subscribe(dep, () => {
      recalculate();
    }, options);
    unsubscribers.push(unsub);
  });

  return {
    get value() {
      return currentValue;
    },
    unsubscribe() {
      unsubscribers.forEach(unsub => unsub());
    }
  };
}

/**
 * Create a derived store that maintains its own reactive value
 * Unlike computed(), this creates a subscribable value
 *
 * @param {TerraphimState} state - State instance
 * @param {string[]} dependencies - Array of paths to watch
 * @param {Function} deriveFn - Function to derive value
 * @param {*} [initialValue] - Initial value
 * @returns {Object} Derived store with { value, subscribe(), unsubscribe() }
 *
 * @example
 * const userCount = derived(
 *   state,
 *   ['users'],
 *   (users) => users.length,
 *   0
 * );
 *
 * userCount.subscribe((count) => {
 *   console.log('User count:', count);
 * });
 */
export function derived(state, dependencies, deriveFn, initialValue = null) {
  let currentValue = initialValue;
  const subscribers = new Set();
  const unsubscribers = [];

  const recalculate = () => {
    const values = dependencies.map(dep => state.get(dep));
    const newValue = deriveFn(...values);

    if (newValue !== currentValue) {
      const oldValue = currentValue;
      currentValue = newValue;
      subscribers.forEach(callback => {
        try {
          callback(currentValue, oldValue);
        } catch (error) {
          console.error('Error in derived store callback:', error);
        }
      });
    }
  };

  // Initial calculation
  recalculate();

  // Subscribe to all dependencies
  dependencies.forEach(dep => {
    const unsub = state.subscribe(dep, () => {
      recalculate();
    });
    unsubscribers.push(unsub);
  });

  return {
    get value() {
      return currentValue;
    },
    subscribe(callback, options = {}) {
      subscribers.add(callback);

      // Call immediately if requested
      if (options.immediate) {
        callback(currentValue, undefined);
      }

      // Return unsubscribe function
      return () => {
        subscribers.delete(callback);
      };
    },
    unsubscribe() {
      unsubscribers.forEach(unsub => unsub());
      subscribers.clear();
    }
  };
}

/**
 * Create an action creator that encapsulates state mutations
 * Actions provide a clean API for state changes and can include validation/logging
 *
 * @param {TerraphimState} state - State instance
 * @param {string} name - Action name (for debugging)
 * @param {Function} actionFn - Function that performs the action
 * @returns {Function} Action function
 *
 * @example
 * const setTheme = createAction(state, 'setTheme', (theme) => {
 *   if (!['light', 'dark'].includes(theme)) {
 *     throw new Error('Invalid theme');
 *   }
 *   state.set('theme', theme);
 * });
 *
 * setTheme('dark');
 */
export function createAction(state, name, actionFn) {
  return function action(...args) {
    try {
      return actionFn(state, ...args);
    } catch (error) {
      console.error(`Action "${name}" failed:`, error);
      throw error;
    }
  };
}

/**
 * Create a validator for state values
 * Validators can be attached as middleware to prevent invalid state
 *
 * @param {Object} schema - Validation schema
 * @returns {Function} Validator middleware function
 *
 * @example
 * const validator = validate({
 *   'user.email': (value) => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value),
 *   'user.age': (value) => value >= 0 && value <= 120
 * });
 *
 * state.use(validator);
 */
export function validate(schema) {
  return (action, payload) => {
    if (action !== 'set') return;

    const { path, value } = payload;

    // Check if we have a validator for this path
    for (const [schemaPath, validatorFn] of Object.entries(schema)) {
      if (path === schemaPath || path.startsWith(`${schemaPath}.`)) {
        const isValid = validatorFn(value, path);

        if (!isValid) {
          console.error(`Validation failed for path "${path}":`, value);
          return false; // Cancel the operation
        }
      }
    }
  };
}

/**
 * Create a logger middleware for debugging
 * Logs all state changes to console
 *
 * @param {Object} [options={}] - Logger options
 * @param {boolean} [options.logGet=false] - Log get operations
 * @param {boolean} [options.logSet=true] - Log set operations
 * @param {Function} [options.filter] - Filter function (action, payload) => boolean
 * @returns {Function} Logger middleware
 *
 * @example
 * const logger = createLogger({ logSet: true });
 * state.use(logger);
 */
export function createLogger(options = {}) {
  const {
    logGet = false,
    logSet = true,
    filter = null
  } = options;

  return (action, payload) => {
    if (filter && !filter(action, payload)) {
      return;
    }

    if (action === 'set' && logSet) {
      console.log(`[State] SET ${payload.path} =`, payload.value);
    }

    if (action === 'get' && logGet) {
      console.log(`[State] GET ${payload.path}`);
    }
  };
}

/**
 * Create a persistence middleware for specific paths
 * Saves specific state paths to localStorage
 *
 * @param {string[]} paths - Array of paths to persist
 * @param {Object} [options={}] - Persistence options
 * @param {string} [options.prefix='terraphim'] - localStorage prefix
 * @param {number} [options.debounce=500] - Debounce delay
 * @returns {Function} Persistence middleware
 *
 * @example
 * const persist = createPersistence(['theme', 'user.preferences']);
 * state.use(persist);
 */
export function createPersistence(paths, options = {}) {
  const {
    prefix = 'terraphim',
    debounce = 500
  } = options;

  const timers = new Map();

  return (action, payload) => {
    if (action !== 'set') return;

    const { path, value } = payload;

    // Check if this path should be persisted
    const shouldPersist = paths.some(p =>
      path === p || path.startsWith(`${p}.`)
    );

    if (!shouldPersist) return;

    // Debounce the persistence
    const key = `${prefix}:${path}`;

    if (timers.has(key)) {
      clearTimeout(timers.get(key));
    }

    timers.set(key, setTimeout(() => {
      try {
        localStorage.setItem(key, JSON.stringify(value));
      } catch (error) {
        console.error(`Failed to persist ${path}:`, error);
      }
      timers.delete(key);
    }, debounce));
  };
}

/**
 * Restore persisted state from localStorage
 *
 * @param {TerraphimState} state - State instance
 * @param {string[]} paths - Array of paths to restore
 * @param {string} [prefix='terraphim'] - localStorage prefix
 *
 * @example
 * restorePersistedState(state, ['theme', 'user.preferences']);
 */
export function restorePersistedState(state, paths, prefix = 'terraphim') {
  paths.forEach(path => {
    const key = `${prefix}:${path}`;
    try {
      const stored = localStorage.getItem(key);
      if (stored !== null) {
        const value = JSON.parse(stored);
        state.set(path, value, true); // Silent update
      }
    } catch (error) {
      console.error(`Failed to restore ${path}:`, error);
    }
  });
}

/**
 * Create a synchronizer that keeps two state instances in sync
 *
 * @param {TerraphimState} source - Source state
 * @param {TerraphimState} target - Target state
 * @param {Object} [mapping={}] - Path mapping (source -> target)
 * @returns {Function} Cleanup function
 *
 * @example
 * const cleanup = syncStates(
 *   localState,
 *   globalState,
 *   { 'user': 'app.user', 'theme': 'app.theme' }
 * );
 */
export function syncStates(source, target, mapping = {}) {
  const unsubscribers = [];

  // If no mapping provided, sync entire state
  if (Object.keys(mapping).length === 0) {
    const unsub = source.subscribe('*', (value, oldValue, path) => {
      target.set(path, value, true);
    });
    unsubscribers.push(unsub);
  } else {
    // Sync specific paths with mapping
    for (const [sourcePath, targetPath] of Object.entries(mapping)) {
      const unsub = source.subscribe(sourcePath, (value) => {
        target.set(targetPath, value, true);
      });
      unsubscribers.push(unsub);
    }
  }

  // Return cleanup function
  return () => {
    unsubscribers.forEach(unsub => unsub());
  };
}

/**
 * Create a readonly view of state
 * Prevents modifications while allowing subscriptions
 *
 * @param {TerraphimState} state - State instance
 * @param {string[]} [allowedPaths] - Optional array of allowed paths
 * @returns {Object} Readonly state interface
 *
 * @example
 * const readonly = createReadonly(state, ['user', 'config']);
 * readonly.get('user.name'); // OK
 * readonly.subscribe('user.name', callback); // OK
 * readonly.set('user.name', 'foo'); // Error
 */
export function createReadonly(state, allowedPaths = null) {
  return {
    get(path) {
      if (allowedPaths && !allowedPaths.some(p => path.startsWith(p))) {
        throw new Error(`Access denied to path: ${path}`);
      }
      return state.get(path);
    },
    subscribe(path, callback, options) {
      if (allowedPaths && !allowedPaths.some(p => path.startsWith(p))) {
        throw new Error(`Access denied to path: ${path}`);
      }
      return state.subscribe(path, callback, options);
    },
    set() {
      throw new Error('Cannot modify readonly state');
    },
    batch() {
      throw new Error('Cannot modify readonly state');
    }
  };
}

/**
 * Create a batched update helper
 * Provides a cleaner API for batch updates
 *
 * @param {TerraphimState} state - State instance
 * @returns {Function} Batch update function
 *
 * @example
 * const update = batchUpdate(state);
 *
 * update({
 *   'user.name': 'Alice',
 *   'user.email': 'alice@example.com',
 *   'user.role': 'admin'
 * });
 */
export function batchUpdate(state) {
  return (updates) => {
    state.batch(() => {
      for (const [path, value] of Object.entries(updates)) {
        state.set(path, value);
      }
    });
  };
}

/**
 * Create a selector function for derived data
 * Memoizes the result to avoid unnecessary recalculations
 *
 * @param {Function} selectorFn - Selector function
 * @returns {Function} Memoized selector
 *
 * @example
 * const getActiveUsers = createSelector(
 *   (state) => state.get('users').filter(u => u.active)
 * );
 *
 * const activeUsers = getActiveUsers(state);
 */
export function createSelector(selectorFn) {
  let lastArgs = null;
  let lastResult = null;

  return (...args) => {
    // Simple shallow equality check
    if (lastArgs && lastArgs.length === args.length &&
        lastArgs.every((arg, i) => arg === args[i])) {
      return lastResult;
    }

    lastArgs = args;
    lastResult = selectorFn(...args);
    return lastResult;
  };
}

/**
 * Wait for a condition to be true in state
 * Returns a promise that resolves when the condition is met
 *
 * @param {TerraphimState} state - State instance
 * @param {string} path - Path to watch
 * @param {Function} condition - Condition function (value) => boolean
 * @param {number} [timeout=0] - Optional timeout in ms (0 = no timeout)
 * @returns {Promise<*>} Promise that resolves with the value
 *
 * @example
 * await waitFor(state, 'user', (user) => user !== null);
 * console.log('User loaded!');
 *
 * // With timeout
 * try {
 *   await waitFor(state, 'data', (data) => data.loaded, 5000);
 * } catch (error) {
 *   console.error('Timeout waiting for data');
 * }
 */
export function waitFor(state, path, condition, timeout = 0) {
  return new Promise((resolve, reject) => {
    let timeoutId = null;
    let unsubscribe = null;

    const cleanup = () => {
      if (timeoutId) clearTimeout(timeoutId);
      if (unsubscribe) unsubscribe();
    };

    // Check initial value
    const initialValue = state.get(path);
    if (condition(initialValue)) {
      resolve(initialValue);
      return;
    }

    // Set up timeout if specified
    if (timeout > 0) {
      timeoutId = setTimeout(() => {
        cleanup();
        reject(new Error(`Timeout waiting for condition on path: ${path}`));
      }, timeout);
    }

    // Subscribe to changes
    unsubscribe = state.subscribe(path, (value) => {
      if (condition(value)) {
        cleanup();
        resolve(value);
      }
    });
  });
}

/**
 * Create a debounced setter
 * Debounces state updates to avoid rapid changes
 *
 * @param {TerraphimState} state - State instance
 * @param {number} delay - Debounce delay in ms
 * @returns {Function} Debounced set function
 *
 * @example
 * const debouncedSet = createDebouncedSetter(state, 300);
 * debouncedSet('search.query', 'hello'); // Debounced
 */
export function createDebouncedSetter(state, delay) {
  const timers = new Map();

  return (path, value) => {
    if (timers.has(path)) {
      clearTimeout(timers.get(path));
    }

    timers.set(path, setTimeout(() => {
      state.set(path, value);
      timers.delete(path);
    }, delay));
  };
}

/**
 * Create a throttled setter
 * Throttles state updates to limit frequency
 *
 * @param {TerraphimState} state - State instance
 * @param {number} delay - Throttle delay in ms
 * @returns {Function} Throttled set function
 *
 * @example
 * const throttledSet = createThrottledSetter(state, 100);
 * throttledSet('mouse.x', event.clientX); // Throttled
 */
export function createThrottledSetter(state, delay) {
  const lastCalls = new Map();

  return (path, value) => {
    const now = Date.now();
    const lastCall = lastCalls.get(path) || 0;

    if (now - lastCall >= delay) {
      state.set(path, value);
      lastCalls.set(path, now);
    }
  };
}
