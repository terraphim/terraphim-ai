import '@testing-library/jest-dom';

// Simple test setup without complex mocking
// We're using real APIs instead of mocks for integration testing

// Basic DOM compatibility fixes for JSDOM
Object.defineProperty(HTMLInputElement.prototype, 'selectionStart', {
	get() {
		return 0;
	},
	set() {},
	configurable: true,
});

Object.defineProperty(HTMLInputElement.prototype, 'setSelectionRange', {
	value: () => {},
	configurable: true,
});
