/**
 * @fileoverview Integration tests for TerraphimState + TerraphimElement
 */

import { TerraphimElement } from '../terraphim-element.js';
import { TerraphimState } from '../terraphim-state.js';

// Test utilities
const assert = {
  equal(actual, expected, message = '') {
    if (actual !== expected) {
      throw new Error(`${message}\nExpected: ${expected}\nActual: ${actual}`);
    }
  },

  deepEqual(actual, expected, message = '') {
    const actualStr = JSON.stringify(actual);
    const expectedStr = JSON.stringify(expected);
    if (actualStr !== expectedStr) {
      throw new Error(`${message}\nExpected: ${expectedStr}\nActual: ${actualStr}`);
    }
  },

  ok(value, message = '') {
    if (!value) {
      throw new Error(`${message}\nExpected truthy value, got: ${value}`);
    }
  },

  notOk(value, message = '') {
    if (value) {
      throw new Error(`${message}\nExpected falsy value, got: ${value}`);
    }
  }
};

// Test runner
class TestRunner {
  constructor() {
    this.tests = [];
    this.results = [];
  }

  test(name, fn) {
    this.tests.push({ name, fn });
  }

  async run() {
    console.log('Running State Integration tests...\n');

    for (const { name, fn } of this.tests) {
      try {
        await fn();
        this.results.push({ name, passed: true });
        console.log(`✓ ${name}`);
      } catch (error) {
        this.results.push({ name, passed: false, error });
        console.error(`✗ ${name}`);
        console.error(`  ${error.message}`);
      }
    }

    const passed = this.results.filter(r => r.passed).length;
    const failed = this.results.filter(r => !r.passed).length;

    console.log(`\n${passed} passed, ${failed} failed, ${this.results.length} total`);

    return { passed, failed, total: this.results.length };
  }
}

const runner = new TestRunner();

// Helper to create test component
function createTestComponent(options = {}) {
  class TestComponent extends TerraphimElement {
    static get observedAttributes() {
      return options.observedAttributes || [];
    }

    static get properties() {
      return options.properties || {};
    }

    constructor() {
      super();
      if (options.useShadow) {
        this.attachShadow({ mode: 'open' });
      }
    }

    render() {
      if (options.render) {
        options.render.call(this);
      }
    }

    onConnected() {
      if (options.onConnected) {
        options.onConnected.call(this);
      }
    }

    onDisconnected() {
      if (options.onDisconnected) {
        options.onDisconnected.call(this);
      }
    }
  }

  const tagName = `test-component-${Math.random().toString(36).substr(2, 9)}`;
  customElements.define(tagName, TestComponent);
  return { tagName, TestComponent };
}

// Tests

