<<<<<<< HEAD
import { describe, it, expect, beforeEach, beforeAll } from 'vitest';
import { render, fireEvent, screen, waitFor } from '@testing-library/svelte/svelte5';
=======
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { is_tauri, role, theme } from './stores';
>>>>>>> origin/main
import ThemeSwitcher from './ThemeSwitcher.svelte';

// Test configuration
const TEST_TIMEOUT = 5000; // 5 seconds for API calls

// Stub TAURI IPC to prevent invoke errors
(global as any).__TAURI_IPC__ = () => {};

// Mock fetch for config loading
const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('ThemeSwitcher Component - Real Integration', () => {
<<<<<<< HEAD
  beforeAll(async () => {
    // Set up for web-based testing (not Tauri)
    is_tauri.set(false);
  });

  beforeEach(async () => {
    // Reset to default state
    role.set('Test Role');
    theme.set('spacelab');
    
    // Mock successful config response
    mockFetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({
        status: 'success',
        config: {
          id: 'test-config',
          global_shortcut: 'Ctrl+Shift+S',
          roles: {
            'Test Role': {
              name: 'Test Role',
              shortname: 'test',
              relevance_function: 'TitleScorer',
              terraphim_it: false,
              theme: 'spacelab',
              haystacks: [],
              kg: { url: '', local_path: '', local_type: 'json', public: false, publish: false }
            },
            'Engineer': {
              name: 'Engineer',
              shortname: 'engineer',
              relevance_function: 'TitleScorer',
              terraphim_it: false,
              theme: 'spacelab',
              haystacks: [],
              kg: { url: '', local_path: '', local_type: 'json', public: false, publish: false }
            }
          },
          selected_role: 'Test Role'
        }
      })
    });
  });
