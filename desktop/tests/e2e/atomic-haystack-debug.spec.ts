import { test, expect } from '@playwright/test';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const ATOMIC_SERVER_URL = 'http://localhost:9883/';
const ATOMIC_SERVER_SECRET = process.env.ATOMIC_SERVER_SECRET;

test.describe('Atomic Server Haystack Debug Tests', () => {
	let configPath: string;
	let terraphimServer: any;

	test.beforeAll(async () => {
		// Create a specific test configuration for atomic server debugging
		configPath = path.join(__dirname, 'atomic-debug-config.json');

		const config = {
			id: 'AtomicDebug',
			global_shortcut: 'Ctrl+Shift+D',
			roles: {
				'Atomic Debug Test': {
					shortname: 'AtomicDebug',
					name: 'Atomic Debug Test',
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
				},
			},
			default_role: 'Atomic Debug Test',
			selected_role: 'Atomic Debug Test',
		};

		fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
		console.log('🔧 Created atomic debug configuration');
	});

	test.afterAll(async () => {
		// Cleanup
		if (fs.existsSync(configPath)) {
			fs.unlinkSync(configPath);
		}
	});

	test('should debug atomic server secret format', async ({ page }) => {
		console.log('🔍 Debugging atomic server secret format...');

		if (!ATOMIC_SERVER_SECRET) {
			throw new Error('ATOMIC_SERVER_SECRET environment variable is not set');
		}

		// Test the secret format in the browser context
		const secretAnalysis = await page.evaluate((secret) => {
			try {
				console.log('Secret length:', secret.length);
				console.log('Secret (first 50 chars):', secret.substring(0, 50));

				// Test base64 decoding
				const decoded = atob(secret);
				console.log('✅ Base64 decode successful,', decoded.length, 'bytes');

				// Test JSON parsing
				const json = JSON.parse(decoded);
				console.log('✅ JSON parse successful');
				console.log('JSON keys:', Object.keys(json));

				// Check required fields
				const hasPrivateKey = !!json.privateKey;
				const hasSubject = !!json.subject;
				const hasPublicKey = !!json.publicKey;

				console.log('Has privateKey:', hasPrivateKey);
				console.log('Has subject:', hasSubject);
				console.log('Has publicKey:', hasPublicKey);

				if (hasPrivateKey) {
					try {
						atob(json.privateKey);
						console.log('✅ privateKey base64 decode successful');
					} catch (e) {
						console.log('❌ privateKey base64 decode failed:', e.message);
					}
				}

				return {
					success: true,
					length: secret.length,
					hasPrivateKey,
					hasSubject,
					hasPublicKey,
					subject: json.subject,
					keys: Object.keys(json),
				};
			} catch (error) {
				console.error('❌ Secret analysis failed:', error);
				return {
					success: false,
					error: error.message,
					length: secret.length,
				};
			}
		}, ATOMIC_SERVER_SECRET);

		console.log('📊 Secret analysis result:', secretAnalysis);
		expect(secretAnalysis.success).toBeTruthy();
		expect(secretAnalysis.hasPrivateKey).toBeTruthy();
		expect(secretAnalysis.hasSubject).toBeTruthy();
	});

	test('should test atomic server connectivity with detailed logging', async ({ page }) => {
		console.log('🔍 Testing atomic server connectivity with detailed logging...');

		// Test atomic server root endpoint
		const rootResponse = await page.request.get(ATOMIC_SERVER_URL, {
			headers: { Accept: 'application/json' },
		});

		console.log('📊 Root response status:', rootResponse.status());
		expect(rootResponse.ok()).toBeTruthy();

		const rootData = await rootResponse.json();
		console.log('📊 Root data keys:', Object.keys(rootData));
		console.log('📊 Root children count:', rootData.children?.length || 0);

		// Test haystack endpoint specifically
		const haystackResponse = await page.request.get(`${ATOMIC_SERVER_URL}haystack`, {
			headers: { Accept: 'application/json' },
		});

		console.log('📊 Haystack response status:', haystackResponse.status());
		if (haystackResponse.ok()) {
			const haystackData = await haystackResponse.json();
			console.log('📊 Haystack data:', haystackData);
		}

		// Test agent endpoint - get the subject from the secret
		if (!ATOMIC_SERVER_SECRET) {
			throw new Error('ATOMIC_SERVER_SECRET environment variable is not set');
		}

		const agentSubject = await page.evaluate((secret) => {
			try {
				const decoded = atob(secret);
				const json = JSON.parse(decoded);
				return json.subject;
			} catch (error) {
				throw new Error(`Failed to extract agent subject: ${error}`);
			}
		}, ATOMIC_SERVER_SECRET);

		const agentResponse = await page.request.get(agentSubject, {
			headers: { Accept: 'application/json' },
		});

		console.log('📊 Agent response status:', agentResponse.status());
		if (agentResponse.ok()) {
			const agentData = await agentResponse.json();
			console.log('📊 Agent data:', agentData);
		}
	});

	test('should test atomic server search functionality', async ({ page }) => {
		console.log('🔍 Testing atomic server search functionality...');

		// Test search endpoint
		const searchResponse = await page.request.get(`${ATOMIC_SERVER_URL}search?q=test`, {
			headers: { Accept: 'application/json' },
		});

		console.log('📊 Search response status:', searchResponse.status());
		if (searchResponse.ok()) {
			const searchData = await searchResponse.json();
			console.log('📊 Search data structure:', Object.keys(searchData));
			console.log('📊 Search results count:', searchData.results?.length || 0);
		}
	});

	test('should test terraphim server atomic integration with detailed error handling', async () => {
		console.log('🔍 Testing Terraphim server atomic integration...');

		try {
			// Start Terraphim server with atomic configuration
			const updateResponse = await fetch('http://localhost:8000/config', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: fs.readFileSync(configPath, 'utf8'),
			});

			console.log('📊 Config update response status:', updateResponse.status());
			if (!updateResponse.ok) {
				const errorText = await updateResponse.text();
				console.log('❌ Config update failed:', errorText);
				throw new Error(`Config update failed: ${updateResponse.status} - ${errorText}`);
			}

			console.log('✅ Successfully updated Terraphim server config');

			// Wait for configuration to be applied
			await new Promise((resolve) => setTimeout(() => resolve(undefined), 3000));

			// Test atomic haystack search through Terraphim
			const searchResponse = await fetch('http://localhost:8000/documents/search', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify({
					search_term: 'test',
					role: 'Atomic Debug Test',
					limit: 10,
				}),
			});

			console.log('📊 Terraphim search response status:', searchResponse.status());
			console.log(
				'📊 Terraphim search response headers:',
				Object.fromEntries(searchResponse.headers.entries())
			);

			if (!searchResponse.ok) {
				const errorText = await searchResponse.text();
				console.log('❌ Terraphim search failed:', errorText);

				// Parse error details
				try {
					const errorJson = JSON.parse(errorText);
					console.log('📊 Error details:', errorJson);

					// Check if it's the Base64 decode error
					if (errorJson.message && errorJson.message.includes('Base64 decode error')) {
						console.log('🔍 This is the Base64 decode error we need to fix!');
						console.log('🔍 Error message:', errorJson.message);
					}
				} catch (e) {
					console.log('📊 Could not parse error as JSON');
				}

				throw new Error(`Terraphim search failed: ${searchResponse.status} - ${errorText}`);
			}

			const searchResults = await searchResponse.json();
			console.log('📊 Terraphim search results:', JSON.stringify(searchResults, null, 2));

			// Verify we got results structure
			expect(searchResults).toBeDefined();

			if (searchResults.results && Array.isArray(searchResults.results)) {
				console.log(
					`✅ Terraphim search returned ${searchResults.results.length} results from atomic server`
				);

				if (searchResults.results.length > 0) {
					const firstResult = searchResults.results[0];
					expect(firstResult).toHaveProperty('content');
					console.log('✅ Search results have proper structure');
				}
			}
		} catch (error) {
			console.error('❌ Atomic integration test error:', error);
			throw error;
		}
	});

	test('should test atomic server secret with different formats', async ({ page }) => {
		console.log('🔍 Testing atomic server secret with different formats...');

		const formatTests = await page.evaluate((secret) => {
			const tests = [];

			// Test 1: Original secret
			try {
				const decoded = atob(secret);
				const json = JSON.parse(decoded);
				tests.push({
					name: 'Original secret',
					success: true,
					length: secret.length,
					keys: Object.keys(json),
				});
			} catch (e) {
				tests.push({
					name: 'Original secret',
					success: false,
					error: e.message,
					length: secret.length,
				});
			}

			// Test 2: Secret with padding
			try {
				let paddedSecret = secret;
				while (paddedSecret.length % 4 !== 0) {
					paddedSecret += '=';
				}
				const decoded = atob(paddedSecret);
				const json = JSON.parse(decoded);
				tests.push({
					name: 'Secret with padding',
					success: true,
					length: paddedSecret.length,
					keys: Object.keys(json),
				});
			} catch (e) {
				tests.push({
					name: 'Secret with padding',
					success: false,
					error: e.message,
				});
			}

			// Test 3: Secret with whitespace removed
			try {
				const cleanSecret = secret.replace(/\s/g, '');
				const decoded = atob(cleanSecret);
				const json = JSON.parse(decoded);
				tests.push({
					name: 'Secret with whitespace removed',
					success: true,
					length: cleanSecret.length,
					keys: Object.keys(json),
				});
			} catch (e) {
				tests.push({
					name: 'Secret with whitespace removed',
					success: false,
					error: e.message,
				});
			}

			return tests;
		}, ATOMIC_SERVER_SECRET);

		console.log('📊 Format test results:', formatTests);

		// All tests should pass since Node.js is more lenient
		formatTests.forEach((test) => {
			expect(test.success).toBeTruthy();
		});
	});
});
