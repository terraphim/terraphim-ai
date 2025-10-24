/**
 * @fileoverview Tests for TerraphimObservable mixin
 */

import { TerraphimElement } from '../terraphim-element.js';
import { TerraphimObservable } from '../terraphim-observable.js';

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
    console.log('Running TerraphimObservable tests...\n');

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

// Create test component with observable mixin
function createObservableComponent() {
  class TestComponent extends TerraphimObservable(TerraphimElement) {
    constructor() {
      super();
    }
  }

  const tagName = `test-observable-${Math.random().toString(36).substr(2, 9)}`;
  customElements.define(tagName, TestComponent);
  return { tagName, TestComponent };
}

// Tests

runner.test('Should create observable objects', () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({ count: 0 });

  assert.ok(state, 'Should create observable object');
  assert.equal(state.count, 0, 'Should preserve initial values');
});

runner.test('Should detect simple property changes', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({ count: 0 });
  let notified = false;
  let receivedPath = null;

  element.subscribe('count', (path, oldVal, newVal) => {
    notified = true;
    receivedPath = path;
  });

  state.count = 1;

  // Wait for batched notification
  await new Promise(resolve => setTimeout(resolve, 0));

  assert.ok(notified, 'Should notify on property change');
  assert.equal(receivedPath, 'count', 'Should receive correct path');
});

runner.test('Should detect nested property changes', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({
    user: { name: 'Alice', age: 30 }
  });

  let changes = [];

  element.subscribe('user.name', (path, oldVal, newVal) => {
    changes.push({ path, oldVal, newVal });
  });

  state.user.name = 'Bob';

  // Wait for batched notification
  await new Promise(resolve => setTimeout(resolve, 0));

  assert.equal(changes.length, 1, 'Should notify on nested change');
  assert.equal(changes[0].path, 'user.name', 'Should have correct path');
  assert.equal(changes[0].oldVal, 'Alice', 'Should have old value');
  assert.equal(changes[0].newVal, 'Bob', 'Should have new value');
});

runner.test('Should support wildcard subscriptions', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({
    foo: 'bar',
    baz: 'qux'
  });

  let changeCount = 0;

  element.subscribe('*', () => {
    changeCount++;
  });

  state.foo = 'changed';
  state.baz = 'changed';

  // Wait for batched notifications
  await new Promise(resolve => setTimeout(resolve, 0));

  assert.equal(changeCount, 2, 'Should notify wildcard for all changes');
});

runner.test('Should notify parent paths', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({
    user: { profile: { name: 'Alice' } }
  });

  let userNotified = false;
  let profileNotified = false;

  element.subscribe('user', () => {
    userNotified = true;
  });

  element.subscribe('user.profile', () => {
    profileNotified = true;
  });

  state.user.profile.name = 'Bob';

  // Wait for batched notifications
  await new Promise(resolve => setTimeout(resolve, 0));

  assert.ok(userNotified, 'Should notify parent path "user"');
  assert.ok(profileNotified, 'Should notify parent path "user.profile"');
});

runner.test('Should support unsubscribe', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({ count: 0 });

  let notifyCount = 0;
  const unsubscribe = element.subscribe('count', () => {
    notifyCount++;
  });

  state.count = 1;
  await new Promise(resolve => setTimeout(resolve, 0));

  unsubscribe();

  state.count = 2;
  await new Promise(resolve => setTimeout(resolve, 0));

  assert.equal(notifyCount, 1, 'Should not notify after unsubscribe');
});

runner.test('Should batch multiple changes', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({ a: 1, b: 2, c: 3 });

  let notifyCount = 0;
  element.subscribe('*', () => {
    notifyCount++;
  });

  // Make multiple changes synchronously
  state.a = 10;
  state.b = 20;
  state.c = 30;

  // Wait for batched notification
  await new Promise(resolve => setTimeout(resolve, 0));

  assert.equal(notifyCount, 3, 'Should batch notifications (one per change path)');
});

runner.test('Should support withoutBatching', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({ count: 0 });

  let notifyCount = 0;
  element.subscribe('count', () => {
    notifyCount++;
  });

  element.withoutBatching(() => {
    state.count = 1;
    state.count = 2;
    state.count = 3;
  });

  assert.equal(notifyCount, 3, 'Should notify immediately without batching');
});

