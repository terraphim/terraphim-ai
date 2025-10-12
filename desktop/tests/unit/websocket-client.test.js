/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

// Mock WebSocket before importing the client
const mockWebSocket = vi.fn();
mockWebSocket.prototype.send = vi.fn();
mockWebSocket.prototype.close = vi.fn();
mockWebSocket.CONNECTING = 0;
mockWebSocket.OPEN = 1;
mockWebSocket.CLOSING = 2;
mockWebSocket.CLOSED = 3;

global.WebSocket = mockWebSocket;

// Load the WebSocket client code
const fs = await import('fs');
const path = await import('path');

const websocketClientPath = path.resolve(process.cwd(), '../examples/agent-workflows/shared/websocket-client.js');
const websocketClientCode = fs.readFileSync(websocketClientPath, 'utf8');

// Execute the code in global scope to define TerraphimWebSocketClient
eval(websocketClientCode);

describe('TerraphimWebSocketClient', () => {
  let client;
  let mockWs;

  beforeEach(() => {
    vi.clearAllMocks();

    // Create a mock WebSocket instance
    mockWs = {
      send: vi.fn(),
      close: vi.fn(),
      readyState: WebSocket.OPEN,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      onopen: null,
      onmessage: null,
      onclose: null,
      onerror: null
    };

    mockWebSocket.mockImplementation(() => mockWs);

    client = new global.TerraphimWebSocketClient({
      url: 'ws://localhost:8000/ws',
      reconnectInterval: 100,
      maxReconnectAttempts: 2
    });
  });

  afterEach(() => {
    if (client) {
      client.disconnect();
    }
  });

  describe('Connection Management', () => {
    it('should initialize with correct configuration', () => {
      expect(client.url).toBe('ws://localhost:8000/ws');
      expect(client.reconnectInterval).toBe(100);
      expect(client.maxReconnectAttempts).toBe(2);
      expect(client.isConnected).toBe(false);
    });

    it('should establish WebSocket connection on initialization', () => {
      expect(mockWebSocket).toHaveBeenCalledWith('ws://localhost:8000/ws');
    });

    it('should set up event handlers', () => {
      expect(mockWs.onopen).toBeDefined();
      expect(mockWs.onmessage).toBeDefined();
      expect(mockWs.onclose).toBeDefined();
      expect(mockWs.onerror).toBeDefined();
    });
  });

  describe('Message Protocol', () => {
    beforeEach(() => {
      // Simulate successful connection
      client.isConnected = true;
      mockWs.readyState = WebSocket.OPEN;
    });

    it('should send heartbeat with correct command_type format', () => {
      client.startHeartbeat();

      // Manually trigger heartbeat
      const heartbeatMessage = {
        command_type: 'heartbeat',
        session_id: null,
        workflow_id: null,
        data: {
          timestamp: expect.any(String)
        }
      };

      // Send heartbeat manually to test format
      client.send(heartbeatMessage);

      expect(mockWs.send).toHaveBeenCalledWith(JSON.stringify(heartbeatMessage));
    });

    it('should start workflow with correct message format', () => {
      const sessionId = client.startWorkflow('test_workflow', { param: 'value' });

      expect(mockWs.send).toHaveBeenCalledWith(JSON.stringify({
        command_type: 'start_workflow',
        workflow_id: sessionId,
        session_id: sessionId,
        data: {
          workflowType: 'test_workflow',
          config: { param: 'value' },
          timestamp: expect.any(String)
        }
      }));
    });

    it('should pause workflow with correct message format', () => {
      const sessionId = 'test-session-123';
      client.pauseWorkflow(sessionId);

      expect(mockWs.send).toHaveBeenCalledWith(JSON.stringify({
        command_type: 'pause_workflow',
        session_id: sessionId,
        workflow_id: sessionId,
        data: {
          timestamp: expect.any(String)
        }
      }));
    });

    it('should stop workflow with correct message format', () => {
      const sessionId = 'test-session-123';
      client.stopWorkflow(sessionId);

      expect(mockWs.send).toHaveBeenCalledWith(JSON.stringify({
        command_type: 'stop_workflow',
        session_id: sessionId,
        workflow_id: sessionId,
        data: {
          timestamp: expect.any(String)
        }
      }));
    });

    it('should not use legacy type field', () => {
      client.startWorkflow('test', {});

      const sentMessage = JSON.parse(mockWs.send.mock.calls[0][0]);
      expect(sentMessage).not.toHaveProperty('type');
      expect(sentMessage).toHaveProperty('command_type');
    });
  });

  describe('Message Handling', () => {
    it('should handle workflow started message', () => {
      const message = {
        response_type: 'workflow_started',
        workflowId: 'test-workflow',
        sessionId: 'test-session',
        data: { steps: ['step1', 'step2'] }
      };

      client.handleMessage(message);

      const session = client.getWorkflowSession('test-session');
      expect(session).toBeDefined();
      expect(session.workflowId).toBe('test-workflow');
      expect(session.status).toBe('running');
      expect(session.steps).toEqual(['step1', 'step2']);
    });

    it('should handle workflow progress message', () => {
      // First create a session
      const startMessage = {
        response_type: 'workflow_started',
        workflowId: 'test-workflow',
        sessionId: 'test-session',
        data: { steps: ['step1', 'step2'] }
      };
      client.handleMessage(startMessage);

      // Then send progress update
      const progressMessage = {
        response_type: 'workflow_progress',
        workflowId: 'test-workflow',
        sessionId: 'test-session',
        data: { currentStep: 1, progress: 50 }
      };
      client.handleMessage(progressMessage);

      const session = client.getWorkflowSession('test-session');
      expect(session.currentStep).toBe(1);
      expect(session.progress).toBe(50);
    });

    it('should handle malformed messages gracefully', () => {
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      // Test various malformed messages
      client.handleMessage(null);
      client.handleMessage('not an object');
      client.handleMessage({});
      client.handleMessage({ no_response_type: 'test' });

      expect(consoleSpy).toHaveBeenCalledWith('Received malformed WebSocket message:', null);
      expect(consoleSpy).toHaveBeenCalledWith('Received WebSocket message without response_type field:', {});

      consoleSpy.mockRestore();
    });

    it('should respond to heartbeat messages', () => {
      const heartbeatMessage = {
        response_type: 'heartbeat',
        data: { timestamp: '2023-01-01T00:00:00Z' }
      };

      client.handleMessage(heartbeatMessage);

      expect(mockWs.send).toHaveBeenCalledWith(JSON.stringify({
        command_type: 'heartbeat_response',
        session_id: null,
        workflow_id: null,
        data: {
          timestamp: expect.any(String)
        }
      }));
    });
  });

  describe('Event Subscription', () => {
    it('should allow subscribing to events', () => {
      const callback = vi.fn();
      const unsubscribe = client.subscribe('workflow_started', callback);

      expect(typeof unsubscribe).toBe('function');

      // Trigger event
      client.emit('workflow_started', { test: 'data' });
      expect(callback).toHaveBeenCalledWith({ test: 'data' });
    });

    it('should allow unsubscribing from events', () => {
      const callback = vi.fn();
      const unsubscribe = client.subscribe('workflow_started', callback);

      unsubscribe();

      client.emit('workflow_started', { test: 'data' });
      expect(callback).not.toHaveBeenCalled();
    });
  });

  describe('Session Management', () => {
    it('should generate unique session IDs', () => {
      const id1 = client.generateSessionId();
      const id2 = client.generateSessionId();

      expect(id1).not.toBe(id2);
      expect(id1).toMatch(/^session_[a-z0-9]+_\d+$/);
    });

    it('should track workflow sessions', () => {
      const sessionId = 'test-session';
      const message = {
        response_type: 'workflow_started',
        workflowId: 'test-workflow',
        sessionId: sessionId,
        data: { steps: [] }
      };

      client.handleMessage(message);

      const session = client.getWorkflowSession(sessionId);
      expect(session).toBeDefined();
      expect(session.workflowId).toBe('test-workflow');
    });

    it('should clean up session data on stop workflow', () => {
      const sessionId = 'test-session';

      // Add session
      client.workflowSessions.set(sessionId, { test: 'data' });

      // Stop workflow should clean up
      client.stopWorkflow(sessionId);

      expect(client.getWorkflowSession(sessionId)).toBeUndefined();
    });
  });

  describe('Connection Status', () => {
    it('should provide connection status information', () => {
      const status = client.getConnectionStatus();

      expect(status).toHaveProperty('connected');
      expect(status).toHaveProperty('reconnectAttempts');
      expect(status).toHaveProperty('activeSessions');
      expect(status).toHaveProperty('subscribers');
    });
  });

  describe('Message Queuing', () => {
    it('should queue messages when disconnected', () => {
      client.isConnected = false;

      const message = { command_type: 'test', data: {} };
      client.send(message);

      expect(mockWs.send).not.toHaveBeenCalled();
      expect(client.messageQueue).toContain(message);
    });

    it('should flush queued messages when connected', () => {
      // Add message to queue while disconnected
      client.isConnected = false;
      const message = { command_type: 'test', data: {} };
      client.send(message);

      // Simulate connection
      client.isConnected = true;
      mockWs.readyState = WebSocket.OPEN;
      client.flushMessageQueue();

      expect(mockWs.send).toHaveBeenCalledWith(JSON.stringify(message));
      expect(client.messageQueue).toHaveLength(0);
    });
  });
});
