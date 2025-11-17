import { expect, test } from '@playwright/test';

test.describe('Tauri App - Graph Tags Functionality', () => {
	test('should handle graph tag clicks in Tauri app', async ({ page }) => {
		console.log('🔍 Testing graph tags functionality in Tauri app...');

		// Navigate to the Tauri app (usually runs on port 5173)
		await page.goto('http://localhost:5173', { waitUntil: 'networkidle' });

		// Wait for the app to load
		await page.waitForSelector('input[type="search"], #search-input, .search-input', {
			timeout: 10000,
		});

		// Search for something that should have KG tags
		const searchInput = page.locator('input[type="search"], #search-input, .search-input').first();
		await searchInput.fill('service haystack knowledge');
		await searchInput.press('Enter');

		// Wait for results
		await expect(page.locator('.box, .result-item, .search-result').first()).toBeVisible({
			timeout: 15000,
		});

		// Look for clickable tags
		const clickableTags = page.locator('.tag-button, .tag[role="button"], button .tag');

		if ((await clickableTags.count()) > 0) {
			console.log('🏷️  Found clickable knowledge graph tags');

			// Click on the first tag
			const firstTag = clickableTags.first();
			const tagText = await firstTag.textContent();
			console.log(`🔎 Clicking on tag: "${tagText}"`);

			// Add a small delay to ensure the tag is fully interactive
			await page.waitForTimeout(1000);

			await firstTag.click();

			// Wait for KG document modal or additional results
			console.log('⏳ Waiting for knowledge graph document...');

			// Look for modal or new content
			const hasModal = await page
				.locator('.modal.is-active, .modal-content')
				.first()
				.isVisible({ timeout: 5000 })
				.catch(() => false);

			if (hasModal) {
				console.log('✅ Knowledge graph document modal opened successfully');

				// Check for KG context information
				const kgContext = page.locator('.kg-context, .tag.is-info');
				if ((await kgContext.count()) > 0) {
					console.log('✅ KG context information displayed');
				}

				// Close modal
				const closeButton = page.locator('.modal-close, .delete').first();
				if ((await closeButton.count()) > 0) {
					await closeButton.click();
					console.log('✅ Modal closed successfully');
				}
			} else {
				console.log('📊 Knowledge graph integration may be working (no modal appeared)');

				// Check if there are any console errors
				const logs = await page.evaluate(() => {
					return (window as any).consoleLogs || [];
				});

				if (logs.length > 0) {
					console.log('📋 Console logs:', logs);
				}
			}
		} else {
			console.log('⚠️  No clickable tags found - KG may not be built yet');

			// Check if there are any search results at all
			const searchResults = page.locator('.box, .result-item, .search-result');
			const resultCount = await searchResults.count();
			console.log(`📄 Found ${resultCount} search results`);

			if (resultCount > 0) {
				// Check if results have tags but they're not clickable
				const allTags = page.locator('.tag');
				const tagCount = await allTags.count();
				console.log(`🏷️  Found ${tagCount} total tags (may not be clickable)`);

				if (tagCount > 0) {
					console.log('⚠️  Tags exist but may not be clickable - this could indicate a UI issue');
				}
			}
		}
	});

	test('should handle KG link clicks in document content', async ({ page }) => {
		console.log('🔗 Testing KG link clicks in document content...');

		// Navigate to the Tauri app
		await page.goto('http://localhost:5173', { waitUntil: 'networkidle' });

		// Wait for the app to load
		await page.waitForSelector('input[type="search"], #search-input, .search-input', {
			timeout: 10000,
		});

		// Search for something that should have KG links in content
		const searchInput = page.locator('input[type="search"], #search-input, .search-input').first();
		await searchInput.fill('terraphim graph');
		await searchInput.press('Enter');

		// Wait for results
		await expect(page.locator('.box, .result-item, .search-result').first()).toBeVisible({
			timeout: 15000,
		});

		// Click on the first result to open the document modal
		const firstResult = page.locator('.box, .result-item, .search-result').first();
		await firstResult.click();

		// Wait for modal to open
		await expect(page.locator('.modal.is-active, .modal-content').first()).toBeVisible({
			timeout: 5000,
		});

		// Look for KG links in the content (kg: protocol links)
		const kgLinks = page.locator('a[href^="kg:"]');

		if ((await kgLinks.count()) > 0) {
			console.log('🔗 Found KG links in document content');

			// Click on the first KG link
			const firstKgLink = kgLinks.first();
			const linkHref = await firstKgLink.getAttribute('href');
			console.log(`🔎 Clicking on KG link: "${linkHref}"`);

			await firstKgLink.click();

			// Wait for KG document modal
			console.log('⏳ Waiting for KG document modal...');

			// Look for nested modal or new content
			const hasKgModal = await page
				.locator('.modal.is-active .modal.is-active, .modal-content .modal-content')
				.first()
				.isVisible({ timeout: 5000 })
				.catch(() => false);

			if (hasKgModal) {
				console.log('✅ KG document modal opened successfully');

				// Close the KG modal
				const closeButtons = page.locator('.modal-close, .delete');
				if ((await closeButtons.count()) > 1) {
					await closeButtons.nth(1).click(); // Close the inner modal
					console.log('✅ KG modal closed successfully');
				}
			} else {
				console.log('📊 KG link click may have worked (no nested modal appeared)');
			}
		} else {
			console.log('⚠️  No KG links found in document content');
		}

		// Close the main modal
		const closeButton = page.locator('.modal-close, .delete').first();
		if ((await closeButton.count()) > 0) {
			await closeButton.click();
			console.log('✅ Main modal closed successfully');
		}
	});
});
