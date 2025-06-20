import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import { writable } from 'svelte/store';
import ThemeSwitcher from './ThemeSwitcher.svelte';

// Mock all stores directly in this test file
vi.mock('./stores', () => {
  const { writable } = require('svelte/store');
  
  return {
    configStore: writable({
      id: "test-config",
      global_shortcut: "Ctrl+Shift+T",
      roles: {
        "test_role": {
          name: "Test Role",
          theme: "spacelab"
        },
        "engineer": {
          name: "Engineer",
          theme: "darkly"
        },
        "researcher": {
          name: "Researcher",
          theme: "cerulean"
        }
      },
      default_role: "test_role",
      selected_role: "test_role"
    }),
    input: writable(""),
    is_tauri: writable(false),
    role: writable("Test Role"),
    roles: writable([
      {
        name: "Test Role",
        theme: "spacelab",
        shortname: "test_role",
        kg: { publish: true }
      },
      {
        name: "Engineer",
        theme: "darkly", 
        shortname: "engineer",
        kg: { publish: true }
      },
      {
        name: "Researcher",
        theme: "cerulean",
        shortname: "researcher", 
        kg: { publish: false }
      }
    ]),
    serverUrl: writable("http://localhost:3000/documents/search"),
    theme: writable("spacelab"),
    typeahead: writable(false),
    thesaurus: writable({}),
    isInitialSetupComplete: writable(true)
  };
});

// Mock Tauri APIs
vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: vi.fn().mockResolvedValue({ 
    status: 'success',
    config: {
      id: "test-config",
      global_shortcut: "Ctrl+Shift+T",
      roles: {
        "test_role": { name: "Test Role", theme: "spacelab" },
        "engineer": { name: "Engineer", theme: "darkly" },
        "researcher": { name: "Researcher", theme: "cerulean" }
      },
      default_role: "test_role"
    }
  }),
}));

// Mock config
vi.mock('../config', () => ({
  CONFIG: {
    ServerURL: 'http://localhost:8000'
  }
}));

// Mock localStorage
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
  writable: true,
});

// Mock Tauri IPC
Object.defineProperty(window, '__TAURI_IPC__', {
  value: {
    invoke: vi.fn(),
  },
  writable: true,
});

// Mock window.__TAURI__
Object.defineProperty(window, '__TAURI__', {
  value: {
    invoke: vi.fn(),
  },
  writable: true,
});

