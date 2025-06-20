import '@testing-library/jest-dom';
import { beforeEach, vi } from 'vitest';
import { writable } from 'svelte/store';

// Extend global types for our test utilities
declare global {
  var mockTauriApi: any;
  var localStorageMock: any;
  var resetMocks: () => void;
}

// Mock Tauri API
beforeEach(() => {
  // Mock Tauri global
  Object.defineProperty(window, '__TAURI_IPC__', {
    value: {
      invoke: vi.fn(),
    },
    writable: true,
  });

  // Mock Tauri invoke function
  Object.defineProperty(window, '__TAURI__', {
    value: {
      invoke: vi.fn(),
    },
    writable: true,
  });

  // Mock localStorage
  Object.defineProperty(window, 'localStorage', {
    value: {
      getItem: vi.fn(),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    },
    writable: true,
  });

  // Mock global fetch
  Object.defineProperty(window, 'fetch', {
    value: vi.fn(),
    writable: true,
  });
});

// Mock all stores with proper Svelte writable stores
vi.mock('../lib/stores', () => {
  const { writable } = require('svelte/store');
  
  return {
    configStore: writable({
      id: "test-config",
      global_shortcut: "Ctrl+Shift+T",
      roles: {
        "test_role": {
          name: "Test Role",
          theme: "spacelab"
        }
      },
      default_role: "test_role",
      selected_role: "test_role"
    }),
    input: writable(""),
    is_tauri: writable(false),
    role: writable("test_role"),
    roles: writable({
      "test_role": {
        name: "Test Role",
        theme: "spacelab",
        kg: { publish: true }
      }
    }),
    serverUrl: writable("http://localhost:3000/documents/search"),
    theme: writable("spacelab"),
    typeahead: writable(false),
    thesaurus: writable([]),
    isInitialSetupComplete: writable(true)
  };
});

// Mock Tauri API modules
vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/window', () => ({
  appWindow: {
    hide: vi.fn(),
    show: vi.fn(),
    setFocus: vi.fn(),
  },
}));

vi.mock('@tauri-apps/api/globalShortcut', () => ({
  register: vi.fn(),
  unregister: vi.fn(),
}));

vi.mock('@tauri-apps/api/app', () => ({
  getName: vi.fn().mockResolvedValue('Test App'),
  getVersion: vi.fn().mockResolvedValue('1.0.0'),
}));

// Mock window.matchMedia for CSS media queries
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(), // deprecated
    removeListener: vi.fn(), // deprecated
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Global test utilities
global.mockTauriApi = {
  invoke: vi.fn(),
  listen: vi.fn(),
  emit: vi.fn(),
};
global.localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};

// Cleanup function to reset mocks
global.resetMocks = () => {
  vi.clearAllMocks();
  global.localStorageMock.getItem.mockClear();
  global.localStorageMock.setItem.mockClear();
  global.localStorageMock.removeItem.mockClear();
  global.localStorageMock.clear.mockClear();
  global.mockTauriApi.invoke.mockClear();
  global.mockTauriApi.listen.mockClear();
  global.mockTauriApi.emit.mockClear();
}; 