runner.test('Should support batch() method', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({ a: 1, b: 2 });

  let notifyCount = 0;
  element.subscribe('*', () => {
    notifyCount++;
  });

  await element.batch(() => {
    state.a = 10;
    state.b = 20;
  });

  assert.equal(notifyCount, 2, 'Should batch all changes together');
});

runner.test('Should handle array operations', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({ items: [1, 2, 3] });

  let changes = [];
  element.subscribe('items', (path) => {
    changes.push(path);
  });

  state.items.push(4);
  await new Promise(resolve => setTimeout(resolve, 0));

  assert.ok(changes.length > 0, 'Should detect array modifications');
});

runner.test('Should handle delete operations', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({ foo: 'bar', baz: 'qux' });

  let deleted = [];
  element.subscribe('*', (path, oldVal, newVal) => {
    if (newVal === undefined) {
      deleted.push(path);
    }
  });

  delete state.foo;
  await new Promise(resolve => setTimeout(resolve, 0));

  assert.equal(deleted.length, 1, 'Should detect property deletion');
  assert.equal(deleted[0], 'foo', 'Should have correct deleted path');
});

runner.test('Should get subscriber count', () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  element.subscribe('count', () => {});
  element.subscribe('count', () => {});
  element.subscribe('name', () => {});

  assert.equal(element.getSubscriberCount('count'), 2, 'Should count subscribers correctly');
  assert.equal(element.getSubscriberCount('name'), 1, 'Should count subscribers correctly');
  assert.equal(element.getSubscriberCount('other'), 0, 'Should return 0 for no subscribers');
});

runner.test('Should clear all subscriptions', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({ count: 0 });

  let notifyCount = 0;
  element.subscribe('count', () => {
    notifyCount++;
  });

  element.clearSubscriptions();

  state.count = 1;
  await new Promise(resolve => setTimeout(resolve, 0));

  assert.equal(notifyCount, 0, 'Should not notify after clearing subscriptions');
});

runner.test('Should handle multiple observable objects', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state1 = element.observe({ count: 0 });
  const state2 = element.observe({ name: 'Alice' });

  let changes = [];

  element.subscribe('*', (path) => {
    changes.push(path);
  });

  state1.count = 1;
  state2.name = 'Bob';

  await new Promise(resolve => setTimeout(resolve, 0));

  assert.equal(changes.length, 2, 'Should track changes across multiple observables');
});

runner.test('Should not notify on same value', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({ count: 5 });

  let notifyCount = 0;
  element.subscribe('count', () => {
    notifyCount++;
  });

  state.count = 5; // Same value
  await new Promise(resolve => setTimeout(resolve, 0));

  assert.equal(notifyCount, 0, 'Should not notify when value is unchanged');
});

runner.test('Should support unsubscribe by path', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({ count: 0 });

  let notifyCount = 0;
  element.subscribe('count', () => {
    notifyCount++;
  });
  element.subscribe('count', () => {
    notifyCount++;
  });

  element.unsubscribe('count'); // Remove all

  state.count = 1;
  await new Promise(resolve => setTimeout(resolve, 0));

  assert.equal(notifyCount, 0, 'Should remove all subscribers for path');
});

runner.test('Should handle deeply nested objects', async () => {
  const { tagName } = createObservableComponent();
  const element = document.createElement(tagName);

  const state = element.observe({
    level1: {
      level2: {
        level3: {
          value: 'deep'
        }
      }
    }
  });

  let changes = [];
  element.subscribe('level1.level2.level3.value', (path, old, val) => {
    changes.push({ path, old, val });
  });

  state.level1.level2.level3.value = 'changed';
  await new Promise(resolve => setTimeout(resolve, 0));

  assert.equal(changes.length, 1, 'Should detect deeply nested changes');
  assert.equal(changes[0].val, 'changed', 'Should have correct new value');
});

// Run all tests
export async function runTests() {
  return await runner.run();
}

// Auto-run if loaded directly
if (typeof window !== 'undefined') {
  window.runTerraphimObservableTests = runTests;
}
