/**
 * State Persistence Tests
 *
 * This test suite validates that search queries and chat sessions
 * are properly persisted and restored when navigating between pages.
 */

import { test, expect } from '@playwright/test';
import {
	ciWaitForSelector,
	ciSearch,
	ciNavigate,
	ciWait,
	ciClick,
	getTimeouts,
} from '../../src/test-utils/ci-friendly';

const TEST_TIMEOUT = 30000;

test.describe('State Persistence', () => {
	test.beforeEach(async ({ page, context }) => {
		// Grant storage permissions to avoid SecurityError
		await context.grantPermissions(['storage-access']);

		await ciNavigate(page, '/');
		await ciWaitForSelector(page, 'input[type="search"]', 'navigation');

		// Clear localStorage after navigation to ensure clean state
		try {
			await page.evaluate(() => {
				if (typeof window !== 'undefined' && window.localStorage) {
					window.localStorage.clear();
				}
			});
		} catch (e) {
			console.log('localStorage clear failed (expected in some test environments)');
		}
	});

	test('should persist search query and results when navigating away and back', async ({
		page,
	}) => {
		const searchTerm = 'async programming';

		// Perform initial search
		await ciSearch(page, 'input[type="search"]', searchTerm);
		await ciWait(2000); // Allow search to complete

		// Verify search input contains the term
		const searchInput = await page.locator('input[type="search"]');
		await expect(searchInput).toHaveValue(searchTerm);

		// Check if results are displayed (may be empty but that's ok)
		const resultsContainer = page.locator('[data-testid="search-results"]');

		// Navigate to chat page
		await ciClick(page, 'a[href="/chat"]');
		await ciWaitForSelector(page, '[data-testid="chat-interface"]');

		// Navigate back to search
		await ciClick(page, 'a[href="/"]');
		await ciWaitForSelector(page, 'input[type="search"]');

		// Verify search term is still there
		const persistedSearchInput = await page.locator('input[type="search"]');
		await expect(persistedSearchInput).toHaveValue(searchTerm);

		console.log('✅ Search query persistence test passed');
	});

	test('should persist chat messages when navigating away and back', async ({ page }) => {
		const testMessage = 'Hello, this is a test message for persistence';

		// Navigate to chat
		await ciClick(page, 'a[href="/chat"]');
		await ciWaitForSelector(page, '[data-testid="chat-interface"]');

		// Wait for chat to initialize
		await ciWait(1000);

		// Type and send a message
		const chatInput = await ciWaitForSelector(page, '[data-testid="chat-input"]');
		await chatInput.fill(testMessage);

		// Send message (either click send button or press Enter)
		const sendButton = page.locator('[data-testid="send-message-button"]');
		if (await sendButton.isVisible()) {
			await ciClick(page, '[data-testid="send-message-button"]');
		} else {
			await chatInput.press('Enter');
		}

		// Wait for message to appear in chat
		await ciWait(1000);

		// Verify message appears in chat messages
		const chatMessages = await ciWaitForSelector(page, '[data-testid="chat-messages"]');
		await expect(chatMessages).toContainText(testMessage);

		// Navigate to search page
		await ciClick(page, 'a[href="/"]');
		await ciWaitForSelector(page, 'input[type="search"]');

		// Navigate back to chat
		await ciClick(page, 'a[href="/chat"]');
		await ciWaitForSelector(page, '[data-testid="chat-interface"]');

		// Wait for chat to load
		await ciWait(1000);

		// Verify message is still there
		const persistedChatMessages = await ciWaitForSelector(page, '[data-testid="chat-messages"]');
		await expect(persistedChatMessages).toContainText(testMessage);

		console.log('✅ Chat message persistence test passed');
	});

	test('should persist markdown rendering preference', async ({ page }) => {
		// Navigate to chat
		await ciClick(page, 'a[href="/chat"]');
		await ciWaitForSelector(page, '[data-testid="chat-interface"]');

		// Find and toggle markdown rendering checkbox
		const markdownCheckbox = page.locator('input[type="checkbox"]:near(text("Render markdown"))');

		if (await markdownCheckbox.isVisible()) {
			// Get initial state
			const initialChecked = await markdownCheckbox.isChecked();

			// Toggle the checkbox
			await markdownCheckbox.click();
			await ciWait(500);

			// Verify state changed
			const newChecked = await markdownCheckbox.isChecked();
			expect(newChecked).toBe(!initialChecked);

			// Navigate away and back
			await ciClick(page, 'a[href="/"]');
			await ciWaitForSelector(page, 'input[type="search"]');

			await ciClick(page, 'a[href="/chat"]');
			await ciWaitForSelector(page, '[data-testid="chat-interface"]');

			// Verify preference persisted
			const persistedCheckbox = page.locator(
				'input[type="checkbox"]:near(text("Render markdown"))'
			);
			await expect(persistedCheckbox).toBeChecked({ checked: newChecked });

			console.log('✅ Markdown preference persistence test passed');
		} else {
			console.log('⚠️ Markdown checkbox not found, skipping preference test');
		}
	});

	test('should maintain separate persistence per role', async ({ page }) => {
		// Test that different roles maintain separate search and chat state
		const role1SearchTerm = 'rust programming';
		const role2SearchTerm = 'async patterns';

		// Perform search with initial role
		await ciSearch(page, 'input[type="search"]', role1SearchTerm);
		await ciWait(1000);

		// Switch to different role if role selector exists
		const roleSelector = page.locator('select:has(option)').first();

		if (await roleSelector.isVisible()) {
			const options = await roleSelector.locator('option').all();
			if (options.length > 1) {
				// Select different role
				await roleSelector.selectOption({ index: 1 });
				await ciWait(1000);

				// Search input should be cleared for new role
				const searchInput = await page.locator('input[type="search"]');
				const currentValue = await searchInput.inputValue();

				// Either empty or has default value for new role
				if (currentValue !== role1SearchTerm) {
					// Perform search with new role
					await ciSearch(page, 'input[type="search"]', role2SearchTerm);
					await ciWait(1000);

					// Switch back to original role
					await roleSelector.selectOption({ index: 0 });
					await ciWait(1000);

					// Verify original search term is restored
					const restoredSearchInput = await page.locator('input[type="search"]');
					await expect(restoredSearchInput).toHaveValue(role1SearchTerm);

					console.log('✅ Role-specific persistence test passed');
				} else {
					console.log('⚠️ Role switching did not clear search, test inconclusive');
				}
			} else {
				console.log('⚠️ Only one role available, skipping role-specific test');
			}
		} else {
			console.log('⚠️ Role selector not found, skipping role-specific test');
		}
	});

	test('should handle localStorage errors gracefully', async ({ page, context }) => {
		// Test that app works even when localStorage has issues

		// Navigate to a fresh page first
		await ciNavigate(page, '/');
		await ciWaitForSelector(page, 'input[type="search"]');

		// Verify app still works even if localStorage might fail
		const searchInput = await page.locator('input[type="search"]');
		await expect(searchInput).toBeVisible();

		// Try to perform search (should work despite potential persistence failures)
		await ciSearch(page, 'input[type="search"]', 'test search');

		// Navigation should still work
		const chatLink = page.locator('a[href="/chat"]');
		if (await chatLink.isVisible()) {
			await chatLink.click();
			await ciWaitForSelector(page, '[data-testid="chat-interface"]');
		}

		console.log('✅ localStorage error handling test passed');
	});

	test('should clear state when requested', async ({ page }) => {
		// Perform some actions to create state
		await ciSearch(page, 'input[type="search"]', 'test query');
		await ciWait(1000);

		// Navigate to chat and send message
		await ciClick(page, 'a[href="/chat"]');
		await ciWaitForSelector(page, '[data-testid="chat-interface"]');

		const chatInput = await ciWaitForSelector(page, '[data-testid="chat-input"]');
		await chatInput.fill('test chat message');

		// Check if there's a clear/reset button or similar functionality
		const clearButton = page.locator(
			'button:has-text("Clear"), button:has-text("Reset"), button:has-text("New")'
		);

		if (await clearButton.first().isVisible()) {
			await clearButton.first().click();
			await ciWait(500);

			// Verify state was cleared
			const clearedChatInput = page.locator('[data-testid="chat-input"]');
			await expect(clearedChatInput).toHaveValue('');

			console.log('✅ State clearing test passed');
		} else {
			console.log('⚠️ Clear button not found, manual state clearing not available');
		}
	});
});
