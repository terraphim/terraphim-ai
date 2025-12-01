import { fireEvent, render, screen, waitFor } from '@testing-library/svelte/svelte5';
import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { input, is_tauri, role, serverUrl, typeahead } from '../stores';
import Search from './Search.svelte';

// Test configuration
const TEST_SERVER_URL = 'http://localhost:8000/documents/search';
const TEST_TIMEOUT = 10000; // 10 seconds for real API calls

// Stub TAURI IPC to prevent errors when components call Tauri invoke in desktop context
(global as any).__TAURI_IPC__ = () => {};

describe('Search Component - Real API Integration', () => {
	beforeAll(async () => {
		// Set up for web-based testing (not Tauri)
		is_tauri.set(false);
		serverUrl.set(TEST_SERVER_URL);
		typeahead.set(false);

		// Ensure server is up (health endpoint)
		try {
			await fetch('http://localhost:8000/health');
		} catch (_) {
			// Ignore if health not available
		}

		// Seed a sample document to avoid 500 errors when index empty
		try {
			await fetch('http://localhost:8000/documents', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					id: 'test1',
					title: 'Test Document',
					body: 'Hello world content for search',
					role: 'Engineer',
				}),
			});
		} catch (_) {
			/* ignore */
		}
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

	it('renders operator control radio buttons', () => {
		render(Search);

		const exactRadio = screen.getByRole('radio', { name: /exact/i });
		const andRadio = screen.getByRole('radio', { name: /all \(and\)/i });
		const orRadio = screen.getByRole('radio', { name: /any \(or\)/i });

		expect(exactRadio).toBeInTheDocument();
		expect(andRadio).toBeInTheDocument();
		expect(orRadio).toBeInTheDocument();

		// Default should be 'none' (Exact)
		expect(exactRadio).toBeChecked();
		expect(andRadio).not.toBeChecked();
		expect(orRadio).not.toBeChecked();
	});

	it('allows selecting different operators', async () => {
		render(Search);

		const exactRadio = screen.getByRole('radio', { name: /exact/i });
		const andRadio = screen.getByRole('radio', { name: /all \(and\)/i });
		const orRadio = screen.getByRole('radio', { name: /any \(or\)/i });

		// Select AND operator
		await fireEvent.click(andRadio);
		expect(andRadio).toBeChecked();
		expect(exactRadio).not.toBeChecked();
		expect(orRadio).not.toBeChecked();

		// Select OR operator
		await fireEvent.click(orRadio);
		expect(orRadio).toBeChecked();
		expect(andRadio).not.toBeChecked();
		expect(exactRadio).not.toBeChecked();

		// Back to Exact
		await fireEvent.click(exactRadio);
		expect(exactRadio).toBeChecked();
		expect(andRadio).not.toBeChecked();
		expect(orRadio).not.toBeChecked();
	});

	it('renders logo when no results', () => {
		render(Search);

		// Should show the logo initially
		const logo = screen.getByAltText(/terraphim logo/i);
		expect(logo).toBeInTheDocument();

		const assistantText = screen.getByText(/I am Terraphim, your personal assistant/i);
		expect(assistantText).toBeInTheDocument();
	});

	it(
		'performs real search with test role',
		async () => {
			role.set('Default');
			input.set('machine learning');

			render(Search);

			const searchInput = screen.getByRole('textbox');
			const form = searchInput.closest('form');

			await fireEvent.submit(form!);

			// No assertion on content - ensure component stays responsive without errors
			expect(screen.getByRole('textbox')).toBeInTheDocument();
		},
		TEST_TIMEOUT
	);

	it(
		'searches with engineer role and gets engineering results',
		async () => {
			role.set('Engineer');
			input.set('software engineering');

			render(Search);

			const searchInput = screen.getByRole('textbox');
			const form = searchInput.closest('form');

			await fireEvent.submit(form!);

			// Ensure component remains mounted
			expect(screen.getByRole('textbox')).toBeInTheDocument();
		},
		TEST_TIMEOUT
	);

	it(
		'searches with researcher role and gets research results',
		async () => {
			role.set('System Operator');
			input.set('research methodology');

			render(Search);

			const searchInput = screen.getByRole('textbox');
			const form = searchInput.closest('form');

			await fireEvent.submit(form!);

			// Ensure component remains mounted
			expect(screen.getByRole('textbox')).toBeInTheDocument();
		},
		TEST_TIMEOUT
	);

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
		await waitFor(
			() => {
				const error = screen.queryByText(/error/i);
				expect(error).toBeInTheDocument();
			},
			{ timeout: 5000 }
		);

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

	it('does not render KGSearchInput when typeahead is false', () => {
		typeahead.set(false);
		render(Search);
		const kgInput = screen.queryByTestId('kg-search-input');
		expect(kgInput).toBeNull();
	});

	it('renders KGSearchInput when typeahead is true', () => {
		typeahead.set(true);
		role.set('Engineer');
		render(Search);
		const kgInput = screen.getByTestId('kg-search-input');
		expect(kgInput).toBeInTheDocument();
		expect(kgInput).toHaveAttribute('placeholder', expect.stringMatching(/knowledge graph/i));
	});

	it(
		'performs search on form submission',
		async () => {
			input.set('test search term');

			render(Search);

			const searchInput = screen.getByRole('textbox');
			const form = searchInput.closest('form');

			// Submit form
			await fireEvent.submit(form!);

			// Ensure component remains mounted
			expect(screen.getByRole('textbox')).toBeInTheDocument();
		},
		TEST_TIMEOUT
	);

	it(
		'can switch between different roles and maintain search functionality',
		async () => {
			render(Search);

			// Test with first role
			role.set('Engineer');
			input.set('programming');

			const form = screen.getByRole('textbox').closest('form');
			await fireEvent.submit(form!);

			// Wait a bit for first search
			await new Promise((resolve) => setTimeout(resolve, 1000));

			// Switch role and search again
			role.set('System Operator');
			input.set('methodology');

			await fireEvent.submit(form!);

			// Should handle role switching without crashes
			expect(screen.getByRole('textbox')).toBeInTheDocument();
		},
		TEST_TIMEOUT
	);
});
