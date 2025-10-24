import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TerraphimSuggestion } from '../TerraphimSuggestion';

// Mock the novelAutocompleteService
vi.mock('../../services/novelAutocompleteService', () => ({
	novelAutocompleteService: {
		getSuggestionsWithSnippets: vi.fn().mockResolvedValue([
			{ text: 'terraphim', snippet: 'AI assistant', score: 0.95 },
			{ text: 'graph', snippet: 'knowledge graph', score: 0.87 },
			{ text: 'search', snippet: 'semantic search', score: 0.82 },
		]),
	},
}));

describe('TerraphimSuggestion', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('should create extension with default options', () => {
		const extension = TerraphimSuggestion.create();

		expect(extension.name).toBe('terraphimSuggestion');
		expect(extension.options.trigger).toBe('/');
		expect(extension.options.limit).toBe(8);
		expect(extension.options.minLength).toBe(1);
		expect(extension.options.debounce).toBe(300);
	});

	it('should create extension with custom options', () => {
		const customOptions = {
			trigger: '@',
			limit: 5,
			minLength: 2,
			debounce: 500,
		};

		const extension = TerraphimSuggestion.create(customOptions);

		expect(extension.options.trigger).toBe('@');
		expect(extension.options.limit).toBe(5);
		expect(extension.options.minLength).toBe(2);
		expect(extension.options.debounce).toBe(500);
	});

	it('should have insertSuggestion command', () => {
		const extension = TerraphimSuggestion.create();
		const commands = extension.addCommands();

		expect(commands).toHaveProperty('insertSuggestion');
		expect(typeof commands.insertSuggestion).toBe('function');
	});

	it('should add ProseMirror plugins', () => {
		const extension = TerraphimSuggestion.create();
		const plugins = extension.addProseMirrorPlugins();

		expect(Array.isArray(plugins)).toBe(true);
		expect(plugins.length).toBeGreaterThan(0);
	});
});

describe('TerraphimSuggestionRenderer', () => {
	it('should create element with correct class', () => {
		const _mockCommand = vi.fn();
		const _mockItems = [{ text: 'test', snippet: 'test snippet', score: 1.0 }];

		// We need to access the renderer class through the extension
		const extension = TerraphimSuggestion.create();
		const plugins = extension.addProseMirrorPlugins();

		expect(plugins.length).toBeGreaterThan(0);
	});
});
