# Terraphim Base Components

Pure vanilla JavaScript foundation for building modern web components with reactivity, events, and lifecycle management. No build tools required, works via `file://` protocol.

## Overview

This library provides four core building blocks for creating sophisticated web components:

1. **TerraphimElement** - Base class with lifecycle hooks, property reflection, and DOM utilities
2. **TerraphimObservable** - Mixin for reactive state management with proxy-based observation
3. **TerraphimEvents** - Global event bus and mixin for cross-component communication
4. **TerraphimState** - Global state management with path-based subscriptions and persistence

## Installation

No installation required. Simply import the modules directly:

```javascript
import { TerraphimElement } from './base/terraphim-element.js';
import { TerraphimObservable } from './base/terraphim-observable.js';
import { TerraphimEvents, TerraphimEventBus } from './base/terraphim-events.js';
import { TerraphimState } from './base/terraphim-state.js';
```

Or use the barrel export:

```javascript
import {
  TerraphimElement,
  TerraphimObservable,
  TerraphimEvents,
  TerraphimState
} from './base/index.js';
```

## TerraphimElement

Base class for all Terraphim web components. Extends `HTMLElement` with essential features.

### Features

- **Lifecycle Hooks**: `onConnected`, `onDisconnected`, `onAttributeChanged`
- **Property System**: Type conversion, reflection, and observation
- **Event Handling**: Simplified event management with automatic cleanup
- **DOM Utilities**: Shortcuts for querying and rendering
- **Shadow DOM Support**: First-class support for Shadow DOM encapsulation

### Basic Usage

```javascript
class MyComponent extends TerraphimElement {
  static get observedAttributes() {
    return ['title', 'count'];
  }

  static get properties() {
    return {
      title: { type: String, reflect: true },
      count: { type: Number, reflect: true, default: 0 }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  onConnected() {
    console.log('Component connected!');
    this.listen('click', () => this.handleClick());
  }

  handleClick() {
    this.count++;
  }

  render() {
    this.setHTML(this.shadowRoot, `
      <div class="container">
        <h1>${this.title}</h1>
        <p>Count: ${this.count}</p>
      </div>
    `);
  }
}

customElements.define('my-component', MyComponent);
```

### Property Definitions

Properties are defined with type conversion and optional attribute reflection:

```javascript
static get properties() {
  return {
    // String property with reflection
    name: { type: String, reflect: true },

    // Number with default value
    age: { type: Number, default: 18 },

    // Boolean (reflected as presence/absence)
    disabled: { type: Boolean, reflect: true },

    // Object with factory default
    config: { type: Object, default: () => ({ theme: 'light' }) },

    // Array
    items: { type: Array, default: () => [] }
  };
}
```

### Lifecycle Hooks

```javascript
class MyComponent extends TerraphimElement {
  onConnected() {
    // Called when element is added to DOM
    this.setupEventListeners();
  }

  onDisconnected() {
    // Called when element is removed from DOM
    // Automatic cleanup already handled
  }

  onAttributeChanged(name, oldValue, newValue) {
    // Called when observed attribute changes
    console.log(`${name}: ${oldValue} -> ${newValue}`);
  }

  propertyChangedCallback(name, oldValue, newValue) {
    // Called when property changes
    // Default behavior: schedule re-render
  }
}
```

### Event Handling

```javascript
class MyComponent extends TerraphimElement {
  onConnected() {
    // Listen to own events (auto cleanup on disconnect)
    this.listen('click', this.handleClick);

    // Listen to other elements
    this.listenTo(window, 'resize', this.handleResize);

    // Custom cleanup
    const interval = setInterval(() => this.update(), 1000);
    this.addCleanup(() => clearInterval(interval));
  }

  handleClick(event) {
    // Emit custom event
    this.emit('item-selected', { id: this.id });
  }
}
```

### DOM Utilities

```javascript
class MyComponent extends TerraphimElement {
  render() {
    // Set innerHTML safely
    this.setHTML(this.shadowRoot, `<div>Content</div>`);

    // Query single element
    const button = this.$('button.primary');

    // Query all elements
    const items = this.$$('.list-item');

    // Request re-render
    this.requestUpdate();
  }
}
```

