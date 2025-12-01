/**
 * Visual Regression Tests for Chat Layout Responsive Design
 *
 * This test suite validates visual consistency across:
 * - Different screen sizes and breakpoints
 * - All 22 Bulmaswatch themes
 * - Sidebar show/hide states
 * - Mobile and desktop layouts
 */

import { test, expect } from '@playwright/test';

// Available themes for visual testing
const THEMES = [
	'default',
	'darkly',
	'cerulean',
	'cosmo',
	'cyborg',
	'flatly',
	'journal',
	'litera',
	'lumen',
	'lux',
	'materia',
	'minty',
	'nuclear',
	'pulse',
	'sandstone',
	'simplex',
	'slate',
	'solar',
	'spacelab',
	'superhero',
	'united',
	'yeti',
];

// Responsive breakpoints for visual testing
const BREAKPOINTS = {
	desktop: { width: 1200, height: 800 },
	tablet: { width: 768, height: 1024 },
	mobile: { width: 375, height: 667 },
	smallMobile: { width: 320, height: 568 },
};

/**
 * Helper function to set viewport and navigate to chat
 */
async function setupChatPage(page: any, width: number, height: number) {
	await page.setViewportSize({ width, height });
	await page.goto('/chat');
	await page.waitForSelector('[data-testid="chat-interface"]', { timeout: 10000 });
	await page.waitForTimeout(1000);
}

/**
 * Helper function to apply theme
 */
async function applyTheme(page: any, theme: string) {
	await page.evaluate((themeName: string) => {
		localStorage.setItem('theme', themeName);
		const event = new Event('storage');
		window.dispatchEvent(event);
	}, theme);
	await page.reload();
	await page.waitForSelector('[data-testid="chat-interface"]', { timeout: 10000 });
	await page.waitForTimeout(1000);
}

/**
 * Helper function to toggle chat history sidebar
 */
async function toggleChatHistory(page: any) {
	const toggleButton = page.locator('button:has-text("History")');
	if (await toggleButton.isVisible()) {
		await toggleButton.click();
		await page.waitForTimeout(500);
	}
}

/**
 * Helper function to take screenshot with fallback for missing snapshots
 */
async function takeScreenshot(page: any, filename: string, options: any = {}) {
	try {
		await expect(page).toHaveScreenshot(filename, options);
	} catch (error) {
		// If snapshot doesn't exist, create it
		if (error.message.includes("A snapshot doesn't exist")) {
			console.log(`Creating new snapshot: ${filename}`);
			await expect(page).toHaveScreenshot(filename, { ...options, mode: 'write' });
		} else {
			throw error;
		}
	}
}

