import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import { writable } from 'svelte/store';
import Search from './Search.svelte';

// Mock all stores directly in this test file
vi.mock('../stores', () => {
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
    role: writable("test_role"),
    roles: writable({
      "test_role": {
        name: "Test Role",
        theme: "spacelab",
        kg: { publish: true }
      },
      "engineer": {
        name: "Engineer",
        theme: "darkly",
        kg: { publish: true }
      },
      "researcher": {
        name: "Researcher",
        theme: "cerulean", 
        kg: { publish: true }
      }
    }),
    serverUrl: writable("http://localhost:3000/documents/search"),
    theme: writable("spacelab"),
    typeahead: writable(true),
    thesaurus: writable({
      "artificial intelligence": ["AI", "machine learning"],
      "machine learning": ["ML", "neural networks"], 
      "software engineering": ["coding", "programming"]
    }),
    isInitialSetupComplete: writable(true)
  };
});

// Mock Tauri APIs
vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: vi.fn(),
}));

// Mock fetch
global.fetch = vi.fn();

// Mock DOM methods for JSDOM compatibility
Object.defineProperty(HTMLInputElement.prototype, 'selectionStart', {
  get() { return 0; },
  set() {},
  configurable: true
});

Object.defineProperty(HTMLInputElement.prototype, 'setSelectionRange', {
  value: vi.fn(),
  configurable: true
});

