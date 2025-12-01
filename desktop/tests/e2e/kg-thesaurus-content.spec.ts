/**
 * End-to-End tests for Knowledge Graph Thesaurus Content Addition
 *
 * This test suite validates that the KG search modal properly adds the full
 * thesaurus content (JSON) to conversation context, not just statistics.
 */

import { test, expect } from '@playwright/test';

// Test configuration
const TEST_CONFIG = {
	APP_URL: 'http://localhost:5173',
	SEARCH_TIMEOUT: 10000,
	AUTOCOMPLETE_DEBOUNCE: 300,
	TEST_ROLE: 'Engineer',
	DEFAULT_CONVERSATION_TITLE: 'KG Thesaurus Test Conversation',
};

// Mock thesaurus data structure
const MOCK_THESAURUS_CONTENT = {
	async: {
		id: 1,
		normalized_term: 'async_programming',
		definition: 'Asynchronous programming paradigm in Rust using futures and async/await syntax',
		synonyms: ['asynchronous', 'non-blocking', 'concurrent'],
		related_terms: ['futures', 'tokio', 'runtime', 'spawn'],
		usage_examples: ['async fn main() { ... }', 'let result = async_function().await;'],
		url: 'https://docs.rs/tokio/latest/tokio/',
		metadata: {
			category: 'programming',
			language: 'rust',
			complexity: 'intermediate',
		},
	},
	tokio: {
		id: 2,
		normalized_term: 'tokio_runtime',
		definition:
			'Asynchronous runtime for Rust providing I/O, networking, and concurrency primitives',
		synonyms: ['runtime', 'async_runtime', 'tokio_runtime'],
		related_terms: ['async', 'futures', 'spawn', 'runtime'],
		usage_examples: ['#[tokio::main]', 'tokio::spawn(async { ... })'],
		url: 'https://docs.rs/tokio/latest/tokio/runtime/',
		metadata: {
			category: 'runtime',
			language: 'rust',
			complexity: 'intermediate',
		},
	},
	futures: {
		id: 3,
		normalized_term: 'future_trait',
		definition: 'Trait representing an asynchronous computation that may not have completed yet',
		synonyms: ['future', 'promise', 'async_value'],
		related_terms: ['async', 'tokio', 'await', 'poll'],
		usage_examples: ['impl Future for MyStruct { ... }', 'let future = async_function();'],
		url: 'https://docs.rs/futures/latest/futures/',
		metadata: {
			category: 'trait',
			language: 'rust',
			complexity: 'advanced',
		},
	},
};

