/**
 * @fileoverview Tests for TerraphimEvents global event bus
 */

import { TerraphimElement } from '../terraphim-element.js';
import { TerraphimEvents, TerraphimEventBus, createEvent } from '../terraphim-events.js';

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
    console.log('Running TerraphimEvents tests...\n');

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

// Create test component with event bus mixin
function createEventBusComponent() {
  class TestComponent extends TerraphimEventBus(TerraphimElement) {
    constructor() {
      super();
    }
  }

  const tagName = `test-eventbus-${Math.random().toString(36).substr(2, 9)}`;
  customElements.define(tagName, TestComponent);
  return { tagName, TestComponent };
}

// Tests

runner.test('TerraphimEvents should be defined', () => {
  assert.ok(TerraphimEvents, 'TerraphimEvents should exist');
  assert.ok(typeof TerraphimEvents.on === 'function', 'Should have on method');
  assert.ok(typeof TerraphimEvents.emit === 'function', 'Should have emit method');
});

runner.test('Should register and emit events', () => {
  // Clear previous listeners
  TerraphimEvents.clear();

  let received = null;
  TerraphimEvents.on('test:event', (data) => {
    received = data;
  });

  TerraphimEvents.emit('test:event', { foo: 'bar' });

  assert.deepEqual(received, { foo: 'bar' }, 'Should receive event data');

  TerraphimEvents.clear();
});

runner.test('Should support multiple listeners', () => {
  TerraphimEvents.clear();

  let count1 = 0;
  let count2 = 0;

  TerraphimEvents.on('test:multi', () => count1++);
  TerraphimEvents.on('test:multi', () => count2++);

  TerraphimEvents.emit('test:multi');

  assert.equal(count1, 1, 'First listener should be called');
  assert.equal(count2, 1, 'Second listener should be called');

  TerraphimEvents.clear();
});

runner.test('Should support once listeners', () => {
  TerraphimEvents.clear();

  let callCount = 0;
  TerraphimEvents.once('test:once', () => callCount++);

  TerraphimEvents.emit('test:once');
  TerraphimEvents.emit('test:once');
  TerraphimEvents.emit('test:once');

  assert.equal(callCount, 1, 'Once listener should only be called once');

  TerraphimEvents.clear();
});

runner.test('Should remove specific listeners', () => {
  TerraphimEvents.clear();

  let count = 0;
  const handler = () => count++;

  TerraphimEvents.on('test:remove', handler);
  TerraphimEvents.emit('test:remove');

  assert.equal(count, 1, 'Should call handler');

  TerraphimEvents.off('test:remove', handler);
  TerraphimEvents.emit('test:remove');

  assert.equal(count, 1, 'Should not call handler after removal');

  TerraphimEvents.clear();
});

runner.test('Should remove all listeners for event', () => {
  TerraphimEvents.clear();

  let count1 = 0;
  let count2 = 0;

  TerraphimEvents.on('test:clear', () => count1++);
  TerraphimEvents.on('test:clear', () => count2++);

  TerraphimEvents.off('test:clear'); // Remove all

  TerraphimEvents.emit('test:clear');

  assert.equal(count1, 0, 'First listener should not be called');
  assert.equal(count2, 0, 'Second listener should not be called');

  TerraphimEvents.clear();
});

runner.test('Should return unsubscribe function', () => {
  TerraphimEvents.clear();

  let count = 0;
  const unsubscribe = TerraphimEvents.on('test:unsub', () => count++);

  TerraphimEvents.emit('test:unsub');
  assert.equal(count, 1, 'Should call handler');

  unsubscribe();
  TerraphimEvents.emit('test:unsub');
  assert.equal(count, 1, 'Should not call after unsubscribe');

  TerraphimEvents.clear();
});

runner.test('Should get listener count', () => {
  TerraphimEvents.clear();

  TerraphimEvents.on('test:count', () => {});
  TerraphimEvents.on('test:count', () => {});

  assert.equal(TerraphimEvents.getListenerCount('test:count'), 2, 'Should return correct count');

  TerraphimEvents.clear();
});

runner.test('Should get event names', () => {
  TerraphimEvents.clear();

  TerraphimEvents.on('event:one', () => {});
  TerraphimEvents.on('event:two', () => {});

  const names = TerraphimEvents.getEventNames();

  assert.ok(names.includes('event:one'), 'Should include first event');
  assert.ok(names.includes('event:two'), 'Should include second event');

  TerraphimEvents.clear();
});

runner.test('Should check if event has listeners', () => {
  TerraphimEvents.clear();

  assert.notOk(TerraphimEvents.hasListeners('test:has'), 'Should return false initially');

  TerraphimEvents.on('test:has', () => {});

  assert.ok(TerraphimEvents.hasListeners('test:has'), 'Should return true after adding listener');

  TerraphimEvents.clear();
});

runner.test('Should clear all events', () => {
  TerraphimEvents.clear();

  TerraphimEvents.on('event:one', () => {});
  TerraphimEvents.on('event:two', () => {});

  TerraphimEvents.clear();

  assert.equal(TerraphimEvents.getEventNames().length, 0, 'Should clear all events');
});