=======
	// Skip integration tests in CI environment where server setup is complex
	const isCI = process.env.CI || process.env.GITHUB_ACTIONS;

	beforeAll(async () => {
		// Set up for web-based testing (not Tauri)
		is_tauri.set(false);
	});

	beforeEach(async () => {
		// Reset to default state
		role.set('Test Role');
		theme.set('spacelab');
	});

	it('renders role selector dropdown', () => {
		render(ThemeSwitcher);

		const selectElement = screen.getByRole('combobox');
		expect(selectElement).toBeInTheDocument();
	});

	it(
		'displays available roles in dropdown',
		async () => {
			if (isCI) {
				console.log('Skipping ThemeSwitcher integration test in CI environment');
				return;
			}

			render(ThemeSwitcher);

			await waitFor(
				() => {
					const selectElement = screen.getByRole('combobox') as HTMLSelectElement;
					expect(selectElement.options.length).toBeGreaterThan(0);
				},
				{ timeout: TEST_TIMEOUT }
			);
		},
		TEST_TIMEOUT
	);

	it('changes role when option is selected', async () => {
		render(ThemeSwitcher);

		const selectElement = screen.getByRole('combobox') as HTMLSelectElement;

		// Wait for component to be ready
		await waitFor(() => {
			expect(selectElement).toBeInTheDocument();
		});

		// Get current value
		const initialValue = selectElement.value;

		// Find a different option to select
		const options = Array.from(selectElement.querySelectorAll('option'));
		const differentOption = options.find((opt) => opt.value !== initialValue);

		if (differentOption) {
			await fireEvent.change(selectElement, { target: { value: differentOption.value } });

			// Should update the select value
			expect(selectElement.value).toBe(differentOption.value);
		}
	});

	it(
		'loads and displays configuration from server',
		async () => {
			if (isCI) {
				console.log('Skipping ThemeSwitcher integration test in CI environment');
				return;
			}

			render(ThemeSwitcher);

			// Wait for config to load
			await waitFor(
				() => {
					const selectElement = screen.getByRole('combobox');
					const options = selectElement.querySelectorAll('option');
					expect(options.length).toBeGreaterThan(0);
				},
				{ timeout: TEST_TIMEOUT }
			);

			// Should have loaded roles from configuration
			const selectElement = screen.getByRole('combobox');
			expect(selectElement).toBeInTheDocument();
		},
		TEST_TIMEOUT
	);

	it('handles role switching and theme updates', async () => {
		render(ThemeSwitcher);

		const selectElement = screen.getByRole('combobox') as HTMLSelectElement;

		// Wait for component to be ready
		await waitFor(() => {
			expect(selectElement).toBeInTheDocument();
		});

		// Get all available options
		const options = Array.from(selectElement.querySelectorAll('option')) as HTMLOptionElement[];

		if (options.length > 1) {
			// Switch to a different role
			const newRole = options[1].value;
			await fireEvent.change(selectElement, { target: { value: newRole } });

			// Should update without crashing
			expect(selectElement.value).toBe(newRole);
		}
	});

	it('handles configuration fetch errors gracefully', async () => {
		// Set invalid server URL to trigger error
		const originalFetch = global.fetch;
		global.fetch = () => Promise.reject(new Error('Network error'));

		render(ThemeSwitcher);

		// Should render without crashing even with network errors
		await waitFor(() => {
			const selectElement = screen.getByRole('combobox');
			expect(selectElement).toBeInTheDocument();
		});

		// Restore original fetch
		global.fetch = originalFetch;
	});

	it('maintains theme consistency across role changes', async () => {
		render(ThemeSwitcher);

		const selectElement = screen.getByRole('combobox') as HTMLSelectElement;

		// Wait for component to load
		await waitFor(() => {
			expect(selectElement).toBeInTheDocument();
		});

		const options = Array.from(selectElement.querySelectorAll('option')) as HTMLOptionElement[];

		// Test switching between multiple roles if available
		for (let i = 0; i < Math.min(options.length, 3); i++) {
			await fireEvent.change(selectElement, { target: { value: options[i].value } });

			// Should update successfully
			expect(selectElement.value).toBe(options[i].value);

			// Small delay between changes
			await new Promise((resolve) => setTimeout(resolve, 100));
		}
	});

	it('handles Tauri environment detection', () => {
		// Test with simulated Tauri environment
		window.__TAURI__ = { invoke: () => Promise.resolve() } as any;

		render(ThemeSwitcher);

		// Component should render without errors in Tauri environment
		expect(screen.getByRole('combobox')).toBeInTheDocument();

		// Clean up
		(window as any).__TAURI__ = undefined;
	});

	it('handles non-Tauri environment', () => {
		// Ensure we're in non-Tauri environment
		(window as any).__TAURI__ = undefined;
>>>>>>> origin/main

		render(ThemeSwitcher);

		// Component should render without errors in non-Tauri environment
		expect(screen.getByRole('combobox')).toBeInTheDocument();
	});

	it(
		'displays role names correctly',
		async () => {
			if (isCI) {
				console.log('Skipping ThemeSwitcher integration test in CI environment');
				return;
			}

			render(ThemeSwitcher);

			await waitFor(
				() => {
					const selectElement = screen.getByRole('combobox');
					const options = selectElement.querySelectorAll('option');
					expect(options.length).toBeGreaterThan(0);
				},
				{ timeout: TEST_TIMEOUT }
			);

			// Should have meaningful role names
			const selectElement = screen.getByRole('combobox');
			const options = Array.from(selectElement.querySelectorAll('option'));

			options.forEach((option) => {
				expect(option.textContent).toBeTruthy();
				expect(option.textContent?.length).toBeGreaterThan(0);
			});
		},
		TEST_TIMEOUT
	);

	it('persists role selection across interactions', async () => {
		render(ThemeSwitcher);

		const selectElement = screen.getByRole('combobox') as HTMLSelectElement;

		await waitFor(() => {
			expect(selectElement).toBeInTheDocument();
		});

		const options = Array.from(selectElement.querySelectorAll('option')) as HTMLOptionElement[];

		if (options.length > 1) {
			const selectedRole = options[1].value;

			// Select a role
			await fireEvent.change(selectElement, { target: { value: selectedRole } });

			// Should maintain the selection
			expect(selectElement.value).toBe(selectedRole);

			// Interact with component again
			await fireEvent.blur(selectElement);
			await fireEvent.focus(selectElement);

			// Should still have the same selection
			expect(selectElement.value).toBe(selectedRole);
		}
	});
});
