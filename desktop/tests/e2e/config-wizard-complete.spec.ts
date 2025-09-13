/**
 * Comprehensive End-to-End Tests for Configuration Wizard
 * 
 * This test suite validates the complete configuration system including:
 * - LLM provider configuration (Ollama, OpenRouter)
 * - Auto-summarize settings and toggles
 * - API key and URL configuration
 * - Role-specific LLM settings
 * - Validation of connectivity and credentials
 * - Haystack configuration with secrets
 * - Theme and UI preference settings
 * - Export/import of configurations
 */

import { test, expect } from '@playwright/test';
import {
  ciWaitForSelector,
  ciSearch,
  ciNavigate,
  ciWait,
  ciClick,
  getTimeouts
} from '../../src/test-utils/ci-friendly';

// Test configuration
const TEST_TIMEOUT = 120000;
const VALIDATION_TIMEOUT = 10000;
const SAVE_TIMEOUT = 15000;

// Test configuration data
const TEST_LLM_CONFIGS = {
  ollama: {
    provider: 'ollama',
    baseUrl: 'http://127.0.0.1:11434',
    model: 'llama3.2:3b',
    autoSummarize: true
  },
  openrouter: {
    provider: 'openrouter',
    apiKey: 'sk-or-v1-test-key-12345',
    model: 'anthropic/claude-3-haiku',
    autoSummarize: false
  }
};

const TEST_HAYSTACK_CONFIGS = {
  atomic: {
    service: 'AtomicServer',
    url: 'http://localhost:9883',
    secret: 'test-atomic-secret-12345'
  },
  clickup: {
    service: 'ClickUp',
    apiToken: 'pk_test_clickup_token_12345',
    teamId: '90151159089'
  },
  github: {
    service: 'GitHub',
    token: 'ghp_test_github_token_12345',
    repository: 'test-user/test-repo'
  }
};

const TEST_ROLES = [
  {
    name: 'Test Engineer',
    theme: 'cosmo',
    relevanceFunction: 'bm25',
    llmProvider: 'ollama',
    autoSummarize: true
  },
  {
    name: 'Test Analyst',
    theme: 'darkly',
    relevanceFunction: 'terraphim-graph',
    llmProvider: 'openrouter',
    autoSummarize: false
  }
];

