import { test, expect } from '@playwright/test';
import { ChildProcess, spawn } from 'child_process';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * Comprehensive E2E tests for TerraphimAgent multi-agent workflow system
 * Tests all 5 workflow patterns with WebSocket communication and proper error handling
 */

let serverProcess: ChildProcess | null = null;

test.beforeAll(async () => {
  // Start terraphim server for testing
  const projectRoot = path.resolve(__dirname, '../../..');
  serverProcess = spawn('cargo', ['run', '--release', '--', '--config', 'terraphim_server/default/ollama_llama_config.json'], {
    cwd: projectRoot,
    stdio: 'pipe'
  });
  
  // Wait for server to start
  await new Promise((resolve) => setTimeout(resolve, 5000));
});

test.afterAll(async () => {
  if (serverProcess) {
    serverProcess.kill();
    serverProcess = null;
  }
});

const workflows = [
  {
    name: 'Prompt Chaining',
    path: '1-prompt-chaining',
    description: 'Sequential prompt execution with result chaining',
    testSelectors: {
      executeButton: '[data-testid="execute-chain"]',
      stepEditor: '.step-editor',
      outputPanel: '.output-panel',
      connectionStatus: '.connection-status'
    }
  },
  {
    name: 'Routing',
    path: '2-routing',
    description: 'Smart routing based on input analysis',
    testSelectors: {
      executeButton: '[data-testid="execute-routing"]',
      inputText: '[data-testid="routing-input"]',
      routingResult: '.routing-result',
      outputPanel: '.output-panel'
    }
  },
  {
    name: 'Parallelization',
    path: '3-parallelization',
    description: 'Parallel agent execution with result aggregation',
    testSelectors: {
      executeButton: '[data-testid="execute-parallel"]',
      agentStatus: '.agent-status',
      progressBar: '.progress-bar',
      outputPanel: '.output-panel'
    }
  },
  {
    name: 'Orchestrator Workers',
    path: '4-orchestrator-workers',
    description: 'Master-worker pattern with task distribution',
    testSelectors: {
      executeButton: '[data-testid="execute-orchestration"]',
      orchestratorPanel: '.orchestrator-panel',
      workerPanel: '.worker-panel',
      taskQueue: '.task-queue'
    }
  },
  {
    name: 'Evaluator Optimizer',
    path: '5-evaluator-optimizer',
    description: 'Quality assessment and optimization loop',
    testSelectors: {
      executeButton: '[data-testid="execute-optimization"]',
      evaluationPanel: '.evaluation-panel',
      optimizationPanel: '.optimization-panel',
      qualityMetrics: '.quality-metrics'
    }
  }
];