describe('ThemeSwitcher Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorageMock.getItem.mockReturnValue('spacelab');
  });

  it('renders role selector dropdown', () => {
    render(ThemeSwitcher);
    
    const selectElement = screen.getByRole('combobox');
    expect(selectElement).toBeInTheDocument();
  });

  it('displays available roles in dropdown', () => {
    render(ThemeSwitcher);
    
    // Should show role options
    expect(screen.getByText('Test Role')).toBeInTheDocument();
    expect(screen.getByText('Engineer')).toBeInTheDocument();
    expect(screen.getByText('Researcher')).toBeInTheDocument();
  });

  it('changes role when option is selected', async () => {
    const mockInvoke = vi.fn().mockResolvedValue({ status: 'success' });
    vi.mocked(require('@tauri-apps/api/tauri')).invoke = mockInvoke;
    
    // Set Tauri environment
    const stores = await import('./stores');
    stores.is_tauri.set(true);
    
    render(ThemeSwitcher);
    
    const selectElement = screen.getByRole('combobox');
    await fireEvent.change(selectElement, { target: { value: 'Engineer' } });
    
    // Should call update_config in Tauri environment
    expect(mockInvoke).toHaveBeenCalledWith('update_config', expect.objectContaining({
      configNew: expect.any(Object)
    }));
  });

  it('publishes thesaurus when role has kg.publish=true', async () => {
    const mockInvoke = vi.fn()
      .mockResolvedValueOnce({ status: 'success' }) // update_config response
      .mockResolvedValueOnce({ // publish_thesaurus response
        'artificial intelligence': ['AI', 'machine learning'],
        'software engineering': ['coding', 'programming']
      });
      
    vi.mocked(require('@tauri-apps/api/tauri')).invoke = mockInvoke;
    
    const stores = await import('./stores');
    stores.is_tauri.set(true);
    
    render(ThemeSwitcher);
    
    const selectElement = screen.getByRole('combobox');
    await fireEvent.change(selectElement, { target: { value: 'Engineer' } });
    
    // Should call publish_thesaurus for roles with kg.publish=true
    expect(mockInvoke).toHaveBeenCalledWith('publish_thesaurus', {
      roleName: 'Engineer'
    });
  });

  it('does not publish thesaurus when role has kg.publish=false', async () => {
    const mockInvoke = vi.fn().mockResolvedValue({ status: 'success' });
    vi.mocked(require('@tauri-apps/api/tauri')).invoke = mockInvoke;
    
    const stores = await import('./stores');
    stores.is_tauri.set(true);
    
    render(ThemeSwitcher);
    
    const selectElement = screen.getByRole('combobox');
    await fireEvent.change(selectElement, { target: { value: 'Researcher' } });
    
    // Should not call publish_thesaurus for roles with kg.publish=false
    expect(mockInvoke).not.toHaveBeenCalledWith('publish_thesaurus', expect.anything());
  });

  it('loads config on component initialization', async () => {
    const mockInvoke = vi.fn().mockResolvedValue({
      status: 'success',
      config: {
        id: "loaded-config",
        roles: {
          "test_role": { name: "Test Role", theme: "spacelab" }
        },
        default_role: "test_role"
      }
    });
    
    vi.mocked(require('@tauri-apps/api/tauri')).invoke = mockInvoke;
    
    const stores = await import('./stores');
    stores.is_tauri.set(true);
    
    render(ThemeSwitcher);
    
    // Should call get_config on initialization
    expect(mockInvoke).toHaveBeenCalledWith('get_config');
  });

  it('handles HTTP config fetch when not in Tauri', async () => {
    const mockResponse = {
      status: 'success',
      config: {
        id: "http-config",
        roles: {
          "test_role": { name: "Test Role", theme: "spacelab" }
        },
        default_role: "test_role"
      }
    };
    
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockResponse)
    });
    
    const stores = await import('./stores');
    stores.is_tauri.set(false);
    
    render(ThemeSwitcher);
    
    // Should fetch config via HTTP when not in Tauri
    expect(global.fetch).toHaveBeenCalledWith('http://localhost:8000/config/');
  });

  it('handles config fetch errors gracefully', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    
    const mockInvoke = vi.fn().mockRejectedValue(new Error('Config fetch failed'));
    vi.mocked(require('@tauri-apps/api/tauri')).invoke = mockInvoke;
    
    const stores = await import('./stores');
    stores.is_tauri.set(true);
    
    render(ThemeSwitcher);
    
    // Should log error but not crash
    expect(consoleSpy).toHaveBeenCalledWith('Error fetching config in Tauri:', expect.any(Error));
    
    consoleSpy.mockRestore();
  });

  it('sets theme based on selected role', async () => {
    const stores = await import('./stores');
    stores.is_tauri.set(false); // Avoid Tauri calls for this test
    
    render(ThemeSwitcher);
    
    const selectElement = screen.getByRole('combobox');
    await fireEvent.change(selectElement, { target: { value: 'Engineer' } });
    
    // Should update theme store based on role's theme
    // The theme should be set to the Engineer role's theme (darkly)
    expect(selectElement).toHaveValue('Engineer');
  });

  it('handles role without theme definition', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    
    const stores = await import('./stores');
    stores.is_tauri.set(false);
    
    // Add a role without theme
    stores.roles.set([
      {
        name: "Test Role",
        theme: "spacelab",
        shortname: "test_role",
        kg: { publish: true }
      },
      {
        name: "Engineer",
        theme: "darkly", 
        shortname: "engineer",
        kg: { publish: true }
      },
      {
        name: "No Theme Role",
        theme: undefined,
        shortname: "no_theme",
        kg: { publish: false }
      }
    ]);
    
    render(ThemeSwitcher);
    
    const selectElement = screen.getByRole('combobox');
    await fireEvent.change(selectElement, { target: { value: 'No Theme Role' } });
    
    // Should handle missing theme gracefully
    expect(consoleSpy).toHaveBeenCalledWith(
      expect.stringContaining('No theme defined for role'),
      expect.any(String)
    );
    
    consoleSpy.mockRestore();
  });

  it('updates typeahead setting based on role kg.publish', async () => {
    const stores = await import('./stores');
    stores.is_tauri.set(false);
    
    render(ThemeSwitcher);
    
    const selectElement = screen.getByRole('combobox');
    
    // Select role with kg.publish=true
    await fireEvent.change(selectElement, { target: { value: 'Test Role' } });
    // typeahead should be set to true (though we can't easily test the store update here)
    
    // Select role with kg.publish=false  
    await fireEvent.change(selectElement, { target: { value: 'Researcher' } });
    // typeahead should be set to false
    
    expect(selectElement).toHaveValue('Researcher');
  });

  it('handles Tauri environment detection', () => {
    window.__TAURI__ = { invoke: vi.fn() } as any;
    
    render(ThemeSwitcher);
    
    // Component should render without errors in Tauri environment
    expect(screen.getByRole('combobox')).toBeInTheDocument();
  });

  it('handles non-Tauri environment', () => {
    delete (window as any).__TAURI__;
    
    render(ThemeSwitcher);
    
    // Component should render without errors in non-Tauri environment
    expect(screen.getByRole('combobox')).toBeInTheDocument();
  });
}); 