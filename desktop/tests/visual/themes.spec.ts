import { test, expect } from '@playwright/test';

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

test.describe('Theme Visual Regression Tests', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/');
		await page.waitForSelector('input[type="search"]', { timeout: 30000 });
	});

	for (const theme of THEMES) {
		test(`should render ${theme} theme correctly`, async ({ page }) => {
			// Change to the specific theme
			await page.evaluate((themeName) => {
				localStorage.setItem('theme', themeName);
				// Trigger theme change
				const event = new Event('storage');
				window.dispatchEvent(event);
			}, theme);

			// Reload to apply theme
			await page.reload();
			await page.waitForSelector('input[type="search"]', { timeout: 30000 });

			// Wait for theme to be applied
			await page.waitForTimeout(1000);

			// Take screenshot of the main page
			await expect(page).toHaveScreenshot(`theme-${theme}-main.png`, {
				fullPage: true,
				animations: 'disabled',
			});
		});
	}

	test('should maintain visual consistency across theme changes', async ({ page }) => {
		const searchInput = page.locator('input[type="search"]');

		// Enter some text
		await searchInput.fill('visual regression test');

		// Test a few key themes
		const keyThemes = ['default', 'darkly', 'cerulean', 'nuclear'];

		for (const theme of keyThemes) {
			await page.evaluate((themeName) => {
				localStorage.setItem('theme', themeName);
				const event = new Event('storage');
				window.dispatchEvent(event);
			}, theme);

			// Wait for theme to apply
			await page.waitForTimeout(1000);

			// Take screenshot with content
			await expect(page).toHaveScreenshot(`theme-${theme}-with-content.png`, {
				fullPage: true,
				animations: 'disabled',
			});

			// Verify search input still contains text
			expect(await searchInput.inputValue()).toBe('visual regression test');
		}
	});

	test('should render theme switcher dropdown correctly', async ({ page }) => {
		// Try to open theme switcher
		const themeButton = page.locator('button:has-text("Theme")');

		if (await themeButton.isVisible()) {
			await themeButton.click();
			await page.waitForTimeout(500);

			// Take screenshot of opened dropdown
			await expect(page).toHaveScreenshot('theme-switcher-dropdown.png', {
				animations: 'disabled',
			});
		}
	});

	test('should handle theme transitions smoothly', async ({ page }) => {
		const themes = ['default', 'darkly', 'cerulean'];

		for (let i = 0; i < themes.length; i++) {
			await page.evaluate((themeName) => {
				localStorage.setItem('theme', themeName);
				const event = new Event('storage');
				window.dispatchEvent(event);
			}, themes[i]);

			// Wait for transition
			await page.waitForTimeout(500);

			// Take screenshot during transition
			await expect(page).toHaveScreenshot(`theme-transition-${i}.png`, {
				animations: 'disabled',
			});
		}
	});

	test('should render search suggestions with different themes', async ({ page }) => {
		const searchInput = page.locator('input[type="search"]');

		// Type to potentially trigger suggestions
		await searchInput.fill('artificial');
		await page.waitForTimeout(500);

		// Test with different themes
		const testThemes = ['default', 'darkly', 'lux'];

		for (const theme of testThemes) {
			await page.evaluate((themeName) => {
				localStorage.setItem('theme', themeName);
				const event = new Event('storage');
				window.dispatchEvent(event);
			}, theme);

			await page.waitForTimeout(500);

			// Take screenshot with suggestions (if any)
			await expect(page).toHaveScreenshot(`suggestions-${theme}.png`, {
				animations: 'disabled',
			});
		}
	});

	test('should render navigation elements consistently', async ({ page }) => {
		const footer = page.locator('footer');

		// Hover to reveal navigation
		await footer.hover();
		await page.waitForTimeout(500);

		const testThemes = ['default', 'darkly', 'cerulean'];

		for (const theme of testThemes) {
			await page.evaluate((themeName) => {
				localStorage.setItem('theme', themeName);
				const event = new Event('storage');
				window.dispatchEvent(event);
			}, theme);

			await page.waitForTimeout(500);

			// Keep footer hovered
			await footer.hover();
			await page.waitForTimeout(200);

			// Take screenshot of navigation
			await expect(page).toHaveScreenshot(`navigation-${theme}.png`, {
				animations: 'disabled',
			});
		}
	});

	test('should handle responsive design across themes', async ({ page }) => {
		const viewports = [
			{ width: 1920, height: 1080, name: 'desktop' },
			{ width: 1024, height: 768, name: 'tablet' },
			{ width: 375, height: 667, name: 'mobile' },
		];

		const testThemes = ['default', 'darkly'];

		for (const theme of testThemes) {
			for (const viewport of viewports) {
				await page.setViewportSize(viewport);
				await page.waitForTimeout(500);

				await page.evaluate((themeName) => {
					localStorage.setItem('theme', themeName);
					const event = new Event('storage');
					window.dispatchEvent(event);
				}, theme);

				await page.waitForTimeout(500);

				// Take screenshot at different viewport sizes
				await expect(page).toHaveScreenshot(`responsive-${theme}-${viewport.name}.png`, {
					fullPage: true,
					animations: 'disabled',
				});
			}
		}
	});

	test('should maintain accessibility contrast across themes', async ({ page }) => {
		// This test checks visual rendering but accessibility tools would be needed
		// for actual contrast measurement

		const accessibilityThemes = ['default', 'darkly', 'lux', 'materia'];

		for (const theme of accessibilityThemes) {
			await page.evaluate((themeName) => {
				localStorage.setItem('theme', themeName);
				const event = new Event('storage');
				window.dispatchEvent(event);
			}, theme);

			await page.waitForTimeout(500);

			// Take screenshot for manual accessibility review
			await expect(page).toHaveScreenshot(`accessibility-${theme}.png`, {
				fullPage: true,
				animations: 'disabled',
			});
		}
	});
});
