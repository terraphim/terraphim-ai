# Migration Guide: Svelte Stores to TerraphimState

This guide helps you migrate from Svelte stores to the TerraphimState vanilla state management system.

## Table of Contents

- [Why Migrate?](#why-migrate)
- [Core Concepts Mapping](#core-concepts-mapping)
- [Migration Patterns](#migration-patterns)
- [Step-by-Step Migration](#step-by-step-migration)
- [Common Patterns](#common-patterns)
- [Troubleshooting](#troubleshooting)

## Why Migrate?

### Benefits of TerraphimState

1. **No Build Required**: Works directly in the browser without compilation
2. **Framework Agnostic**: Use with any component system or vanilla JS
3. **Better Performance**: Path-based subscriptions and batching
4. **Enhanced Debugging**: Time travel, snapshots, history tracking
5. **Persistence Built-in**: localStorage integration out of the box
6. **Flexible Subscriptions**: Debounce, throttle, RAF, wildcards
7. **Web Components Ready**: Seamless integration with TerraphimElement

### When to Migrate

- Converting from Svelte to vanilla Web Components
- Need to support file:// protocol (no build step)
- Want better debugging and dev tools
- Need built-in persistence without extra libraries
- Building component library independent of frameworks

## Core Concepts Mapping

### Svelte Store → TerraphimState

| Svelte Store | TerraphimState | Notes |
|--------------|----------------|-------|
| `writable(value)` | `new TerraphimState({ value })` | Basic reactive state |
| `readable(value)` | `createReadonly(state)` | Read-only state |
| `derived(store, fn)` | `derived(state, deps, fn)` | Computed values |
| `$store` | `state.get(path)` | Read value |
| `$store = value` | `state.set(path, value)` | Write value |
| `store.subscribe(fn)` | `state.subscribe(path, fn)` | Subscribe to changes |
| `store.set(value)` | `state.set(path, value)` | Set value |
| `store.update(fn)` | Custom action | Update with function |
| `get(store)` | `state.get(path)` | Get current value |

## Migration Patterns

### 1. Basic Writable Store

**Before (Svelte):**
```javascript
import { writable } from 'svelte/store';

export const theme = writable('light');
export const user = writable(null);
export const count = writable(0);
```

**After (TerraphimState):**
```javascript
import { TerraphimState } from './components/base/terraphim-state.js';

export const state = new TerraphimState({
  theme: 'light',
  user: null,
  count: 0
});
```

### 2. Subscribing to Changes

**Before (Svelte Component):**
```svelte
<script>
  import { theme } from './stores.js';

  let currentTheme;
  const unsubscribe = theme.subscribe(value => {
    currentTheme = value;
  });

  onDestroy(unsubscribe);
</script>
```

**After (Web Component):**
```javascript
class MyComponent extends TerraphimElement {
  onConnected() {
    this.bindState(state, 'theme', (value) => {
      this.currentTheme = value;
      this.requestUpdate();
    });
    // Automatic cleanup on disconnect
  }
}
```

### 3. Reactive Statements

**Before (Svelte):**
```svelte
<script>
  import { firstName, lastName } from './stores.js';

  $: fullName = `${$firstName} ${$lastName}`;
</script>

<p>{fullName}</p>
```

**After (TerraphimState):**
```javascript
import { computed } from './components/base/state-helpers.js';

const fullName = computed(
  state,
  ['firstName', 'lastName'],
  (first, last) => `${first} ${last}`
);

// In component
class MyComponent extends TerraphimElement {
  render() {
    this.setHTML(this.shadowRoot, `
      <p>${fullName.value}</p>
    `);
  }
}
```

### 4. Derived Stores

**Before (Svelte):**
```javascript
import { derived } from 'svelte/store';
import { items } from './stores.js';

export const itemCount = derived(
  items,
  $items => $items.length
);

export const totalPrice = derived(
  items,
  $items => $items.reduce((sum, item) => sum + item.price, 0)
);
```

**After (TerraphimState):**
```javascript
import { derived } from './components/base/state-helpers.js';

export const itemCount = derived(
  state,
  ['items'],
  (items) => items.length,
  0
);

export const totalPrice = derived(
  state,
  ['items'],
  (items) => items.reduce((sum, item) => sum + item.price, 0),
  0
);
```

### 5. Store with Custom Logic

**Before (Svelte):**
```javascript
import { writable } from 'svelte/store';

function createCounter() {
  const { subscribe, set, update } = writable(0);

  return {
    subscribe,
    increment: () => update(n => n + 1),
    decrement: () => update(n => n - 1),
    reset: () => set(0)
  };
}

export const counter = createCounter();
```

**After (TerraphimState):**
```javascript
import { createAction } from './components/base/state-helpers.js';

const state = new TerraphimState({ counter: 0 });

export const increment = createAction(state, 'increment', (state) => {
  const current = state.get('counter');
  state.set('counter', current + 1);
});

export const decrement = createAction(state, 'decrement', (state) => {
  const current = state.get('counter');
  state.set('counter', current - 1);
});

export const reset = createAction(state, 'reset', (state) => {
  state.set('counter', 0);
});

// Subscribe
export function subscribeToCounter(callback) {
  return state.subscribe('counter', callback);
}
```

### 6. Nested State

**Before (Svelte):**
```javascript
import { writable } from 'svelte/store';

export const user = writable({
  profile: {
    name: '',
    email: ''
  },
  preferences: {
    theme: 'light',
    notifications: true
  }
});

// Update nested value
user.update(u => ({
  ...u,
  profile: {
    ...u.profile,
    name: 'Alice'
  }
}));
```

**After (TerraphimState):**
```javascript
const state = new TerraphimState({
  user: {
    profile: {
      name: '',
      email: ''
    },
    preferences: {
      theme: 'light',
      notifications: true
    }
  }
});

// Update nested value - much simpler!
state.set('user.profile.name', 'Alice');
```

### 7. Persistence

**Before (Svelte with custom persistence):**
```javascript
import { writable } from 'svelte/store';

function persistent(key, initialValue) {
  const stored = localStorage.getItem(key);
  const store = writable(stored ? JSON.parse(stored) : initialValue);

  store.subscribe(value => {
    localStorage.setItem(key, JSON.stringify(value));
  });

  return store;
}

export const theme = persistent('theme', 'light');
```

**After (TerraphimState with built-in persistence):**
```javascript
import { createPersistence, restorePersistedState } from './components/base/state-helpers.js';

const state = new TerraphimState({
  theme: 'light'
});

// Add persistence
const persist = createPersistence(['theme'], {
  prefix: 'myapp',
  debounce: 500
});

state.use(persist);

// Restore on init
restorePersistedState(state, ['theme'], 'myapp');
```

Or use global persistence:

```javascript
const state = new TerraphimState(
  { theme: 'light' },
  { persist: true, storagePrefix: 'myapp' }
);
```

### 8. Multiple Derived Stores

**Before (Svelte):**
```javascript
import { derived } from 'svelte/store';
import { firstName, lastName, email } from './stores.js';

export const userInfo = derived(
  [firstName, lastName, email],
  ([$firstName, $lastName, $email]) => ({
    fullName: `${$firstName} ${$lastName}`,
    email: $email,
    initials: `${$firstName[0]}${$lastName[0]}`
  })
);
```

**After (TerraphimState):**
```javascript
import { computed } from './components/base/state-helpers.js';

const userInfo = computed(
  state,
  ['firstName', 'lastName', 'email'],
  (firstName, lastName, email) => ({
    fullName: `${firstName} ${lastName}`,
    email,
    initials: `${firstName[0]}${lastName[0]}`
  })
);
```

## Step-by-Step Migration

### Step 1: Create Global State

Replace multiple Svelte stores with a single state instance:

**Before:**
```javascript
// stores/theme.js
export const theme = writable('light');

// stores/user.js
export const user = writable(null);

// stores/config.js
export const config = writable({});
```

**After:**
```javascript
// state/index.js
import { createGlobalState } from './components/base/terraphim-state.js';

export const globalState = createGlobalState({
  theme: 'light',
  user: null,
  config: {}
});
```

### Step 2: Update Component Subscriptions

**Before (Svelte):**
```svelte
<script>
  import { theme } from './stores/theme.js';
  import { onDestroy } from 'svelte';

  let currentTheme;
  const unsubscribe = theme.subscribe(value => {
    currentTheme = value;
  });

  onDestroy(unsubscribe);
</script>

<div class="theme-{currentTheme}">
  Content
</div>
```

**After (Web Component):**
```javascript
class ThemeComponent extends TerraphimElement {
  static get properties() {
    return {
      currentTheme: { type: String }
    };
  }

  onConnected() {
    this.bindState(globalState, 'theme', 'currentTheme');
  }

  render() {
    this.setHTML(this.shadowRoot, `
      <div class="theme-${this.currentTheme}">
        Content
      </div>
    `);
  }
}
```

### Step 3: Convert Store Updates

**Before (Svelte):**
```javascript
import { theme } from './stores/theme.js';

// Update
theme.set('dark');

// Update with function
theme.update(current => current === 'light' ? 'dark' : 'light');
```

**After (TerraphimState):**
```javascript
import { globalState } from './state/index.js';

// Update
globalState.set('theme', 'dark');

// Update with function (use action)
import { createAction } from './components/base/state-helpers.js';

const toggleTheme = createAction(globalState, 'toggleTheme', (state) => {
  const current = state.get('theme');
  state.set('theme', current === 'light' ? 'dark' : 'light');
});

toggleTheme();
```

### Step 4: Convert Derived Stores

**Before:**
```javascript
import { derived } from 'svelte/store';
import { user } from './stores/user.js';

export const isAdmin = derived(
  user,
  $user => $user?.role === 'admin'
);
```

**After:**
```javascript
import { derived } from './components/base/state-helpers.js';
import { globalState } from './state/index.js';

export const isAdmin = derived(
  globalState,
  ['user'],
  (user) => user?.role === 'admin',
  false
);
```

### Step 5: Update Event Handlers

**Before (Svelte):**
```svelte
<button on:click={() => $count++}>
  Increment: {$count}
</button>
```

**After (Web Component):**
```javascript
class CounterButton extends TerraphimElement {
  static get properties() {
    return {
      count: { type: Number }
    };
  }

  onConnected() {
    this.bindState(globalState, 'count', 'count');
  }

  handleClick() {
    const current = this.getState(globalState, 'count');
    this.setState(globalState, 'count', current + 1);
  }

  render() {
    this.setHTML(this.shadowRoot, `
      <button>Increment: ${this.count}</button>
    `);

    this.$('button').addEventListener('click', () => this.handleClick());
  }
}
```

## Common Patterns

### Pattern: Form Input Binding

**Before (Svelte):**
```svelte
<script>
  import { searchQuery } from './stores.js';
</script>

<input bind:value={$searchQuery} />
```

**After (Web Component):**
```javascript
class SearchInput extends TerraphimElement {
  onConnected() {
    this.bindState(globalState, 'search.query', (value) => {
      this.$('input').value = value;
    }, { immediate: true });

    this.$('input').addEventListener('input', (e) => {
      this.setState(globalState, 'search.query', e.target.value);
    });
  }

  render() {
    this.setHTML(this.shadowRoot, `<input type="text" />`);
  }
}
```

### Pattern: Conditional Rendering

**Before (Svelte):**
```svelte
<script>
  import { isLoggedIn } from './stores.js';
</script>

{#if $isLoggedIn}
  <p>Welcome back!</p>
{:else}
  <p>Please log in</p>
{/if}
```

**After (Web Component):**
```javascript
class LoginStatus extends TerraphimElement {
  onConnected() {
    this.bindState(globalState, 'user.isLoggedIn', () => {
      this.requestUpdate();
    });
  }

  render() {
    const isLoggedIn = this.getState(globalState, 'user.isLoggedIn');

    this.setHTML(this.shadowRoot, isLoggedIn
      ? `<p>Welcome back!</p>`
      : `<p>Please log in</p>`
    );
  }
}
```

### Pattern: List Rendering

**Before (Svelte):**
```svelte
<script>
  import { items } from './stores.js';
</script>

<ul>
  {#each $items as item}
    <li>{item.name}</li>
  {/each}
</ul>
```

**After (Web Component):**
```javascript
class ItemList extends TerraphimElement {
  onConnected() {
    this.bindState(globalState, 'items', () => {
      this.requestUpdate();
    });
  }

  render() {
    const items = this.getState(globalState, 'items') || [];

    this.setHTML(this.shadowRoot, `
      <ul>
        ${items.map(item => `<li>${item.name}</li>`).join('')}
      </ul>
    `);
  }
}
```

### Pattern: Async Data Loading

**Before (Svelte):**
```javascript
import { writable } from 'svelte/store';

export const data = writable({ loading: true, result: null });

async function loadData() {
  data.set({ loading: true, result: null });
  try {
    const result = await fetch('/api/data').then(r => r.json());
    data.set({ loading: false, result });
  } catch (error) {
    data.set({ loading: false, error });
  }
}
```

**After (TerraphimState):**
```javascript
import { createAction } from './components/base/state-helpers.js';

const state = new TerraphimState({
  data: { loading: false, result: null, error: null }
});

const loadData = createAction(state, 'loadData', async (state) => {
  state.set('data.loading', true);
  state.set('data.error', null);

  try {
    const result = await fetch('/api/data').then(r => r.json());
    state.batch(() => {
      state.set('data.loading', false);
      state.set('data.result', result);
    });
  } catch (error) {
    state.batch(() => {
      state.set('data.loading', false);
      state.set('data.error', error.message);
    });
  }
});
```

## Troubleshooting

### Issue: Subscriptions not cleaning up

**Problem:**
```javascript
class MyComponent extends TerraphimElement {
  onConnected() {
    state.subscribe('value', callback); // Not cleaned up!
  }
}
```

**Solution:**
```javascript
class MyComponent extends TerraphimElement {
  onConnected() {
    // Option 1: Use bindState (automatic cleanup)
    this.bindState(state, 'value', callback);

    // Option 2: Add to cleanup manually
    const unsub = state.subscribe('value', callback);
    this.addCleanup(unsub);
  }
}
```

### Issue: Too many re-renders

**Problem:**
```javascript
state.subscribe('data', () => {
  state.set('derived', computeValue()); // Triggers another update!
});
```

**Solution:**
```javascript
// Use derived() instead
const derived = derived(state, ['data'], computeValue);

// Or use silent update
state.subscribe('data', () => {
  state.set('derived', computeValue(), true); // Silent
});
```

### Issue: Nested updates are verbose

**Problem:**
```javascript
// Svelte was simple
$user.profile.name = 'Alice';

// TerraphimState requires set()
const user = state.get('user');
user.profile.name = 'Alice'; // Doesn't trigger updates!
state.set('user', user); // Must explicitly set
```

**Solution:**
```javascript
// Use path-based updates
state.set('user.profile.name', 'Alice');

// Or use batch for multiple updates
state.batch(() => {
  state.set('user.profile.name', 'Alice');
  state.set('user.profile.email', 'alice@example.com');
});
```

### Issue: Missing reactivity

**Problem:**
```javascript
// Direct property access doesn't work
this.value = state.get('value');
state.set('value', 42); // Component not updated
```

**Solution:**
```javascript
// Subscribe to updates
this.bindState(state, 'value', 'value');
// Now this.value updates automatically
```

## Migration Checklist

- [ ] Create global TerraphimState instance
- [ ] Convert writable stores to state paths
- [ ] Convert derived stores to computed/derived helpers
- [ ] Update component subscriptions to use bindState()
- [ ] Convert store.set() calls to state.set()
- [ ] Convert store.update() calls to actions
- [ ] Add persistence configuration
- [ ] Remove Svelte store imports
- [ ] Test all state updates
- [ ] Verify cleanup on component disconnect
- [ ] Update tests
- [ ] Remove Svelte dependencies

## Complete Example

### Before (Svelte)

```javascript
// stores.js
import { writable, derived } from 'svelte/store';

export const theme = writable('light');
export const user = writable(null);
export const items = writable([]);

export const itemCount = derived(items, $items => $items.length);
```

```svelte
<!-- App.svelte -->
<script>
  import { theme, user, itemCount } from './stores.js';

  function toggleTheme() {
    theme.update(t => t === 'light' ? 'dark' : 'light');
  }
</script>

<div class="app theme-{$theme}">
  {#if $user}
    <p>Welcome, {$user.name}!</p>
  {/if}

  <p>Items: {$itemCount}</p>

  <button on:click={toggleTheme}>
    Toggle Theme
  </button>
</div>
```

### After (TerraphimState + Web Components)

```javascript
// state.js
import { TerraphimState } from './components/base/terraphim-state.js';
import { derived, createAction } from './components/base/state-helpers.js';

export const state = new TerraphimState({
  theme: 'light',
  user: null,
  items: []
});

export const itemCount = derived(
  state,
  ['items'],
  (items) => items.length,
  0
);

export const toggleTheme = createAction(state, 'toggleTheme', (state) => {
  const current = state.get('theme');
  state.set('theme', current === 'light' ? 'dark' : 'light');
});
```

```javascript
// app-component.js
import { TerraphimElement } from './components/base/terraphim-element.js';
import { state, itemCount, toggleTheme } from './state.js';

class AppComponent extends TerraphimElement {
  static get properties() {
    return {
      theme: { type: String },
      user: { type: Object }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  onConnected() {
    this.bindState(state, 'theme', 'theme');
    this.bindState(state, 'user', 'user');
    this.bindState(state, 'items', () => this.requestUpdate());
  }

  handleToggleTheme() {
    toggleTheme();
  }

  render() {
    this.setHTML(this.shadowRoot, `
      <div class="app theme-${this.theme}">
        ${this.user ? `<p>Welcome, ${this.user.name}!</p>` : ''}

        <p>Items: ${itemCount.value}</p>

        <button id="toggleBtn">Toggle Theme</button>
      </div>
    `);

    this.$('#toggleBtn').addEventListener('click', () => this.handleToggleTheme());
  }
}

customElements.define('app-component', AppComponent);
```

## Conclusion

Migrating from Svelte stores to TerraphimState provides:

- **Simpler nested state management** with path-based access
- **Better performance** with batching and optimizations
- **Enhanced debugging** with time travel and snapshots
- **Built-in persistence** without extra libraries
- **Framework independence** - works anywhere
- **No build required** - runs directly in browsers

The migration is straightforward: replace store instances with state paths, convert subscriptions to bindState(), and use helper utilities for advanced patterns.

For more information, see [STATE-MANAGEMENT.md](./STATE-MANAGEMENT.md).