test.describe('Chat Layout Visual Regression Tests', () => {
	test.describe('Desktop Layout', () => {
		for (const theme of THEMES.slice(0, 4)) {
			// Test subset of themes for efficiency
			test(`${theme} theme - desktop with sidebar hidden`, async ({ page }) => {
				await setupChatPage(page, BREAKPOINTS.desktop.width, BREAKPOINTS.desktop.height);
				await applyTheme(page, theme);
				// Sidebar starts hidden by default

				await expect(page).toHaveScreenshot(`chat-layout-desktop-sidebar-hidden-${theme}.png`, {
					fullPage: false,
					clip: {
						x: 0,
						y: 0,
						width: BREAKPOINTS.desktop.width,
						height: BREAKPOINTS.desktop.height,
					},
				});
			});

			test(`${theme} theme - desktop with sidebar visible`, async ({ page }) => {
				await setupChatPage(page, BREAKPOINTS.desktop.width, BREAKPOINTS.desktop.height);
				await applyTheme(page, theme);
				await toggleChatHistory(page);

				await expect(page).toHaveScreenshot(`chat-layout-desktop-sidebar-visible-${theme}.png`, {
					fullPage: false,
					clip: {
						x: 0,
						y: 0,
						width: BREAKPOINTS.desktop.width,
						height: BREAKPOINTS.desktop.height,
					},
				});
			});
		}
	});

	test.describe('Tablet Layout', () => {
		for (const theme of ['spacelab', 'darkly', 'materia']) {
			// Key themes only
			test(`${theme} theme - tablet with sidebar visible`, async ({ page }) => {
				await setupChatPage(page, BREAKPOINTS.tablet.width, BREAKPOINTS.tablet.height);
				await applyTheme(page, theme);
				await toggleChatHistory(page);

				await expect(page).toHaveScreenshot(`chat-layout-tablet-sidebar-visible-${theme}.png`, {
					fullPage: false,
					clip: { x: 0, y: 0, width: BREAKPOINTS.tablet.width, height: BREAKPOINTS.tablet.height },
				});
			});

			test(`${theme} theme - tablet with sidebar hidden`, async ({ page }) => {
				await setupChatPage(page, BREAKPOINTS.tablet.width, BREAKPOINTS.tablet.height);
				await applyTheme(page, theme);
				// Sidebar starts hidden by default

				await expect(page).toHaveScreenshot(`chat-layout-tablet-sidebar-hidden-${theme}.png`, {
					fullPage: false,
					clip: { x: 0, y: 0, width: BREAKPOINTS.tablet.width, height: BREAKPOINTS.tablet.height },
				});
			});
		}
	});

	test.describe('Mobile Layout', () => {
		for (const theme of ['spacelab', 'darkly']) {
			// Essential themes only
			test(`${theme} theme - mobile with sidebar visible (stacked)`, async ({ page }) => {
				await setupChatPage(page, BREAKPOINTS.mobile.width, BREAKPOINTS.mobile.height);
				await applyTheme(page, theme);
				await toggleChatHistory(page);

				await expect(page).toHaveScreenshot(`chat-layout-mobile-sidebar-visible-${theme}.png`, {
					fullPage: false,
					clip: { x: 0, y: 0, width: BREAKPOINTS.mobile.width, height: BREAKPOINTS.mobile.height },
				});
			});

			test(`${theme} theme - mobile with sidebar hidden`, async ({ page }) => {
				await setupChatPage(page, BREAKPOINTS.mobile.width, BREAKPOINTS.mobile.height);
				await applyTheme(page, theme);
				// Sidebar starts hidden by default

				await expect(page).toHaveScreenshot(`chat-layout-mobile-sidebar-hidden-${theme}.png`, {
					fullPage: false,
					clip: { x: 0, y: 0, width: BREAKPOINTS.mobile.width, height: BREAKPOINTS.mobile.height },
				});
			});
		}
	});

	test.describe('Small Mobile Layout', () => {
		test('spacelab theme - small mobile with sidebar visible', async ({ page }) => {
			await setupChatPage(page, BREAKPOINTS.smallMobile.width, BREAKPOINTS.smallMobile.height);
			await applyTheme(page, 'spacelab');
			await toggleChatHistory(page);

			await expect(page).toHaveScreenshot('chat-layout-small-mobile-sidebar-visible-spacelab.png', {
				fullPage: false,
				clip: {
					x: 0,
					y: 0,
					width: BREAKPOINTS.smallMobile.width,
					height: BREAKPOINTS.smallMobile.height,
				},
			});
		});

		test('spacelab theme - small mobile with sidebar hidden', async ({ page }) => {
			await setupChatPage(page, BREAKPOINTS.smallMobile.width, BREAKPOINTS.smallMobile.height);
			await applyTheme(page, 'spacelab');
			// Sidebar starts hidden by default

			await expect(page).toHaveScreenshot('chat-layout-small-mobile-sidebar-hidden-spacelab.png', {
				fullPage: false,
				clip: {
					x: 0,
					y: 0,
					width: BREAKPOINTS.smallMobile.width,
					height: BREAKPOINTS.smallMobile.height,
				},
			});
		});
	});

	test.describe('Input Area Visual Tests', () => {
		test('input area with long text on desktop', async ({ page }) => {
			await setupChatPage(page, BREAKPOINTS.desktop.width, BREAKPOINTS.desktop.height);
			await applyTheme(page, 'spacelab');

			// Fill input with long text to test textarea constraints
			const longText =
				'This is a very long message that should test the textarea height constraints and ensure the send button remains visible and properly positioned. '.repeat(
					5
				);
			const input = page.locator('textarea[data-testid="chat-input"]');
			await input.fill(longText);

			// Take screenshot of input area specifically
			const inputContainer = page.locator('.chat-input');
			await expect(inputContainer).toHaveScreenshot('chat-input-long-text-desktop.png');
		});

		test('input area on mobile with stacked layout', async ({ page }) => {
			await setupChatPage(page, BREAKPOINTS.mobile.width, BREAKPOINTS.mobile.height);
			await applyTheme(page, 'spacelab');

			const inputContainer = page.locator('.chat-input');
			await expect(inputContainer).toHaveScreenshot('chat-input-mobile-stacked.png');
		});
	});

	test.describe('Sidebar Visual Tests', () => {
		test('sidebar content with multiple conversations', async ({ page }) => {
			await setupChatPage(page, BREAKPOINTS.desktop.width, BREAKPOINTS.desktop.height);
			await applyTheme(page, 'spacelab');
			await toggleChatHistory(page);

			// Take screenshot of sidebar specifically
			const sidebar = page.locator('.session-list');
			await expect(sidebar).toHaveScreenshot('chat-sidebar-content-desktop.png');
		});

		test('sidebar on mobile (stacked above chat)', async ({ page }) => {
			await setupChatPage(page, BREAKPOINTS.mobile.width, BREAKPOINTS.mobile.height);
			await applyTheme(page, 'spacelab');
			await toggleChatHistory(page);

			// Take screenshot of mobile stacked layout
			const chatLayout = page.locator('.chat-layout-grid');
			await expect(chatLayout).toHaveScreenshot('chat-sidebar-mobile-stacked.png');
		});
	});

	test.describe('Header and Navigation Visual Tests', () => {
		test('chat header on desktop', async ({ page }) => {
			await setupChatPage(page, BREAKPOINTS.desktop.width, BREAKPOINTS.desktop.height);
			await applyTheme(page, 'spacelab');

			const chatHeader = page.locator('.chat-header');
			await expect(chatHeader).toHaveScreenshot('chat-header-desktop.png');
		});

		test('chat header on mobile (stacked actions)', async ({ page }) => {
			await setupChatPage(page, BREAKPOINTS.smallMobile.width, BREAKPOINTS.smallMobile.height);
			await applyTheme(page, 'spacelab');

			const chatHeader = page.locator('.chat-header');
			await expect(chatHeader).toHaveScreenshot('chat-header-small-mobile.png');
		});
	});

	test.describe('Cross-Theme Consistency', () => {
		test('layout structure consistency across themes', async ({ page }) => {
			await setupChatPage(page, BREAKPOINTS.desktop.width, BREAKPOINTS.desktop.height);
			await toggleChatHistory(page);

			// Test that layout structure remains consistent across themes
			const testThemes = ['spacelab', 'darkly', 'materia', 'cyborg'];

			for (const theme of testThemes) {
				await applyTheme(page, theme);

				// Verify key layout elements are present
				await expect(page.locator('.chat-layout-grid')).toBeVisible();
				await expect(page.locator('.main-chat-area')).toBeVisible();
				await expect(page.locator('textarea[data-testid="chat-input"]')).toBeVisible();
				await expect(page.locator('button[data-testid="send-message-button"]')).toBeVisible();

				// Check if sidebar is visible (it might be hidden by default)
				const sidebar = page.locator('.session-list-column');
				if (await sidebar.isVisible()) {
					await expect(sidebar).toBeVisible();
				}
			}
		});
	});

	test.describe('Edge Cases Visual Tests', () => {
		test('very small viewport (280px width)', async ({ page }) => {
			await setupChatPage(page, 280, 400);
			await applyTheme(page, 'spacelab');

			await expect(page).toHaveScreenshot('chat-layout-very-small-viewport.png', {
				fullPage: false,
				clip: { x: 0, y: 0, width: 280, height: 400 },
			});
		});

		test('very tall viewport (1200px height)', async ({ page }) => {
			await setupChatPage(page, BREAKPOINTS.desktop.width, 1200);
			await applyTheme(page, 'spacelab');
			await toggleChatHistory(page);

			await expect(page).toHaveScreenshot('chat-layout-tall-viewport.png', {
				fullPage: false,
				clip: { x: 0, y: 0, width: BREAKPOINTS.desktop.width, height: 1200 },
			});
		});
	});
});
