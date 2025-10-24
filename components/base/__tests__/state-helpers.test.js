/**
 * @fileoverview Tests for State Helper Utilities
 */

import { TerraphimState } from '../terraphim-state.js';
import {
  computed,
  derived,
  createAction,
  validate,
  createLogger,
  createPersistence,
  restorePersistedState,
  syncStates,
  createReadonly,
  batchUpdate,
  createSelector,
  waitFor,
  createDebouncedSetter,
  createThrottledSetter
} from '../state-helpers.js';

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
    console.log('Running State Helpers tests...\n');

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

runner.test('computed() should create computed value', () => {
  const state = new TerraphimState({
    user: { firstName: 'John', lastName: 'Doe' }
  });

  const fullName = computed(
    state,
    ['user.firstName', 'user.lastName'],
    (first, last) => `${first} ${last}`
  );

  assert.equal(fullName.value, 'John Doe', 'Should compute initial value');
});

runner.test('computed() should update when dependencies change', async () => {
  const state = new TerraphimState({
    user: { firstName: 'John', lastName: 'Doe' }
  });

  const fullName = computed(
    state,
    ['user.firstName', 'user.lastName'],
    (first, last) => `${first} ${last}`
  );

  state.set('user.firstName', 'Jane');

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(fullName.value, 'Jane Doe', 'Should recompute on dependency change');

  fullName.unsubscribe();
});

runner.test('derived() should create derived store', () => {
  const state = new TerraphimState({ items: [1, 2, 3] });

  const count = derived(
    state,
    ['items'],
    (items) => items.length,
    0
  );

  assert.equal(count.value, 3, 'Should derive initial value');

  count.unsubscribe();
});

runner.test('derived() should notify subscribers', async () => {
  const state = new TerraphimState({ items: [1, 2, 3] });

  const count = derived(
    state,
    ['items'],
    (items) => items.length,
    0
  );

  let received = null;
  count.subscribe((value) => {
    received = value;
  });

  state.set('items', [1, 2, 3, 4]);

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(received, 4, 'Should notify on change');

  count.unsubscribe();
});

runner.test('createAction() should create action function', () => {
  const state = new TerraphimState({ theme: 'light' });

  const setTheme = createAction(state, 'setTheme', (state, theme) => {
    state.set('theme', theme);
  });

  setTheme('dark');

  assert.equal(state.get('theme'), 'dark', 'Should execute action');
});

runner.test('createAction() should handle errors', () => {
  const state = new TerraphimState();

  const failingAction = createAction(state, 'fail', () => {
    throw new Error('Test error');
  });

  assert.throws(() => failingAction(), 'Should throw on error');
});

runner.test('validate() should create validator middleware', () => {
  const state = new TerraphimState({ age: 25 });

  const validator = validate({
    'age': (value) => value >= 0 && value <= 120
  });

  state.use(validator);

  state.set('age', 30);
  assert.equal(state.get('age'), 30, 'Should allow valid value');

  state.set('age', 999);
  assert.equal(state.get('age'), 30, 'Should reject invalid value');
});

runner.test('createLogger() should log state changes', () => {
  const state = new TerraphimState();
  const logs = [];

  // Capture console.log
  const originalLog = console.log;
  console.log = (...args) => logs.push(args);

  const logger = createLogger({ logSet: true });
  state.use(logger);

  state.set('value', 42);

  console.log = originalLog;

  assert.ok(logs.length > 0, 'Should log state changes');
  assert.ok(logs[0].join(' ').includes('SET'), 'Should log SET action');
});

runner.test('createPersistence() should persist specific paths', async () => {
  // Clear localStorage
  localStorage.clear();

  const state = new TerraphimState({ theme: 'light', other: 'value' });

  const persist = createPersistence(['theme'], {
    prefix: 'test',
    debounce: 50
  });

  state.use(persist);

  state.set('theme', 'dark');

  await new Promise(resolve => setTimeout(resolve, 60));

  const stored = localStorage.getItem('test:theme');
  assert.equal(JSON.parse(stored), 'dark', 'Should persist to localStorage');

  localStorage.clear();
});