test.describe('KG Thesaurus Content Addition E2E', () => {
	test.beforeEach(async ({ page }) => {
		console.log('🚀 Setting up KG thesaurus content test environment...');

		// Navigate to the application
		await page.goto(TEST_CONFIG.APP_URL);

		// Wait for the app to load completely
		await page.waitForLoadState('networkidle');

		// Ensure we're in chat mode with a conversation
		await page.waitForSelector('[data-testid="chat-interface"]', {
			timeout: TEST_CONFIG.SEARCH_TIMEOUT,
		});

		console.log('✅ Test environment setup complete');
	});

	test.describe('Thesaurus Content Addition', () => {
		test('should add complete thesaurus JSON content to context', async ({ page }) => {
			console.log('📚 Testing complete thesaurus content addition...');

			// Open KG search modal
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');

			// Click "Add Complete Thesaurus to Context" button
			const addThesaurusButton = page.locator(
				'button:has-text("Add Complete Thesaurus to Context")'
			);
			await expect(addThesaurusButton).toBeVisible();
			await addThesaurusButton.click();

			// Verify modal closes
			await expect(page.locator('[data-testid="kg-search-modal"]')).toBeHidden({
				timeout: 3000,
			});

			// Wait for context to be added
			await page.waitForTimeout(2000);

			// Verify thesaurus content appears in context panel
			const contextPanel = page.locator('[data-testid="context-panel"]');
			await expect(contextPanel).toBeVisible({ timeout: 5000 });

			// Verify the context contains the full thesaurus JSON structure
			// Check for key thesaurus terms
			await expect(contextPanel).toContainText('async', { timeout: 3000 });
			await expect(contextPanel).toContainText('tokio', { timeout: 3000 });
			await expect(contextPanel).toContainText('futures', { timeout: 3000 });

			// Verify thesaurus structure elements are present
			await expect(contextPanel).toContainText('normalized_term', { timeout: 3000 });
			await expect(contextPanel).toContainText('definition', { timeout: 3000 });
			await expect(contextPanel).toContainText('synonyms', { timeout: 3000 });
			await expect(contextPanel).toContainText('related_terms', { timeout: 3000 });
			await expect(contextPanel).toContainText('usage_examples', { timeout: 3000 });
			await expect(contextPanel).toContainText('metadata', { timeout: 3000 });

			// Verify specific content from our mock data
			await expect(contextPanel).toContainText('async_programming', { timeout: 3000 });
			await expect(contextPanel).toContainText('Asynchronous programming paradigm', {
				timeout: 3000,
			});
			await expect(contextPanel).toContainText('tokio_runtime', { timeout: 3000 });
			await expect(contextPanel).toContainText('Asynchronous runtime for Rust', { timeout: 3000 });

			console.log('✅ Complete thesaurus content successfully added to context');
		});

		test('should verify thesaurus content is properly formatted as JSON', async ({ page }) => {
			console.log('🔍 Testing thesaurus JSON formatting...');

			// Add thesaurus content
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');
			await page.click('button:has-text("Add Complete Thesaurus to Context")');
			await expect(page.locator('[data-testid="kg-search-modal"]')).toBeHidden({ timeout: 3000 });

			// Wait for context to load
			await page.waitForTimeout(2000);

			// Get the context content
			const contextPanel = page.locator('[data-testid="context-panel"]');
			const contextText = await contextPanel.textContent();

			// Verify JSON structure is present
			expect(contextText).toContain('{');
			expect(contextText).toContain('}');
			expect(contextText).toContain('"async"');
			expect(contextText).toContain('"tokio"');
			expect(contextText).toContain('"futures"');

			// Verify JSON structure for a specific term
			const asyncSection = contextText?.match(/"async":\s*\{[^}]*\}/);
			expect(asyncSection).toBeTruthy();

			if (asyncSection) {
				const asyncContent = asyncSection[0];
				expect(asyncContent).toContain('"id":');
				expect(asyncContent).toContain('"normalized_term":');
				expect(asyncContent).toContain('"definition":');
				expect(asyncContent).toContain('"synonyms":');
				expect(asyncContent).toContain('"related_terms":');
				expect(asyncContent).toContain('"usage_examples":');
				expect(asyncContent).toContain('"metadata":');
			}

			console.log('✅ Thesaurus content properly formatted as JSON');
		});

		test('should verify thesaurus content includes all term definitions', async ({ page }) => {
			console.log('📖 Testing comprehensive term definitions...');

			// Add thesaurus content
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');
			await page.click('button:has-text("Add Complete Thesaurus to Context")');
			await expect(page.locator('[data-testid="kg-search-modal"]')).toBeHidden({ timeout: 3000 });

			// Wait for context to load
			await page.waitForTimeout(2000);

			const contextPanel = page.locator('[data-testid="context-panel"]');
			const contextText = await contextPanel.textContent();

			// Verify all mock terms are present
			Object.keys(MOCK_THESAURUS_CONTENT).forEach((term) => {
				expect(contextText).toContain(`"${term}"`);
			});

			// Verify definitions are included
			Object.values(MOCK_THESAURUS_CONTENT).forEach((termData) => {
				expect(contextText).toContain(termData.definition);
				expect(contextText).toContain(termData.normalized_term);
			});

			// Verify synonyms are included
			Object.values(MOCK_THESAURUS_CONTENT).forEach((termData) => {
				termData.synonyms.forEach((synonym) => {
					expect(contextText).toContain(synonym);
				});
			});

			// Verify related terms are included
			Object.values(MOCK_THESAURUS_CONTENT).forEach((termData) => {
				termData.related_terms.forEach((relatedTerm) => {
					expect(contextText).toContain(relatedTerm);
				});
			});

			// Verify usage examples are included
			Object.values(MOCK_THESAURUS_CONTENT).forEach((termData) => {
				termData.usage_examples.forEach((example) => {
					expect(contextText).toContain(example);
				});
			});

			console.log('✅ All term definitions and metadata included in thesaurus content');
		});

		test('should verify thesaurus content is searchable and usable by AI', async ({ page }) => {
			console.log('🤖 Testing thesaurus content usability for AI...');

			// Add thesaurus content
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');
			await page.click('button:has-text("Add Complete Thesaurus to Context")');
			await expect(page.locator('[data-testid="kg-search-modal"]')).toBeHidden({ timeout: 3000 });

			// Wait for context to load
			await page.waitForTimeout(2000);

			// Verify context item shows it's a thesaurus/JSON content
			const contextItem = page.locator('[data-testid="context-item-kg-index"]');
			await expect(contextItem).toBeVisible({ timeout: 3000 });

			// Verify the context item indicates it contains thesaurus data
			await expect(contextItem).toContainText('Thesaurus Data', { timeout: 3000 });
			await expect(contextItem).toContainText('JSON', { timeout: 3000 });
			await expect(contextItem).toContainText('comprehensive vocabulary context', {
				timeout: 3000,
			});

			// Verify statistics are shown (but not the only content)
			await expect(contextItem).toContainText(/\d+ terms/, { timeout: 3000 });
			await expect(contextItem).toContainText(/\d+ nodes/, { timeout: 3000 });
			await expect(contextItem).toContainText(/\d+ edges/, { timeout: 3000 });

			// Verify the context item can be expanded to show full content
			const expandButton = contextItem.locator('[data-testid="expand-context-button"]');
			if (await expandButton.isVisible()) {
				await expandButton.click();

				// Verify expanded view shows the full JSON content
				const expandedContent = contextItem.locator('[data-testid="expanded-context-content"]');
				await expect(expandedContent).toBeVisible();

				const expandedText = await expandedContent.textContent();
				expect(expandedText).toContain('{');
				expect(expandedText).toContain('"async"');
				expect(expandedText).toContain('"definition"');
			}

			console.log('✅ Thesaurus content properly formatted for AI consumption');
		});

		test('should handle thesaurus addition errors gracefully', async ({ page }) => {
			console.log('❌ Testing thesaurus addition error handling...');

			// Mock network failure for thesaurus addition
			await page.route('**/conversations/*/context/kg/index', (route) => {
				route.fulfill({
					status: 500,
					contentType: 'application/json',
					body: JSON.stringify({
						status: 'error',
						error: 'Failed to add thesaurus to context',
					}),
				});
			});

			// Open modal and try to add thesaurus
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');
			await page.click('button:has-text("Add Complete Thesaurus to Context")');

			// Verify error message is shown
			const errorMessage = page.locator('[data-testid="kg-search-error"]');
			await expect(errorMessage).toBeVisible({ timeout: 5000 });
			await expect(errorMessage).toContainText(/failed|error/i);

			// Verify modal doesn't close on error
			await expect(page.locator('[data-testid="kg-search-modal"]')).toBeVisible();

			console.log('✅ Thesaurus addition errors handled gracefully');
		});

		test('should verify thesaurus content is not just statistics', async ({ page }) => {
			console.log('📊 Testing that thesaurus content goes beyond statistics...');

			// Add thesaurus content
			await page.click('[data-testid="kg-search-button"]');
			await page.waitForSelector('[data-testid="kg-search-modal"]');
			await page.click('button:has-text("Add Complete Thesaurus to Context")');
			await expect(page.locator('[data-testid="kg-search-modal"]')).toBeHidden({ timeout: 3000 });

			// Wait for context to load
			await page.waitForTimeout(2000);

			const contextPanel = page.locator('[data-testid="context-panel"]');
			const contextText = await contextPanel.textContent();

			// Verify content includes more than just statistics
			expect(contextText).toContain('definition');
			expect(contextText).toContain('synonyms');
			expect(contextText).toContain('related_terms');
			expect(contextText).toContain('usage_examples');
			expect(contextText).toContain('metadata');

			// Verify specific definitions are present (not just counts)
			expect(contextText).toContain('Asynchronous programming paradigm');
			expect(contextText).toContain('Asynchronous runtime for Rust');
			expect(contextText).toContain('Trait representing an asynchronous computation');

			// Verify synonyms are present
			expect(contextText).toContain('asynchronous');
			expect(contextText).toContain('non-blocking');
			expect(contextText).toContain('concurrent');

			// Verify usage examples are present
			expect(contextText).toContain('async fn main()');
			expect(contextText).toContain('#[tokio::main]');
			expect(contextText).toContain('impl Future for MyStruct');

			// Verify metadata is present
			expect(contextText).toContain('programming');
			expect(contextText).toContain('runtime');
			expect(contextText).toContain('trait');

			console.log('✅ Thesaurus content includes comprehensive data beyond statistics');
		});
	});

	test.afterEach(async ({ page }) => {
		console.log('🧹 Cleaning up thesaurus test environment...');

		// Close any open modals
		const modal = page.locator('[data-testid="kg-search-modal"]');
		if (await modal.isVisible()) {
			await page.keyboard.press('Escape');
		}

		// Clear any added context items
		const clearContextButton = page.locator('[data-testid="clear-context-button"]');
		if (await clearContextButton.isVisible()) {
			await clearContextButton.click();
		}

		console.log('✅ Thesaurus test cleanup complete');
	});
});

// Helper functions for thesaurus testing
async function addThesaurusToContext(page: any) {
	await page.click('[data-testid="kg-search-button"]');
	await page.waitForSelector('[data-testid="kg-search-modal"]');
	await page.click('button:has-text("Add Complete Thesaurus to Context")');
	await expect(page.locator('[data-testid="kg-search-modal"]')).toBeHidden({ timeout: 3000 });
	await page.waitForTimeout(2000); // Wait for context to load
}

async function verifyThesaurusContent(page: any, expectedTerms: string[]) {
	const contextPanel = page.locator('[data-testid="context-panel"]');
	const contextText = await contextPanel.textContent();

	expectedTerms.forEach((term) => {
		expect(contextText).toContain(term);
	});
}