runner.test('bindState() should bind state to property', async () => {
  const state = new TerraphimState({ theme: 'light' });

  const { tagName } = createTestComponent({
    properties: {
      currentTheme: { type: String }
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  element.bindState(state, 'theme', 'currentTheme');

  state.set('theme', 'dark');

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(element.currentTheme, 'dark', 'Should bind state to property');

  document.body.removeChild(element);
});

runner.test('bindState() should bind state to callback', async () => {
  const state = new TerraphimState({ count: 0 });

  let receivedValue = null;

  const { tagName } = createTestComponent({
    onConnected() {
      this.bindState(state, 'count', (value) => {
        receivedValue = value;
      });
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  state.set('count', 42);

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(receivedValue, 42, 'Should call callback with new value');

  document.body.removeChild(element);
});

runner.test('bindState() should support immediate option', () => {
  const state = new TerraphimState({ value: 123 });

  let receivedValue = null;

  const { tagName } = createTestComponent({
    onConnected() {
      this.bindState(state, 'value', (value) => {
        receivedValue = value;
      }, { immediate: true });
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  assert.equal(receivedValue, 123, 'Should call immediately with current value');

  document.body.removeChild(element);
});

runner.test('bindState() should cleanup on disconnect', async () => {
  const state = new TerraphimState({ value: 0 });

  let callCount = 0;

  const { tagName } = createTestComponent({
    onConnected() {
      this.bindState(state, 'value', () => {
        callCount++;
      });
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  state.set('value', 1);
  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(callCount, 1, 'Should call while connected');

  document.body.removeChild(element);

  state.set('value', 2);
  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(callCount, 1, 'Should not call after disconnect');
});

runner.test('setState() should update state', () => {
  const state = new TerraphimState({ value: 0 });

  const { tagName } = createTestComponent();

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  element.setState(state, 'value', 42);

  assert.equal(state.get('value'), 42, 'Should update state');

  document.body.removeChild(element);
});

runner.test('getState() should read state', () => {
  const state = new TerraphimState({ value: 123 });

  const { tagName } = createTestComponent();

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  const value = element.getState(state, 'value');

  assert.equal(value, 123, 'Should read state');

  document.body.removeChild(element);
});

runner.test('Should handle multiple state bindings', async () => {
  const state = new TerraphimState({
    theme: 'light',
    role: 'user',
    count: 0
  });

  let themeValue = null;
  let roleValue = null;
  let countValue = null;

  const { tagName } = createTestComponent({
    onConnected() {
      this.bindState(state, 'theme', (value) => { themeValue = value; });
      this.bindState(state, 'role', (value) => { roleValue = value; });
      this.bindState(state, 'count', (value) => { countValue = value; });
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  state.set('theme', 'dark');
  state.set('role', 'admin');
  state.set('count', 99);

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(themeValue, 'dark', 'Should update theme');
  assert.equal(roleValue, 'admin', 'Should update role');
  assert.equal(countValue, 99, 'Should update count');

  document.body.removeChild(element);
});

runner.test('Should handle nested path bindings', async () => {
  const state = new TerraphimState({
    user: { profile: { name: 'Alice' } }
  });

  let receivedName = null;

  const { tagName } = createTestComponent({
    onConnected() {
      this.bindState(state, 'user.profile.name', (value) => {
        receivedName = value;
      });
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  state.set('user.profile.name', 'Bob');

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(receivedName, 'Bob', 'Should bind to nested path');

  document.body.removeChild(element);
});

runner.test('Should handle wildcard bindings', async () => {
  const state = new TerraphimState({
    items: [{ id: 1 }, { id: 2 }]
  });

  let changedPath = null;

  const { tagName } = createTestComponent({
    onConnected() {
      this.bindState(state, 'items.*', (value, oldValue, path) => {
        changedPath = path;
      });
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  state.set('items.0.name', 'Item 1');

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.ok(changedPath.includes('items'), 'Should match wildcard binding');

  document.body.removeChild(element);
});

runner.test('Should update component on state change with property binding', async () => {
  const state = new TerraphimState({ title: 'Initial' });

  let renderCount = 0;

  const { tagName } = createTestComponent({
    useShadow: true,
    properties: {
      displayTitle: { type: String }
    },
    onConnected() {
      this.bindState(state, 'title', 'displayTitle');
    },
    render() {
      renderCount++;
      this.setHTML(this.shadowRoot, `<h1>${this.displayTitle}</h1>`);
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  // Wait for initial render
  await new Promise(resolve => requestAnimationFrame(resolve));
  const initialRenders = renderCount;

  state.set('title', 'Updated');

  // Wait for state update and re-render
  await new Promise(resolve => setTimeout(resolve, 10));
  await new Promise(resolve => requestAnimationFrame(resolve));

  assert.ok(renderCount > initialRenders, 'Should trigger re-render');

  const h1 = element.shadowRoot.querySelector('h1');
  assert.equal(h1.textContent, 'Updated', 'Should display updated title');

  document.body.removeChild(element);
});

runner.test('Should handle component with multiple state sources', async () => {
  const globalState = new TerraphimState({ theme: 'light' });
  const localState = new TerraphimState({ count: 0 });

  let themeValue = null;
  let countValue = null;

  const { tagName } = createTestComponent({
    onConnected() {
      this.bindState(globalState, 'theme', (value) => { themeValue = value; });
      this.bindState(localState, 'count', (value) => { countValue = value; });
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  globalState.set('theme', 'dark');
  localState.set('count', 42);

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(themeValue, 'dark', 'Should bind to global state');
  assert.equal(countValue, 42, 'Should bind to local state');

  document.body.removeChild(element);
});

runner.test('Should handle deep state bindings', async () => {
  const state = new TerraphimState({
    user: { profile: { settings: { theme: 'light' } } }
  });

  let notified = false;

  const { tagName } = createTestComponent({
    onConnected() {
      this.bindState(state, 'user', (value, oldValue, path) => {
        notified = true;
      }, { deep: true });
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  state.set('user.profile.settings.theme', 'dark');

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.ok(notified, 'Should notify on deep changes');

  document.body.removeChild(element);
});

runner.test('Should handle state updates during component lifecycle', async () => {
  const state = new TerraphimState({ initialized: false });

  let connectedValue = null;
  let updatedValue = null;

  const { tagName } = createTestComponent({
    onConnected() {
      connectedValue = this.getState(state, 'initialized');
      this.setState(state, 'initialized', true);

      this.bindState(state, 'initialized', (value) => {
        updatedValue = value;
      });
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(connectedValue, false, 'Should read initial state');
  assert.equal(state.get('initialized'), true, 'Should update state in onConnected');
  assert.equal(updatedValue, true, 'Should receive updated value');

  document.body.removeChild(element);
});

runner.test('Should cleanup all state bindings on disconnect', async () => {
  const state = new TerraphimState({
    a: 1,
    b: 2,
    c: 3
  });

  let callCount = 0;

  const { tagName } = createTestComponent({
    onConnected() {
      this.bindState(state, 'a', () => callCount++);
      this.bindState(state, 'b', () => callCount++);
      this.bindState(state, 'c', () => callCount++);
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  state.set('a', 10);
  state.set('b', 20);
  state.set('c', 30);

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(callCount, 3, 'Should call all bindings');

  document.body.removeChild(element);

  callCount = 0;

  state.set('a', 100);
  state.set('b', 200);
  state.set('c', 300);

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(callCount, 0, 'Should cleanup all bindings');
});

runner.test('Should handle state binding with debounce', async () => {
  const state = new TerraphimState({ search: '' });

  let callCount = 0;
  let lastValue = null;

  const { tagName } = createTestComponent({
    onConnected() {
      this.bindState(state, 'search', (value) => {
        callCount++;
        lastValue = value;
      }, { debounce: 50 });
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  state.set('search', 'a');
  state.set('search', 'ab');
  state.set('search', 'abc');

  // Should not be called yet
  assert.equal(callCount, 0, 'Should not call before debounce');

  await new Promise(resolve => setTimeout(resolve, 60));

  assert.equal(callCount, 1, 'Should call once after debounce');
  assert.equal(lastValue, 'abc', 'Should have last value');

  document.body.removeChild(element);
});

runner.test('Should handle state binding with useRAF', async () => {
  const state = new TerraphimState({ value: 0 });

  let callCount = 0;

  const { tagName } = createTestComponent({
    onConnected() {
      this.bindState(state, 'value', () => {
        callCount++;
      }, { useRAF: true });
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  state.set('value', 1);
  state.set('value', 2);
  state.set('value', 3);

  // Should not be called yet
  assert.equal(callCount, 0, 'Should not call before RAF');

  await new Promise(resolve => requestAnimationFrame(resolve));

  assert.equal(callCount, 1, 'Should call once after RAF');

  document.body.removeChild(element);
});

// Run all tests
export async function runTests() {
  return await runner.run();
}

// Auto-run if loaded directly
if (typeof window !== 'undefined') {
  window.runStateIntegrationTests = runTests;
}
