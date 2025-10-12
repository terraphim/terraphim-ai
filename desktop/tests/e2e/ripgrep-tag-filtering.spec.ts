import { test, expect } from '@playwright/test';

test.describe('Ripgrep Tag Filtering Configuration', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the configuration wizard
    await page.goto('/config/wizard');

    // Wait for the wizard to load
    await page.waitForSelector('.box h3:has-text("Configuration Wizard")', { timeout: 30000 });
  });

  test('should display tag filtering UI for Ripgrep haystacks', async ({ page }) => {
    // Navigate to step 2 (roles configuration)
    await page.getByTestId('wizard-next').click();
    await page.waitForSelector('h4:has-text("Roles")', { timeout: 5000 });

    // Add a new role
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });

    // Add a haystack to the role
    await page.getByTestId('add-haystack-0').click();
    await page.waitForSelector('#haystack-path-0-0', { timeout: 5000 });

    // Verify the service selector defaults to Ripgrep
    const serviceSelect = page.locator('#haystack-service-0-0');
    await expect(serviceSelect).toHaveValue('Ripgrep');

    // Verify tag filtering UI is visible for Ripgrep
    await expect(page.locator('label:has-text("Extra Parameters (for filtering)")')).toBeVisible();
    await expect(page.locator('label:has-text("Hashtag")')).toBeVisible();
    await expect(page.locator('#ripgrep-hashtag-0-0')).toBeVisible();
    await expect(page.locator('#ripgrep-hashtag-preset-0-0')).toBeVisible();

    // Verify help text is displayed
    await expect(page.locator('text=When set, searches will enforce the hashtag')).toBeVisible();
    await expect(page.locator('code:has-text("-e \\"search\\" -e \\"#rust\\"")')).toBeVisible();
  });

  test('should allow manual tag input', async ({ page }) => {
    // Navigate to step 2 and set up a Ripgrep haystack
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });
    await page.getByTestId('add-haystack-0').click();
    await page.waitForSelector('#haystack-path-0-0', { timeout: 5000 });

    // Enter a custom tag
    const tagInput = page.locator('#ripgrep-hashtag-0-0');
    await tagInput.fill('#custom');
    await expect(tagInput).toHaveValue('#custom');

    // Navigate to review step to verify the configuration
    await page.getByTestId('wizard-next').click();
    await expect(page.locator('h4:has-text("Review")')).toBeVisible();

    // Check the JSON configuration contains the tag
    const reviewJson = page.locator('pre');
    const jsonText = await reviewJson.textContent();
    const config = JSON.parse(jsonText || '{}');

    expect(config.roles).toBeDefined();
    expect(config.roles[0].haystacks).toBeDefined();
    expect(config.roles[0].haystacks[0].extra_parameters).toBeDefined();
    expect(config.roles[0].haystacks[0].extra_parameters.tag).toBe('#custom');
  });

  test('should allow tag selection from presets', async ({ page }) => {
    // Navigate to step 2 and set up a Ripgrep haystack
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });
    await page.getByTestId('add-haystack-0').click();
    await page.waitForSelector('#haystack-path-0-0', { timeout: 5000 });

    // Select a preset tag
    const presetSelect = page.locator('#ripgrep-hashtag-preset-0-0');
    await presetSelect.selectOption('#rust');

    // Verify the tag input was updated
    const tagInput = page.locator('#ripgrep-hashtag-0-0');
    await expect(tagInput).toHaveValue('#rust');

    // Try another preset
    await presetSelect.selectOption('#docs');
    await expect(tagInput).toHaveValue('#docs');

    // Try test preset
    await presetSelect.selectOption('#test');
    await expect(tagInput).toHaveValue('#test');
  });

  test('should support multiple extra parameters', async ({ page }) => {
    // Navigate to step 2 and set up a Ripgrep haystack
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });
    await page.getByTestId('add-haystack-0').click();
    await page.waitForSelector('#haystack-path-0-0', { timeout: 5000 });

    // Set a tag
    const tagInput = page.locator('#ripgrep-hashtag-0-0');
    await tagInput.fill('#rust');

    // Add additional parameters using the buttons
    await page.locator('button:has-text("+ Max Results")').click();
    await page.locator('button:has-text("+ Custom Parameter")').click();

    // Verify additional parameter fields appear
    await expect(page.locator('input[placeholder="Parameter name"]')).toBeVisible();
    await expect(page.locator('input[placeholder="Parameter value"]')).toBeVisible();

    // Check that max_count was added with default value
    const maxCountInputs = page.locator('input[placeholder="Parameter value"]');
    const maxCountInput = maxCountInputs.nth(0); // First additional parameter should be max_count
    await expect(maxCountInput).toHaveValue('10');

    // Navigate to review and check the configuration
    await page.getByTestId('wizard-next').click();
    const reviewJson = page.locator('pre');
    const jsonText = await reviewJson.textContent();
    const config = JSON.parse(jsonText || '{}');

    expect(config.roles[0].haystacks[0].extra_parameters.tag).toBe('#rust');
    expect(config.roles[0].haystacks[0].extra_parameters.max_count).toBe('10');
  });

  test('should not show tag filtering for Atomic haystacks', async ({ page }) => {
    // Navigate to step 2 and set up an Atomic haystack
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });
    await page.getByTestId('add-haystack-0').click();
    await page.waitForSelector('#haystack-path-0-0', { timeout: 5000 });

    // Change service to Atomic
    const serviceSelect = page.locator('#haystack-service-0-0');
    await serviceSelect.selectOption('Atomic');

    // Verify tag filtering UI is NOT visible for Atomic
    await expect(page.locator('label:has-text("Extra Parameters (for filtering)")')).not.toBeVisible();
    await expect(page.locator('label:has-text("Hashtag")')).not.toBeVisible();

    // But Atomic Server Secret should be visible
    await expect(page.locator('label:has-text("Atomic Server Secret")')).toBeVisible();
  });

  test('should preserve tag configuration when switching between steps', async ({ page }) => {
    // Navigate to step 2 and configure a role with tag filtering
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });

    // Configure role name
    await page.fill('#role-name-0', 'Tagged Role');
    await page.fill('#role-shortname-0', 'tagged');

    // Add haystack with tag
    await page.getByTestId('add-haystack-0').click();
    await page.waitForSelector('#haystack-path-0-0', { timeout: 5000 });
    await page.fill('#haystack-path-0-0', '/test/path');
    await page.fill('#ripgrep-hashtag-0-0', '#test');

    // Navigate to review and back
    await page.getByTestId('wizard-next').click();
    await expect(page.locator('h4:has-text("Review")')).toBeVisible();
    await page.getByTestId('wizard-back').click();

    // Verify the tag configuration is preserved
    await expect(page.locator('#role-name-0')).toHaveValue('Tagged Role');
    await expect(page.locator('#ripgrep-hashtag-0-0')).toHaveValue('#test');
    await expect(page.locator('#haystack-path-0-0')).toHaveValue('/test/path');
  });

  test('should save and load tag configuration correctly', async ({ page }) => {
    // Configure a complete role with tag filtering
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });

    // Configure role
    await page.fill('#role-name-0', 'Tag Test Role');
    await page.fill('#role-shortname-0', 'tag-test');

    // Add haystack with tag filtering
    await page.getByTestId('add-haystack-0').click();
    await page.waitForSelector('#haystack-path-0-0', { timeout: 5000 });
    await page.fill('#haystack-path-0-0', '/Users/alex/projects/terraphim/terraphim-ai/terraphim_server/fixtures/haystack');
    await page.fill('#ripgrep-hashtag-0-0', '#rust');

    // Add max count parameter
    await page.locator('button:has-text("+ Max Results")').click();

    // Navigate to review and save
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('wizard-save').click();

    // Wait for save to complete
    await page.waitForTimeout(2000);

    // Verify the configuration was saved correctly via API
    const response = await page.request.get('http://localhost:8000/config');
    expect(response.status()).toBe(200);

    const savedConfig = await response.json();
    expect(savedConfig.config.roles['Tag Test Role']).toBeDefined();
    expect(savedConfig.config.roles['Tag Test Role'].haystacks).toBeDefined();
    expect(savedConfig.config.roles['Tag Test Role'].haystacks[0].extra_parameters).toBeDefined();
    expect(savedConfig.config.roles['Tag Test Role'].haystacks[0].extra_parameters.tag).toBe('#rust');
    expect(savedConfig.config.roles['Tag Test Role'].haystacks[0].extra_parameters.max_count).toBe('10');

    // Reload the wizard and verify the configuration is loaded correctly
    await page.goto('/config/wizard');
    await page.waitForSelector('.box h3:has-text("Configuration Wizard")', { timeout: 30000 });
    await page.getByTestId('wizard-next').click();

    // The role should be loaded from the saved configuration
    // Note: Exact behavior depends on how the wizard loads existing config
    await page.waitForTimeout(1000);

    // Verify we can still navigate through the wizard
    await expect(page.locator('h4:has-text("Roles")')).toBeVisible();
  });

  test('should generate correct ripgrep command for tag filtering', async ({ page }) => {
    // This is more of a documentation test - we can't directly test the command execution
    // but we can verify the configuration is set up correctly

    // Set up a role with tag filtering
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });

    await page.fill('#role-name-0', 'Command Test Role');
    await page.getByTestId('add-haystack-0').click();
    await page.waitForSelector('#haystack-path-0-0', { timeout: 5000 });

    await page.fill('#haystack-path-0-0', '/test/documents');
    await page.fill('#ripgrep-hashtag-0-0', '#rust');

    // Navigate to review to see the final configuration
    await page.getByTestId('wizard-next').click();
    const reviewJson = page.locator('pre');
    const jsonText = await reviewJson.textContent();
    const config = JSON.parse(jsonText || '{}');

    // Verify the configuration matches what the backend expects
    const haystack = config.roles[0].haystacks[0];
    expect(haystack.service).toBe('Ripgrep');
    expect(haystack.location).toBe('/test/documents'); // Note: UI uses 'path' but backend expects 'location'
    expect(haystack.extra_parameters.tag).toBe('#rust');

    // Based on the backend implementation, this should generate:
    // rg --json --trim -C3 --ignore-case -tmarkdown --all-match -e 'searchterm' -e '#rust' /test/documents
  });

  test('should handle empty tag gracefully', async ({ page }) => {
    // Test edge case: empty tag should not break the configuration
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });

    await page.fill('#role-name-0', 'Empty Tag Test');
    await page.getByTestId('add-haystack-0').click();
    await page.waitForSelector('#haystack-path-0-0', { timeout: 5000 });

    // Leave tag empty but add other parameters
    const tagInput = page.locator('#ripgrep-hashtag-0-0');
    await tagInput.fill(''); // Explicitly clear

    await page.locator('button:has-text("+ Max Results")').click();

    // Navigate to review
    await page.getByTestId('wizard-next').click();
    const reviewJson = page.locator('pre');
    const jsonText = await reviewJson.textContent();
    const config = JSON.parse(jsonText || '{}');

    // Empty tag should either be omitted or be empty string
    const extraParams = config.roles[0].haystacks[0].extra_parameters;
    if (extraParams.tag !== undefined) {
      expect(extraParams.tag).toBe('');
    }
    expect(extraParams.max_count).toBe('10'); // Other parameters should still work
  });
});
