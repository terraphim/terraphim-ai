import { test, expect } from '@playwright/test';

test.describe('LLM Provider Error Reproduction', () => {
  test('should reproduce LLM provider error by sending message', async ({ page }) => {
    // Listen for network requests
    const chatRequests: { url: string; method: string; body: string }[] = [];
    const chatResponses: { url: string; status: number; body: string }[] = [];

    page.on('request', request => {
      if (request.url().includes('/chat')) {
        chatRequests.push({
          url: request.url(),
          method: request.method(),
          body: request.postData() || ''
        });
      }
    });

    page.on('response', async response => {
      if (response.url().includes('/chat')) {
        const body = await response.text();
        chatResponses.push({
          url: response.url(),
          status: response.status(),
          body
        });
      }
    });

    // Navigate to chat page
    await page.goto('http://localhost:5173/chat');

    // Wait for page to load
    await page.waitForTimeout(3000);

    // Find the chat input (textarea)
    const chatInput = page.locator('textarea');
    await chatInput.waitFor({ state: 'visible' });

    // Type a message
    await chatInput.fill('Hello, this is a test message');
    await page.waitForTimeout(1000);

    // Look for send button or try Enter key
    const sendButton = page.locator('button:has-text("Send"), button[type="submit"], button[aria-label*="Send"]');
    const hasSendButton = await sendButton.isVisible();

    if (hasSendButton) {
      console.log('Found send button, clicking...');
      await sendButton.click();
    } else {
      console.log('No send button found, trying Enter key...');
      await chatInput.press('Enter');
    }

    // Wait for network requests
    await page.waitForTimeout(3000);

    // Check for chat requests
    console.log('Chat requests:', chatRequests);
    console.log('Chat responses:', chatResponses);

    // Look for error messages on the page
    const errorSelectors = [
      'text=No LLM provider configured for this role',
      'text=LLM provider',
      '.error',
      '.error-message',
      '[class*="error"]',
      '.alert',
      '.notification',
      '.warning'
    ];

    let foundError = false;
    for (const selector of errorSelectors) {
      const elements = page.locator(selector);
      const count = await elements.count();
      if (count > 0) {
        console.log(`Found ${count} elements matching "${selector}"`);
        for (let i = 0; i < count; i++) {
          const text = await elements.nth(i).textContent();
          console.log(`  ${i + 1}: ${text}`);
          if (text?.includes('LLM provider')) {
            foundError = true;
          }
        }
      }
    }

    // Check if we got a 404 or other error from the chat endpoint
    const chatError = chatResponses.find(r => r.status >= 400);
    if (chatError) {
      console.log('❌ Chat endpoint error:', chatError);
      foundError = true;
    }

    if (!foundError) {
      console.log('ℹ️ No LLM provider error found. Checking page state...');

      // Check if message was sent successfully
      const messages = page.locator('.message, .response, .assistant-message, [class*="message"]');
      const messageCount = await messages.count();
      console.log(`Found ${messageCount} message elements`);

      for (let i = 0; i < messageCount; i++) {
        const text = await messages.nth(i).textContent();
        console.log(`Message ${i + 1}: ${text}`);
      }
    }

    // Take a screenshot for debugging
    await page.screenshot({ path: 'chat-after-send.png' });
  });
});
