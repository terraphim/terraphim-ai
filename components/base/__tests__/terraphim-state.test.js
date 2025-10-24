/**
 * @fileoverview Tests for TerraphimState class
 */

import { TerraphimState, createGlobalState, getGlobalState } from '../terraphim-state.js';

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
  },

  throws(fn, message = '') {
    let thrown = false;
    try {
      fn();
    } catch (e) {
      thrown = true;
    }
    if (!thrown) {
      throw new Error(`${message}\nExpected function to throw`);
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
    console.log('Running TerraphimState tests...\n');

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

// Tests

runner.test('TerraphimState should be defined', () => {
  assert.ok(TerraphimState, 'TerraphimState class should exist');
  assert.ok(TerraphimState.prototype instanceof EventTarget, 'Should extend EventTarget');
});

runner.test('Should initialize with empty state by default', () => {
  const state = new TerraphimState();
  assert.deepEqual(state.getSnapshot(), {}, 'Should have empty initial state');
});

runner.test('Should initialize with provided state', () => {
  const initialState = { theme: 'dark', count: 42 };
  const state = new TerraphimState(initialState);
  assert.deepEqual(state.getSnapshot(), initialState, 'Should initialize with provided state');
});

runner.test('Should get values by path', () => {
  const state = new TerraphimState({
    user: { name: 'Alice', age: 30 },
    config: { theme: 'dark' }
  });

  assert.equal(state.get('user.name'), 'Alice', 'Should get nested value');
  assert.equal(state.get('user.age'), 30, 'Should get number value');
  assert.equal(state.get('config.theme'), 'dark', 'Should get deep value');
  assert.deepEqual(state.get('user'), { name: 'Alice', age: 30 }, 'Should get object');
});

runner.test('Should set values by path', () => {
  const state = new TerraphimState();

  state.set('theme', 'dark');
  assert.equal(state.get('theme'), 'dark', 'Should set simple value');

  state.set('user.name', 'Bob');
  assert.equal(state.get('user.name'), 'Bob', 'Should set nested value');

  state.set('config.haystacks.0.name', 'GitHub');
  assert.equal(state.get('config.haystacks.0.name'), 'GitHub', 'Should set array item');
});

runner.test('Should create intermediate objects when setting nested paths', () => {
  const state = new TerraphimState();

  state.set('a.b.c.d', 'value');
  assert.equal(state.get('a.b.c.d'), 'value', 'Should create all intermediate objects');
  assert.ok(typeof state.get('a') === 'object', 'Should create object at a');
  assert.ok(typeof state.get('a.b') === 'object', 'Should create object at a.b');
  assert.ok(typeof state.get('a.b.c') === 'object', 'Should create object at a.b.c');
});

runner.test('Should subscribe to value changes', async () => {
  const state = new TerraphimState({ count: 0 });

  let called = false;
  let receivedValue = null;

  state.subscribe('count', (value) => {
    called = true;
    receivedValue = value;
  });

  state.set('count', 42);

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.ok(called, 'Subscription should be called');
  assert.equal(receivedValue, 42, 'Should receive new value');
});

runner.test('Should subscribe to nested paths', async () => {
  const state = new TerraphimState({ user: { name: 'Alice' } });

  let receivedValue = null;

  state.subscribe('user.name', (value) => {
    receivedValue = value;
  });

  state.set('user.name', 'Bob');

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(receivedValue, 'Bob', 'Should receive nested value change');
});

runner.test('Should support wildcard subscriptions', async () => {
  const state = new TerraphimState({ items: [{ id: 1 }, { id: 2 }] });

  let changedPath = null;

  state.subscribe('items.*', (value, oldValue, path) => {
    changedPath = path;
  });

  state.set('items.0.name', 'Item 1');

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(changedPath, 'items.0.name', 'Should match wildcard subscription');
});

runner.test('Should notify parent path subscribers on deep changes', async () => {
  const state = new TerraphimState({ user: { profile: { name: 'Alice' } } });

  let notified = false;

  state.subscribe('user', (value, oldValue, path) => {
    notified = true;
  }, { deep: true });

  state.set('user.profile.name', 'Bob');

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.ok(notified, 'Parent path subscriber should be notified with deep option');
});

runner.test('Should call subscription immediately with immediate option', () => {
  const state = new TerraphimState({ value: 42 });

  let receivedValue = null;

  state.subscribe('value', (value) => {
    receivedValue = value;
  }, { immediate: true });

  assert.equal(receivedValue, 42, 'Should call immediately with current value');
});

runner.test('Should unsubscribe correctly', async () => {
  const state = new TerraphimState({ count: 0 });

  let callCount = 0;

  const unsubscribe = state.subscribe('count', () => {
    callCount++;
  });

  state.set('count', 1);
  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(callCount, 1, 'Should be called before unsubscribe');

  unsubscribe();

  state.set('count', 2);
  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(callCount, 1, 'Should not be called after unsubscribe');
});

runner.test('Should batch multiple updates', async () => {
  const state = new TerraphimState({ a: 1, b: 2, c: 3 });

  let callCount = 0;

  state.subscribe('a', () => callCount++);
  state.subscribe('b', () => callCount++);
  state.subscribe('c', () => callCount++);

  state.batch(() => {
    state.set('a', 10);
    state.set('b', 20);
    state.set('c', 30);
  });

  await new Promise(resolve => setTimeout(resolve, 10));

  // Each subscription should be called once for the batch
  assert.equal(callCount, 3, 'Should batch notifications');
});

runner.test('Should handle middleware', () => {
  const state = new TerraphimState();

  let middlewareCalled = false;
  let action = null;
  let payload = null;

  state.use((a, p) => {
    middlewareCalled = true;
    action = a;
    payload = p;
  });

  state.set('value', 42);

  assert.ok(middlewareCalled, 'Middleware should be called');
  assert.equal(action, 'set', 'Should pass action name');
  assert.deepEqual(payload, { path: 'value', value: 42 }, 'Should pass payload');
});

runner.test('Should cancel operation when middleware returns false', () => {
  const state = new TerraphimState({ value: 1 });

  state.use((action, payload) => {
    if (payload.path === 'value' && payload.value === 999) {
      return false; // Cancel
    }
  });

  state.set('value', 999);

  assert.equal(state.get('value'), 1, 'Should not update when middleware cancels');
});

runner.test('Should get and restore snapshots', () => {
  const state = new TerraphimState({ a: 1, b: 2 });

  const snapshot1 = state.getSnapshot();

  state.set('a', 10);
  state.set('b', 20);

  assert.equal(state.get('a'), 10, 'Should have new value');

  state.restoreSnapshot(snapshot1);

  assert.equal(state.get('a'), 1, 'Should restore to snapshot');
  assert.equal(state.get('b'), 2, 'Should restore all values');
});

runner.test('Should support time travel with debug mode', () => {
  const state = new TerraphimState({ value: 0 }, { debug: true });

  state.set('value', 1);
  state.set('value', 2);
  state.set('value', 3);

  assert.equal(state.get('value'), 3, 'Should have latest value');

  state.undo(1);
  assert.equal(state.get('value'), 2, 'Should undo one step');

  state.undo(2);
  assert.equal(state.get('value'), 0, 'Should undo multiple steps');

  state.redo(1);
  assert.equal(state.get('value'), 1, 'Should redo one step');
});

runner.test('Should clear state', async () => {
  const state = new TerraphimState({ a: 1, b: 2 });

  let notified = false;
  state.subscribe('a', () => { notified = true; });

  state.clear();

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.deepEqual(state.getSnapshot(), {}, 'Should clear all state');
  assert.ok(notified, 'Should notify subscribers');
});

runner.test('Should reset to new initial state', () => {
  const state = new TerraphimState({ old: 'value' });

  state.reset({ new: 'value' });

  assert.deepEqual(state.getSnapshot(), { new: 'value' }, 'Should reset to new state');
});

runner.test('Should debounce subscriptions', async () => {
  const state = new TerraphimState({ count: 0 });

  let callCount = 0;

  state.subscribe('count', () => {
    callCount++;
  }, { debounce: 50 });

  state.set('count', 1);
  state.set('count', 2);
  state.set('count', 3);

  // Should not be called yet
  assert.equal(callCount, 0, 'Should not call before debounce delay');

  await new Promise(resolve => setTimeout(resolve, 60));

  assert.equal(callCount, 1, 'Should call once after debounce delay');
});

runner.test('Should use requestAnimationFrame with useRAF option', async () => {
  const state = new TerraphimState({ count: 0 });

  let callCount = 0;

  state.subscribe('count', () => {
    callCount++;
  }, { useRAF: true });

  state.set('count', 1);
  state.set('count', 2);
  state.set('count', 3);

  // Should not be called yet
  assert.equal(callCount, 0, 'Should not call immediately');

  await new Promise(resolve => requestAnimationFrame(resolve));

  assert.equal(callCount, 1, 'Should call once after RAF');
});

runner.test('Should support custom compare function', async () => {
  const state = new TerraphimState({ value: { count: 0 } });

  let callCount = 0;

  state.subscribe('value', () => {
    callCount++;
  }, {
    compare: (a, b) => JSON.stringify(a) === JSON.stringify(b)
  });

  state.set('value', { count: 0 });
  await new Promise(resolve => setTimeout(resolve, 10));

  // Should not be called because values are considered equal
  assert.equal(callCount, 0, 'Should not call when compare returns true');

  state.set('value', { count: 1 });
  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(callCount, 1, 'Should call when compare returns false');
});

runner.test('Should handle once option', async () => {
  const state = new TerraphimState({ count: 0 });

  let callCount = 0;

  state.subscribe('count', () => {
    callCount++;
  }, { once: true });

  state.set('count', 1);
  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(callCount, 1, 'Should call once');

  state.set('count', 2);
  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(callCount, 1, 'Should not call again after once');
});

runner.test('Should dispatch custom events on state changes', async () => {
  const state = new TerraphimState({ value: 1 });

  let eventDetail = null;

  state.addEventListener('state-changed', (e) => {
    eventDetail = e.detail;
  });

  state.set('value', 42);

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.ok(eventDetail, 'Should dispatch custom event');
  assert.equal(eventDetail.path, 'value', 'Should include path');
  assert.equal(eventDetail.newValue, 42, 'Should include new value');
  assert.equal(eventDetail.oldValue, 1, 'Should include old value');
});

runner.test('Should create global state singleton', () => {
  // Clear any existing global state
  delete window.__TERRAPHIM_STATE__;

  const state1 = createGlobalState({ test: true });
  const state2 = createGlobalState({ test: false });

  assert.equal(state1, state2, 'Should return same instance');
  assert.equal(state1.get('test'), true, 'Should keep first state');
});

runner.test('Should get global state', () => {
  const state = createGlobalState({ value: 123 });
  const retrieved = getGlobalState();

  assert.equal(state, retrieved, 'Should retrieve global state');
});

runner.test('Should handle array indices in paths', () => {
  const state = new TerraphimState({
    items: [
      { name: 'Item 1' },
      { name: 'Item 2' },
      { name: 'Item 3' }
    ]
  });

  assert.equal(state.get('items.0.name'), 'Item 1', 'Should get first item');
  assert.equal(state.get('items.1.name'), 'Item 2', 'Should get second item');
  assert.equal(state.get('items.2.name'), 'Item 3', 'Should get third item');

  state.set('items.1.name', 'Updated Item');
  assert.equal(state.get('items.1.name'), 'Updated Item', 'Should update array item');
});

runner.test('Should return undefined for non-existent paths', () => {
  const state = new TerraphimState({ a: { b: { c: 1 } } });

  assert.equal(state.get('a.b.c'), 1, 'Should get existing value');
  assert.equal(state.get('a.b.d'), undefined, 'Should return undefined for missing key');
  assert.equal(state.get('x.y.z'), undefined, 'Should return undefined for missing path');
});

runner.test('Should handle null and undefined values', () => {
  const state = new TerraphimState();

  state.set('nullValue', null);
  state.set('undefinedValue', undefined);

  assert.equal(state.get('nullValue'), null, 'Should store null');
  assert.equal(state.get('undefinedValue'), undefined, 'Should store undefined');
});

runner.test('Should handle silent updates', async () => {
  const state = new TerraphimState({ value: 1 });

  let called = false;
  state.subscribe('value', () => { called = true; });

  state.set('value', 2, true); // Silent update

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(state.get('value'), 2, 'Should update value');
  assert.notOk(called, 'Should not notify subscribers');
});

runner.test('Should cache parsed paths for performance', () => {
  const state = new TerraphimState();

  // Access the same path multiple times
  state.set('a.b.c', 1);
  state.get('a.b.c');
  state.get('a.b.c');
  state.get('a.b.c');

  // The path cache should have been used
  assert.ok(state._pathCache.has('a.b.c'), 'Should cache parsed paths');
});

runner.test('Should handle deep cloning correctly', () => {
  const originalData = {
    nested: { array: [1, 2, { deep: 'value' }] }
  };

  const state = new TerraphimState(originalData);
  const snapshot = state.getSnapshot();

  // Modify original
  originalData.nested.array[0] = 999;

  assert.equal(snapshot.nested.array[0], 1, 'Should deep clone state');
  assert.equal(state.get('nested.array.0'), 1, 'State should not be affected');
});

runner.test('Should handle history size limit', () => {
  const state = new TerraphimState({ value: 0 }, { debug: true });

  // Create more than max history size (50)
  for (let i = 1; i <= 60; i++) {
    state.set('value', i);
  }

  const history = state.getHistory();

  assert.ok(history.length <= 50, 'Should limit history size');
});

runner.test('Should support middleware removal', () => {
  const state = new TerraphimState();

  let callCount = 0;
  const removeMiddleware = state.use(() => {
    callCount++;
  });

  state.set('value', 1);
  assert.equal(callCount, 1, 'Should call middleware');

  removeMiddleware();

  state.set('value', 2);
  assert.equal(callCount, 1, 'Should not call after removal');
});

// Run all tests
export async function runTests() {
  return await runner.run();
}

// Auto-run if loaded directly
if (typeof window !== 'undefined') {
  window.runTerraphimStateTests = runTests;
}
