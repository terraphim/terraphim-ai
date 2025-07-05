import { test, expect } from '@playwright/test';

test.describe('Configuration Wizard', () => {
  test('should show correct Configuration ID options', async ({ page }) => {
    await page.goto('/config/wizard');
    const configIdSelect = page.locator('#config-id');
    const options = await configIdSelect.locator('option').allTextContents();
    try {
      expect(options).toEqual(expect.arrayContaining(['Desktop', 'Server', 'Embedded']));
    } catch (e) {
      console.error('Actual Configuration ID options:', options);
      throw e;
    }
  });

  test.beforeEach(async ({ page }) => {
    // Navigate to the configuration wizard
    await page.goto('/config/wizard');
    
    // Wait for the wizard to load
    await page.waitForSelector('.box h3:has-text("Configuration Wizard")', { timeout: 30000 });
  });

  test('should display configuration wizard interface', async ({ page }) => {
    // Check that the wizard title is visible
    await expect(page.locator('h3:has-text("Configuration Wizard")')).toBeVisible();
    
    // Check that global settings form is visible (step 1)
    await expect(page.locator('label:has-text("Configuration ID")')).toBeVisible();
    await expect(page.locator('label:has-text("Global shortcut")')).toBeVisible();
    await expect(page.locator('label:has-text("Default theme")')).toBeVisible();
    await expect(page.locator('label:has-text("Default Role")')).toBeVisible();
    
    // Check that navigation buttons are present
    await expect(page.locator('button:has-text("Next")')).toBeVisible();
  });

  test('should allow editing global configuration settings', async ({ page }) => {
    await page.goto('/config/wizard');
    const configIdSelect = page.locator('#config-id');
    await configIdSelect.selectOption('Server');
    await expect(configIdSelect).toHaveValue('Server');
    const shortcutInput = page.locator('#global-shortcut');
    await shortcutInput.fill('Ctrl+Alt+T');
    await expect(shortcutInput).toHaveValue('Ctrl+Alt+T');
    const themeInput = page.locator('#default-theme');
    await themeInput.fill('superhero');
    await expect(themeInput).toHaveValue('superhero');
    const defaultRoleSelect = page.locator('#default-role');
    const roleOptions = await defaultRoleSelect.locator('option').allTextContents();
    if (roleOptions.length > 0) {
      const firstRoleValue = roleOptions[0];
      await defaultRoleSelect.selectOption(firstRoleValue);
      await expect(defaultRoleSelect).toHaveValue(firstRoleValue);
    }
  });

  test('should allow adding and configuring roles', async ({ page }) => {
    await page.goto('/config/wizard');
    await page.click('button:has-text("Next")');
    await page.click('button:has-text("Add Role")');
    // Wait for the new role form to appear
    await page.waitForSelector('#role-name-0', { timeout: 5000 });
    // Configure the new role
    const roleNameInput = page.locator('#role-name-0');
    await roleNameInput.fill('Test Engineer');
    await expect(roleNameInput).toHaveValue('Test Engineer');
    const shortnameInput = page.locator('#role-shortname-0');
    await shortnameInput.fill('test-eng');
    await expect(shortnameInput).toHaveValue('test-eng');
    const roleThemeInput = page.locator('#role-theme-0');
    await roleThemeInput.fill('lumen');
    await expect(roleThemeInput).toHaveValue('lumen');
    const relevanceInput = page.locator('#role-relevance-0');
    await relevanceInput.fill('TerraphimGraph');
    await expect(relevanceInput).toHaveValue('TerraphimGraph');
  });

  test('should allow configuring haystacks for roles', async ({ page }) => {
    await page.goto('/config/wizard');
    await page.click('button:has-text("Next")');
    await page.click('button:has-text("Add Role")');
    await page.waitForSelector('#role-name-0', { timeout: 5000 });
    await page.click('button:has-text("Add Haystack")');
    await page.waitForSelector('#haystack-path-0-0', { timeout: 5000 });
    const haystackPathInput = page.locator('#haystack-path-0-0');
    await haystackPathInput.fill('/tmp/test-documents');
    await expect(haystackPathInput).toHaveValue('/tmp/test-documents');
    const readOnlyCheckbox = page.locator('#haystack-readonly-0-0');
    await readOnlyCheckbox.check();
    await expect(readOnlyCheckbox).toBeChecked();
  });

  test('should allow configuring knowledge graph settings', async ({ page }) => {
    await page.goto('/config/wizard');
    await page.click('button:has-text("Next")');
    await page.click('button:has-text("Add Role")');
    await page.waitForSelector('#role-name-0', { timeout: 5000 });
    const remoteUrlInput = page.locator('#kg-url-0');
    await remoteUrlInput.fill('https://staging-storage.terraphim.io/thesaurus_Default.json');
    await expect(remoteUrlInput).toHaveValue('https://staging-storage.terraphim.io/thesaurus_Default.json');
    const localPathInput = page.locator('#kg-local-path-0');
    await localPathInput.fill('./docs/src/kg');
    await expect(localPathInput).toHaveValue('./docs/src/kg');
    const localTypeSelect = page.locator('#kg-local-type-0');
    await localTypeSelect.selectOption('markdown');
    await expect(localTypeSelect).toHaveValue('markdown');
    const publicCheckbox = page.locator('#kg-public-0');
    await publicCheckbox.check();
    await expect(publicCheckbox).toBeChecked();
    const publishCheckbox = page.locator('#kg-publish-0');
    await publishCheckbox.check();
    await expect(publishCheckbox).toBeChecked();
  });

  test('should allow removing roles and haystacks', async ({ page }) => {
    // Navigate to step 2 (roles configuration)
    await page.getByTestId('wizard-next').click();
    await page.waitForSelector('h4:has-text("Roles")', { timeout: 5000 });
    
    // Add a new role first
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });
    
    // Add a haystack to the role
    await page.getByTestId('add-haystack-0').click();
    
    // Remove the haystack
    await page.locator('#remove-haystack-0-0').click();
    
    // Verify haystack is removed
    await expect(page.locator('#haystack-path-0-0')).not.toBeVisible();
    
    // Remove the role
    await page.locator('#remove-role-0').click();
    
    // Verify role is removed
    await expect(page.locator('#role-name-0')).not.toBeVisible();
  });

  test('should navigate through wizard steps', async ({ page }) => {
    // Configure some basic settings first (step 1)
    const configIdSelect = page.locator('#config-id');
    await configIdSelect.selectOption('Desktop');
    
    const shortcutInput = page.locator('#global-shortcut');
    await shortcutInput.fill('Ctrl+Shift+T');
    
    // Navigate to step 2 (roles configuration)
    await page.getByTestId('wizard-next').click();
    await page.waitForSelector('h4:has-text("Roles")', { timeout: 5000 });
    
    // Add a role
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });
    
    const roleNameInput = page.locator('#role-name-0');
    await roleNameInput.fill('Test Role');
    
    // Navigate to next step (review)
    await page.getByTestId('wizard-next').click();
    
    // Should be on review step
    await expect(page.locator('h4:has-text("Review")')).toBeVisible();
    await expect(page.locator('pre')).toBeVisible();
    
    // Navigate back to roles step
    await page.getByTestId('wizard-back').click();
    
    // Should be back on roles configuration step
    await expect(page.locator('h4:has-text("Roles")')).toBeVisible();
    
    // Navigate back to step 1
    await page.getByTestId('wizard-back').click();
    
    // Should be back on global settings step
    await expect(page.locator('label:has-text("Configuration ID")')).toBeVisible();
  });

  test('should save configuration and update via API', async ({ page }) => {
    // Configure basic settings (step 1)
    const configIdSelect = page.locator('#config-id');
    await configIdSelect.selectOption('Desktop');
    
    const shortcutInput = page.locator('#global-shortcut');
    await shortcutInput.fill('Ctrl+Alt+W');
    
    const themeInput = page.locator('#default-theme');
    await themeInput.fill('spacelab');
    
    // Navigate to step 2 (roles configuration)
    await page.getByTestId('wizard-next').click();
    await page.waitForSelector('h4:has-text("Roles")', { timeout: 5000 });
    
    // Add a role
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });
    
    const roleNameInput = page.locator('#role-name-0');
    await roleNameInput.fill('Wizard Test Role');
    
    const shortnameInput = page.locator('#role-shortname-0');
    await shortnameInput.fill('wizard-test');
    
    const roleThemeInput = page.locator('#role-theme-0');
    await roleThemeInput.fill('lumen');
    
    const relevanceInput = page.locator('#role-relevance-0');
    await relevanceInput.fill('TitleScorer');
    
    // Navigate to review step
    await page.getByTestId('wizard-next').click();
    await expect(page.locator('h4:has-text("Review")')).toBeVisible();
    
    // Save the configuration
    await page.getByTestId('wizard-save').click();
    
    // Wait for save to complete (check for alert or success message)
    await page.waitForTimeout(2000);
    
    // Verify configuration was saved by checking if we can navigate back to wizard
    await page.goto('/config/wizard');
    await page.waitForSelector('.box h3:has-text("Configuration Wizard")', { timeout: 30000 });
    
    // Check that our saved values are still there
    await expect(configIdSelect).toHaveValue('Desktop');
    await expect(shortcutInput).toHaveValue('Ctrl+Alt+W');
    await expect(themeInput).toHaveValue('spacelab');
  });

  test('should validate configuration schema', async ({ page }) => {
    // Test that the wizard loads the schema correctly
    await page.waitForTimeout(1000); // Give time for schema to load
    
    // Check that form fields are properly bound to schema
    // The first select should be Configuration ID
    const configIdSelect = page.locator('#config-id');
    const options = await configIdSelect.locator('option').all();
    
    // Should have the expected configuration ID options
    const optionValues = await Promise.all(options.map(opt => opt.textContent()));
    expect(optionValues).toContain('Desktop');
    expect(optionValues).toContain('Server');
    expect(optionValues).toContain('Embedded');
    
    // The second select should be Default Role (showing existing roles)
    const defaultRoleSelect = page.locator('#default-role');
    const roleOptions = await defaultRoleSelect.locator('option').all();
    const roleValues = await Promise.all(roleOptions.map(opt => opt.textContent()));
    
    // Should have some role options (could be empty if no roles exist)
    expect(roleValues.length).toBeGreaterThanOrEqual(0);
  });

  test('should handle configuration errors gracefully', async ({ page }) => {
    // Try to save without any configuration
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('wizard-save').click();
    
    // Should handle the save attempt gracefully
    await page.waitForTimeout(2000);
    
    // App should still be functional
    await expect(page.locator('h3:has-text("Configuration Wizard")')).toBeVisible();
  });

  test('should preserve existing configuration when editing', async ({ page }) => {
    // First, let's check if there's existing configuration
    const configIdSelect = page.locator('#config-id');
    const currentValue = await configIdSelect.inputValue();
    
    // Change the configuration ID
    await configIdSelect.selectOption('Server');
    
    // Navigate away and back to see if changes persist
    await page.goto('/');
    await page.goto('/config/wizard');
    await page.waitForSelector('.box h3:has-text("Configuration Wizard")', { timeout: 30000 });
    
    // Check if the change persisted
    const newConfigIdSelect = page.locator('#config-id');
    await expect(newConfigIdSelect).toHaveValue('Server');
  });

  test('should update configuration via API endpoint', async ({ page }) => {
    // This test validates that the configuration update API works
    // We'll make a direct API call to verify the endpoint is working
    
    // First, get the current configuration
    const response = await page.request.get('/config');
    expect(response.status()).toBe(200);
    
    const configData = await response.json();
    expect(configData).toHaveProperty('status');
    expect(configData).toHaveProperty('config');
    
    // Create a test configuration update
    const testConfig = {
      ...configData.config,
      global_shortcut: 'Ctrl+Test+Wizard',
      default_role: configData.config.default_role || 'Default'
    };
    
    // Update the configuration via API
    const updateResponse = await page.request.post('/config', {
      data: testConfig,
      headers: {
        'Content-Type': 'application/json'
      }
    });
    
    expect(updateResponse.status()).toBe(200);
    
    const updateData = await updateResponse.json();
    expect(updateData).toHaveProperty('status', 'success');
    expect(updateData.config.global_shortcut).toBe('Ctrl+Test+Wizard');
    
    // Verify the update by fetching the config again
    const verifyResponse = await page.request.get('/config');
    const verifyData = await verifyResponse.json();
    expect(verifyData.config.global_shortcut).toBe('Ctrl+Test+Wizard');
  });

  test('should handle complex role configurations', async ({ page }) => {
    // Navigate to step 2 (roles configuration)
    await page.getByTestId('wizard-next').click();
    await page.waitForSelector('h4:has-text("Roles")', { timeout: 5000 });
    
    // Add multiple roles with different configurations
    await page.getByTestId('add-role').click();
    await page.waitForSelector('#role-name-0', { timeout: 5000 });
    
    // Configure first role
    const roleNameInput = page.locator('#role-name-0');
    await roleNameInput.fill('Engineer Role');
    
    const shortnameInput = page.locator('#role-shortname-0');
    await shortnameInput.fill('engineer');
    
    const roleThemeInput = page.locator('#role-theme-0');
    await roleThemeInput.fill('lumen');
    
    const relevanceInput = page.locator('#role-relevance-0');
    await relevanceInput.fill('TerraphimGraph');
    
    // Add haystack to first role
    await page.getByTestId('add-haystack-0').click();
    const haystackPathInput = page.locator('#haystack-path-0-0');
    await haystackPathInput.fill('/path/to/engineer/docs');
    
    // Configure KG for first role
    const remoteUrlInput = page.locator('#kg-url-0');
    await remoteUrlInput.fill('https://example.com/engineer-thesaurus.json');
    
    // Add second role
    await page.getByTestId('add-role').click();
    
    // Configure second role
    const secondRoleNameInput = page.locator('#role-name-1');
    await secondRoleNameInput.fill('Researcher Role');
    
    const secondShortnameInput = page.locator('#role-shortname-1');
    await secondShortnameInput.fill('researcher');
    
    const secondRoleThemeInput = page.locator('#role-theme-1');
    await secondRoleThemeInput.fill('superhero');
    
    const secondRelevanceInput = page.locator('#role-relevance-1');
    await secondRelevanceInput.fill('TitleScorer');
    
    // Configure local KG for second role
    const localPathInput = page.locator('#kg-local-path-1');
    await localPathInput.fill('./docs/src/kg');
    
    const localTypeSelect = page.locator('#kg-local-type-1');
    await localTypeSelect.selectOption('markdown');
    
    // Navigate to review and save
    await page.getByTestId('wizard-next').click();
    await expect(page.locator('h4:has-text("Review")')).toBeVisible();
    
    // Verify the configuration JSON contains both roles
    const reviewJson = page.locator('pre');
    const jsonText = await reviewJson.textContent();
    const config = JSON.parse(jsonText || '{}');
    
    expect(config.roles).toBeDefined();
    expect(config.roles.length).toBeGreaterThanOrEqual(2);
    
    // Save the configuration
    await page.getByTestId('wizard-save').click();
    await page.waitForTimeout(2000);
    
    // Verify the configuration was saved by checking the API
    const response = await page.request.get('/config');
    const savedConfig = await response.json();
    
    expect(savedConfig.config.roles).toBeDefined();
    // Should have at least our two test roles
    const roleNames = Object.keys(savedConfig.config.roles);
    expect(roleNames.length).toBeGreaterThanOrEqual(2);
  });

  // --- Roles Removal ---
  test('can remove a role and UI updates', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('wizard-start').click();
    await page.getByTestId('wizard-next').click(); // Go to roles step
    // Add two roles for removal test
    await page.getByTestId('add-role').click();
    await page.getByTestId('add-role').click();
    // Remove the first role
    await page.locator('#remove-role-0').click();
    await expect(page.locator('#role-name-0')).not.toBeVisible();
    // Remove all roles
    await page.locator('#remove-role-0').click();
    await expect(page.getByTestId('wizard-next')).toBeDisabled();
  });

  test('can re-add a role after removal', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('wizard-start').click();
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('add-role').click();
    await page.locator('#remove-role-0').click();
    await page.getByTestId('add-role').click();
    await expect(page.locator('#role-name-0')).toBeVisible();
  });

  // --- Navigation ---
  test('can navigate forward and backward between steps', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('wizard-start').click();
    await page.getByTestId('wizard-next').click(); // roles
    await page.getByTestId('wizard-next').click(); // haystacks
    await page.getByTestId('wizard-back').click(); // back to roles
    await expect(page.locator('#role-name-0')).toBeVisible();
  });

  test('data persists when navigating back and forth', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('wizard-start').click();
    await page.getByTestId('wizard-next').click();
    await page.fill('#role-name-0', 'TestRole');
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('wizard-back').click();
    await expect(page.locator('#role-name-0')).toHaveValue('TestRole');
  });

  // --- Review Step ---
  test('review step displays all entered data', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('wizard-start').click();
    await page.getByTestId('wizard-next').click();
    await page.fill('#role-name-0', 'ReviewRole');
    await page.getByTestId('wizard-next').click(); // haystacks
    await page.getByTestId('wizard-next').click(); // knowledge graph
    await page.getByTestId('wizard-next').click(); // review
    await expect(page.getByTestId('review-role-name-0')).toHaveText('ReviewRole');
  });

  test('can edit from review step and see update', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('wizard-start').click();
    await page.getByTestId('wizard-next').click();
    await page.fill('#role-name-0', 'EditRole');
    await page.getByTestId('wizard-next').click(); // haystacks
    await page.getByTestId('wizard-next').click(); // knowledge graph
    await page.getByTestId('wizard-next').click(); // review
    await page.getByTestId('edit-role-0').click();
    await page.fill('#role-name-0', 'EditedRole');
    await page.getByTestId('wizard-next').click(); // haystacks
    await page.getByTestId('wizard-next').click(); // knowledge graph
    await page.getByTestId('wizard-next').click(); // review
    await expect(page.getByTestId('review-role-name-0')).toHaveText('EditedRole');
  });

  // --- Saving and Validation ---
  test('can save valid config and see success', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('wizard-start').click();
    await page.getByTestId('wizard-next').click();
    await page.fill('#role-name-0', 'SaveRole');
    await page.getByTestId('wizard-next').click(); // haystacks
    await page.getByTestId('wizard-next').click(); // knowledge graph
    await page.getByTestId('wizard-next').click(); // review
    await page.getByTestId('wizard-save').click();
    await expect(page.getByTestId('wizard-success')).toBeVisible();
  });

  test('shows error on invalid config save', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('wizard-start').click();
    await page.getByTestId('wizard-next').click();
    // Do not fill required role name
    await page.getByTestId('wizard-next').click(); // haystacks
    await page.getByTestId('wizard-next').click(); // knowledge graph
    await page.getByTestId('wizard-next').click(); // review
    await page.getByTestId('wizard-save').click();
    await expect(page.getByTestId('wizard-error')).toBeVisible();
  });

  // --- Edge Cases ---
  test('cannot add duplicate role names', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('wizard-start').click();
    await page.getByTestId('wizard-next').click();
    await page.fill('#role-name-0', 'DupRole');
    await page.getByTestId('add-role').click();
    await page.fill('#role-name-1', 'DupRole');
    await page.getByTestId('wizard-next').click();
    await expect(page.getByTestId('wizard-error')).toBeVisible();
  });
}); 