import { test, expect } from '@playwright/test';
import { spawn, ChildProcess } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const ATOMIC_SERVER_URL = process.env.ATOMIC_SERVER_URL || 'http://localhost:9883/';
const ATOMIC_SERVER_SECRET = process.env.ATOMIC_SERVER_SECRET;

// Skip tests if atomic server secret is not available
const shouldSkipAtomicTests = !ATOMIC_SERVER_SECRET;

class TerraphimServerManager {
	private process: ChildProcess | null = null;
	private port: number = 8000;

	async start(): Promise<void> {
		console.log('🚀 Starting Terraphim server with new storage backend...');

		const serverPath = path.join(__dirname, '../../../target/release/terraphim_server');

		this.process = spawn(serverPath, [], {
			stdio: ['ignore', 'pipe', 'pipe'],
			cwd: path.join(__dirname, '../../..'),
		});

		return new Promise((resolve, reject) => {
			const timeout = setTimeout(() => {
				reject(new Error('Terraphim server startup timeout'));
			}, 30000);

			this.process!.stdout?.on('data', (data) => {
				const output = data.toString();
				console.log('Terraphim server:', output.trim());
				if (output.includes('listening on')) {
					clearTimeout(timeout);
					resolve();
				}
			});

			this.process!.stderr?.on('data', (data) => {
				const error = data.toString();
				console.error('Terraphim server error:', error.trim());
				// Don't reject on stderr - some warnings are normal
			});

			this.process!.on('error', (error) => {
				clearTimeout(timeout);
				reject(error);
			});

			this.process!.on('exit', (code) => {
				if (code !== 0) {
					clearTimeout(timeout);
					reject(new Error(`Terraphim server exited with code ${code}`));
				}
			});
		});
	}

	async waitForReady(): Promise<void> {
		console.log('⏳ Waiting for Terraphim server to be ready...');

		for (let i = 0; i < 30; i++) {
			try {
				const response = await fetch(`http://localhost:${this.port}/health`);
				if (response.ok) {
					console.log('✅ Terraphim server is ready');
					return;
				}
			} catch {
				// Continue waiting
			}
			await new Promise((resolve) => setTimeout(resolve, 1000));
		}
		throw new Error('Terraphim server failed to become ready');
	}

	async stop(): Promise<void> {
		if (this.process) {
			this.process.kill('SIGTERM');
			await new Promise((resolve) => setTimeout(resolve, 2000));
			if (!this.process.killed) {
				this.process.kill('SIGKILL');
			}
			this.process = null;
			console.log('🛑 Terraphim server stopped');
		}
	}
}

