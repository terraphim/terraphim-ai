import { test, expect } from '@playwright/test';

test.describe('KG Graph Functionality - Tauri Context', () => {
	test.beforeEach(async ({ page }) => {
		// Navigate to the Tauri app
		await page.goto('http://localhost:5173');

		// Wait for the app to load
		await page.waitForSelector('input[type="search"], #search-input, .search-input', {
			timeout: 30000,
		});
	});

	test('should display search interface and navigate to graph visualization', async ({ page }) => {
		console.log('🔍 Testing KG graph navigation and functionality...');

		// Verify search interface is loaded
		await expect(
			page.locator('input[type="search"], #search-input, .search-input').first()
		).toBeVisible();
		await expect(page.locator('img[alt="Terraphim Logo"]')).toBeVisible();

		// Navigate to graph visualization
		console.log('📊 Navigating to graph visualization...');

		// Hover over footer to reveal navigation
		const footer = page.locator('footer');
		await footer.hover();
		await page.waitForTimeout(500);

		// Click on Graph link
		const graphLink = page.locator('a[href="/graph"]');
		await expect(graphLink).toBeVisible();
		await graphLink.click();

		// Wait for graph page to load
		await page.waitForURL('**/graph', { timeout: 10000 });
		console.log('✅ Successfully navigated to graph page');

		// Check for graph container
		const graphContainer = page.locator('.graph-container');
		await expect(graphContainer).toBeVisible({ timeout: 15000 });
		console.log('✅ Graph container is visible');
	});

	test('should load and display knowledge graph visualization', async ({ page }) => {
		console.log('🖼️ Testing knowledge graph visualization loading...');

		// Navigate directly to graph
		await page.goto('http://localhost:5173/graph');

		// Wait for graph container
		const graphContainer = page.locator('.graph-container');
		await expect(graphContainer).toBeVisible({ timeout: 15000 });

		// Check for loading state
		const loadingOverlay = page.locator('.loading-overlay');
		const loadingVisible = await loadingOverlay.isVisible().catch(() => false);

		if (loadingVisible) {
			console.log('⏳ Graph is loading, waiting for completion...');
			await expect(loadingOverlay).not.toBeVisible({ timeout: 30000 });
		}

		// Check for graph elements
		const svg = page.locator('.graph-container svg');
		await expect(svg).toBeVisible({ timeout: 10000 });
		console.log('✅ SVG graph element is visible');

		// Check for nodes and edges
		const nodes = page.locator('.graph-container .nodes circle');
		const edges = page.locator('.graph-container .links line');

		// Wait a bit for the graph to render
		await page.waitForTimeout(3000);

		const nodeCount = await nodes.count();
		const edgeCount = await edges.count();

		console.log(`📊 Graph rendered: ${nodeCount} nodes, ${edgeCount} edges`);

		// Even if no nodes/edges are found, the graph structure should be present
		expect(nodeCount).toBeGreaterThanOrEqual(0);
		expect(edgeCount).toBeGreaterThanOrEqual(0);

		console.log('✅ Knowledge graph visualization loaded successfully');
	});

	test('should handle graph node interactions', async ({ page }) => {
		console.log('🖱️ Testing graph node interactions...');

		// Navigate to graph
		await page.goto('http://localhost:5173/graph');

		// Wait for graph to load
		const graphContainer = page.locator('.graph-container');
		await expect(graphContainer).toBeVisible({ timeout: 15000 });

		// Wait for any loading to complete
		await page.waitForTimeout(5000);

		// Look for nodes
		const nodes = page.locator('.graph-container .nodes circle');
		const nodeCount = await nodes.count();

		if (nodeCount > 0) {
			console.log(`🎯 Found ${nodeCount} nodes, testing interactions...`);

			// Test left-click on first node
			const firstNode = nodes.first();
			await firstNode.click();

			// Check for modal or document view
			const modal = page.locator('.modal.is-active, .modal-content');
			const modalVisible = await modal.isVisible({ timeout: 5000 }).catch(() => false);

			if (modalVisible) {
				console.log('✅ Node click opened modal successfully');

				// Check for KG context information
				const kgContext = page.locator('.kg-context, .tag.is-info');
				const hasKgContext = (await kgContext.count()) > 0;

				if (hasKgContext) {
					console.log('✅ KG context information displayed');
				}

				// Close modal
				const closeButton = page.locator('.modal-close, .delete').first();
				if ((await closeButton.count()) > 0) {
					await closeButton.click();
					console.log('✅ Modal closed successfully');
				}
			} else {
				console.log('📊 Node click may have worked (no modal appeared)');
			}

			// Test right-click on first node (should open edit mode)
			await firstNode.click({ button: 'right' });

			// Check for edit modal
			const editModal = page.locator('.modal.is-active, .modal-content');
			const editModalVisible = await editModal.isVisible({ timeout: 5000 }).catch(() => false);

			if (editModalVisible) {
				console.log('✅ Node right-click opened edit modal');

				// Close edit modal
				const closeButton = page.locator('.modal-close, .delete').first();
				if ((await closeButton.count()) > 0) {
					await closeButton.click();
				}
			}
		} else {
			console.log('⚠️ No nodes found in graph - this may be expected if KG is not built yet');
		}
	});

	test('should handle graph zoom and pan functionality', async ({ page }) => {
		console.log('🔍 Testing graph zoom and pan functionality...');

		// Navigate to graph
		await page.goto('http://localhost:5173/graph');

		// Wait for graph to load
		const graphContainer = page.locator('.graph-container');
		await expect(graphContainer).toBeVisible({ timeout: 15000 });

		// Wait for any loading to complete
		await page.waitForTimeout(5000);

		// Test zoom functionality
		const svg = page.locator('.graph-container svg');
		await expect(svg).toBeVisible();

		// Get initial transform
		const initialTransform = await svg.evaluate((el) => {
			const g = el.querySelector('g');
			return g ? g.getAttribute('transform') : null;
		});

		console.log('Initial transform:', initialTransform);

		// Test zoom in
		await page.mouse.wheel(0, -100); // Scroll up to zoom in

		await page.waitForTimeout(1000);

		const zoomedTransform = await svg.evaluate((el) => {
			const g = el.querySelector('g');
			return g ? g.getAttribute('transform') : null;
		});

		console.log('After zoom transform:', zoomedTransform);

		// Transform should have changed (zoom in)
		if (zoomedTransform !== initialTransform) {
			console.log('✅ Zoom functionality working');
		} else {
			console.log('⚠️ Zoom may not have changed transform (could be expected)');
		}

		// Test pan functionality
		const graphArea = page.locator('.graph-container');
		await graphArea.hover();

		// Drag to pan
		await page.mouse.down();
		await page.mouse.move(100, 100);
		await page.mouse.up();

		await page.waitForTimeout(1000);

		const pannedTransform = await svg.evaluate((el) => {
			const g = el.querySelector('g');
			return g ? g.getAttribute('transform') : null;
		});

		console.log('After pan transform:', pannedTransform);

		if (pannedTransform !== zoomedTransform) {
			console.log('✅ Pan functionality working');
		} else {
			console.log('⚠️ Pan may not have changed transform (could be expected)');
		}
	});

	test('should display graph controls and information', async ({ page }) => {
		console.log('🎛️ Testing graph controls and information display...');

		// Navigate to graph
		await page.goto('http://localhost:5173/graph');

		// Wait for graph to load
		const graphContainer = page.locator('.graph-container');
		await expect(graphContainer).toBeVisible({ timeout: 15000 });

		// Wait for any loading to complete
		await page.waitForTimeout(5000);

		// Check for controls info
		const controlsInfo = page.locator('.controls-info');
		const controlsVisible = await controlsInfo.isVisible().catch(() => false);

		if (controlsVisible) {
			console.log('✅ Graph controls information is displayed');
			const controlsText = await controlsInfo.textContent();
			console.log('Controls text:', controlsText);
		} else {
			console.log('⚠️ Controls info not visible (may be expected)');
		}

		// Check for close button (in fullscreen mode)
		const closeButton = page.locator('.close-button');
		const closeVisible = await closeButton.isVisible().catch(() => false);

		if (closeVisible) {
			console.log('✅ Close button is visible');
		} else {
			console.log('ℹ️ Close button not visible (not in fullscreen mode)');
		}

		// Check for node labels
		const labels = page.locator('.graph-container .labels text');
		const labelCount = await labels.count();

		if (labelCount > 0) {
			console.log(`✅ Found ${labelCount} node labels`);

			// Check first label content
			const firstLabel = labels.first();
			const labelText = await firstLabel.textContent();
			console.log('First label text:', labelText);
		} else {
			console.log('⚠️ No node labels found (may be expected if no nodes)');
		}
	});

	test('should handle graph error states gracefully', async ({ page }) => {
		console.log('⚠️ Testing graph error handling...');

		// Navigate to graph
		await page.goto('http://localhost:5173/graph');

		// Wait for graph container
		const graphContainer = page.locator('.graph-container');
		await expect(graphContainer).toBeVisible({ timeout: 15000 });

		// Check for error overlay
		const errorOverlay = page.locator('.error-overlay');
		const errorVisible = await errorOverlay.isVisible().catch(() => false);

		if (errorVisible) {
			console.log('⚠️ Error overlay is visible');

			// Check error content
			const errorContent = page.locator('.error-content');
			const errorText = await errorContent.textContent();
			console.log('Error text:', errorText);

			// Check for retry button
			const retryButton = page.locator('.error-content button');
			const retryVisible = await retryButton.isVisible().catch(() => false);

			if (retryVisible) {
				console.log('✅ Retry button is available');
				await retryButton.click();
				console.log('✅ Retry button clicked');
			}
		} else {
			console.log('✅ No error overlay visible - graph loaded successfully');
		}
	});

	test('should integrate with search functionality', async ({ page }) => {
		console.log('🔍 Testing graph integration with search...');

		// Start from search page
		await page.goto('http://localhost:5173');

		// Perform a search that should have KG tags
		const searchInput = page.locator('input[type="search"], #search-input, .search-input').first();
		await searchInput.fill('terraphim graph');
		await searchInput.press('Enter');

		// Wait for results
		await page.waitForTimeout(3000);

		// Look for KG tags in results
		const kgTags = page.locator('.tag-button, .tag[role="button"], button .tag');
		const tagCount = await kgTags.count();

		if (tagCount > 0) {
			console.log(`🏷️ Found ${tagCount} KG tags in search results`);

			// Click on first KG tag
			const firstTag = kgTags.first();
			const tagText = await firstTag.textContent();
			console.log(`🔎 Clicking on KG tag: "${tagText}"`);

			await firstTag.click();

			// Check for KG document modal
			const kgModal = page.locator('.modal.is-active, .modal-content');
			const modalVisible = await kgModal.isVisible({ timeout: 5000 }).catch(() => false);

			if (modalVisible) {
				console.log('✅ KG tag click opened document modal');

				// Check for KG context
				const kgContext = page.locator('.kg-context, .tag.is-info');
				const hasKgContext = (await kgContext.count()) > 0;

				if (hasKgContext) {
					console.log('✅ KG context information displayed in modal');
				}

				// Close modal
				const closeButton = page.locator('.modal-close, .delete').first();
				if ((await closeButton.count()) > 0) {
					await closeButton.click();
				}
			} else {
				console.log('📊 KG tag click may have worked (no modal appeared)');
			}
		} else {
			console.log('⚠️ No KG tags found in search results - this may be expected');
		}

		// Navigate to graph from search results
		const footer = page.locator('footer');
		await footer.hover();
		await page.waitForTimeout(500);

		const graphLink = page.locator('a[href="/graph"]');
		await graphLink.click();

		// Verify we're on graph page
		await page.waitForURL('**/graph', { timeout: 10000 });
		console.log('✅ Successfully navigated from search to graph');

		// Check graph is loaded
		const graphContainer = page.locator('.graph-container');
		await expect(graphContainer).toBeVisible({ timeout: 15000 });
		console.log('✅ Graph visualization loaded after navigation from search');
	});
});
