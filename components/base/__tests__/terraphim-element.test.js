/**
 * @fileoverview Tests for TerraphimElement base class
 */

import { TerraphimElement } from '../terraphim-element.js';

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
    console.log('Running TerraphimElement tests...\n');

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

    onAttributeChanged(name, oldValue, newValue) {
      if (options.onAttributeChanged) {
        options.onAttributeChanged.call(this, name, oldValue, newValue);
      }
    }
  }

  const tagName = `test-component-${Math.random().toString(36).substr(2, 9)}`;
  customElements.define(tagName, TestComponent);
  return { tagName, TestComponent };
}

// Tests

runner.test('TerraphimElement should be defined', () => {
  assert.ok(TerraphimElement, 'TerraphimElement class should exist');
  assert.ok(TerraphimElement.prototype instanceof HTMLElement, 'Should extend HTMLElement');
});

runner.test('Should initialize with default properties', () => {
  const { tagName } = createTestComponent();
  const element = document.createElement(tagName);

  assert.ok(element._cleanupFunctions, 'Should have cleanup functions array');
  assert.equal(element._isConnected, false, 'Should not be connected initially');
  assert.ok(Array.isArray(element._cleanupFunctions), 'Cleanup functions should be array');
});

runner.test('Should handle connectedCallback lifecycle', () => {
  let connected = false;
  const { tagName } = createTestComponent({
    onConnected() {
      connected = true;
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  assert.ok(connected, 'onConnected should be called');
  assert.equal(element._isConnected, true, 'Should be marked as connected');

  document.body.removeChild(element);
});

runner.test('Should handle disconnectedCallback lifecycle', () => {
  let disconnected = false;
  const { tagName } = createTestComponent({
    onDisconnected() {
      disconnected = true;
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);
  document.body.removeChild(element);

  assert.ok(disconnected, 'onDisconnected should be called');
  assert.equal(element._isConnected, false, 'Should be marked as disconnected');
});

runner.test('Should convert property types correctly', () => {
  const { tagName } = createTestComponent({
    properties: {
      stringProp: { type: String, default: 'default' },
      numberProp: { type: Number, default: 0 },
      boolProp: { type: Boolean, default: false },
      objProp: { type: Object, default: () => ({}) }
    }
  });

  const element = document.createElement(tagName);

  // String conversion
  element.stringProp = 123;
  assert.equal(element.stringProp, '123', 'Should convert to string');

  // Number conversion
  element.numberProp = '42';
  assert.equal(element.numberProp, 42, 'Should convert to number');

  // Boolean conversion
  element.boolProp = 'true';
  assert.equal(element.boolProp, true, 'Should convert to boolean');

  element.boolProp = 'false';
  assert.equal(element.boolProp, false, 'Should handle "false" string');
});

runner.test('Should reflect properties to attributes', () => {
  const { tagName } = createTestComponent({
    observedAttributes: ['title', 'count', 'active'],
    properties: {
      title: { type: String, reflect: true },
      count: { type: Number, reflect: true },
      active: { type: Boolean, reflect: true }
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  element.title = 'Test';
  assert.equal(element.getAttribute('title'), 'Test', 'String should reflect to attribute');

  element.count = 42;
  assert.equal(element.getAttribute('count'), '42', 'Number should reflect to attribute');

  element.active = true;
  assert.equal(element.getAttribute('active'), '', 'Boolean true should reflect as empty string');

  element.active = false;
  assert.equal(element.hasAttribute('active'), false, 'Boolean false should remove attribute');

  document.body.removeChild(element);
});

runner.test('Should sync attributes to properties', () => {
  const { tagName } = createTestComponent({
    observedAttributes: ['title', 'count'],
    properties: {
      title: { type: String },
      count: { type: Number }
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  element.setAttribute('title', 'Hello');
  assert.equal(element.title, 'Hello', 'Attribute should sync to property');

  element.setAttribute('count', '99');
  assert.equal(element.count, 99, 'Attribute should sync and convert to number');

  document.body.removeChild(element);
});

runner.test('Should emit custom events', () => {
  const { tagName } = createTestComponent();
  const element = document.createElement(tagName);

  let eventData = null;
  element.addEventListener('test-event', (e) => {
    eventData = e.detail;
  });

  element.emit('test-event', { foo: 'bar' });

  assert.deepEqual(eventData, { foo: 'bar' }, 'Should emit event with correct data');
});

runner.test('Should manage event listeners with cleanup', () => {
  const { tagName } = createTestComponent();
  const element = document.createElement(tagName);
  document.body.appendChild(element);

  let clickCount = 0;
  element.listen('click', () => {
    clickCount++;
  });

  element.click();
  assert.equal(clickCount, 1, 'Should handle event');

  document.body.removeChild(element);
  element.click();
  assert.equal(clickCount, 1, 'Should cleanup event listener on disconnect');
});

runner.test('Should listen to other elements with cleanup', () => {
  const { tagName } = createTestComponent();
  const element = document.createElement(tagName);
  const button = document.createElement('button');

  document.body.appendChild(element);
  document.body.appendChild(button);

  let clickCount = 0;
  element.listenTo(button, 'click', () => {
    clickCount++;
  });

  button.click();
  assert.equal(clickCount, 1, 'Should listen to other element');

  document.body.removeChild(element);
  button.click();
  assert.equal(clickCount, 1, 'Should cleanup external listener on disconnect');

  document.body.removeChild(button);
});

runner.test('Should provide $ and $$ selectors', () => {
  const { tagName } = createTestComponent({
    useShadow: true,
    render() {
      this.setHTML(this.shadowRoot, `
        <div class="container">
          <button class="btn">Button 1</button>
          <button class="btn">Button 2</button>
        </div>
      `);
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  const container = element.$('.container');
  assert.ok(container, 'Should find single element');

  const buttons = element.$$('.btn');
  assert.equal(buttons.length, 2, 'Should find multiple elements');

  document.body.removeChild(element);
});

runner.test('Should handle custom cleanup functions', () => {
  const { tagName } = createTestComponent();
  const element = document.createElement(tagName);
  document.body.appendChild(element);

  let cleanupCalled = false;
  element.addCleanup(() => {
    cleanupCalled = true;
  });

  document.body.removeChild(element);
  assert.ok(cleanupCalled, 'Custom cleanup should be called');
});

runner.test('Should auto-render on property changes', async () => {
  let renderCount = 0;
  const { tagName } = createTestComponent({
    properties: {
      title: { type: String }
    },
    render() {
      renderCount++;
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  // Wait for initial render
  await new Promise(resolve => requestAnimationFrame(resolve));
  const initialRenders = renderCount;

  element.title = 'New Title';

  // Wait for render
  await new Promise(resolve => requestAnimationFrame(resolve));

  assert.ok(renderCount > initialRenders, 'Should trigger render on property change');

  document.body.removeChild(element);
});

runner.test('Should debounce multiple renders', async () => {
  let renderCount = 0;
  const { tagName } = createTestComponent({
    properties: {
      count: { type: Number }
    },
    render() {
      renderCount++;
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  // Wait for initial render
  await new Promise(resolve => requestAnimationFrame(resolve));
  renderCount = 0;

  // Multiple rapid property changes
  element.count = 1;
  element.count = 2;
  element.count = 3;

  // Wait for render
  await new Promise(resolve => requestAnimationFrame(resolve));

  assert.equal(renderCount, 1, 'Should debounce multiple renders into one');

  document.body.removeChild(element);
});

runner.test('Should convert camelCase to kebab-case for attributes', () => {
  const { tagName } = createTestComponent({
    observedAttributes: ['my-value'],
    properties: {
      myValue: { type: String, reflect: true }
    }
  });

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  element.myValue = 'test';
  assert.equal(element.getAttribute('my-value'), 'test', 'Should convert camelCase to kebab-case');

  document.body.removeChild(element);
});

runner.test('Should handle object and array properties', () => {
  const { tagName } = createTestComponent({
    properties: {
      data: { type: Object, default: () => ({ foo: 'bar' }) },
      items: { type: Array, default: () => [1, 2, 3] }
    }
  });

  const element = document.createElement(tagName);

  assert.deepEqual(element.data, { foo: 'bar' }, 'Should initialize object property');
  assert.deepEqual(element.items, [1, 2, 3], 'Should initialize array property');

  element.data = { baz: 'qux' };
  assert.deepEqual(element.data, { baz: 'qux' }, 'Should update object property');
});

runner.test('Should call propertyChangedCallback', () => {
  let propertyChanges = [];

  class TestComponent extends TerraphimElement {
    static get properties() {
      return {
        value: { type: String }
      };
    }

    propertyChangedCallback(name, oldValue, newValue) {
      propertyChanges.push({ name, oldValue, newValue });
    }
  }

  const tagName = `test-component-${Math.random().toString(36).substr(2, 9)}`;
  customElements.define(tagName, TestComponent);

  const element = document.createElement(tagName);
  document.body.appendChild(element);

  element.value = 'test';

  assert.equal(propertyChanges.length, 1, 'Should call propertyChangedCallback');
  assert.equal(propertyChanges[0].name, 'value', 'Should pass property name');
  assert.equal(propertyChanges[0].newValue, 'test', 'Should pass new value');

  document.body.removeChild(element);
});

// Run all tests
export async function runTests() {
  return await runner.run();
}

// Auto-run if loaded directly
if (typeof window !== 'undefined') {
  window.runTerraphimElementTests = runTests;
}