test.describe('Atomic Server Tests', () => {
	test('should connect to atomic server', async () => {
		test.skip(shouldSkipAtomicTests, 'ATOMIC_SERVER_SECRET not available');
		const response = await fetch(ATOMIC_SERVER_URL);
		expect(response.status).toBeLessThan(500);
		console.log('✅ Atomic server is accessible');
	});

	test('should validate environment variables', async () => {
		test.skip(shouldSkipAtomicTests, 'ATOMIC_SERVER_SECRET not available');
		expect(ATOMIC_SERVER_URL).toBeTruthy();
		expect(ATOMIC_SERVER_SECRET).toBeTruthy();
		expect(ATOMIC_SERVER_URL).toMatch(/^https?:\/\//);
		console.log('✅ Environment variables are valid');
	});
});

test.describe('Complete Atomic Server Integration', () => {
	test.skip(shouldSkipAtomicTests, 'ATOMIC_SERVER_SECRET not available');
	let terraphimServer: TerraphimServerManager;
	const configPath = path.join(__dirname, 'atomic-integration-config.json');

	test.beforeAll(async () => {
		// Create role configuration with atomic server haystack
		const config = {
			id: 'Server',
			global_shortcut: 'Ctrl+Shift+A',
			roles: {
				'Atomic Integration Test': {
					shortname: 'AtomicTest',
					name: 'Atomic Integration Test',
					relevance_function: 'title-scorer',
					theme: 'spacelab',
					kg: null,
					haystacks: [
						{
							location: ATOMIC_SERVER_URL,
							service: 'Atomic',
							read_only: true,
							atomic_server_secret: ATOMIC_SERVER_SECRET,
						},
					],
					extra: {},
					terraphim_it: false,
				},
			},
			default_role: 'Atomic Integration Test',
			selected_role: 'Atomic Integration Test',
		};

		fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
		console.log('📝 Created atomic server role configuration');

		// Start Terraphim server
		terraphimServer = new TerraphimServerManager();
		await terraphimServer.start();
		await terraphimServer.waitForReady();
	});

	test.afterAll(async () => {
		if (terraphimServer) {
			await terraphimServer.stop();
		}

		// Cleanup
		if (fs.existsSync(configPath)) {
			fs.unlinkSync(configPath);
		}
	});

	test('should perform atomic server haystack search and return results', async () => {
		console.log('🔍 Testing atomic server haystack search...');

		try {
			// Update Terraphim server configuration with atomic role
			const updateResponse = await fetch('http://localhost:8000/config', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: fs.readFileSync(configPath, 'utf8'),
			});

			if (updateResponse.ok) {
				console.log('✅ Successfully updated Terraphim server config');
			} else {
				console.log(
					'⚠️ Config update response:',
					updateResponse.status,
					await updateResponse.text()
				);
			}

			// Wait a moment for configuration to be applied
			await new Promise((resolve) => setTimeout(resolve, 3000));

			// Perform search through atomic haystack
			const searchResponse = await fetch('http://localhost:8000/documents/search', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify({
					search_term: 'test',
					role: 'Atomic Integration Test',
					limit: 10,
				}),
			});

			console.log('🔍 Search response status:', searchResponse.status);
			console.log(
				'🔍 Search response headers:',
				Object.fromEntries(searchResponse.headers.entries())
			);

			if (!searchResponse.ok) {
				const errorText = await searchResponse.text();
				console.log('❌ Search response error:', errorText);
				throw new Error(`Search failed: ${searchResponse.status} - ${errorText}`);
			}

			// Check if response is JSON
			const contentType = searchResponse.headers.get('content-type');
			if (!contentType || !contentType.includes('application/json')) {
				const responseText = await searchResponse.text();
				console.log('⚠️ Non-JSON response:', responseText.substring(0, 200));
				throw new Error('Expected JSON response but got: ' + contentType);
			}

			const searchResults = await searchResponse.json();
			console.log('🔍 Search results:', JSON.stringify(searchResults, null, 2));

			// Verify we got results structure
			expect(searchResults).toBeDefined();

			// Even if no documents match, we should get a valid response structure
			if (searchResults.results && Array.isArray(searchResults.results)) {
				console.log(
					`✅ Search returned ${searchResults.results.length} results from atomic server haystack`
				);

				// If we have results, verify they have the expected structure
				if (searchResults.results.length > 0) {
					const firstResult = searchResults.results[0];
					expect(firstResult).toHaveProperty('content');
					console.log('✅ Search results have proper structure');
					console.log('✅ ATOMIC SERVER HAYSTACK INTEGRATION WORKING - RESULTS RETURNED!');
				} else {
					console.log('ℹ️ No results found, but search structure is valid');
				}
			} else {
				console.log('ℹ️ Search response structure:', Object.keys(searchResults));
			}
		} catch (error) {
			console.error('❌ Search test error:', error);
			throw error;
		}
	});

	test('should validate atomic server role configuration structure', async () => {
		console.log('🔧 Testing role configuration structure...');

		const configData = fs.readFileSync(configPath, 'utf8');
		const config = JSON.parse(configData);

		expect(config.roles['Atomic Integration Test']).toBeDefined();
		expect(config.roles['Atomic Integration Test'].haystacks).toHaveLength(1);
		expect(config.roles['Atomic Integration Test'].haystacks[0].service).toBe('Atomic');
		expect(config.roles['Atomic Integration Test'].haystacks[0].location).toBe(ATOMIC_SERVER_URL);
		expect(config.roles['Atomic Integration Test'].haystacks[0].atomic_server_secret).toBe(
			ATOMIC_SERVER_SECRET
		);

		console.log('✅ Atomic server role configuration is valid');
	});
});