runner.test('restorePersistedState() should restore from localStorage', () => {
  localStorage.clear();

  // Manually store value
  localStorage.setItem('test:theme', JSON.stringify('dark'));

  const state = new TerraphimState({ theme: 'light' });

  restorePersistedState(state, ['theme'], 'test');

  assert.equal(state.get('theme'), 'dark', 'Should restore from localStorage');

  localStorage.clear();
});

runner.test('syncStates() should sync two states', async () => {
  const source = new TerraphimState({ value: 1 });
  const target = new TerraphimState({ value: 0 });

  const cleanup = syncStates(source, target, { 'value': 'value' });

  source.set('value', 42);

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(target.get('value'), 42, 'Should sync to target');

  cleanup();
});

runner.test('createReadonly() should prevent modifications', () => {
  const state = new TerraphimState({ value: 1 });

  const readonly = createReadonly(state);

  assert.equal(readonly.get('value'), 1, 'Should allow reads');

  assert.throws(() => readonly.set('value', 2), 'Should prevent writes');
  assert.throws(() => readonly.batch(() => {}), 'Should prevent batch');
});

runner.test('createReadonly() should respect allowed paths', () => {
  const state = new TerraphimState({ allowed: 1, denied: 2 });

  const readonly = createReadonly(state, ['allowed']);

  assert.equal(readonly.get('allowed'), 1, 'Should allow reading allowed path');

  assert.throws(
    () => readonly.get('denied'),
    'Should deny reading non-allowed path'
  );
});

runner.test('batchUpdate() should batch multiple updates', async () => {
  const state = new TerraphimState({ a: 1, b: 2, c: 3 });

  let callCount = 0;
  state.subscribe('a', () => callCount++);
  state.subscribe('b', () => callCount++);
  state.subscribe('c', () => callCount++);

  const update = batchUpdate(state);

  update({
    'a': 10,
    'b': 20,
    'c': 30
  });

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(state.get('a'), 10, 'Should update all values');
  assert.equal(state.get('b'), 20, 'Should update all values');
  assert.equal(state.get('c'), 30, 'Should update all values');
  assert.equal(callCount, 3, 'Should batch notifications');
});

runner.test('createSelector() should memoize results', () => {
  const state = new TerraphimState({ items: [1, 2, 3, 4, 5] });

  let callCount = 0;
  const getEvenItems = createSelector((state) => {
    callCount++;
    return state.get('items').filter(x => x % 2 === 0);
  });

  const result1 = getEvenItems(state);
  const result2 = getEvenItems(state);

  assert.deepEqual(result1, [2, 4], 'Should return correct result');
  assert.equal(result1, result2, 'Should return same instance');
  assert.equal(callCount, 1, 'Should only compute once');
});

runner.test('waitFor() should wait for condition', async () => {
  const state = new TerraphimState({ loaded: false });

  const promise = waitFor(state, 'loaded', (value) => value === true);

  setTimeout(() => state.set('loaded', true), 50);

  const result = await promise;

  assert.equal(result, true, 'Should resolve with value');
});

runner.test('waitFor() should timeout', async () => {
  const state = new TerraphimState({ loaded: false });

  let timedOut = false;

  try {
    await waitFor(state, 'loaded', (value) => value === true, 50);
  } catch (error) {
    timedOut = true;
  }

  assert.ok(timedOut, 'Should timeout');
});

runner.test('waitFor() should resolve immediately if condition is met', async () => {
  const state = new TerraphimState({ loaded: true });

  const result = await waitFor(state, 'loaded', (value) => value === true);

  assert.equal(result, true, 'Should resolve immediately');
});

runner.test('createDebouncedSetter() should debounce updates', async () => {
  const state = new TerraphimState({ value: 0 });

  let callCount = 0;
  state.subscribe('value', () => callCount++);

  const debouncedSet = createDebouncedSetter(state, 50);

  debouncedSet('value', 1);
  debouncedSet('value', 2);
  debouncedSet('value', 3);

  // Should not have updated yet
  assert.equal(callCount, 0, 'Should not update immediately');

  await new Promise(resolve => setTimeout(resolve, 60));

  assert.equal(state.get('value'), 3, 'Should update to last value');
  assert.equal(callCount, 1, 'Should only call once');
});

