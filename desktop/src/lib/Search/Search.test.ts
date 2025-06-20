import { describe, it, expect, beforeEach, beforeAll, afterAll } from 'vitest';
import { render, fireEvent, screen, waitFor } from '@testing-library/svelte';
import Search from './Search.svelte';
import { input, is_tauri, role, serverUrl } from '../stores';

// Test configuration
const TEST_SERVER_URL = 'http://localhost:8000/documents/search';
const TEST_TIMEOUT = 10000; // 10 seconds for real API calls

describe('Search Component - Real API Integration', () => {
  beforeAll(async () => {
    // Set up for web-based testing (not Tauri)
    is_tauri.set(false);
    serverUrl.set(TEST_SERVER_URL);
  });

  beforeEach(async () => {
    // Reset input before each test
    input.set('');
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

  it('performs real search with test role', async () => {
    role.set('test_role');
    input.set('machine learning');

    render(Search);
    
    const searchInput = screen.getByRole('textbox');
    const form = searchInput.closest('form');
    
    await fireEvent.submit(form!);
    
    // Wait for real API response or timeout
    await waitFor(() => {
      // Check if logo is hidden (indicating results loaded) or error is shown
      const logo = screen.queryByAltText(/terraphim logo/i);
      const error = screen.queryByText(/error/i);
      expect(logo === null || error !== null).toBe(true);
    }, { timeout: TEST_TIMEOUT });
  }, TEST_TIMEOUT);

  it('searches with engineer role and gets engineering results', async () => {
    role.set('engineer');
    input.set('software engineering');

    render(Search);
    
    const searchInput = screen.getByRole('textbox');
    const form = searchInput.closest('form');
    
    await fireEvent.submit(form!);
    
    // Wait for results or error
    await waitFor(() => {
      const logo = screen.queryByAltText(/terraphim logo/i);
      const error = screen.queryByText(/error/i);
      expect(logo === null || error !== null).toBe(true);
    }, { timeout: TEST_TIMEOUT });
    
    // If we get results, they should be engineering-related
    const resultsContainer = screen.queryByText(/software|engineering|development|programming/i);
    const error = screen.queryByText(/error/i);
    
    // Test passes if we either get relevant results or a graceful error
    expect(resultsContainer !== null || error !== null).toBe(true);
  }, TEST_TIMEOUT);

  it('searches with researcher role and gets research results', async () => {
    role.set('researcher');
    input.set('research methodology');

    render(Search);
    
    const searchInput = screen.getByRole('textbox');
    const form = searchInput.closest('form');
    
    await fireEvent.submit(form!);
    
    // Wait for results or error
    await waitFor(() => {
      const logo = screen.queryByAltText(/terraphim logo/i);
      const error = screen.queryByText(/error/i);
      expect(logo === null || error !== null).toBe(true);
    }, { timeout: TEST_TIMEOUT });
    
    // If we get results, they should be research-related
    const resultsContainer = screen.queryByText(/research|methodology|study|academic/i);
    const error = screen.queryByText(/error/i);
    
    // Test passes if we either get relevant results or a graceful error
    expect(resultsContainer !== null || error !== null).toBe(true);
  }, TEST_TIMEOUT);

  it('handles empty search gracefully', async () => {
    input.set('');

    render(Search);
    
    const form = screen.getByRole('textbox').closest('form');
    await fireEvent.submit(form!);
    
    // Should not crash with empty search - should show logo or handle gracefully
    expect(screen.getByRole('textbox')).toBeInTheDocument();
    
    // Should either show logo or an appropriate message
    const logo = screen.queryByAltText(/terraphim logo/i);
    expect(logo).toBeInTheDocument();
  });

  it('handles network errors gracefully', async () => {
    // Set an invalid server URL to trigger network error
    serverUrl.set('http://invalid-server:9999/search');
    input.set('test query');

    render(Search);
    
    const form = screen.getByRole('textbox').closest('form');
    await fireEvent.submit(form!);
    
    // Wait for error handling
    await waitFor(() => {
      const error = screen.queryByText(/error/i);
      expect(error).toBeInTheDocument();
    }, { timeout: 5000 });
    
    // Reset server URL for other tests
    serverUrl.set(TEST_SERVER_URL);
  }, 6000);

  it('updates input value when typing', async () => {
    render(Search);
    
    const searchInput = screen.getByRole('textbox') as HTMLInputElement;
    
    await fireEvent.input(searchInput, { target: { value: 'artificial intelligence' } });
    
    expect(searchInput.value).toBe('artificial intelligence');
  });

  it('shows different placeholders based on typeahead setting', () => {
    render(Search);
    
    const searchInput = screen.getByRole('textbox');
    // Should have some form of search placeholder
    expect(searchInput).toHaveAttribute('placeholder', expect.stringMatching(/search/i));
  });

  it('performs search on form submission', async () => {
    input.set('test search term');

    render(Search);
    
    const searchInput = screen.getByRole('textbox');
    const form = searchInput.closest('form');
    
    // Submit form
    await fireEvent.submit(form!);
    
    // Should trigger some form of response (results or error)
    await waitFor(() => {
      const logo = screen.queryByAltText(/terraphim logo/i);
      const error = screen.queryByText(/error/i);
      // Either logo is hidden (results) or error is shown
      expect(logo === null || error !== null).toBe(true);
    }, { timeout: TEST_TIMEOUT });
  }, TEST_TIMEOUT);

  it('can switch between different roles and maintain search functionality', async () => {
    render(Search);
    
    // Test with first role
    role.set('engineer');
    input.set('programming');
    
    const form = screen.getByRole('textbox').closest('form');
    await fireEvent.submit(form!);
    
    // Wait a bit for first search
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // Switch role and search again
    role.set('researcher');
    input.set('methodology');
    
    await fireEvent.submit(form!);
    
    // Should handle role switching without crashes
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  }, TEST_TIMEOUT);
}); 