runner.test('Should handle errors in listeners gracefully', () => {
  TerraphimEvents.clear();

  let called = false;

  TerraphimEvents.on('test:error', () => {
    throw new Error('Test error');
  });

  TerraphimEvents.on('test:error', () => {
    called = true;
  });

  // Should not throw
  TerraphimEvents.emit('test:error');

  assert.ok(called, 'Should call other listeners even if one throws');

  TerraphimEvents.clear();
});

runner.test('TerraphimEventBus mixin should add global event methods', () => {
  const { tagName } = createEventBusComponent();
  const element = document.createElement(tagName);

  assert.ok(typeof element.onGlobal === 'function', 'Should have onGlobal method');
  assert.ok(typeof element.emitGlobal === 'function', 'Should have emitGlobal method');
  assert.ok(typeof element.offGlobal === 'function', 'Should have offGlobal method');
});

runner.test('TerraphimEventBus should subscribe to global events', () => {
  TerraphimEvents.clear();

  const { tagName } = createEventBusComponent();
  const element = document.createElement(tagName);
  document.body.appendChild(element);

  let received = null;
  element.onGlobal('test:global', (data) => {
    received = data;
  });

  TerraphimEvents.emit('test:global', { foo: 'bar' });

  assert.deepEqual(received, { foo: 'bar' }, 'Should receive global event');

  document.body.removeChild(element);
  TerraphimEvents.clear();
});

runner.test('TerraphimEventBus should emit global events', () => {
  TerraphimEvents.clear();

  const { tagName } = createEventBusComponent();
  const element = document.createElement(tagName);

  let received = null;
  TerraphimEvents.on('test:emit', (data) => {
    received = data;
  });

  element.emitGlobal('test:emit', { baz: 'qux' });

  assert.deepEqual(received, { baz: 'qux' }, 'Should emit global event');

  TerraphimEvents.clear();
});

runner.test('TerraphimEventBus should cleanup on disconnect', () => {
  TerraphimEvents.clear();

  const { tagName } = createEventBusComponent();
  const element = document.createElement(tagName);
  document.body.appendChild(element);

  let callCount = 0;
  element.onGlobal('test:cleanup', () => {
    callCount++;
  });

  TerraphimEvents.emit('test:cleanup');
  assert.equal(callCount, 1, 'Should receive event while connected');

  document.body.removeChild(element);

  TerraphimEvents.emit('test:cleanup');
  assert.equal(callCount, 1, 'Should not receive event after disconnect');

  TerraphimEvents.clear();
});

runner.test('TerraphimEventBus should support once', () => {
  TerraphimEvents.clear();

  const { tagName } = createEventBusComponent();
  const element = document.createElement(tagName);
  document.body.appendChild(element);

  let callCount = 0;
  element.onceGlobal('test:once-mixin', () => {
    callCount++;
  });

  TerraphimEvents.emit('test:once-mixin');
  TerraphimEvents.emit('test:once-mixin');

  assert.equal(callCount, 1, 'Should only receive event once');

  document.body.removeChild(element);
  TerraphimEvents.clear();
});

runner.test('createEvent should create proper CustomEvent', () => {
  const event = createEvent('test-event', { foo: 'bar' });

  assert.ok(event instanceof CustomEvent, 'Should be CustomEvent');
  assert.equal(event.type, 'test-event', 'Should have correct type');
  assert.deepEqual(event.detail, { foo: 'bar' }, 'Should have correct detail');
  assert.equal(event.bubbles, true, 'Should bubble by default');
  assert.equal(event.composed, true, 'Should be composed by default');
});

runner.test('createEvent should accept custom options', () => {
  const event = createEvent('test-event', null, {
    bubbles: false,
    cancelable: true
  });

  assert.equal(event.bubbles, false, 'Should override bubbles');
  assert.equal(event.cancelable, true, 'Should set cancelable');
});

runner.test('Should support namespaced events', () => {
  TerraphimEvents.clear();

  let userCalls = 0;
  let dataCalls = 0;

  TerraphimEvents.on('user:login', () => userCalls++);
  TerraphimEvents.on('user:logout', () => userCalls++);
  TerraphimEvents.on('data:update', () => dataCalls++);

  TerraphimEvents.emit('user:login');
  TerraphimEvents.emit('user:logout');
  TerraphimEvents.emit('data:update');

  assert.equal(userCalls, 2, 'Should handle user namespace events');
  assert.equal(dataCalls, 1, 'Should handle data namespace events');

  TerraphimEvents.clear();
});

runner.test('Should handle rapid event emissions', () => {
  TerraphimEvents.clear();

  let count = 0;
  TerraphimEvents.on('test:rapid', () => count++);

  for (let i = 0; i < 100; i++) {
    TerraphimEvents.emit('test:rapid');
  }

  assert.equal(count, 100, 'Should handle rapid emissions');

  TerraphimEvents.clear();
});

// Run all tests
export async function runTests() {
  return await runner.run();
}

// Auto-run if loaded directly
if (typeof window !== 'undefined') {
  window.runTerraphimEventsTests = runTests;
}
