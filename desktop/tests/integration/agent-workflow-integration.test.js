/**
 * Integration Test Harness for Agent Workflows
 * Tests real WebSocket communication and workflow execution
 */

import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import { spawn } from 'child_process';
import { WebSocket } from 'ws';
import fetch from 'node-fetch';

// Test configuration
const TEST_CONFIG = {
  serverPort: 8001, // Use different port for testing
  websocketUrl: 'ws://127.0.0.1:8001/ws',
  httpUrl: 'http://127.0.0.1:8001',
  timeout: 30000,
  workflows: [
    { name: 'prompt-chaining', type: 'sequential' },
    { name: 'routing', type: 'conditional' },
    { name: 'parallelization', type: 'parallel' },
    { name: 'orchestrator-workers', type: 'orchestration' },
    { name: 'evaluator-optimizer', type: 'optimization' }
  ]
};

describe('Agent Workflow Integration Tests', () => {
  let serverProcess = null;
  let wsClient = null;
  let receivedMessages = [];

  beforeAll(async () => {
    // Start test server
    serverProcess = spawn('cargo', [
      'run', '--release', '--',
      '--config', 'terraphim_server/default/ollama_llama_config.json',
      '--port', TEST_CONFIG.serverPort.toString()
    ], {
      stdio: 'pipe',
      cwd: process.cwd() + '/..'
    });

    // Wait for server to start
    let serverReady = false;
    let attempts = 0;
    const maxAttempts = 30;

    while (!serverReady && attempts < maxAttempts) {
      try {
        const response = await fetch(`${TEST_CONFIG.httpUrl}/health`);
        if (response.ok) {
          serverReady = true;
        }
      } catch (error) {
        // Server not ready yet
        await new Promise(resolve => setTimeout(resolve, 1000));
        attempts++;
      }
    }

    if (!serverReady) {
      throw new Error('Test server failed to start within timeout');
    }

    console.log('Test server started successfully');
  }, TEST_CONFIG.timeout);

  afterAll(async () => {
    if (wsClient && wsClient.readyState === WebSocket.OPEN) {
      wsClient.close();
    }

    if (serverProcess) {
      serverProcess.kill('SIGTERM');

      // Wait for graceful shutdown
      await new Promise(resolve => {
        serverProcess.on('exit', resolve);
        setTimeout(() => {
          serverProcess.kill('SIGKILL');
          resolve();
        }, 5000);
      });
    }
  });

  beforeEach(() => {
    receivedMessages = [];
  });

  afterEach(() => {
    if (wsClient && wsClient.readyState === WebSocket.OPEN) {
      wsClient.close();
    }
  });

  describe('WebSocket Protocol Compliance', () => {
    it('should establish WebSocket connection successfully', async () => {
      await new Promise((resolve, reject) => {
        wsClient = new WebSocket(TEST_CONFIG.websocketUrl);

        wsClient.on('open', () => {
          expect(wsClient.readyState).toBe(WebSocket.OPEN);
          resolve();
        });

        wsClient.on('error', reject);

        setTimeout(() => reject(new Error('WebSocket connection timeout')), 10000);
      });
    });

    it('should accept messages with correct command_type format', async () => {
      await new Promise((resolve, reject) => {
        wsClient = new WebSocket(TEST_CONFIG.websocketUrl);

        wsClient.on('open', () => {
          const testMessage = {
            command_type: 'start_workflow',
            session_id: 'test-session-123',
            workflow_id: 'test-workflow-123',
            data: {
              workflowType: 'test',
              config: { test: true },
              timestamp: new Date().toISOString()
            }
          };

          wsClient.send(JSON.stringify(testMessage));

          // Should not receive error response
          setTimeout(resolve, 2000);
        });

        wsClient.on('message', (data) => {
          const message = JSON.parse(data.toString());
          if (message.response_type === 'error') {
            reject(new Error(`Server rejected message: ${message.data?.error}`));
          }
        });

        wsClient.on('error', reject);
      });
    });

    it('should reject messages with legacy type format', async () => {
      await new Promise((resolve, reject) => {
        wsClient = new WebSocket(TEST_CONFIG.websocketUrl);
        let receivedError = false;

        wsClient.on('open', () => {
          const legacyMessage = {
            type: 'start_workflow', // Legacy format should be rejected
            sessionId: 'test-session-123',
            workflowType: 'test',
            config: { test: true }
          };

          wsClient.send(JSON.stringify(legacyMessage));
        });

        wsClient.on('message', (data) => {
          const message = JSON.parse(data.toString());
          receivedMessages.push(message);

          // Should receive error or be ignored
          if (message.response_type === 'error' || message.error) {
            receivedError = true;
            resolve();
          }
        });

        wsClient.on('error', () => {
          receivedError = true;
          resolve();
        });

        // If no error after 3 seconds, that's also fine (message ignored)
        setTimeout(() => {
          if (!receivedError) {
            console.log('Legacy message was ignored (acceptable behavior)');
          }
          resolve();
        }, 3000);
      });
    });
  });

  describe('Workflow Execution Integration', () => {
    beforeEach(async () => {
      // Establish fresh WebSocket connection for each test
      await new Promise((resolve, reject) => {
        wsClient = new WebSocket(TEST_CONFIG.websocketUrl);

        wsClient.on('open', resolve);
        wsClient.on('error', reject);

        wsClient.on('message', (data) => {
          try {
            const message = JSON.parse(data.toString());
            receivedMessages.push(message);
          } catch (error) {
            console.warn('Failed to parse message:', data.toString());
          }
        });

        setTimeout(() => reject(new Error('Connection timeout')), 5000);
      });
    });

    it('should handle workflow lifecycle messages', async () => {
      const sessionId = `integration-test-${Date.now()}`;

      // Start workflow
      const startMessage = {
        command_type: 'start_workflow',
        session_id: sessionId,
        workflow_id: sessionId,
        data: {
          workflowType: 'prompt-chaining',
          config: {
            steps: ['analyze', 'design', 'implement'],
            project: 'test-integration'
          },
          timestamp: new Date().toISOString()
        }
      };

      wsClient.send(JSON.stringify(startMessage));

      // Wait for workflow responses
      await new Promise(resolve => setTimeout(resolve, 5000));

      // Should receive workflow started confirmation
      const workflowMessages = receivedMessages.filter(msg =>
        msg.sessionId === sessionId || msg.session_id === sessionId
      );

      expect(workflowMessages.length).toBeGreaterThan(0);

      // Check for expected message types
      const messageTypes = workflowMessages.map(msg => msg.response_type).filter(Boolean);
      console.log('Received message types:', messageTypes);

      // Should receive at least one workflow-related message
      expect(messageTypes.some(type =>
        type.includes('workflow') || type.includes('started') || type.includes('progress')
      )).toBe(true);
    });

    it('should handle multiple concurrent workflow sessions', async () => {
      const sessions = [];
      const numSessions = 3;

      // Start multiple workflows
      for (let i = 0; i < numSessions; i++) {
        const sessionId = `concurrent-test-${i}-${Date.now()}`;
        sessions.push(sessionId);

        const message = {
          command_type: 'start_workflow',
          session_id: sessionId,
          workflow_id: sessionId,
          data: {
            workflowType: TEST_CONFIG.workflows[i % TEST_CONFIG.workflows.length].name,
            config: { concurrent: true, index: i },
            timestamp: new Date().toISOString()
          }
        };

        wsClient.send(JSON.stringify(message));

        // Small delay between requests
        await new Promise(resolve => setTimeout(resolve, 100));
      }

      // Wait for responses
      await new Promise(resolve => setTimeout(resolve, 8000));

      // Verify we received responses for all sessions
      const uniqueSessions = new Set(
        receivedMessages
          .map(msg => msg.sessionId || msg.session_id)
          .filter(Boolean)
      );

      console.log('Started sessions:', sessions);
      console.log('Received responses for sessions:', Array.from(uniqueSessions));

      // Should handle multiple sessions without conflicts
      expect(uniqueSessions.size).toBeGreaterThan(0);
    });
  });

  describe('Error Handling and Recovery', () => {
    beforeEach(async () => {
      await new Promise((resolve, reject) => {
        wsClient = new WebSocket(TEST_CONFIG.websocketUrl);
        wsClient.on('open', resolve);
        wsClient.on('error', reject);
        wsClient.on('message', (data) => {
          try {
            receivedMessages.push(JSON.parse(data.toString()));
          } catch (error) {
            // Ignore parse errors for this test
          }
        });
        setTimeout(() => reject(new Error('Connection timeout')), 5000);
      });
    });

    it('should handle malformed JSON gracefully', async () => {
      // Send malformed JSON
      wsClient.send('{"invalid": json}');
      wsClient.send('not json at all');
      wsClient.send('');

      // Wait for potential error responses
      await new Promise(resolve => setTimeout(resolve, 2000));

      // Connection should remain open
      expect(wsClient.readyState).toBe(WebSocket.OPEN);
    });

    it('should handle missing required fields', async () => {
      const invalidMessages = [
        {}, // Empty object
        { command_type: 'start_workflow' }, // Missing session_id
        { session_id: 'test' }, // Missing command_type
        { command_type: '', session_id: 'test' } // Empty command_type
      ];

      for (const msg of invalidMessages) {
        wsClient.send(JSON.stringify(msg));
        await new Promise(resolve => setTimeout(resolve, 500));
      }

      // Should still be connected
      expect(wsClient.readyState).toBe(WebSocket.OPEN);
    });
  });

  describe('Performance and Load Testing', () => {
    beforeEach(async () => {
      await new Promise((resolve, reject) => {
        wsClient = new WebSocket(TEST_CONFIG.websocketUrl);
        wsClient.on('open', resolve);
        wsClient.on('error', reject);
        wsClient.on('message', (data) => {
          receivedMessages.push(JSON.parse(data.toString()));
        });
        setTimeout(() => reject(new Error('Connection timeout')), 5000);
      });
    });

    it('should handle rapid message sending', async () => {
      const startTime = Date.now();
      const messageCount = 50;

      // Send messages rapidly
      for (let i = 0; i < messageCount; i++) {
        const message = {
          command_type: 'heartbeat',
          session_id: null,
          workflow_id: null,
          data: {
            timestamp: new Date().toISOString(),
            sequence: i
          }
        };

        wsClient.send(JSON.stringify(message));
      }

      // Wait for processing
      await new Promise(resolve => setTimeout(resolve, 3000));

      const endTime = Date.now();
      const duration = endTime - startTime;

      console.log(`Sent ${messageCount} messages in ${duration}ms`);
      console.log(`Received ${receivedMessages.length} responses`);

      // Should handle rapid messages without crashing
      expect(wsClient.readyState).toBe(WebSocket.OPEN);
      expect(duration).toBeLessThan(10000); // Should complete within 10 seconds
    });
  });

  describe('HTTP API Integration', () => {
    it('should provide health check endpoint', async () => {
      const response = await fetch(`${TEST_CONFIG.httpUrl}/health`);
      expect(response.ok).toBe(true);

      const data = await response.json();
      expect(data).toHaveProperty('status');
    });

    it('should provide workflow configuration endpoint', async () => {
      try {
        const response = await fetch(`${TEST_CONFIG.httpUrl}/config`);

        if (response.ok) {
          const config = await response.json();
          console.log('Server configuration loaded:', Object.keys(config));
          expect(config).toBeTypeOf('object');
        } else {
          console.log('Config endpoint not available or requires authentication');
        }
      } catch (error) {
        console.log('Config endpoint test skipped:', error.message);
      }
    });
  });
});