## TerraphimObservable

Mixin that adds reactive state management using ES6 Proxies.

### Features

- **Automatic Change Detection**: Tracks all property changes
- **Path-Based Subscriptions**: Subscribe to specific paths or all changes
- **Nested Object Support**: Deeply observe nested objects and arrays
- **Batch Updates**: Automatically batches changes for performance
- **Parent Path Notifications**: Changes notify parent paths

### Basic Usage

```javascript
class CounterComponent extends TerraphimObservable(TerraphimElement) {
  constructor() {
    super();

    // Create observable state
    this.state = this.observe({
      count: 0,
      user: {
        name: 'Alice',
        age: 30
      }
    });

    // Subscribe to all changes
    this.subscribe('*', () => {
      this.render();
    });

    // Subscribe to specific property
    this.subscribe('count', (path, oldVal, newVal) => {
      console.log(`Count: ${oldVal} -> ${newVal}`);
    });

    // Subscribe to nested property
    this.subscribe('user.name', (path, oldVal, newVal) => {
      console.log(`Name changed to ${newVal}`);
    });
  }

  increment() {
    this.state.count++; // Automatically triggers subscribers
  }
}
```

### Subscriptions

```javascript
// Subscribe to specific path
const unsubscribe = this.subscribe('user.name', (path, old, val) => {
  console.log(`Name: ${old} -> ${val}`);
});

// Subscribe to parent path (notified on any child change)
this.subscribe('user', (path, old, val) => {
  console.log(`User object changed at ${path}`);
});

// Subscribe to all changes
this.subscribe('*', (path, old, val) => {
  console.log(`Something changed: ${path}`);
});

// Unsubscribe
unsubscribe();
```

### Batch Updates

```javascript
// Automatic batching (default)
this.state.count++;
this.state.user.name = 'Bob';
this.state.items.push(newItem);
// All changes batched and notified together

// Explicit batch
await this.batch(() => {
  this.state.a = 1;
  this.state.b = 2;
  this.state.c = 3;
});

// Disable batching temporarily
this.withoutBatching(() => {
  this.state.count++; // Notifies immediately
  this.state.count++; // Notifies immediately
});
```

### Advanced Features

```javascript
// Get subscriber count
const count = this.getSubscriberCount('user.name');

// Clear all subscriptions
this.clearSubscriptions();

// Multiple observable objects
const state1 = this.observe({ count: 0 });
const state2 = this.observe({ name: 'Alice' });
```

## TerraphimEvents

Global event bus for cross-component communication without tight coupling.

### Features

- **Global Event Bus**: Singleton for application-wide events
- **Namespace Support**: Organize events with colon-separated namespaces
- **Once Listeners**: Automatic cleanup after first invocation
- **Component Mixin**: Integrate event bus with auto-cleanup
- **Shadow DOM Compatible**: Events properly composed for Shadow DOM

### Global Event Bus

```javascript
import { TerraphimEvents } from './base/terraphim-events.js';

// Listen for events
TerraphimEvents.on('user:login', (userData) => {
  console.log('User logged in:', userData);
});

// Emit events
TerraphimEvents.emit('user:login', { id: 123, name: 'Alice' });

// One-time listeners
TerraphimEvents.once('app:ready', () => {
  console.log('App initialized');
});

// Remove listeners
const handler = (data) => console.log(data);
TerraphimEvents.on('data:update', handler);
TerraphimEvents.off('data:update', handler);

// Check listeners
if (TerraphimEvents.hasListeners('search:complete')) {
  console.log('Someone is listening');
}
```

### Component Mixin

```javascript
class SearchComponent extends TerraphimEventBus(TerraphimElement) {
  onConnected() {
    // Subscribe with automatic cleanup on disconnect
    this.onGlobal('theme:changed', (theme) => {
      this.updateTheme(theme);
    });

    this.onceGlobal('config:loaded', (config) => {
      this.initialize(config);
    });
  }

  performSearch(query) {
    // Emit global events
    this.emitGlobal('search:start', { query });

    // ... perform search ...

    this.emitGlobal('search:complete', { results });
  }
}
```

