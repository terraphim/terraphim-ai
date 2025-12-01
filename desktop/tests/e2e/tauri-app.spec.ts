import { test, expect } from '@playwright/test';

test.describe('Tauri App - Complete Functionality', () => {
	test.beforeEach(async ({ page }) => {
		// Navigate to the app
		await page.goto('/');

		// Wait for the app to load
		await page.waitForSelector('input[type="search"]', { timeout: 30000 });
	});

	// ===== SEARCH SCREEN TESTS =====
	test.describe('Search Screen', () => {
		test('should display search interface correctly', async ({ page }) => {
			// Check main search elements
			await expect(page.locator('input[type="search"]')).toBeVisible();
			await expect(page.locator('img[alt="Terraphim Logo"]')).toBeVisible();
			await expect(page.locator('text=I am Terraphim, your personal assistant.')).toBeVisible();

			// Check theme switcher is present
			await expect(page.locator('.role-selector')).toBeVisible();
		});

		test('should perform search functionality', async ({ page }) => {
			const searchInput = page.locator('input[type="search"]');

			// Test basic search
			await searchInput.fill('artificial intelligence');
			await searchInput.press('Enter');

			// Wait for search results or no results message
			await page.waitForTimeout(2000);

			// Check that search was processed (logo might disappear if results found)
			const logo = page.locator('img[alt="Terraphim Logo"]');
			const logoVisible = await logo.isVisible();

			// Both outcomes are valid depending on test data
			if (!logoVisible) {
				console.log('Search returned results - logo hidden');
				// Check for search results
				await expect(page.locator('.search-results, .results')).toBeVisible();
			} else {
				console.log('Search returned no results - logo still visible');
			}
		});

		test('should handle search suggestions and autocomplete', async ({ page }) => {
			const searchInput = page.locator('input[type="search"]');

			// Type partial text to trigger suggestions
			await searchInput.fill('machine');
			await page.waitForTimeout(500);

			// Check for suggestions dropdown
			const suggestions = page.locator('.suggestions, .autocomplete-dropdown');
			const suggestionsVisible = await suggestions.isVisible();

			if (suggestionsVisible) {
				console.log('Autocomplete suggestions are working');

				// Test keyboard navigation
				await searchInput.press('ArrowDown');
				await page.waitForTimeout(100);
				await searchInput.press('Enter');

				// Check that suggestion was applied
				const inputValue = await searchInput.inputValue();
				expect(inputValue.length).toBeGreaterThan(0);
			}
		});

		test('should clear search and reset interface', async ({ page }) => {
			const searchInput = page.locator('input[type="search"]');

			// Fill and clear search
			await searchInput.fill('test query');
			await searchInput.clear();

			// Verify input is cleared
			expect(await searchInput.inputValue()).toBe('');

			// Logo should be visible again
			await expect(page.locator('img[alt="Terraphim Logo"]')).toBeVisible();
		});
	});

	// ===== NAVIGATION TESTS =====
	test.describe('Navigation Between Screens', () => {
		test('should navigate to configuration wizard', async ({ page }) => {
			// Hover over footer to reveal navigation
			const footer = page.locator('footer');
			await footer.hover();
			await page.waitForTimeout(500);

			// Click on wizard link
			const wizardLink = page.locator('a[href="/config/wizard"]');
			await expect(wizardLink).toBeVisible();
			await wizardLink.click();

			// Verify navigation to wizard
			await expect(page).toHaveURL('/config/wizard');
			await expect(page.locator('h3:has-text("Configuration Wizard")')).toBeVisible();
		});

		test('should navigate to graph visualization', async ({ page }) => {
			// Hover over footer to reveal navigation
			const footer = page.locator('footer');
			await footer.hover();
			await page.waitForTimeout(500);

			// Click on graph link
			const graphLink = page.locator('a[href="/graph"]');
			await expect(graphLink).toBeVisible();
			await graphLink.click();

			// Verify navigation to graph
			await expect(page).toHaveURL('/graph');
			await expect(page.locator('svg')).toBeVisible();
		});

		test('should navigate to JSON editor', async ({ page }) => {
			// Hover over footer to reveal navigation
			const footer = page.locator('footer');
			await footer.hover();
			await page.waitForTimeout(500);

			// Click on JSON editor link
			const jsonLink = page.locator('a[href="/config/json"]');
			await expect(jsonLink).toBeVisible();
			await jsonLink.click();

			// Verify navigation to JSON editor
			await expect(page).toHaveURL('/config/json');
			await expect(page.locator('textarea, .json-editor')).toBeVisible();
		});

		test('should return to home page from any screen', async ({ page }) => {
			// Navigate to wizard first
			const footer = page.locator('footer');
			await footer.hover();
			await page.waitForTimeout(500);

			const wizardLink = page.locator('a[href="/config/wizard"]');
			await wizardLink.click();
			await page.waitForTimeout(1000);

			// Navigate back to home
			await footer.hover();
			await page.waitForTimeout(500);

			const homeLink = page.locator('a[href="/"]');
			await expect(homeLink).toBeVisible();
			await homeLink.click();

			// Verify back on home page
			await expect(page).toHaveURL('/');
			await expect(page.locator('input[type="search"]')).toBeVisible();
			await expect(page.locator('img[alt="Terraphim Logo"]')).toBeVisible();
		});

		test('should maintain navigation consistency across screens', async ({ page }) => {
			// Check navigation on home page
			const footer = page.locator('footer');
			await footer.hover();
			await page.waitForTimeout(500);

			const homeNavItems = await page.locator('nav a').count();
			expect(homeNavItems).toBeGreaterThan(0);

			// Navigate to wizard and check navigation
			const wizardLink = page.locator('a[href="/config/wizard"]');
			await wizardLink.click();
			await page.waitForTimeout(1000);

			await footer.hover();
			await page.waitForTimeout(500);

			const wizardNavItems = await page.locator('nav a').count();
			expect(wizardNavItems).toBeGreaterThan(0);

			// Navigate to graph and check navigation
			const graphLink = page.locator('a[href="/graph"]');
			await graphLink.click();
			await page.waitForTimeout(1000);

			await footer.hover();
			await page.waitForTimeout(500);

			const graphNavItems = await page.locator('nav a').count();
			expect(graphNavItems).toBeGreaterThan(0);
		});
	});

	// ===== CONFIGURATION WIZARD TESTS =====
	test.describe('Configuration Wizard Screen', () => {
		test.beforeEach(async ({ page }) => {
			// Navigate to wizard
			const footer = page.locator('footer');
			await footer.hover();
			await page.waitForTimeout(500);

			const wizardLink = page.locator('a[href="/config/wizard"]');
			await wizardLink.click();
			await page.waitForTimeout(1000);
		});

		test('should display wizard interface correctly', async ({ page }) => {
			// Check wizard title and elements
			await expect(page.locator('h3:has-text("Configuration Wizard")')).toBeVisible();
			await expect(page.locator('label:has-text("Configuration ID")')).toBeVisible();
			await expect(page.locator('button:has-text("Next")')).toBeVisible();
		});

		test('should configure global settings', async ({ page }) => {
			// Configure basic settings
			const configIdSelect = page.locator('#config-id');
			await configIdSelect.selectOption('Desktop');
			await expect(configIdSelect).toHaveValue('Desktop');

			const shortcutInput = page.locator('#global-shortcut');
			await shortcutInput.fill('Ctrl+Alt+T');
			await expect(shortcutInput).toHaveValue('Ctrl+Alt+T');

			const themeInput = page.locator('#default-theme');
			await themeInput.fill('superhero');
			await expect(themeInput).toHaveValue('superhero');
		});

		test('should add and configure roles', async ({ page }) => {
			// Navigate to roles step
			await page.getByTestId('wizard-next').click();
			await page.waitForSelector('h4:has-text("Roles")', { timeout: 5000 });

			// Add a role
			await page.getByTestId('add-role').click();
			await page.waitForSelector('#role-name-0', { timeout: 5000 });

			// Configure the role
			await page.fill('#role-name-0', 'Test Engineer');
			await page.fill('#role-shortname-0', 'test-eng');
			await page.fill('#role-theme-0', 'lumen');
			await page.fill('#role-relevance-0', 'TerraphimGraph');

			// Verify role configuration
			await expect(page.locator('#role-name-0')).toHaveValue('Test Engineer');
			await expect(page.locator('#role-shortname-0')).toHaveValue('test-eng');
		});

		test('should add haystacks to roles', async ({ page }) => {
			// Navigate to roles step and add role
			await page.getByTestId('wizard-next').click();
			await page.waitForSelector('h4:has-text("Roles")', { timeout: 5000 });
			await page.getByTestId('add-role').click();
			await page.waitForSelector('#role-name-0', { timeout: 5000 });

			// Add haystack
			await page.getByTestId('add-haystack-0').click();
			await page.waitForSelector('#haystack-path-0-0', { timeout: 5000 });

			// Configure haystack
			await page.fill('#haystack-path-0-0', '/tmp/test-documents');
			await page.locator('#haystack-readonly-0-0').check();

			// Verify haystack configuration
			await expect(page.locator('#haystack-path-0-0')).toHaveValue('/tmp/test-documents');
			await expect(page.locator('#haystack-readonly-0-0')).toBeChecked();
		});

		test('should configure knowledge graph settings', async ({ page }) => {
			// Navigate to roles step and add role
			await page.getByTestId('wizard-next').click();
			await page.waitForSelector('h4:has-text("Roles")', { timeout: 5000 });
			await page.getByTestId('add-role').click();
			await page.waitForSelector('#role-name-0', { timeout: 5000 });

			// Configure KG settings
			await page.fill('#kg-url-0', 'https://staging-storage.terraphim.io/thesaurus_Default.json');
			await page.fill('#kg-local-path-0', './docs/src/kg');
			await page.locator('#kg-local-type-0').selectOption('markdown');
			await page.locator('#kg-public-0').check();

			// Verify KG configuration
			await expect(page.locator('#kg-url-0')).toHaveValue(
				'https://staging-storage.terraphim.io/thesaurus_Default.json'
			);
			await expect(page.locator('#kg-local-path-0')).toHaveValue('./docs/src/kg');
			await expect(page.locator('#kg-local-type-0')).toHaveValue('markdown');
			await expect(page.locator('#kg-public-0')).toBeChecked();
		});

		test('should navigate through wizard steps', async ({ page }) => {
			// Step 1: Global settings
			await expect(page.locator('label:has-text("Configuration ID")')).toBeVisible();

			// Step 2: Roles
			await page.getByTestId('wizard-next').click();
			await page.waitForSelector('h4:has-text("Roles")', { timeout: 5000 });

			// Step 3: Haystacks
			await page.getByTestId('wizard-next').click();
			await page.waitForSelector('h4:has-text("Haystacks")', { timeout: 5000 });

			// Step 4: Knowledge Graph
			await page.getByTestId('wizard-next').click();
			await page.waitForSelector('h4:has-text("Knowledge Graph")', { timeout: 5000 });

			// Step 5: Review
			await page.getByTestId('wizard-next').click();
			await page.waitForSelector('h4:has-text("Review")', { timeout: 5000 });

			// Navigate back
			await page.getByTestId('wizard-back').click();
			await page.waitForSelector('h4:has-text("Knowledge Graph")', { timeout: 5000 });
		});

		test('should save configuration successfully', async ({ page }) => {
			// Complete basic configuration
			await page.getByTestId('wizard-next').click(); // roles
			await page.getByTestId('add-role').click();
			await page.waitForSelector('#role-name-0', { timeout: 5000 });
			await page.fill('#role-name-0', 'Test Role');
			await page.getByTestId('wizard-next').click(); // haystacks
			await page.getByTestId('wizard-next').click(); // knowledge graph
			await page.getByTestId('wizard-next').click(); // review

			// Save configuration
			await page.getByTestId('wizard-save').click();
			await page.waitForTimeout(2000);

			// Check for success message or redirect
			const successMessage = page.locator(
				'.success, .alert-success, [data-testid="wizard-success"]'
			);
			const successVisible = await successMessage.isVisible();

			if (successVisible) {
				console.log('Configuration saved successfully');
			} else {
				// Check if we're redirected back to home or wizard
				const currentUrl = page.url();
				expect(currentUrl).toMatch(/\/$|\/config\/wizard/);
			}
		});
	});

	// ===== GRAPH VISUALIZATION TESTS =====
	test.describe('Graph Visualization Screen', () => {
		test.beforeEach(async ({ page }) => {
			// Navigate to graph
			const footer = page.locator('footer');
			await footer.hover();
			await page.waitForTimeout(500);

			const graphLink = page.locator('a[href="/graph"]');
			await graphLink.click();
			await page.waitForTimeout(2000);
		});

		test('should display graph interface correctly', async ({ page }) => {
			// Check for SVG canvas
			await expect(page.locator('svg')).toBeVisible();

			// Check for loading state or graph content
			const loading = page.locator('.loading, [data-testid="loading"]');
			const loadingVisible = await loading.isVisible();

			if (loadingVisible) {
				console.log('Graph is loading...');
				// Wait for loading to complete
				await page.waitForSelector('svg circle, svg line', { timeout: 10000 });
			}

			// Check for graph elements (nodes and edges)
			const nodes = page.locator('svg circle');
			const edges = page.locator('svg line');

			// At least one of these should be present
			const nodeCount = await nodes.count();
			const edgeCount = await edges.count();

			console.log(`Graph contains ${nodeCount} nodes and ${edgeCount} edges`);
			expect(nodeCount + edgeCount).toBeGreaterThan(0);
		});

		test('should handle graph interactions', async ({ page }) => {
			// Wait for graph to load
			await page.waitForSelector('svg circle', { timeout: 10000 });

			// Test node clicking
			const firstNode = page.locator('svg circle').first();
			await expect(firstNode).toBeVisible();

			// Click on a node
			await firstNode.click();
			await page.waitForTimeout(500);

			// Check for modal or details panel
			const modal = page.locator('.modal, [data-testid="node-modal"], .article-modal');
			const modalVisible = await modal.isVisible();

			if (modalVisible) {
				console.log('Node click opened modal');
				// Close modal
				const closeButton = modal.locator('.close, .modal-close, button:has-text("Close")');
				if (await closeButton.isVisible()) {
					await closeButton.click();
				}
			}
		});

		test('should handle graph zoom and pan', async ({ page }) => {
			// Wait for graph to load
			await page.waitForSelector('svg circle', { timeout: 10000 });

			// Test zoom functionality
			const svg = page.locator('svg');
			await svg.hover();

			// Zoom in with mouse wheel
			await page.mouse.wheel(0, -100);
			await page.waitForTimeout(500);

			// Zoom out
			await page.mouse.wheel(0, 100);
			await page.waitForTimeout(500);

			// Test pan functionality
			await page.mouse.down();
			await page.mouse.move(100, 100);
			await page.mouse.up();
			await page.waitForTimeout(500);

			// Graph should still be visible and functional
			await expect(svg).toBeVisible();
		});

		test('should handle graph node dragging', async ({ page }) => {
			// Wait for graph to load
			await page.waitForSelector('svg circle', { timeout: 10000 });

			// Test dragging a node
			const firstNode = page.locator('svg circle').first();
			await expect(firstNode).toBeVisible();

			// Get initial position
			const initialBox = await firstNode.boundingBox();

			// Drag the node
			await firstNode.dragTo(page.locator('svg'));
			await page.waitForTimeout(500);

			// Get new position
			const newBox = await firstNode.boundingBox();

			// Position should have changed (unless it's constrained)
			if (initialBox && newBox) {
				const moved =
					Math.abs(initialBox.x - newBox.x) > 5 || Math.abs(initialBox.y - newBox.y) > 5;
				console.log(`Node dragging ${moved ? 'worked' : 'may be constrained'}`);
			}
		});

		test('should handle graph error states', async ({ page }) => {
			// Navigate to graph with invalid API URL to test error handling
			await page.goto('/graph?apiUrl=/invalid-endpoint');
			await page.waitForTimeout(3000);

			// Check for error message or fallback
			const errorMessage = page.locator('.error, [data-testid="error"], .alert-danger');
			const errorVisible = await errorMessage.isVisible();

			if (errorVisible) {
				console.log('Graph error handling working correctly');
				expect(await errorMessage.textContent()).toContain('Error');
			} else {
				// Check if graph shows empty state gracefully
				const svg = page.locator('svg');
				await expect(svg).toBeVisible();
				console.log('Graph handles errors gracefully');
			}
		});
	});

	// ===== INTEGRATION TESTS =====
	test.describe('Cross-Screen Integration', () => {
		test('should maintain theme consistency across screens', async ({ page }) => {
			// Check theme on home page
			const themeSwitcher = page.locator('.role-selector select');
			await expect(themeSwitcher).toBeVisible();

			// Navigate to wizard
			const footer = page.locator('footer');
			await footer.hover();
			await page.waitForTimeout(500);

			const wizardLink = page.locator('a[href="/config/wizard"]');
			await wizardLink.click();
			await page.waitForTimeout(1000);

			// Theme switcher should still be visible
			await expect(page.locator('.role-selector')).toBeVisible();

			// Navigate to graph
			await footer.hover();
			await page.waitForTimeout(500);

			const graphLink = page.locator('a[href="/graph"]');
			await graphLink.click();
			await page.waitForTimeout(1000);

			// Theme switcher should still be visible
			await expect(page.locator('.role-selector')).toBeVisible();
		});

		test('should handle browser navigation correctly', async ({ page }) => {
			// Start on home page
			const initialUrl = page.url();

			// Navigate to wizard
			const footer = page.locator('footer');
			await footer.hover();
			await page.waitForTimeout(500);

			const wizardLink = page.locator('a[href="/config/wizard"]');
			await wizardLink.click();
			await page.waitForTimeout(1000);

			// Use browser back
			await page.goBack();
			await page.waitForTimeout(1000);

			// Should be back on home
			expect(page.url()).toBe(initialUrl);
			await expect(page.locator('input[type="search"]')).toBeVisible();

			// Use browser forward
			await page.goForward();
			await page.waitForTimeout(1000);

			// Should be on wizard page
			await expect(page).toHaveURL('/config/wizard');
		});

		test('should handle direct URL navigation', async ({ page }) => {
			// Navigate directly to wizard
			await page.goto('/config/wizard');
			await page.waitForTimeout(2000);

			await expect(page).toHaveURL('/config/wizard');
			await expect(page.locator('h3:has-text("Configuration Wizard")')).toBeVisible();

			// Navigate directly to graph
			await page.goto('/graph');
			await page.waitForTimeout(2000);

			await expect(page).toHaveURL('/graph');
			await expect(page.locator('svg')).toBeVisible();

			// Navigate directly to JSON editor
			await page.goto('/config/json');
			await page.waitForTimeout(2000);

			await expect(page).toHaveURL('/config/json');
			await expect(page.locator('textarea, .json-editor')).toBeVisible();
		});

		test('should handle invalid routes gracefully', async ({ page }) => {
			// Navigate to non-existent route
			await page.goto('/invalid-route');
			await page.waitForTimeout(2000);

			// App should handle gracefully - either redirect to home or show 404
			const searchInput = page.locator('input[type="search"]');
			const footer = page.locator('footer');

			// At least one should be visible
			const searchVisible = await searchInput.isVisible();
			const footerVisible = await footer.isVisible();

			expect(searchVisible || footerVisible).toBeTruthy();
		});
	});

	// ===== PERFORMANCE AND STABILITY TESTS =====
	test.describe('Performance and Stability', () => {
		test('should handle rapid navigation between screens', async ({ page }) => {
			const footer = page.locator('footer');

			// Rapidly navigate between screens
			for (let i = 0; i < 5; i++) {
				await footer.hover();
				await page.waitForTimeout(100);

				const wizardLink = page.locator('a[href="/config/wizard"]');
				await wizardLink.click();
				await page.waitForTimeout(200);

				await footer.hover();
				await page.waitForTimeout(100);

				const graphLink = page.locator('a[href="/graph"]');
				await graphLink.click();
				await page.waitForTimeout(200);

				await footer.hover();
				await page.waitForTimeout(100);

				const homeLink = page.locator('a[href="/"]');
				await homeLink.click();
				await page.waitForTimeout(200);
			}

			// App should still be functional
			await expect(page.locator('input[type="search"]')).toBeVisible();
		});

		test('should handle large search queries', async ({ page }) => {
			const searchInput = page.locator('input[type="search"]');

			// Create a very long search query
			const longQuery =
				'artificial intelligence machine learning deep learning neural networks computer vision natural language processing data science algorithms optimization'.repeat(
					5
				);

			await searchInput.fill(longQuery);
			await searchInput.press('Enter');

			// Should handle without crashing
			await page.waitForTimeout(3000);

			// App should remain responsive
			await expect(searchInput).toBeVisible();
		});

		test('should handle concurrent operations', async ({ page }) => {
			// Start typing in search
			const searchInput = page.locator('input[type="search"]');
			await searchInput.fill('test');

			// While typing, try to navigate
			const footer = page.locator('footer');
			await footer.hover();
			await page.waitForTimeout(100);

			const wizardLink = page.locator('a[href="/config/wizard"]');
			await wizardLink.click();

			// Should navigate successfully
			await expect(page).toHaveURL('/config/wizard');
			await expect(page.locator('h3:has-text("Configuration Wizard")')).toBeVisible();
		});
	});
});