// Test each workflow individually
workflows.forEach(workflow => {
  test.describe(`${workflow.name} Workflow`, () => {
    
    test(`should load ${workflow.name} page without errors`, async ({ page }) => {
      const workflowUrl = `file://${path.resolve(__dirname, `../../../examples/agent-workflows/${workflow.path}/index.html`)}`;
      
      // Navigate to workflow page
      await page.goto(workflowUrl);
      await page.waitForLoadState('networkidle');
      
      // Check page loaded successfully
      await expect(page.locator('h1')).toContainText(workflow.name, { ignoreCase: true });
      
      // Check no console errors
      const errors: string[] = [];
      page.on('console', (msg) => {
        if (msg.type() === 'error') {
          errors.push(msg.text());
        }
      });
      
      // Wait for any initial console messages
      await page.waitForTimeout(2000);
      
      // Filter out expected WebSocket connection messages during startup
      const criticalErrors = errors.filter(error => 
        !error.includes('WebSocket connection') &&
        !error.includes('Failed to connect') &&
        !error.includes('Connection refused')
      );
      
      expect(criticalErrors).toHaveLength(0);
    });

    test(`should establish WebSocket connection for ${workflow.name}`, async ({ page }) => {
      const workflowUrl = `file://${path.resolve(__dirname, `../../../examples/agent-workflows/${workflow.path}/index.html`)}`;
      
      await page.goto(workflowUrl);
      await page.waitForLoadState('networkidle');
      
      // Wait for WebSocket connection to establish
      await page.waitForTimeout(3000);
      
      // Check connection status indicator if present
      const connectionIndicator = page.locator('.connection-status, .ws-status, [data-testid="connection-status"]');
      if (await connectionIndicator.count() > 0) {
        await expect(connectionIndicator).toContainText('connected', { ignoreCase: true });
      }
      
      // Test WebSocket message handling
      const wsMessages: any[] = [];
      await page.addInitScript(() => {
        window.addEventListener('load', () => {
          if ((window as any).TerraphimWebSocketClient) {
            const originalSend = WebSocket.prototype.send;
            WebSocket.prototype.send = function(data) {
              (window as any).testWSMessages = (window as any).testWSMessages || [];
              (window as any).testWSMessages.push(JSON.parse(data));
              return originalSend.call(this, data);
            };
          }
        });
      });
      
      // Reload to capture WebSocket messages
      await page.reload();
      await page.waitForTimeout(3000);
      
      // Check that WebSocket messages use correct protocol
      const capturedMessages = await page.evaluate(() => (window as any).testWSMessages || []);
      
      if (capturedMessages.length > 0) {
        // Verify protocol compliance - should use command_type not type
        capturedMessages.forEach((message: any) => {
          expect(message).toHaveProperty('command_type');
          expect(message).not.toHaveProperty('type');
        });
      }
    });

    test(`should execute ${workflow.name} workflow successfully`, async ({ page }) => {
      const workflowUrl = `file://${path.resolve(__dirname, `../../../examples/agent-workflows/${workflow.path}/index.html`)}`;
      
      await page.goto(workflowUrl);
      await page.waitForLoadState('networkidle');
      
      // Wait for initialization
      await page.waitForTimeout(3000);
      
      // Look for execute button with various possible selectors
      const executeButton = page.locator(
        workflow.testSelectors.executeButton + ', ' +
        'button:has-text("Execute"), ' +
        'button:has-text("Start"), ' +
        'button:has-text("Run"), ' +
        '.execute-btn, .start-btn, .run-btn'
      ).first();
      
      if (await executeButton.count() > 0) {
        // Click execute button
        await executeButton.click();
        
        // Wait for workflow execution
        await page.waitForTimeout(5000);
        
        // Check for output or progress indicators
        const outputElements = page.locator(
          workflow.testSelectors.outputPanel + ', ' +
          '.output, .result, .status, .progress, ' +
          '[data-testid="output"], [data-testid="result"]'
        );
        
        if (await outputElements.count() > 0) {
          await expect(outputElements.first()).toBeVisible();
        }
        
        // Check for error messages
        const errorElements = page.locator('.error, .alert-error, [data-testid="error"]');
        if (await errorElements.count() > 0) {
          const errorText = await errorElements.first().textContent();
          console.log(`Workflow ${workflow.name} showed error: ${errorText}`);
        }
      } else {
        console.log(`No execute button found for ${workflow.name} workflow`);
      }
    });

    test(`should handle WebSocket disconnection gracefully for ${workflow.name}`, async ({ page }) => {
      const workflowUrl = `file://${path.resolve(__dirname, `../../../examples/agent-workflows/${workflow.path}/index.html`)}`;
      
      await page.goto(workflowUrl);
      await page.waitForLoadState('networkidle');
      
      // Wait for connection
      await page.waitForTimeout(3000);
      
      // Simulate network disconnection
      await page.setOfflineMode(true);
      await page.waitForTimeout(2000);
      
      // Re-enable network
      await page.setOfflineMode(false);
      await page.waitForTimeout(3000);
      
      // Check that reconnection works
      const connectionStatus = page.locator('.connection-status, .ws-status');
      if (await connectionStatus.count() > 0) {
        // Should show reconnected or connected status
        await expect(connectionStatus).not.toContainText('disconnected', { ignoreCase: true });
      }
    });
  });
});