### Namespace Patterns

```javascript
// User events
TerraphimEvents.on('user:login', handler);
TerraphimEvents.on('user:logout', handler);
TerraphimEvents.on('user:profile-update', handler);

// Data events
TerraphimEvents.on('data:fetched', handler);
TerraphimEvents.on('data:updated', handler);
TerraphimEvents.on('data:error', handler);

// UI events
TerraphimEvents.on('ui:notification', handler);
TerraphimEvents.on('ui:modal-open', handler);
TerraphimEvents.on('ui:modal-close', handler);
```

### Helper Functions

```javascript
import { createEvent } from './base/terraphim-events.js';

// Create Shadow DOM compatible events
const event = createEvent('item-selected', { id: 123 });
element.dispatchEvent(event);

// Custom options
const event = createEvent('delete', null, {
  bubbles: false,
  cancelable: true
});
```

## Combining Features

Create powerful components by combining all three features:

```javascript
class TodoList extends TerraphimEventBus(TerraphimObservable(TerraphimElement)) {
  static get properties() {
    return {
      filter: { type: String, reflect: true, default: 'all' }
    };
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });

    // Observable state
    this.state = this.observe({
      todos: [],
      newTodoText: ''
    });

    // Re-render on any state change
    this.subscribe('*', () => this.render());
  }

  onConnected() {
    // Listen to global events
    this.onGlobal('todo:add', (todo) => {
      this.state.todos.push(todo);
    });

    // Listen to local events
    this.listen('submit', (e) => {
      e.preventDefault();
      this.addTodo();
    });
  }

  addTodo() {
    const todo = {
      id: Date.now(),
      text: this.state.newTodoText,
      done: false
    };

    this.state.todos.push(todo);
    this.state.newTodoText = '';

    // Emit global event
    this.emitGlobal('todo:added', todo);
  }

  get filteredTodos() {
    switch (this.filter) {
      case 'active':
        return this.state.todos.filter(t => !t.done);
      case 'completed':
        return this.state.todos.filter(t => t.done);
      default:
        return this.state.todos;
    }
  }

  render() {
    this.setHTML(this.shadowRoot, `
      <style>
        .todo-list { list-style: none; padding: 0; }
        .todo-item { padding: 10px; border-bottom: 1px solid #eee; }
        .done { text-decoration: line-through; opacity: 0.6; }
      </style>

      <form>
        <input type="text" value="${this.state.newTodoText}"
               placeholder="What needs to be done?">
        <button type="submit">Add</button>
      </form>

      <ul class="todo-list">
        ${this.filteredTodos.map(todo => `
          <li class="todo-item ${todo.done ? 'done' : ''}">
            <input type="checkbox" ${todo.done ? 'checked' : ''}
                   data-id="${todo.id}">
            <span>${todo.text}</span>
          </li>
        `).join('')}
      </ul>
    `);

    // Attach event listeners after render
    this.$$('input[type="checkbox"]').forEach(checkbox => {
      checkbox.addEventListener('change', (e) => {
        const id = parseInt(e.target.dataset.id);
        const todo = this.state.todos.find(t => t.id === id);
        if (todo) {
          todo.done = e.target.checked;
        }
      });
    });

    this.$('input[type="text"]').addEventListener('input', (e) => {
      this.state.newTodoText = e.target.value;
    });
  }
}

customElements.define('todo-list', TodoList);
```

## TerraphimState

Global state management system with path-based access, subscriptions, and persistence. Ideal for application-wide state that needs to be shared across components.

### Features

- **Path-Based Access**: Get/set values using dot notation (`user.name`, `config.haystacks.0`)
- **Reactive Subscriptions**: Subscribe to specific paths with wildcard support
- **Persistence**: Built-in localStorage integration
- **Batch Updates**: Group multiple changes into single notifications
- **Debugging Tools**: Snapshots, time travel, history tracking
- **Middleware System**: Plugin architecture for validation, logging, etc.
- **Component Integration**: Seamless binding with TerraphimElement

### Basic Usage

```javascript
import { TerraphimState, createGlobalState } from './base/terraphim-state.js';

// Create global state instance
const globalState = createGlobalState({
  theme: 'light',
  user: { name: 'Alice', role: 'admin' },
  config: { haystacks: [] }
}, {
  persist: true,
  storagePrefix: 'terraphim'
});

// Subscribe to changes
globalState.subscribe('theme', (value) => {
  console.log('Theme changed:', value);
});

// Update state
globalState.set('theme', 'dark');

// Get values
const theme = globalState.get('theme');
const userName = globalState.get('user.name');

// Batch updates
globalState.batch(() => {
  globalState.set('user.name', 'Bob');
  globalState.set('user.role', 'user');
});
```

### Component Integration

TerraphimElement provides built-in methods for state binding:

```javascript
class ThemeComponent extends TerraphimElement {
  static get properties() {
    return {
      currentTheme: { type: String }
    };
  }

  onConnected() {
    // Bind state to property (automatic cleanup on disconnect)
    this.bindState(globalState, 'theme', 'currentTheme', {
      immediate: true
    });

    // Or bind to callback
    this.bindState(globalState, 'user.name', (value) => {
      this.$('.username').textContent = value;
    });
  }

  handleClick() {
    // Update state
    const current = this.getState(globalState, 'theme');
    const newTheme = current === 'light' ? 'dark' : 'light';
    this.setState(globalState, 'theme', newTheme);
  }
}
```

### Helper Utilities

```javascript
import {
  computed,
  derived,
  createAction,
  validate,
  waitFor
} from './base/state-helpers.js';

// Computed values
const fullName = computed(
  globalState,
  ['user.firstName', 'user.lastName'],
  (first, last) => `${first} ${last}`
);

// Derived stores
const userCount = derived(
  globalState,
  ['users'],
  (users) => users.length,
  0
);

// Actions
const setTheme = createAction(globalState, 'setTheme', (state, theme) => {
  if (!['light', 'dark'].includes(theme)) {
    throw new Error('Invalid theme');
  }
  state.set('theme', theme);
});

// Validation
const validator = validate({
  'user.email': (value) => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)
});
globalState.use(validator);

// Wait for condition
await waitFor(globalState, 'user', (user) => user !== null);
```

### Wildcard Subscriptions

```javascript
// Subscribe to all items in array
globalState.subscribe('config.haystacks.*', (value, oldValue, path) => {
  console.log(`Haystack changed at ${path}`);
});

// Subscribe to parent (with deep option)
globalState.subscribe('user', (value, oldValue, path) => {
  console.log('User or nested property changed');
}, { deep: true });
```

### Advanced Features

```javascript
// Debounced subscriptions
globalState.subscribe('search.query', handleSearch, {
  debounce: 300
});

// RequestAnimationFrame subscriptions
globalState.subscribe('animation.progress', updateUI, {
  useRAF: true
});

// Once subscriptions
globalState.subscribe('initialized', onInit, {
  once: true
});

// Snapshots and time travel (debug mode)
const state = new TerraphimState({ value: 0 }, { debug: true });
state.set('value', 1);
state.set('value', 2);
state.undo(1); // Go back to value: 1
```

### Documentation

For complete API reference and migration guide:

- **[STATE-MANAGEMENT.md](./STATE-MANAGEMENT.md)** - Complete API documentation
- **[MIGRATION-GUIDE.md](./MIGRATION-GUIDE.md)** - Migrating from Svelte stores

### Examples

See `components/examples/` for working examples:

- `state-theme-switcher.html` - Theme switching with persistence
- `state-role-selector.html` - Role selection with derived values
- `state-search-input.html` - Debounced search with async updates

## Testing

Open `__tests__/test-runner.html` in a browser to run the test suite. No build tools or test runners required.

```bash
# Just open in browser
open components/base/__tests__/test-runner.html
```

All tests are pure JavaScript and run directly in the browser.

## API Reference

### TerraphimElement

#### Static Properties

- `observedAttributes` - Array of attribute names to observe
- `properties` - Property definitions with type and reflection

#### Instance Properties

- `_isConnected` - Boolean indicating connection state
- `_cleanupFunctions` - Array of cleanup functions

#### Instance Methods

- `onConnected()` - Lifecycle hook for connection
- `onDisconnected()` - Lifecycle hook for disconnection
- `onAttributeChanged(name, oldValue, newValue)` - Attribute change hook
- `propertyChangedCallback(name, oldValue, newValue)` - Property change hook
- `emit(eventName, detail, options)` - Emit custom event
- `listen(eventName, handler, options)` - Add event listener with cleanup
- `listenTo(target, eventName, handler, options)` - Listen to external element
- `addCleanup(fn)` - Register cleanup function
- `$(selector)` - Query single element
- `$$(selector)` - Query all elements
- `setHTML(target, html)` - Set innerHTML safely
- `requestUpdate()` - Request re-render

### TerraphimObservable

#### Instance Methods

- `observe(obj)` - Make object observable
- `subscribe(path, callback)` - Subscribe to changes
- `unsubscribe(path, callback)` - Remove subscription
- `batch(fn)` - Execute with batched updates
- `withoutBatching(fn)` - Execute with immediate updates
- `getSubscriberCount(path)` - Get subscriber count
- `clearSubscriptions()` - Clear all subscriptions

### TerraphimEvents

#### Static Methods

- `on(eventName, handler)` - Add event listener
- `once(eventName, handler)` - Add one-time listener
- `off(eventName, handler)` - Remove listener
- `emit(eventName, data)` - Emit event
- `getListenerCount(eventName)` - Get listener count
- `hasListeners(eventName)` - Check for listeners
- `clear(eventName)` - Clear listeners
- `getEventNames()` - Get all event names

### TerraphimEventBus

#### Instance Methods

- `onGlobal(eventName, handler)` - Subscribe to global event
- `onceGlobal(eventName, handler)` - Subscribe once to global event
- `emitGlobal(eventName, data)` - Emit global event
- `offGlobal(eventName, handler)` - Remove global listener

## Best Practices

### Component Structure

```javascript
class MyComponent extends TerraphimObservable(TerraphimElement) {
  // 1. Static properties first
  static get observedAttributes() { return []; }
  static get properties() { return {}; }

  // 2. Constructor - minimal setup
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this.state = this.observe({});
  }

  // 3. Lifecycle hooks
  onConnected() {}
  onDisconnected() {}

  // 4. Event handlers
  handleClick(event) {}

  // 5. Render method
  render() {}
}
```

### Performance Tips

1. **Use batching** - Observable changes are batched by default
2. **Debounce renders** - TerraphimElement automatically debounces renders
3. **Subscribe wisely** - Use specific paths instead of wildcard when possible
4. **Clean up** - Always use `listen`/`listenTo` for automatic cleanup
5. **Shadow DOM** - Encapsulate styles and reduce global CSS impact

### Common Patterns

#### Form Handling

```javascript
class FormComponent extends TerraphimElement {
  onConnected() {
    this.listen('submit', (e) => {
      e.preventDefault();
      const formData = new FormData(e.target);
      this.emit('form-submit', Object.fromEntries(formData));
    });
  }
}
```

#### Async Data Loading

```javascript
class DataComponent extends TerraphimObservable(TerraphimElement) {
  constructor() {
    super();
    this.state = this.observe({
      loading: false,
      data: null,
      error: null
    });
  }

  async loadData() {
    this.state.loading = true;
    this.state.error = null;

    try {
      const response = await fetch('/api/data');
      this.state.data = await response.json();
    } catch (error) {
      this.state.error = error.message;
    } finally {
      this.state.loading = false;
    }
  }
}
```

## Browser Support

Requires modern browser with support for:

- ES6+ (classes, modules, Proxy, etc.)
- Custom Elements v1
- Shadow DOM v1
- ES Modules

Tested in:
- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

## License

MIT License - See project root for details

## Contributing

This is part of the Terraphim AI project. See main project documentation for contribution guidelines.