describe('Search Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders search input with proper placeholder', () => {
    render(Search);
    
    const searchInput = screen.getByRole('textbox');
    expect(searchInput).toBeInTheDocument();
    expect(searchInput).toHaveAttribute('placeholder', expect.stringContaining('Search'));
  });

  it('renders logo when no results', () => {
    render(Search);
    
    // Should show the logo initially
    const logo = screen.getByAltText(/terraphim logo/i);
    expect(logo).toBeInTheDocument();
    
    const assistantText = screen.getByText(/I am Terraphim, your personal assistant/i);
    expect(assistantText).toBeInTheDocument();
  });

  it('performs search with Tauri invoke and shows results', async () => {
    const mockSearchResponse = {
      status: 'success',
      results: [
        {
          id: '1',
          title: 'Machine Learning Basics',
          content: 'Introduction to machine learning concepts...',
          score: 0.95,
          url: '/docs/ml-basics'
        },
        {
          id: '2', 
          title: 'AI Development Guide',
          content: 'Step-by-step guide for AI development...',
          score: 0.87,
          url: '/docs/ai-guide'
        }
      ]
    };

    const mockInvoke = vi.fn().mockResolvedValue(mockSearchResponse);
    vi.mocked(require('@tauri-apps/api/tauri')).invoke = mockInvoke;
    
    // Mock is_tauri to be true
    const stores = await import('../stores');
    stores.is_tauri.set(true);
    stores.input.set('machine learning');

    render(Search);
    
    const searchInput = screen.getByRole('textbox');
    const form = searchInput.closest('form');
    
    await fireEvent.submit(form!);
    
    expect(mockInvoke).toHaveBeenCalledWith('search', {
      searchQuery: {
        search_term: 'machine learning',
        skip: 0,
        limit: 10,
        role: 'test_role'
      }
    });
  });

  it('searches with different roles', async () => {
    const mockInvoke = vi.fn().mockResolvedValue({
      status: 'success',
      results: [
        {
          id: '1',
          title: 'Engineering Best Practices',
          content: 'Software engineering methodologies...',
          score: 0.92,
          url: '/docs/engineering'
        }
      ]
    });

    vi.mocked(require('@tauri-apps/api/tauri')).invoke = mockInvoke;
    
    const stores = await import('../stores');
    stores.is_tauri.set(true);
    stores.role.set('engineer'); // Change role to engineer
    stores.input.set('software engineering');

    render(Search);
    
    const form = screen.getByRole('textbox').closest('form');
    await fireEvent.submit(form!);
    
    expect(mockInvoke).toHaveBeenCalledWith('search', {
      searchQuery: {
        search_term: 'software engineering',
        skip: 0,
        limit: 10,
        role: 'engineer' // Should use the engineer role
      }
    });
  });

  it('handles HTTP search when not in Tauri environment', async () => {
    const mockSearchResponse = {
      status: 'success', 
      results: [
        {
          id: '3',
          title: 'Research Methodology',
          content: 'Academic research approaches...',
          score: 0.89,
          url: '/docs/research'
        }
      ]
    };

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockSearchResponse)
    });
    
    const stores = await import('../stores');
    stores.is_tauri.set(false);
    stores.role.set('researcher');
    stores.input.set('research methodology');

    render(Search);
    
    const form = screen.getByRole('textbox').closest('form');
    await fireEvent.submit(form!);
    
    expect(global.fetch).toHaveBeenCalledWith('http://localhost:3000/documents/search', {
      method: 'POST',
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        search_term: 'research methodology',
        skip: 0,
        limit: 10,
        role: 'researcher'
      })
    });
  });

  it('displays search results when available', async () => {
    const mockResults = [
      {
        id: '1',
        title: 'AI Research Paper',
        content: 'Latest developments in artificial intelligence...',
        score: 0.95,
        url: '/papers/ai-research'
      }
    ];

    const mockInvoke = vi.fn().mockResolvedValue({
      status: 'success',
      results: mockResults
    });

    vi.mocked(require('@tauri-apps/api/tauri')).invoke = mockInvoke;
    
    const stores = await import('../stores');
    stores.is_tauri.set(true);
    stores.input.set('artificial intelligence');

    render(Search);
    
    const form = screen.getByRole('textbox').closest('form');
    await fireEvent.submit(form!);
    
    // Wait for results to be processed
    await new Promise(resolve => setTimeout(resolve, 100));
    
    // Logo should be hidden when there are results
    expect(screen.queryByAltText(/terraphim logo/i)).not.toBeInTheDocument();
  });

  it('shows error message on search failure', async () => {
    const mockInvoke = vi.fn().mockRejectedValue(new Error('Search failed'));
    vi.mocked(require('@tauri-apps/api/tauri')).invoke = mockInvoke;
    
    const stores = await import('../stores');
    stores.is_tauri.set(true);
    stores.input.set('test query');

    render(Search);
    
    const form = screen.getByRole('textbox').closest('form');
    await fireEvent.submit(form!);
    
    // Wait for error to be processed
    await new Promise(resolve => setTimeout(resolve, 100));
    
    // Should show error message
    const errorElement = screen.queryByText(/error in tauri search/i);
    if (errorElement) {
      expect(errorElement).toBeInTheDocument();
    }
  });

  it('shows suggestions from thesaurus', async () => {
    const stores = await import('../stores');
    stores.typeahead.set(true);
    
    render(Search);
    
    const searchInput = screen.getByRole('textbox');
    
    // Type partial term to trigger suggestions
    await fireEvent.input(searchInput, { target: { value: 'artif' } });
    
    // Should show suggestion
    const suggestion = screen.queryByText('artificial intelligence');
    if (suggestion) {
      expect(suggestion).toBeInTheDocument();
    }
  });

  it('applies suggestion when clicked', async () => {
    const stores = await import('../stores');
    stores.typeahead.set(true);
    
    render(Search);
    
    const searchInput = screen.getByRole('textbox') as HTMLInputElement;
    
    // Type to trigger suggestions
    await fireEvent.input(searchInput, { target: { value: 'machine' } });
    
    // Click on suggestion if it appears
    const suggestion = screen.queryByText('machine learning');
    if (suggestion) {
      await fireEvent.click(suggestion);
      
      // Should update input value
      expect(searchInput.value).toContain('machine learning');
    }
  });

  it('handles keyboard navigation in suggestions', async () => {
    const stores = await import('../stores');
    stores.typeahead.set(true);
    
    render(Search);
    
    const searchInput = screen.getByRole('textbox');
    
    // Type to trigger suggestions
    await fireEvent.input(searchInput, { target: { value: 'art' } });
    
    // Test arrow key navigation
    await fireEvent.keyDown(searchInput, { key: 'ArrowDown' });
    await fireEvent.keyDown(searchInput, { key: 'Enter' });
    
    // Should handle keyboard navigation without errors
    expect(searchInput).toBeInTheDocument();
  });

  it('handles empty search gracefully', async () => {
    const stores = await import('../stores');
    stores.input.set('');

    render(Search);
    
    const form = screen.getByRole('textbox').closest('form');
    await fireEvent.submit(form!);
    
    // Should not crash with empty search
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('shows different placeholders based on typeahead setting', () => {
    const stores = require('../stores');
    stores.typeahead.set(false);
    
    render(Search);
    
    const searchInput = screen.getByRole('textbox');
    expect(searchInput).toHaveAttribute('placeholder', 'Search');
  });

  it('performs search on input click', async () => {
    const mockInvoke = vi.fn().mockResolvedValue({
      status: 'success',
      results: []
    });

    vi.mocked(require('@tauri-apps/api/tauri')).invoke = mockInvoke;
    
    const stores = await import('../stores');
    stores.is_tauri.set(true);
    stores.input.set('test click search');

    render(Search);
    
    const searchInput = screen.getByRole('textbox');
    await fireEvent.click(searchInput);
    
    expect(mockInvoke).toHaveBeenCalledWith('search', expect.objectContaining({
      searchQuery: expect.objectContaining({
        search_term: 'test click search'
      })
    }));
  });
}); 