import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte/svelte5';
import NovelWrapper from '../NovelWrapper.svelte';

// Mock the novelAutocompleteService
vi.mock('../../services/novelAutocompleteService', () => ({
	novelAutocompleteService: {
		setRole: vi.fn(),
		testConnection: vi.fn().mockResolvedValue(true),
		buildAutocompleteIndex: vi.fn().mockResolvedValue(true),
		getStatus: vi.fn().mockReturnValue({
			ready: true,
			baseUrl: 'http://localhost:8000',
			sessionId: 'test-session',
			usingTauri: false,
			currentRole: 'Test Role',
			connectionRetries: 0,
			isConnecting: false,
		}),
	},
}));

// Mock the stores
vi.mock('../../stores', () => ({
	is_tauri: { get: vi.fn(() => false) },
	role: { get: vi.fn(() => 'Test Role') },
}));

// Mock the TerraphimSuggestion
vi.mock('../TerraphimSuggestion', () => ({
	TerraphimSuggestion: {
		create: vi.fn().mockReturnValue({
			name: 'terraphimSuggestion',
			options: {
				trigger: '/',
				limit: 8,
				minLength: 1,
				debounce: 300,
			},
		}),
	},
	terraphimSuggestionStyles: '/* CSS styles */',
}));

describe('NovelWrapper', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('should render the Novel editor', () => {
		render(NovelWrapper, {
			props: {
				html: '<p>Test content</p>',
				readOnly: false,
				enableAutocomplete: false,
			},
		});

		// The Novel editor should be rendered
		expect(document.querySelector('.novel-editor')).toBeTruthy();
	});

	it('should render autocomplete controls when enabled', () => {
		render(NovelWrapper, {
			props: {
				html: '<p>Test content</p>',
				readOnly: false,
				enableAutocomplete: true,
			},
		});

		// Should show autocomplete status
		expect(screen.getByText('Local Autocomplete Status:')).toBeInTheDocument();
	});

	it('should show autocomplete configuration', () => {
		render(NovelWrapper, {
			props: {
				html: '<p>Test content</p>',
				readOnly: false,
				enableAutocomplete: true,
				suggestionTrigger: '@',
				maxSuggestions: 5,
			},
		});

		// Should show configuration details
		expect(screen.getByText('Configuration:')).toBeInTheDocument();
		expect(screen.getByText('Trigger: "@" + text')).toBeInTheDocument();
		expect(screen.getByText('Max Results: 5')).toBeInTheDocument();
	});

	it('should render test and rebuild buttons', () => {
		render(NovelWrapper, {
			props: {
				html: '<p>Test content</p>',
				readOnly: false,
				enableAutocomplete: true,
			},
		});

		// Should show control buttons
		expect(screen.getByText('Test')).toBeInTheDocument();
		expect(screen.getByText('Rebuild Index')).toBeInTheDocument();
		expect(screen.getByText('Demo')).toBeInTheDocument();
	});

	it('should not render autocomplete controls when disabled', () => {
		render(NovelWrapper, {
			props: {
				html: '<p>Test content</p>',
				readOnly: false,
				enableAutocomplete: false,
			},
		});

		// Should not show autocomplete status
		expect(screen.queryByText('Local Autocomplete Status:')).not.toBeInTheDocument();
	});
});