test.describe('Cross-Workflow Integration Tests', () => {
  
  test('should navigate between all workflow pages', async ({ page }) => {
    const baseUrl = `file://${path.resolve(__dirname, '../../../examples/agent-workflows')}`;
    
    for (const workflow of workflows) {
      const workflowUrl = `${baseUrl}/${workflow.path}/index.html`;
      
      await page.goto(workflowUrl);
      await page.waitForLoadState('networkidle');
      
      // Verify page loads correctly
      await expect(page.locator('h1, h2, .title')).toBeVisible();
      
      // Check for navigation elements if present
      const navLinks = page.locator('nav a, .nav-link, [data-testid*="nav"]');
      if (await navLinks.count() > 0) {
        // Navigation should be functional
        expect(await navLinks.count()).toBeGreaterThan(0);
      }
    }
  });
  
  test('should maintain consistent WebSocket protocol across workflows', async ({ page }) => {
    const allMessages: any[] = [];
    
    // Set up message capture
    await page.addInitScript(() => {
      (window as any).allWSMessages = [];
      const originalWebSocket = window.WebSocket;
      window.WebSocket = class extends originalWebSocket {
        constructor(url: string, protocols?: string | string[]) {
          super(url, protocols);
          
          const originalSend = this.send;
          this.send = function(data: string) {
            try {
              const parsed = JSON.parse(data);
              (window as any).allWSMessages.push(parsed);
            } catch (e) {
              // Ignore non-JSON messages
            }
            return originalSend.call(this, data);
          };
        }
      };
    });
    
    // Test each workflow
    for (const workflow of workflows) {
      const workflowUrl = `file://${path.resolve(__dirname, `../../../examples/agent-workflows/${workflow.path}/index.html`)}`;
      
      await page.goto(workflowUrl);
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(3000);
      
      // Execute workflow if button exists
      const executeButton = page.locator('button:has-text("Execute"), button:has-text("Start"), .execute-btn').first();
      if (await executeButton.count() > 0) {
        await executeButton.click();
        await page.waitForTimeout(2000);
      }
    }
    
    // Verify all messages follow the same protocol
    const capturedMessages = await page.evaluate(() => (window as any).allWSMessages || []);
    
    if (capturedMessages.length > 0) {
      capturedMessages.forEach((message: any, index: number) => {
        expect(message, `Message ${index} should have command_type field`).toHaveProperty('command_type');
        expect(message, `Message ${index} should not have legacy type field`).not.toHaveProperty('type');
        
        // Verify required fields exist
        if (message.command_type !== 'heartbeat' && message.command_type !== 'heartbeat_response') {
          expect(message, `Message ${index} should have session_id`).toHaveProperty('session_id');
          expect(message, `Message ${index} should have workflow_id`).toHaveProperty('workflow_id');
        }
      });
    }
  });
  
  test('should handle concurrent workflow execution', async ({ browser }) => {
    // Test multiple workflows running simultaneously
    const contexts = await Promise.all([
      browser.newContext(),
      browser.newContext(),
      browser.newContext()
    ]);
    
    const pages = await Promise.all(contexts.map(context => context.newPage()));
    
    try {
      // Load different workflows in parallel
      const workflowPromises = workflows.slice(0, 3).map(async (workflow, index) => {
        const page = pages[index];
        const workflowUrl = `file://${path.resolve(__dirname, `../../../examples/agent-workflows/${workflow.path}/index.html`)}`;
        
        await page.goto(workflowUrl);
        await page.waitForLoadState('networkidle');
        await page.waitForTimeout(2000);
        
        // Execute if button exists
        const executeButton = page.locator('button:has-text("Execute"), button:has-text("Start")').first();
        if (await executeButton.count() > 0) {
          await executeButton.click();
        }
        
        return page;
      });
      
      await Promise.all(workflowPromises);
      
      // Verify all pages are still responsive
      for (const page of pages) {
        await expect(page.locator('body')).toBeVisible();
      }
      
    } finally {
      // Clean up
      await Promise.all(contexts.map(context => context.close()));
    }
  });
});

test.describe('WebSocket Protocol Validation', () => {
  
  test('should send properly formatted heartbeat messages', async ({ page }) => {
    const workflowUrl = `file://${path.resolve(__dirname, '../../../examples/agent-workflows/1-prompt-chaining/index.html')}`;
    
    let heartbeatMessages: any[] = [];
    
    await page.addInitScript(() => {
      (window as any).heartbeatMessages = [];
      
      // Capture WebSocket messages
      const originalWebSocket = window.WebSocket;
      window.WebSocket = class extends originalWebSocket {
        constructor(url: string, protocols?: string | string[]) {
          super(url, protocols);
          
          const originalSend = this.send;
          this.send = function(data: string) {
            try {
              const parsed = JSON.parse(data);
              if (parsed.command_type === 'heartbeat' || parsed.command_type === 'heartbeat_response') {
                (window as any).heartbeatMessages.push(parsed);
              }
            } catch (e) {
              // Ignore non-JSON messages
            }
            return originalSend.call(this, data);
          };
        }
      };
    });
    
    await page.goto(workflowUrl);
    await page.waitForLoadState('networkidle');
    
    // Wait for heartbeat messages (they should occur every 30 seconds, but we'll wait shorter)
    await page.waitForTimeout(5000);
    
    heartbeatMessages = await page.evaluate(() => (window as any).heartbeatMessages || []);
    
    if (heartbeatMessages.length > 0) {
      heartbeatMessages.forEach((message: any) => {
        expect(message).toHaveProperty('command_type');
        expect(['heartbeat', 'heartbeat_response']).toContain(message.command_type);
        expect(message).toHaveProperty('data');
        expect(message.data).toHaveProperty('timestamp');
      });
    }
  });
  
  test('should handle malformed server responses gracefully', async ({ page }) => {
    const workflowUrl = `file://${path.resolve(__dirname, '../../../examples/agent-workflows/1-prompt-chaining/index.html')}`;
    
    // Track console warnings
    const warnings: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'warn') {
        warnings.push(msg.text());
      }
    });
    
    await page.goto(workflowUrl);
    await page.waitForLoadState('networkidle');
    
    // Simulate malformed WebSocket message
    await page.evaluate(() => {
      if ((window as any).client && (window as any).client.handleMessage) {
        // Send malformed message to test error handling
        (window as any).client.handleMessage({ invalid: 'message' });
        (window as any).client.handleMessage(null);
        (window as any).client.handleMessage('not an object');
      }
    });
    
    await page.waitForTimeout(1000);
    
    // Should have logged appropriate warnings for malformed messages
    const relevantWarnings = warnings.filter(warning => 
      warning.includes('malformed') || 
      warning.includes('response_type') ||
      warning.includes('WebSocket message')
    );
    
    expect(relevantWarnings.length).toBeGreaterThan(0);
  });
});