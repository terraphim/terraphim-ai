import { beforeEach, describe, expect, it, vi } from 'vitest';
import { NovelAutocompleteService } from '../novelAutocompleteService';

// Mock the stores
vi.mock('../../stores', () => ({
	is_tauri: { get: vi.fn(() => false) },
	role: { get: vi.fn(() => 'Test Role') },
}));

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn(),
}));

// Mock fetch
global.fetch = vi.fn();

describe('NovelAutocompleteService', () => {
	let service: NovelAutocompleteService;

	beforeEach(() => {
		vi.clearAllMocks();
		service = new NovelAutocompleteService();
	});

	describe('constructor', () => {
		it('should initialize with default values', () => {
			expect(service.getStatus().ready).toBe(false);
			expect(service.getStatus().currentRole).toBe('Default');
			expect(service.getStatus().usingTauri).toBe(false);
		});
	});

	describe('setRole', () => {
		it('should update the current role', () => {
			service.setRole('New Role');
			expect(service.getStatus().currentRole).toBe('New Role');
		});

		it('should reset autocomplete index when role changes', () => {
			// First set a role and mark as ready
			service.setRole('Role 1');
			(service as any).autocompleteIndexBuilt = true;

			// Change role
			service.setRole('Role 2');

			expect(service.getStatus().ready).toBe(false);
		});
	});

	describe('getSuggestions', () => {
		it('should return empty array for empty query', async () => {
			const result = await service.getSuggestions('');
			expect(result).toEqual([]);
		});

		it('should return empty array for whitespace-only query', async () => {
			const result = await service.getSuggestions('   ');
			expect(result).toEqual([]);
		});

		it('should return empty array when not ready and build fails', async () => {
			vi.spyOn(service, 'buildAutocompleteIndex').mockResolvedValue(false);

			const result = await service.getSuggestions('test');
			expect(result).toEqual([]);
		});
	});

	describe('getSuggestionsWithSnippets', () => {
		it('should return empty array for empty query', async () => {
			const result = await service.getSuggestionsWithSnippets('');
			expect(result).toEqual([]);
		});

		it('should call getSuggestions internally', async () => {
			const getSuggestionsSpy = vi.spyOn(service, 'getSuggestions').mockResolvedValue([]);

			await service.getSuggestionsWithSnippets('test');
			expect(getSuggestionsSpy).toHaveBeenCalledWith('test', 10);
		});
	});

	describe('getCompletion', () => {
		it('should return empty completion for empty prompt', async () => {
			const result = await service.getCompletion({ prompt: '' });
			expect(result.text).toBe('');
		});

		it('should return empty completion when not ready', async () => {
			vi.spyOn(service, 'buildAutocompleteIndex').mockResolvedValue(false);

			const result = await service.getCompletion({ prompt: 'test' });
			expect(result.text).toBe('');
		});
	});

	describe('testConnection', () => {
		it('should return false for REST API when server is not responding', async () => {
			(global.fetch as any).mockResolvedValue({
				ok: false,
				status: 404,
			});

			const result = await service.testConnection();
			expect(result).toBe(false);
		});

		it('should return true for REST API when server is responding', async () => {
			(global.fetch as any).mockResolvedValue({
				ok: true,
				status: 200,
			});

			const result = await service.testConnection();
			expect(result).toBe(true);
		});
	});

	describe('refreshIndex', () => {
		it('should reset autocomplete index and rebuild', async () => {
			const buildSpy = vi.spyOn(service, 'buildAutocompleteIndex').mockResolvedValue(true);

			const result = await service.refreshIndex();

			expect(service.getStatus().ready).toBe(false);
			expect(buildSpy).toHaveBeenCalled();
			expect(result).toBe(true);
		});
	});

	describe('getStatus', () => {
		it('should return current service status', () => {
			const status = service.getStatus();

			expect(status).toHaveProperty('ready');
			expect(status).toHaveProperty('baseUrl');
			expect(status).toHaveProperty('sessionId');
			expect(status).toHaveProperty('usingTauri');
			expect(status).toHaveProperty('currentRole');
			expect(status).toHaveProperty('connectionRetries');
			expect(status).toHaveProperty('isConnecting');
		});
	});
});
