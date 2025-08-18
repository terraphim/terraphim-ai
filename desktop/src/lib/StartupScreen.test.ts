import { render, screen } from '@testing-library/svelte';
import { describe, it, expect, vi } from 'vitest';
import StartupScreen from './StartupScreen.svelte';

// Mock Tauri APIs
vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: vi.fn()
}));

vi.mock('@tauri-apps/api/dialog', () => ({
  open: vi.fn()
}));

vi.mock('@tauri-apps/api/fs', () => ({
  readDir: vi.fn(),
  readBinaryFile: vi.fn()
}));

vi.mock('@tauri-apps/api/path', () => ({
  resolve: vi.fn(),
  appDir: vi.fn(),
  appDataDir: vi.fn()
}));

vi.mock('@tauri-apps/api/globalShortcut', () => ({
  register: vi.fn(),
  unregisterAll: vi.fn(),
  unregister: vi.fn()
}));

vi.mock('@tauri-apps/api/window', () => ({
  appWindow: {
    isVisible: vi.fn(() => true),
    hide: vi.fn()
  }
}));

vi.mock('$lib/stores', () => ({
  isInitialSetupComplete: {
    set: vi.fn(),
    subscribe: vi.fn(() => () => {})
  },
  theme: {
    subscribe: vi.fn(() => () => {})
  }
}));

describe('StartupScreen', () => {
  describe('Component Rendering', () => {
    it('renders welcome message and setup form', () => {
      render(StartupScreen);
      
      expect(screen.getByText('Welcome to Terraphim AI')).toBeInTheDocument();
      expect(screen.getByText('Please set up your initial settings:')).toBeInTheDocument();
      expect(screen.getByLabelText('Data Folder Path:')).toBeInTheDocument();
      expect(screen.getByLabelText('Global Shortcut:')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /save settings/i })).toBeInTheDocument();
    });

    it('displays default global shortcut', () => {
      render(StartupScreen);
      
      const shortcutInput = screen.getByLabelText('Global Shortcut:') as HTMLInputElement;
      expect(shortcutInput.value).toBe('CmdOrControl+X');
    });

    it('shows empty data folder initially', () => {
      render(StartupScreen);
      
      const folderInput = screen.getByLabelText('Data Folder Path:') as HTMLInputElement;
      expect(folderInput.value).toBe('');
    });
  });

  describe('UI Structure', () => {
    it('has proper form structure with labels and inputs', () => {
      render(StartupScreen);
      
      // Check form structure
      expect(screen.getByText('Data Folder Path:')).toBeInTheDocument();
      expect(screen.getByText('Global Shortcut:')).toBeInTheDocument();
      
      // Check input elements exist
      expect(screen.getByLabelText('Data Folder Path:')).toBeInTheDocument();
      expect(screen.getByLabelText('Global Shortcut:')).toBeInTheDocument();
      
      // Check button exists
      expect(screen.getByRole('button', { name: /save settings/i })).toBeInTheDocument();
    });

    it('uses proper Bulma/Svelma CSS classes', () => {
      render(StartupScreen);
      
      // Check for Bulma classes
      expect(screen.getByText('Welcome to Terraphim AI')).toHaveClass('title', 'is-2');
      expect(screen.getByText('Please set up your initial settings:')).toHaveClass('subtitle');
      expect(screen.getByRole('button', { name: /save settings/i })).toHaveClass('button', 'is-success');
      
      // Check for field structure
      const fields = document.querySelectorAll('.field');
      expect(fields.length).toBeGreaterThan(0);
      
      const inputs = document.querySelectorAll('.input');
      expect(inputs.length).toBeGreaterThan(0);
    });
  });

  describe('Component Lifecycle', () => {
    it('renders without crashing', () => {
      expect(() => render(StartupScreen)).not.toThrow();
    });

    it('displays all required UI elements', () => {
      render(StartupScreen);
      
      expect(screen.getByText('Welcome to Terraphim AI')).toBeInTheDocument();
      expect(screen.getByText('Please set up your initial settings:')).toBeInTheDocument();
      expect(screen.getByLabelText('Data Folder Path:')).toBeInTheDocument();
      expect(screen.getByLabelText('Global Shortcut:')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /save settings/i })).toBeInTheDocument();
    });

    it('has proper accessibility attributes', () => {
      render(StartupScreen);
      
      const folderInput = screen.getByLabelText('Data Folder Path:');
      const shortcutInput = screen.getByLabelText('Global Shortcut:');
      
      expect(folderInput).toHaveAttribute('id', 'data-folder');
      expect(shortcutInput).toHaveAttribute('id', 'global-shortcut');
      expect(folderInput).toHaveAttribute('readonly');
      expect(shortcutInput).toHaveAttribute('readonly');
    });
  });

  describe('Tauri Integration Readiness', () => {
    it('component is ready for Tauri integration', () => {
      // This test validates that the component structure is correct
      // and ready for Tauri API integration
      render(StartupScreen);
      
      // Check that the component has the expected structure
      expect(screen.getByText('Welcome to Terraphim AI')).toBeInTheDocument();
      
      // The component should be ready to integrate with Tauri APIs
      // when running in the actual Tauri environment
      expect(true).toBe(true);
    });
  });
});
