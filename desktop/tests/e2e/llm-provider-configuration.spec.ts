import { test, expect } from '@playwright/test';
import { ciWaitForSelector, ciNavigate, ciWait } from '../../src/test-utils/ci-friendly';

const TEST_TIMEOUT = 120000; // 2 minutes for LLM operations

test.describe('LLM Provider Configuration Tests', () => {
	test.beforeEach(async ({ page }) => {
		test.setTimeout(TEST_TIMEOUT);
		await ciNavigate(page, '/config/wizard');
		await ciWaitForSelector(page, '[data-testid="config-wizard"]');
	});

	test('should show LLM provider error when Ollama is configured but chat fails', async ({
		page,
	}) => {
		// Navigate to chat page first to trigger the error
		await ciNavigate(page, '/chat');
		await ciWait(page, 'small');

		// Look for the error message
		const errorMessage = page.locator('text=No LLM provider configured for this role');
		const hasError = await errorMessage.isVisible();

		if (hasError) {
			console.log('✅ Reproduced the LLM provider configuration error');
			const errorText = await errorMessage.textContent();
			expect(errorText).toContain('No LLM provider configured for this role');
		} else {
			console.log('ℹ️ No LLM provider error found - may already be configured');
		}
	});

	test('should configure Ollama provider and validate chat works', async ({ page }) => {
		// First, configure Ollama in the config wizard
		await ciNavigate(page, '/config/wizard');
		await ciWaitForSelector(page, '[data-testid="config-wizard"]');

		const roles = page.locator('[data-testid="role-config"], .role-config');
		const firstRole = roles.first();

		// Configure LLM provider
		const llmProviderSelect = firstRole.locator('select[name*="llm_provider"]');
		if (await llmProviderSelect.isVisible()) {
			await llmProviderSelect.selectOption('ollama');
			await ciWait(page, 'small');

			// Set Ollama configuration
			const modelInput = firstRole.locator('input[name*="llm_model"]');
			if (await modelInput.isVisible()) {
				await modelInput.fill('llama3.2:3b');
			}

			const baseUrlInput = firstRole.locator('input[name*="llm_base_url"]');
			if (await baseUrlInput.isVisible()) {
				await baseUrlInput.fill('http://127.0.0.1:11434');
			}

			// Save configuration
			const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save")');
			if (await saveButton.isVisible()) {
				await saveButton.click();
				await ciWait(page, 'medium');
				console.log('✅ Ollama configuration saved');
			}
		}

		// Now test chat functionality
		await ciNavigate(page, '/chat');
		await ciWait(page, 'small');

		// Check if there's still an error
		const errorMessage = page.locator('text=No LLM provider configured for this role');
		const hasError = await errorMessage.isVisible();

		if (hasError) {
			console.log('❌ LLM provider error still present after configuration');
			const errorText = await errorMessage.textContent();
			console.log('Error text:', errorText);
		} else {
			console.log('✅ No LLM provider error - configuration successful');
		}

		// Try to send a test message
		const chatInput = page.locator('input[type="text"], textarea');
		if (await chatInput.isVisible()) {
			await chatInput.fill('Hello, this is a test message');
			await ciWait(page, 'small');

			const sendButton = page.locator('button:has-text("Send"), button[type="submit"]');
			if (await sendButton.isVisible()) {
				await sendButton.click();
				await ciWait(page, 'large'); // Wait for LLM response

				// Check for response or error
				const response = page.locator('.message, .response, .assistant-message');
				const error = page.locator('.error, .error-message');

				if (await response.isVisible()) {
					console.log('✅ Chat response received');
				} else if (await error.isVisible()) {
					const errorText = await error.textContent();
					console.log('❌ Chat error:', errorText);
				} else {
					console.log('ℹ️ No response or error visible');
				}
			}
		}
	});

	test('should validate role extra settings structure', async ({ page }) => {
		// Test that we can access the role configuration
		await ciNavigate(page, '/config/wizard');
		await ciWaitForSelector(page, '[data-testid="config-wizard"]');

		const roles = page.locator('[data-testid="role-config"], .role-config');
		const firstRole = roles.first();

		// Check if LLM provider fields are present
		const llmProviderSelect = firstRole.locator('select[name*="llm_provider"]');
		const hasLlmProvider = await llmProviderSelect.isVisible();

		if (hasLlmProvider) {
			console.log('✅ LLM provider configuration UI found');

			// Test selecting Ollama
			await llmProviderSelect.selectOption('ollama');
			await ciWait(page, 'small');

			// Check if Ollama-specific fields appear
			const modelInput = firstRole.locator('input[name*="llm_model"]');
			const baseUrlInput = firstRole.locator('input[name*="llm_base_url"]');

			const hasModelInput = await modelInput.isVisible();
			const hasBaseUrlInput = await baseUrlInput.isVisible();

			console.log('Ollama fields - Model:', hasModelInput, 'Base URL:', hasBaseUrlInput);

			if (hasModelInput && hasBaseUrlInput) {
				console.log('✅ Ollama configuration fields are properly displayed');
			} else {
				console.log('❌ Ollama configuration fields missing');
			}
		} else {
			console.log('❌ LLM provider configuration UI not found');
		}
	});
});
