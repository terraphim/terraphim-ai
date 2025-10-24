# TerraphimState - Vanilla State Management System

Complete API documentation for the Terraphim AI vanilla JavaScript state management system.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Core API](#core-api)
- [Subscription System](#subscription-system)
- [Helper Utilities](#helper-utilities)
- [Integration with Components](#integration-with-components)
- [Persistence](#persistence)
- [Debugging](#debugging)
- [Advanced Patterns](#advanced-patterns)
- [Performance](#performance)
- [Best Practices](#best-practices)

## Overview

TerraphimState is a lightweight, event-driven state management system built with pure vanilla JavaScript. It provides:

- **Path-based access**: Get/set values using dot notation (`user.name`, `config.haystacks.0`)
- **Reactive subscriptions**: Automatically react to state changes
- **Wildcard support**: Subscribe to patterns like `items.*`
- **Batch updates**: Group multiple changes into single notifications
- **Persistence**: Optional localStorage integration
- **Debugging tools**: Snapshots, time travel, history
- **Middleware system**: Plugin architecture for custom behavior
- **Zero dependencies**: No build tools or frameworks required

## Quick Start

```javascript
import { TerraphimState } from './components/base/terraphim-state.js';

// Create state instance
const state = new TerraphimState({
  theme: 'light',
  user: { name: 'Alice', role: 'admin' },
  config: { haystacks: [] }
});

// Subscribe to changes
state.subscribe('theme', (value) => {
  console.log('Theme changed:', value);
});

// Update state
state.set('theme', 'dark');

// Get values
const currentTheme = state.get('theme');
const userName = state.get('user.name');
```

## Core API

### Constructor

```javascript
new TerraphimState(initialState, options)
```

**Parameters:**
- `initialState` (Object): Initial state object
- `options` (Object): Configuration options
  - `persist` (Boolean): Enable localStorage persistence
  - `storagePrefix` (String): localStorage key prefix (default: 'terraphim')
  - `persistDebounce` (Number): Debounce delay for persistence in ms (default: 500)
  - `debug` (Boolean): Enable debugging mode with time travel

**Example:**
```javascript
const state = new TerraphimState(
  { theme: 'dark' },
  { persist: true, debug: true }
);
```

### get(path)

Get value at path using dot notation.

**Parameters:**
- `path` (String): Path to value (e.g., `"user.name"`, `"items.0.title"`)

**Returns:** Value at path, or `undefined` if not found

**Examples:**
```javascript
state.get('theme');                    // 'dark'
state.get('user.name');                // 'Alice'
state.get('config.haystacks.0.name');  // 'GitHub'
state.get('non.existent.path');        // undefined
```

### set(path, value, silent)

Set value at path. Creates intermediate objects as needed.

**Parameters:**
- `path` (String): Path to update
- `value` (Any): New value
- `silent` (Boolean): Skip notifications if true

**Examples:**
```javascript
state.set('theme', 'dark');
state.set('user.email', 'alice@example.com');
state.set('config.haystacks.0', { name: 'GitHub', url: '...' });
state.set('value', 42, true); // Silent update, no notifications
```

### subscribe(path, callback, options)

Subscribe to changes at path. Supports wildcards and parent paths.

**Parameters:**
- `path` (String): Path to watch (supports `*` wildcard)
- `callback` (Function): Called with `(value, oldValue, path)`
- `options` (Object): Subscription options
  - `immediate` (Boolean): Call immediately with current value
  - `deep` (Boolean): Notify on nested changes
  - `once` (Boolean): Auto-unsubscribe after first call
  - `debounce` (Number): Debounce delay in ms
  - `compare` (Function): Custom equality check `(a, b) => boolean`
  - `useRAF` (Boolean): Defer to requestAnimationFrame

**Returns:** Unsubscribe function

**Examples:**
```javascript
// Basic subscription
const unsubscribe = state.subscribe('theme', (value) => {
  console.log('Theme:', value);
});

// Wildcard subscription
state.subscribe('config.haystacks.*', (value, oldValue, path) => {
  console.log(`Haystack changed at ${path}:`, value);
});

// With options
state.subscribe('user', (user) => {
  updateUI(user);
}, {
  immediate: true,  // Call now with current value
  deep: true,       // Notify on user.name changes
  debounce: 300     // Wait 300ms after last change
});

// One-time subscription
state.subscribe('initialized', (value) => {
  console.log('App initialized');
}, { once: true });

// Custom compare to prevent unnecessary updates
state.subscribe('data', (value) => {
  render(value);
}, {
  compare: (a, b) => JSON.stringify(a) === JSON.stringify(b)
});

// Unsubscribe when done
unsubscribe();
```

### batch(fn)

Batch multiple updates into single notification cycle.

**Parameters:**
- `fn` (Function): Function containing multiple set() calls

**Example:**
```javascript
state.batch(() => {
  state.set('user.name', 'Bob');
  state.set('user.email', 'bob@example.com');
  state.set('user.role', 'admin');
  // All subscribers notified once after function completes
});
```

### use(middleware)

Add middleware function to intercept state operations.

**Parameters:**
- `middleware` (Function): Middleware `(action, payload) => boolean | void`
  - Return `false` to cancel the operation

**Returns:** Remove middleware function

**Example:**
```javascript
// Logging middleware
const removeLogger = state.use((action, payload) => {
  console.log(`[${action}]`, payload);
});

// Validation middleware
state.use((action, payload) => {
  if (action === 'set' && payload.path === 'age') {
    if (payload.value < 0 || payload.value > 120) {
      console.error('Invalid age');
      return false; // Cancel update
    }
  }
});

// Remove middleware
removeLogger();
```

### getSnapshot()

Get deep clone of current state.

**Returns:** Object with complete state copy

**Example:**
```javascript
const snapshot = state.getSnapshot();
console.log(snapshot); // { theme: 'dark', user: {...} }
```

### restoreSnapshot(snapshot)

Restore state from snapshot. Notifies all subscribers.

**Parameters:**
- `snapshot` (Object): State snapshot to restore

**Example:**
```javascript
const backup = state.getSnapshot();

// Make changes
state.set('theme', 'light');
state.set('user.name', 'Charlie');

// Restore
state.restoreSnapshot(backup);
```

### clear()

Clear all state (reset to empty object).

**Example:**
```javascript
state.clear(); // State is now {}
```

### reset(initialState)

Reset state to new initial values.

**Parameters:**
- `initialState` (Object): New initial state

**Example:**
```javascript
state.reset({ theme: 'default', user: null });
```

## Debugging

### Time Travel (Debug Mode Only)

```javascript
const state = new TerraphimState({ value: 0 }, { debug: true });

state.set('value', 1);
state.set('value', 2);
state.set('value', 3);

state.undo(1);  // Go back to value: 2
state.undo(2);  // Go back to value: 0
state.redo(1);  // Go forward to value: 1
```

### getHistory()

Get state history (debug mode only).

**Returns:** Array of `{state, timestamp}` objects

**Example:**
```javascript
const history = state.getHistory();
console.log('State changes:', history.length);
history.forEach((snapshot, index) => {
  console.log(`${index}:`, snapshot.timestamp, snapshot.state);
});
```

## Helper Utilities

### computed()

Create computed value from dependencies.

```javascript
import { computed } from './components/base/state-helpers.js';

const fullName = computed(
  state,
  ['user.firstName', 'user.lastName'],
  (first, last) => `${first} ${last}`
);

console.log(fullName.value); // "Alice Smith"

// Cleanup
fullName.unsubscribe();
```

### derived()

Create derived store (subscribable computed value).

```javascript
import { derived } from './components/base/state-helpers.js';

const userCount = derived(
  state,
  ['users'],
  (users) => users.length,
  0
);

userCount.subscribe((count) => {
  console.log('User count:', count);
});

// Cleanup
userCount.unsubscribe();
```

### createAction()

Create action creator for state mutations.

```javascript
import { createAction } from './components/base/state-helpers.js';

const setTheme = createAction(state, 'setTheme', (state, theme) => {
  if (!['light', 'dark'].includes(theme)) {
    throw new Error('Invalid theme');
  }
  state.set('theme', theme);
});

setTheme('dark'); // Validated and applied
```

### validate()

Create validator middleware.

```javascript
import { validate } from './components/base/state-helpers.js';

const validator = validate({
  'user.email': (value) => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value),
  'user.age': (value) => value >= 0 && value <= 120
});

state.use(validator);

state.set('user.email', 'invalid'); // Rejected
state.set('user.email', 'valid@example.com'); // Accepted
```

### waitFor()

Wait for condition to be true.

```javascript
import { waitFor } from './components/base/state-helpers.js';

// Wait for data to load
await waitFor(state, 'data.loaded', (value) => value === true);
console.log('Data loaded!');

// With timeout
try {
  await waitFor(state, 'user', (user) => user !== null, 5000);
} catch (error) {
  console.error('Timeout waiting for user');
}
```

### Persistence Helpers

```javascript
import {
  createPersistence,
  restorePersistedState
} from './components/base/state-helpers.js';

// Create persistence middleware
const persist = createPersistence(['theme', 'user.preferences'], {
  prefix: 'myapp',
  debounce: 500
});

state.use(persist);

// Restore on app start
restorePersistedState(state, ['theme', 'user.preferences'], 'myapp');
```

### State Synchronization

```javascript
import { syncStates } from './components/base/state-helpers.js';

const localState = new TerraphimState({ value: 1 });
const globalState = new TerraphimState({ value: 0 });

// Sync local to global
const cleanup = syncStates(localState, globalState, {
  'value': 'app.localValue'
});

localState.set('value', 42);
// globalState.get('app.localValue') === 42

// Cleanup
cleanup();
```

### Readonly Views

```javascript
import { createReadonly } from './components/base/state-helpers.js';

const readonly = createReadonly(state, ['user', 'config']);

readonly.get('user.name'); // OK
readonly.subscribe('user.name', callback); // OK
readonly.set('user.name', 'Bob'); // Error: Cannot modify readonly state
```

## Integration with Components

### bindState()

Bind state path to component property or callback.

```javascript
class MyComponent extends TerraphimElement {
  static get properties() {
    return {
      currentTheme: { type: String }
    };
  }

  onConnected() {
    // Bind to property
    this.bindState(globalState, 'theme', 'currentTheme');

    // Bind to callback
    this.bindState(globalState, 'user.name', (value) => {
      this.$('.username').textContent = value;
    });

    // With options
    this.bindState(globalState, 'config', this.updateConfig, {
      immediate: true,
      deep: true
    });
  }
}
```

### setState() / getState()

Convenience methods for updating/reading state.

```javascript
class MyComponent extends TerraphimElement {
  handleClick() {
    const current = this.getState(globalState, 'count');
    this.setState(globalState, 'count', current + 1);
  }
}
```

### Automatic Cleanup

All state bindings are automatically cleaned up when the component disconnects.

```javascript
// No manual cleanup needed!
connectedCallback() {
  this.bindState(state, 'theme', 'currentTheme');
  this.bindState(state, 'user', this.updateUser);
}

disconnectedCallback() {
  // Bindings automatically unsubscribed
  super.disconnectedCallback();
}
```

## Advanced Patterns

### Global State Singleton

```javascript
import { createGlobalState, getGlobalState } from './components/base/terraphim-state.js';

// Create once
const globalState = createGlobalState({
  theme: 'light',
  user: null,
  config: {}
});

// Access anywhere
const state = getGlobalState();
```

### Debounced Updates

```javascript
import { createDebouncedSetter } from './components/base/state-helpers.js';

const debouncedSet = createDebouncedSetter(state, 300);

// Rapid calls are debounced
debouncedSet('search.query', 'h');
debouncedSet('search.query', 'he');
debouncedSet('search.query', 'hello');
// Only final value is set after 300ms
```

### Throttled Updates

```javascript
import { createThrottledSetter } from './components/base/state-helpers.js';

const throttledSet = createThrottledSetter(state, 100);

// Updates limited to once per 100ms
window.addEventListener('mousemove', (e) => {
  throttledSet('mouse.x', e.clientX);
  throttledSet('mouse.y', e.clientY);
});
```

### Batch Updates

```javascript
import { batchUpdate } from './components/base/state-helpers.js';

const update = batchUpdate(state);

update({
  'user.name': 'Alice',
  'user.email': 'alice@example.com',
  'user.role': 'admin'
});
```

### Memoized Selectors

```javascript
import { createSelector } from './components/base/state-helpers.js';

const getActiveUsers = createSelector((state) => {
  console.log('Computing active users...');
  return state.get('users').filter(u => u.active);
});

const activeUsers = getActiveUsers(state); // Computes
const sameUsers = getActiveUsers(state);   // Returns cached result
```

## Performance

### Path Caching

Parsed paths are automatically cached for performance:

```javascript
// First access parses the path
state.get('a.b.c.d.e');

// Subsequent accesses use cached parse
state.get('a.b.c.d.e'); // Faster
```

### Batch Updates

Group related updates to minimize notifications:

```javascript
// Bad: 3 separate notifications
state.set('user.name', 'Alice');
state.set('user.email', 'alice@example.com');
state.set('user.role', 'admin');

// Good: 1 batched notification
state.batch(() => {
  state.set('user.name', 'Alice');
  state.set('user.email', 'alice@example.com');
  state.set('user.role', 'admin');
});
```

### Subscription Optimization

Use appropriate subscription options:

```javascript
// Debounce rapid changes
state.subscribe('search.query', handleSearch, {
  debounce: 300
});

// Use RAF for UI updates
state.subscribe('animation.progress', updateUI, {
  useRAF: true
});

// Custom compare for complex values
state.subscribe('data', render, {
  compare: (a, b) => a.id === b.id && a.version === b.version
});
```

## Best Practices

### 1. Organize State Structure

```javascript
const state = new TerraphimState({
  // UI state
  theme: 'light',
  sidebarOpen: false,

  // User state
  user: {
    id: null,
    name: '',
    role: ''
  },

  // Application data
  config: {
    haystacks: [],
    relevanceFunction: 'BM25'
  },

  // Feature state
  search: {
    query: '',
    typeahead: true,
    results: []
  }
});
```

### 2. Use Actions for Complex Updates

```javascript
const addHaystack = createAction(state, 'addHaystack', (state, haystack) => {
  const haystacks = state.get('config.haystacks') || [];
  state.set('config.haystacks', [...haystacks, haystack]);
});

addHaystack({ name: 'GitHub', url: 'https://github.com' });
```

### 3. Leverage Computed Values

```javascript
// Don't duplicate derived state
const roleLabel = computed(
  state,
  ['user.role'],
  (role) => {
    return role === 'admin' ? 'Administrator' : 'User';
  }
);
```

### 4. Use Deep Subscriptions Sparingly

```javascript
// Too broad - notified on any nested change
state.subscribe('config', callback, { deep: true });

// Better - subscribe to specific paths
state.subscribe('config.theme', themeCallback);
state.subscribe('config.haystacks', haystacksCallback);
```

### 5. Clean Up Subscriptions

```javascript
class MyComponent extends TerraphimElement {
  onConnected() {
    // Automatic cleanup via bindState
    this.bindState(state, 'theme', 'currentTheme');

    // Manual subscription - add to cleanup
    const unsub = state.subscribe('data', this.handleData);
    this.addCleanup(unsub);
  }
}
```

### 6. Persist Wisely

```javascript
// Persist only what's needed
const persist = createPersistence([
  'theme',
  'user.preferences'
  // Don't persist: search results, transient UI state
], { debounce: 500 });
```

### 7. Use Middleware for Cross-Cutting Concerns

```javascript
// Logging in development
if (process.env.NODE_ENV === 'development') {
  state.use(createLogger());
}

// Global validation
state.use(validate({
  'user.email': isValidEmail,
  'user.age': isValidAge
}));

// Analytics
state.use((action, payload) => {
  if (action === 'set') {
    analytics.track('state_change', payload);
  }
});
```

## Example: Complete Application State

```javascript
import { TerraphimState, createGlobalState } from './components/base/terraphim-state.js';
import { createAction, validate, createPersistence } from './components/base/state-helpers.js';

// Initialize global state
const globalState = createGlobalState({
  theme: 'spacelab',
  role: null,
  is_tauri: false,
  serverUrl: 'http://localhost:8000',
  config: {},
  roles: [],
  search: {
    input: '',
    typeahead: true,
    results: []
  },
  thesaurus: null,
  chat: {
    conversations: [],
    currentId: null,
    statistics: null
  }
}, {
  persist: true,
  storagePrefix: 'terraphim',
  debug: process.env.NODE_ENV === 'development'
});

// Add validation
globalState.use(validate({
  'theme': (value) => ['spacelab', 'dark', 'light'].includes(value)
}));

// Add persistence for specific paths
globalState.use(createPersistence([
  'theme',
  'search.typeahead'
]));

// Define actions
export const setTheme = createAction(globalState, 'setTheme', (state, theme) => {
  state.set('theme', theme);
  document.body.setAttribute('data-theme', theme);
});

export const setRole = createAction(globalState, 'setRole', (state, role) => {
  state.set('role', role);
  state.set('config', role.config || {});
});

export const updateSearchInput = createAction(globalState, 'updateSearchInput', (state, input) => {
  state.batch(() => {
    state.set('search.input', input);
    if (!input) {
      state.set('search.results', []);
    }
  });
});

export default globalState;
```

## TypeScript Support

While TerraphimState is written in vanilla JavaScript, you can add JSDoc comments for IDE support:

```javascript
/**
 * @typedef {Object} AppState
 * @property {string} theme
 * @property {Object|null} user
 * @property {Object} config
 */

/**
 * @type {TerraphimState}
 */
const state = new TerraphimState(/** @type {AppState} */ ({
  theme: 'light',
  user: null,
  config: {}
}));
```

## Comparison with Other Solutions

### vs. Svelte Stores
- No build required
- Works with any component system
- More flexible subscription options
- Built-in persistence and debugging

### vs. Redux
- Simpler API (no reducers, actions creators)
- Built-in async support
- Path-based access (no deep nesting)
- Smaller bundle size (no dependencies)

### vs. MobX
- Explicit subscriptions (no proxy magic)
- Better debugging (time travel, snapshots)
- Simpler mental model
- Works without decorators

## License

MIT