runner.test('createThrottledSetter() should throttle updates', async () => {
  const state = new TerraphimState({ value: 0 });

  let callCount = 0;
  state.subscribe('value', () => callCount++);

  const throttledSet = createThrottledSetter(state, 50);

  throttledSet('value', 1);
  await new Promise(resolve => setTimeout(resolve, 10));

  throttledSet('value', 2);
  await new Promise(resolve => setTimeout(resolve, 10));

  throttledSet('value', 3);

  await new Promise(resolve => setTimeout(resolve, 60));

  // First call should go through, others should be throttled
  assert.ok(callCount >= 1, 'Should call at least once');
  assert.ok(callCount < 3, 'Should throttle some calls');
});

runner.test('derived() should support immediate subscription', () => {
  const state = new TerraphimState({ value: 10 });

  const doubled = derived(
    state,
    ['value'],
    (value) => value * 2,
    0
  );

  let received = null;
  doubled.subscribe((value) => {
    received = value;
  }, { immediate: true });

  assert.equal(received, 20, 'Should call immediately');

  doubled.unsubscribe();
});

runner.test('computed() should cleanup subscriptions', () => {
  const state = new TerraphimState({ a: 1, b: 2 });

  const sum = computed(
    state,
    ['a', 'b'],
    (a, b) => a + b
  );

  assert.equal(sum.value, 3, 'Should compute sum');

  sum.unsubscribe();

  // After unsubscribe, internal subscriptions should be cleaned up
  // We can verify by checking subscription count is not growing
  state.set('a', 10);
  state.set('b', 20);

  // Value should not update after unsubscribe
  assert.equal(sum.value, 3, 'Should not recompute after unsubscribe');
});

runner.test('validate() should support nested path validation', () => {
  const state = new TerraphimState({ user: { email: 'test@example.com' } });

  const validator = validate({
    'user.email': (value) => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)
  });

  state.use(validator);

  state.set('user.email', 'valid@email.com');
  assert.equal(state.get('user.email'), 'valid@email.com', 'Should accept valid email');

  state.set('user.email', 'invalid-email');
  assert.equal(state.get('user.email'), 'valid@email.com', 'Should reject invalid email');
});

runner.test('createLogger() should support filtering', () => {
  const state = new TerraphimState();
  const logs = [];

  const originalLog = console.log;
  console.log = (...args) => logs.push(args);

  const logger = createLogger({
    logSet: true,
    filter: (action, payload) => payload.path === 'important'
  });

  state.use(logger);

  state.set('other', 'value');
  state.set('important', 'value');

  console.log = originalLog;

  assert.equal(logs.length, 1, 'Should only log filtered paths');

  const logText = logs[0].join(' ');
  assert.ok(logText.includes('important'), 'Should log important path');
});

runner.test('syncStates() should support wildcard sync', async () => {
  const source = new TerraphimState({ value: 1 });
  const target = new TerraphimState({ value: 0, other: 'data' });

  const cleanup = syncStates(source, target);

  source.set('value', 42);

  await new Promise(resolve => setTimeout(resolve, 10));

  assert.equal(target.get('value'), 42, 'Should sync all changes');

  cleanup();
});

runner.test('derived() should only notify when value actually changes', async () => {
  const state = new TerraphimState({ value: 10 });

  const doubled = derived(
    state,
    ['value'],
    (value) => Math.floor(value / 10) * 10, // Round to nearest 10
    0
  );

  let callCount = 0;
  doubled.subscribe(() => callCount++);

  state.set('value', 11);
  await new Promise(resolve => setTimeout(resolve, 10));

  state.set('value', 12);
  await new Promise(resolve => setTimeout(resolve, 10));

  state.set('value', 20);
  await new Promise(resolve => setTimeout(resolve, 10));

  // Should only notify when rounded value changes (10 -> 20)
  assert.equal(callCount, 1, 'Should only notify on actual changes');

  doubled.unsubscribe();
});

// Run all tests
export async function runTests() {
  return await runner.run();
}

// Auto-run if loaded directly
if (typeof window !== 'undefined') {
  window.runStateHelpersTests = runTests;
}