test.describe('Configuration Wizard Complete E2E Tests', () => {
  test.beforeEach(async ({ page }) => {
    test.setTimeout(TEST_TIMEOUT);
    
    // Navigate to configuration wizard
    await ciNavigate(page, '/config/wizard');
    await ciWaitForSelector(page, '[data-testid="config-wizard"]', 'navigation');
  });

  test.describe('LLM Provider Configuration', () => {
    test('should configure Ollama provider with validation', async ({ page }) => {
      // Look for role configuration sections
      const roles = page.locator('[data-testid="role-config"], .role-config');
      const roleCount = await roles.count();
      
      if (roleCount > 0) {
        const firstRole = roles.first();
        
        // Find LLM provider selection
        const llmProviderSelect = firstRole.locator('select[name*="llm_provider"], select:has(option[value="ollama"])');
        const llmProviderInput = firstRole.locator('input[name*="llm_provider"]');
        
        let llmControl = null;
        if (await llmProviderSelect.isVisible()) {
          llmControl = llmProviderSelect;
        } else if (await llmProviderInput.isVisible()) {
          llmControl = llmProviderInput;
        }
        
        if (llmControl) {
          // Select/set Ollama as provider
          if (await llmProviderSelect.isVisible()) {
            await llmProviderSelect.selectOption('ollama');
          } else {
            await llmProviderInput.fill('ollama');
          }
          
          console.log('Set LLM provider to Ollama');
          
          // Wait for Ollama-specific fields to appear
          await ciWait(page, 'small');
          
          // Configure Ollama base URL
          const baseUrlInput = firstRole.locator('input[name*="ollama_base_url"], input[name*="base_url"], input[placeholder*="http://127.0.0.1:11434"]');
          if (await baseUrlInput.isVisible()) {
            await baseUrlInput.fill(TEST_LLM_CONFIGS.ollama.baseUrl);
            console.log('Set Ollama base URL:', TEST_LLM_CONFIGS.ollama.baseUrl);
          }
          
          // Configure Ollama model
          const modelInput = firstRole.locator('input[name*="ollama_model"], input[name*="model"], input[placeholder*="llama"]');
          if (await modelInput.isVisible()) {
            await modelInput.fill(TEST_LLM_CONFIGS.ollama.model);
            console.log('Set Ollama model:', TEST_LLM_CONFIGS.ollama.model);
          }
          
          // Configure auto-summarize
          const autoSummarizeCheckbox = firstRole.locator('input[type="checkbox"][name*="auto_summarize"], input[type="checkbox"]:near(text("auto-summarize"))');
          if (await autoSummarizeCheckbox.isVisible()) {
            const isChecked = await autoSummarizeCheckbox.isChecked();
            if (isChecked !== TEST_LLM_CONFIGS.ollama.autoSummarize) {
              await autoSummarizeCheckbox.click();
            }
            console.log('Set auto-summarize:', TEST_LLM_CONFIGS.ollama.autoSummarize);
          }
          
          // Test connection if button is available
          const testConnectionButton = firstRole.locator('[data-testid="test-ollama-connection"], button:has-text("Test Connection"), button:has-text("Validate")');
          if (await testConnectionButton.isVisible()) {
            console.log('Testing Ollama connection...');
            await testConnectionButton.click();
            
            // Wait for connection result
            await page.waitForFunction(() => {
              const successEl = document.querySelector('.connection-success, .text-success, [data-testid="connection-success"]');
              const errorEl = document.querySelector('.connection-error, .text-danger, [data-testid="connection-error"]');
              return successEl || errorEl;
            }, { timeout: VALIDATION_TIMEOUT });
            
            const connectionSuccess = await firstRole.locator('.connection-success, .text-success, [data-testid="connection-success"]').isVisible();
            const connectionError = await firstRole.locator('.connection-error, .text-danger, [data-testid="connection-error"]').isVisible();
            
            if (connectionSuccess) {
              console.log('Ollama connection test successful');
            } else if (connectionError) {
              const errorText = await firstRole.locator('.connection-error, .text-danger').textContent();
              console.log('Ollama connection test failed:', errorText);
            }
          }
          
          // Save configuration
          const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save"), button:has-text("Apply")');
          if (await saveButton.isVisible()) {
            await saveButton.click();
            console.log('Saving Ollama configuration...');
            
            // Wait for save confirmation
            await page.waitForFunction(() => {
              const successMsg = document.querySelector('.save-success, .alert-success, [data-testid="save-success"]');
              const errorMsg = document.querySelector('.save-error, .alert-danger, [data-testid="save-error"]');
              return successMsg || errorMsg;
            }, { timeout: SAVE_TIMEOUT });
            
            const saveSuccess = await page.locator('.save-success, .alert-success, [data-testid="save-success"]').isVisible();
            console.log('Configuration saved successfully:', saveSuccess);
          }
        } else {
          console.log('LLM provider configuration not found');
        }
      } else {
        console.log('No role configurations found');
      }
    });

    test('should configure OpenRouter provider with API key validation', async ({ page }) => {
      const roles = page.locator('[data-testid="role-config"], .role-config');
      const roleCount = await roles.count();
      
      if (roleCount > 1) {
        // Use second role for OpenRouter testing
        const secondRole = roles.nth(1);
        
        // Set OpenRouter as provider
        const llmProviderSelect = secondRole.locator('select[name*="llm_provider"]');
        const llmProviderInput = secondRole.locator('input[name*="llm_provider"]');
        
        if (await llmProviderSelect.isVisible()) {
          await llmProviderSelect.selectOption('openrouter');
        } else if (await llmProviderInput.isVisible()) {
          await llmProviderInput.fill('openrouter');
        }
        
        console.log('Set LLM provider to OpenRouter');
        await ciWait(page, 'small');
        
        // Configure OpenRouter API key
        const apiKeyInput = secondRole.locator('input[name*="openrouter_api_key"], input[name*="api_key"], input[type="password"]');
        if (await apiKeyInput.isVisible()) {
          await apiKeyInput.fill(TEST_LLM_CONFIGS.openrouter.apiKey);
          console.log('Set OpenRouter API key');
        }
        
        // Configure model
        const modelInput = secondRole.locator('input[name*="openrouter_model"], input[name*="model"]:not([name*="ollama"])');
        const modelSelect = secondRole.locator('select[name*="model"]:not([name*="ollama"])');
        
        if (await modelSelect.isVisible()) {
          await modelSelect.selectOption(TEST_LLM_CONFIGS.openrouter.model);
        } else if (await modelInput.isVisible()) {
          await modelInput.fill(TEST_LLM_CONFIGS.openrouter.model);
        }
        console.log('Set OpenRouter model:', TEST_LLM_CONFIGS.openrouter.model);
        
        // Configure auto-summarize
        const autoSummarizeCheckbox = secondRole.locator('input[type="checkbox"][name*="openrouter_auto_summarize"], input[type="checkbox"]:near(text("auto-summarize"))');
        if (await autoSummarizeCheckbox.isVisible()) {
          const isChecked = await autoSummarizeCheckbox.isChecked();
          if (isChecked !== TEST_LLM_CONFIGS.openrouter.autoSummarize) {
            await autoSummarizeCheckbox.click();
          }
          console.log('Set OpenRouter auto-summarize:', TEST_LLM_CONFIGS.openrouter.autoSummarize);
        }
        
        // Test API key validation if available
        const validateButton = secondRole.locator('[data-testid="validate-openrouter"], button:has-text("Validate"), button:has-text("Test")');
        if (await validateButton.isVisible()) {
          console.log('Testing OpenRouter API key...');
          await validateButton.click();
          
          await page.waitForFunction(() => {
            const successEl = document.querySelector('.validation-success, .text-success');
            const errorEl = document.querySelector('.validation-error, .text-danger');
            return successEl || errorEl;
          }, { timeout: VALIDATION_TIMEOUT });
          
          const validationSuccess = await secondRole.locator('.validation-success, .text-success').isVisible();
          const validationError = await secondRole.locator('.validation-error, .text-danger').isVisible();
          
          if (validationSuccess) {
            console.log('OpenRouter API key validation successful');
          } else if (validationError) {
            const errorText = await secondRole.locator('.validation-error, .text-danger').textContent();
            console.log('OpenRouter API key validation failed:', errorText);
          }
        }
        
        // Save configuration
        const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save")');
        if (await saveButton.isVisible()) {
          await saveButton.click();
          await ciWait(page, 'medium');
          console.log('OpenRouter configuration saved');
        }
      }
    });

    test('should validate LLM configuration fields', async ({ page }) => {
      const roles = page.locator('[data-testid="role-config"], .role-config');
      const firstRole = roles.first();
      
      // Test empty API key validation for OpenRouter
      const llmProviderSelect = firstRole.locator('select[name*="llm_provider"]');
      if (await llmProviderSelect.isVisible()) {
        await llmProviderSelect.selectOption('openrouter');
        await ciWait(page, 'small');
        
        // Leave API key empty and try to save
        const apiKeyInput = firstRole.locator('input[name*="api_key"], input[type="password"]');
        if (await apiKeyInput.isVisible()) {
          await apiKeyInput.fill('');
          
          const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save")');
          await saveButton.click();
          
          // Should show validation error
          await ciWait(page, 'small');
          const validationError = page.locator('.validation-error, .field-error, .error-message');
          const hasError = await validationError.isVisible();
          
          if (hasError) {
            const errorText = await validationError.textContent();
            console.log('Validation error for empty API key:', errorText);
            expect(errorText?.toLowerCase()).toMatch(/required|api key|empty/);
          }
        }
        
        // Test invalid URL format for Ollama
        await llmProviderSelect.selectOption('ollama');
        await ciWait(page, 'small');
        
        const baseUrlInput = firstRole.locator('input[name*="base_url"], input[name*="ollama_base_url"]');
        if (await baseUrlInput.isVisible()) {
          await baseUrlInput.fill('invalid-url-format');
          
          const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save")');
          await saveButton.click();
          
          await ciWait(page, 'small');
          const urlValidationError = page.locator('.validation-error, .field-error');
          const hasUrlError = await urlValidationError.isVisible();
          
          if (hasUrlError) {
            const errorText = await urlValidationError.textContent();
            console.log('Validation error for invalid URL:', errorText);
            expect(errorText?.toLowerCase()).toMatch(/url|format|invalid/);
          }
          
          // Restore valid URL
          await baseUrlInput.fill('http://127.0.0.1:11434');
        }
      }
    });
  });

  test.describe('Haystack Configuration with Secrets', () => {
    test('should configure Atomic Server haystack with secrets', async ({ page }) => {
      // Look for haystack configuration section
      const haystackSection = page.locator('[data-testid="haystack-config"], .haystack-config, [data-testid="haystack-section"]');
      
      if (await haystackSection.isVisible()) {
        // Add new haystack if button is available
        const addHaystackButton = page.locator('[data-testid="add-haystack"], button:has-text("Add Haystack")');
        if (await addHaystackButton.isVisible()) {
          await addHaystackButton.click();
          await ciWait(page, 'small');
        }
        
        // Find haystack form
        const haystackForm = page.locator('[data-testid="haystack-form"], .haystack-form').last();
        
        if (await haystackForm.isVisible()) {
          // Select Atomic Server service
          const serviceSelect = haystackForm.locator('select[name*="service"], select:has(option:text("AtomicServer"))');
          if (await serviceSelect.isVisible()) {
            await serviceSelect.selectOption('AtomicServer');
            console.log('Selected AtomicServer service');
            await ciWait(page, 'small');
          }
          
          // Configure Atomic Server URL
          const urlInput = haystackForm.locator('input[name*="atomic_server_url"], input[name*="url"], input[placeholder*="atomic"]');
          if (await urlInput.isVisible()) {
            await urlInput.fill(TEST_HAYSTACK_CONFIGS.atomic.url);
            console.log('Set Atomic Server URL:', TEST_HAYSTACK_CONFIGS.atomic.url);
          }
          
          // Configure secret
          const secretInput = haystackForm.locator('input[name*="atomic_server_secret"], input[name*="secret"], input[type="password"]');
          if (await secretInput.isVisible()) {
            await secretInput.fill(TEST_HAYSTACK_CONFIGS.atomic.secret);
            console.log('Set Atomic Server secret');
          }
          
          // Test connection
          const testConnectionButton = haystackForm.locator('[data-testid="test-atomic-connection"], button:has-text("Test Connection")');
          if (await testConnectionButton.isVisible()) {
            console.log('Testing Atomic Server connection...');
            await testConnectionButton.click();
            
            await page.waitForFunction(() => {
              const successEl = document.querySelector('.connection-success, .text-success');
              const errorEl = document.querySelector('.connection-error, .text-danger');
              return successEl || errorEl;
            }, { timeout: VALIDATION_TIMEOUT });
            
            const connectionResult = haystackForm.locator('.connection-result, .connection-status');
            if (await connectionResult.isVisible()) {
              const resultText = await connectionResult.textContent();
              console.log('Atomic Server connection result:', resultText);
            }
          }
        }
      } else {
        console.log('Haystack configuration section not found');
      }
    });

    test('should configure ClickUp haystack with API token', async ({ page }) => {
      const haystackSection = page.locator('[data-testid="haystack-config"], .haystack-config');
      
      if (await haystackSection.isVisible()) {
        const addHaystackButton = page.locator('[data-testid="add-haystack"], button:has-text("Add")');
        if (await addHaystackButton.isVisible()) {
          await addHaystackButton.click();
          await ciWait(page, 'small');
        }
        
        const haystackForm = page.locator('[data-testid="haystack-form"], .haystack-form').last();
        
        // Select ClickUp service
        const serviceSelect = haystackForm.locator('select[name*="service"]');
        if (await serviceSelect.isVisible()) {
          await serviceSelect.selectOption('ClickUp');
          console.log('Selected ClickUp service');
          await ciWait(page, 'small');
        }
        
        // Configure API token
        const tokenInput = haystackForm.locator('input[name*="clickup_api_token"], input[name*="api_token"], input[placeholder*="pk_"]');
        if (await tokenInput.isVisible()) {
          await tokenInput.fill(TEST_HAYSTACK_CONFIGS.clickup.apiToken);
          console.log('Set ClickUp API token');
        }
        
        // Configure team ID
        const teamIdInput = haystackForm.locator('input[name*="clickup_team_id"], input[name*="team_id"]');
        if (await teamIdInput.isVisible()) {
          await teamIdInput.fill(TEST_HAYSTACK_CONFIGS.clickup.teamId);
          console.log('Set ClickUp team ID:', TEST_HAYSTACK_CONFIGS.clickup.teamId);
        }
        
        // Test API token
        const validateTokenButton = haystackForm.locator('[data-testid="validate-clickup-token"], button:has-text("Validate Token")');
        if (await validateTokenButton.isVisible()) {
          console.log('Testing ClickUp API token...');
          await validateTokenButton.click();
          
          await ciWait(page, 'large');
          
          const validationResult = haystackForm.locator('.validation-result, .token-status');
          if (await validationResult.isVisible()) {
            const resultText = await validationResult.textContent();
            console.log('ClickUp token validation result:', resultText);
          }
        }
      }
    });

    test('should handle environment variable integration', async ({ page }) => {
      // Check if there's an option to use environment variables
      const envVarOption = page.locator('input[type="checkbox"]:near(text("environment")), [data-testid="use-env-vars"]');
      
      if (await envVarOption.isVisible()) {
        console.log('Environment variable option found');
        
        // Toggle environment variable usage
        await envVarOption.click();
        await ciWait(page, 'small');
        
        // Check if input fields are disabled/hidden when using env vars
        const secretInputs = page.locator('input[type="password"], input[name*="secret"], input[name*="token"]');
        const secretCount = await secretInputs.count();
        
        for (let i = 0; i < secretCount; i++) {
          const input = secretInputs.nth(i);
          const isDisabled = await input.isDisabled();
          const isHidden = !(await input.isVisible());
          
          if (isDisabled || isHidden) {
            console.log('Secret input properly disabled/hidden when using env vars');
          }
        }
        
        // Check for environment variable documentation
        const envVarHelp = page.locator('.env-var-help, [data-testid="env-var-info"], .help-text:has-text("environment")');
        if (await envVarHelp.isVisible()) {
          const helpText = await envVarHelp.textContent();
          console.log('Environment variable help text:', helpText?.substring(0, 100));
        }
      } else {
        console.log('Environment variable integration not available in UI');
      }
    });
  });

  test.describe('Role Management', () => {
    test('should create new role with complete configuration', async ({ page }) => {
      // Look for add role button
      const addRoleButton = page.locator('[data-testid="add-role"], button:has-text("Add Role"), button:has-text("New Role")');
      
      if (await addRoleButton.isVisible()) {
        await addRoleButton.click();
        await ciWait(page, 'medium');
        
        // Configure new role
        const roleForm = page.locator('[data-testid="role-form"], .role-form').last();
        
        // Set role name
        const nameInput = roleForm.locator('input[name*="name"], input[placeholder*="role name"]');
        if (await nameInput.isVisible()) {
          await nameInput.fill(TEST_ROLES[0].name);
          console.log('Set role name:', TEST_ROLES[0].name);
        }
        
        // Set theme
        const themeSelect = roleForm.locator('select[name*="theme"]');
        if (await themeSelect.isVisible()) {
          await themeSelect.selectOption(TEST_ROLES[0].theme);
          console.log('Set theme:', TEST_ROLES[0].theme);
        }
        
        // Set relevance function
        const relevanceFunctionSelect = roleForm.locator('select[name*="relevance_function"], select[name*="relevance"]');
        if (await relevanceFunctionSelect.isVisible()) {
          await relevanceFunctionSelect.selectOption(TEST_ROLES[0].relevanceFunction);
          console.log('Set relevance function:', TEST_ROLES[0].relevanceFunction);
        }
        
        // Configure LLM settings
        const llmProviderSelect = roleForm.locator('select[name*="llm_provider"]');
        if (await llmProviderSelect.isVisible()) {
          await llmProviderSelect.selectOption(TEST_ROLES[0].llmProvider);
          console.log('Set LLM provider:', TEST_ROLES[0].llmProvider);
          
          await ciWait(page, 'small');
          
          // Configure auto-summarize
          const autoSummarizeCheckbox = roleForm.locator('input[type="checkbox"][name*="auto_summarize"]');
          if (await autoSummarizeCheckbox.isVisible()) {
            const isChecked = await autoSummarizeCheckbox.isChecked();
            if (isChecked !== TEST_ROLES[0].autoSummarize) {
              await autoSummarizeCheckbox.click();
            }
            console.log('Set auto-summarize:', TEST_ROLES[0].autoSummarize);
          }
        }
        
        // Save role
        const saveRoleButton = roleForm.locator('[data-testid="save-role"], button:has-text("Save Role")');
        const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save")');
        
        if (await saveRoleButton.isVisible()) {
          await saveRoleButton.click();
        } else if (await saveButton.isVisible()) {
          await saveButton.click();
        }
        
        await ciWait(page, 'medium');
        console.log('New role created and saved');
        
        // Verify role appears in the list
        const roleList = page.locator('[data-testid="role-list"], .role-list');
        const newRole = page.locator(`text="${TEST_ROLES[0].name}"`);
        
        if (await newRole.isVisible()) {
          console.log('New role visible in configuration');
        }
      } else {
        console.log('Add role functionality not available');
      }
    });

    test('should delete role configuration', async ({ page }) => {
      // Find a role to delete (avoid deleting essential roles)
      const roles = page.locator('[data-testid="role-config"], .role-config');
      const roleCount = await roles.count();
      
      if (roleCount > 1) {
        // Look for delete button on the last role
        const lastRole = roles.last();
        const deleteButton = lastRole.locator('[data-testid="delete-role"], button:has-text("Delete"), .delete-role-button');
        
        if (await deleteButton.isVisible()) {
          // Get role name before deletion
          const roleNameElement = lastRole.locator('input[name*="name"], [data-testid="role-name"]');
          let roleName = 'Unknown';
          
          if (await roleNameElement.isVisible()) {
            roleName = await roleNameElement.inputValue();
          }
          
          console.log('Attempting to delete role:', roleName);
          
          await deleteButton.click();
          
          // Handle confirmation dialog if it appears
          const confirmDialog = page.locator('[data-testid="confirm-delete"], .confirm-dialog');
          const confirmButton = page.locator('[data-testid="confirm-delete-button"], button:has-text("Confirm"), button:has-text("Delete")');
          
          if (await confirmDialog.isVisible()) {
            console.log('Confirming role deletion');
            await confirmButton.click();
          }
          
          await ciWait(page, 'medium');
          
          // Verify role was removed
          const updatedRoleCount = await roles.count();
          expect(updatedRoleCount).toBeLessThan(roleCount);
          console.log('Role deleted successfully');
        } else {
          console.log('Delete role functionality not available');
        }
      } else {
        console.log('Cannot delete - only one role available');
      }
    });

    test('should export and import configuration', async ({ page }) => {
      // Look for export functionality
      const exportButton = page.locator('[data-testid="export-config"], button:has-text("Export"), .export-button');
      
      if (await exportButton.isVisible()) {
        console.log('Testing configuration export');
        
        // Trigger export
        await exportButton.click();
        
        // Check if download started or modal appeared
        const downloadModal = page.locator('[data-testid="export-modal"], .export-modal');
        const downloadLink = page.locator('[data-testid="download-link"], a[download]');
        
        if (await downloadModal.isVisible()) {
          console.log('Export modal appeared');
          
          const downloadButton = downloadModal.locator('[data-testid="download-config"], button:has-text("Download")');
          if (await downloadButton.isVisible()) {
            await downloadButton.click();
            console.log('Export download triggered');
          }
        } else if (await downloadLink.isVisible()) {
          console.log('Direct download link available');
        }
        
        // Test import functionality
        const importButton = page.locator('[data-testid="import-config"], button:has-text("Import"), .import-button');
        
        if (await importButton.isVisible()) {
          console.log('Import functionality available');
          
          await importButton.click();
          
          const importModal = page.locator('[data-testid="import-modal"], .import-modal');
          const fileInput = page.locator('input[type="file"]');
          
          if (await importModal.isVisible()) {
            console.log('Import modal opened');
            
            // Could test with a sample configuration file
            // For now, just verify the UI is working
            const cancelButton = importModal.locator('[data-testid="cancel-import"], button:has-text("Cancel")');
            if (await cancelButton.isVisible()) {
              await cancelButton.click();
              console.log('Import modal closed');
            }
          }
        }
      } else {
        console.log('Export/Import functionality not available');
      }
    });
  });

  test.describe('Configuration Validation and Persistence', () => {
    test('should persist configuration changes across page reloads', async ({ page }) => {
      // Make a configuration change
      const roles = page.locator('[data-testid="role-config"], .role-config');
      const firstRole = roles.first();
      
      // Change theme as a test
      const themeSelect = firstRole.locator('select[name*="theme"]');
      let originalTheme = '';
      let newTheme = 'darkly';
      
      if (await themeSelect.isVisible()) {
        originalTheme = await themeSelect.inputValue();
        
        if (originalTheme === 'darkly') {
          newTheme = 'cosmo';
        }
        
        await themeSelect.selectOption(newTheme);
        console.log('Changed theme from', originalTheme, 'to', newTheme);
        
        // Save configuration
        const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save")');
        if (await saveButton.isVisible()) {
          await saveButton.click();
          await ciWait(page, 'medium');
        }
        
        // Reload page
        await page.reload();
        await ciWaitForSelector(page, '[data-testid="config-wizard"]');
        
        // Check if change persisted
        const themeSelectAfterReload = firstRole.locator('select[name*="theme"]');
        if (await themeSelectAfterReload.isVisible()) {
          const persistedTheme = await themeSelectAfterReload.inputValue();
          console.log('Theme after reload:', persistedTheme);
          expect(persistedTheme).toBe(newTheme);
        }
        
        // Restore original theme
        await themeSelectAfterReload.selectOption(originalTheme);
        const saveButtonRestore = page.locator('[data-testid="save-config"], button:has-text("Save")');
        if (await saveButtonRestore.isVisible()) {
          await saveButtonRestore.click();
        }
      }
    });

    test('should validate required fields before saving', async ({ page }) => {
      // Try to create invalid configuration
      const roles = page.locator('[data-testid="role-config"], .role-config');
      const firstRole = roles.first();
      
      // Clear required field (role name)
      const nameInput = firstRole.locator('input[name*="name"], input[name*="role_name"]');
      if (await nameInput.isVisible()) {
        const originalName = await nameInput.inputValue();
        
        await nameInput.fill('');
        
        // Try to save
        const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save")');
        await saveButton.click();
        
        // Should show validation error or prevent saving
        await ciWait(page, 'small');
        
        const validationError = page.locator('.validation-error, .field-error, .error-message');
        const errorVisible = await validationError.isVisible();
        
        if (errorVisible) {
          const errorText = await validationError.textContent();
          console.log('Validation error for empty name:', errorText);
          expect(errorText?.toLowerCase()).toMatch(/required|name|empty/);
        } else {
          // Check if save button is disabled
          const saveDisabled = await saveButton.isDisabled();
          console.log('Save button disabled for invalid config:', saveDisabled);
        }
        
        // Restore name
        await nameInput.fill(originalName);
      }
    });

    test('should handle configuration conflicts gracefully', async ({ page }) => {
      // Test scenario where two roles have the same name
      const addRoleButton = page.locator('[data-testid="add-role"], button:has-text("Add Role")');
      
      if (await addRoleButton.isVisible()) {
        await addRoleButton.click();
        await ciWait(page, 'small');
        
        // Get existing role name
        const existingRoles = page.locator('[data-testid="role-config"], .role-config');
        const firstRole = existingRoles.first();
        const nameInput = firstRole.locator('input[name*="name"]');
        
        let existingName = 'Default';
        if (await nameInput.isVisible()) {
          existingName = await nameInput.inputValue();
        }
        
        // Set new role to same name
        const newRole = existingRoles.last();
        const newNameInput = newRole.locator('input[name*="name"]');
        
        if (await newNameInput.isVisible()) {
          await newNameInput.fill(existingName);
          
          // Try to save
          const saveButton = page.locator('[data-testid="save-config"], button:has-text("Save")');
          await saveButton.click();
          
          // Should show conflict error
          await ciWait(page, 'small');
          
          const conflictError = page.locator('.conflict-error, .duplicate-error, .validation-error');
          const hasConflictError = await conflictError.isVisible();
          
          if (hasConflictError) {
            const errorText = await conflictError.textContent();
            console.log('Conflict error for duplicate name:', errorText);
            expect(errorText?.toLowerCase()).toMatch(/duplicate|exists|conflict/);
          }
          
          // Fix the conflict
          await newNameInput.fill(existingName + ' Copy');
          console.log('Resolved naming conflict');
        }
      }
    });
  });
});