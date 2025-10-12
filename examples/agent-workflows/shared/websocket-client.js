/**
 * WebSocket Client for Real-time Workflow Updates
 * Provides real-time communication between examples and terraphim server
 */

class TerraphimWebSocketClient {
  constructor(options = {}) {
    this.url = options.url || this.getWebSocketUrl();
    this.reconnectInterval = options.reconnectInterval || 3000;
    this.maxReconnectAttempts = options.maxReconnectAttempts || 10;
    this.heartbeatInterval = options.heartbeatInterval || 30000;
    this.baseHeartbeatInterval = this.heartbeatInterval;
    this.maxHeartbeatInterval = options.maxHeartbeatInterval || 300000; // 5 minutes max
    this.heartbeatScaleFactor = options.heartbeatScaleFactor || 1.2;

    this.ws = null;
    this.isConnected = false;
    this.reconnectAttempts = 0;
    this.heartbeatTimer = null;
    this.messageQueue = [];
    this.subscribers = new Map();
    this.workflowSessions = new Map();

    this.connect();
  }

  getWebSocketUrl() {
    // For local examples, use hardcoded server URL
    if (window.location.protocol === 'file:') {
      return 'ws://127.0.0.1:8000/ws';
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.hostname;
    const port = window.location.port || (protocol === 'wss:' ? '443' : '80');
    return `${protocol}//${host}:${port}/ws`;
  }

  connect() {
    try {
      this.ws = new WebSocket(this.url);
      this.setupEventHandlers();
    } catch (error) {
      console.error('WebSocket connection failed:', error);
      this.scheduleReconnect();
    }
  }

  setupEventHandlers() {
    this.ws.onopen = (event) => {
      console.log('WebSocket connected:', event);
      this.isConnected = true;
      this.reconnectAttempts = 0;

      // Reset heartbeat interval on reconnection
      this.resetHeartbeatInterval();
      this.startHeartbeat();
      this.flushMessageQueue();
      this.emit('connected', { timestamp: new Date() });
    };

    this.ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        this.handleMessage(message);
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error);
      }
    };

    this.ws.onclose = (event) => {
      console.log('WebSocket disconnected:', event);
      this.isConnected = false;
      this.stopHeartbeat();
      this.emit('disconnected', { code: event.code, reason: event.reason });

      if (!event.wasClean && this.reconnectAttempts < this.maxReconnectAttempts) {
        this.scheduleReconnect();
      }
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      this.emit('error', { error, timestamp: new Date() });
    };
  }

  handleMessage(message) {
    // Handle malformed messages
    if (!message || typeof message !== 'object') {
      console.warn('Received malformed WebSocket message:', message);
      return;
    }

    const { response_type, workflowId, sessionId, data } = message;

    // Handle messages without response_type field
    if (!response_type) {
      console.warn('Received WebSocket message without response_type field:', message);
      return;
    }

    switch (response_type) {
      case 'workflow_started':
        this.handleWorkflowStarted(workflowId, sessionId, data);
        break;

      case 'workflow_progress':
        this.handleWorkflowProgress(workflowId, sessionId, data);
        break;

      case 'workflow_completed':
        this.handleWorkflowCompleted(workflowId, sessionId, data);
        break;

      case 'workflow_error':
        this.handleWorkflowError(workflowId, sessionId, data);
        break;

      case 'agent_update':
        this.handleAgentUpdate(workflowId, sessionId, data);
        break;

      case 'quality_assessment':
        this.handleQualityAssessment(workflowId, sessionId, data);
        break;

      case 'heartbeat':
        this.handleHeartbeat(data);
        break;

      case 'pong':
        this.handlePong(data);
        break;

      case 'connection_established':
        this.handleConnectionEstablished(workflowId, sessionId, data);
        break;

      case 'error':
        this.handleServerError(workflowId, sessionId, data);
        break;

      default:
        console.warn('Unknown message response_type:', response_type);
    }

    // Emit to all subscribers
    this.emit(response_type, { workflowId, sessionId, data, timestamp: new Date() });
  }

  handleWorkflowStarted(workflowId, sessionId, data) {
    this.workflowSessions.set(sessionId, {
      workflowId,
      status: 'running',
      startTime: new Date(),
      steps: data.steps || [],
      currentStep: 0
    });
  }

  handleWorkflowProgress(workflowId, sessionId, data) {
    const session = this.workflowSessions.get(sessionId);
    if (session) {
      session.currentStep = data.currentStep || session.currentStep;
      session.progress = data.progress || 0;
      session.lastUpdate = new Date();
    }
  }

  handleWorkflowCompleted(workflowId, sessionId, data) {
    const session = this.workflowSessions.get(sessionId);
    if (session) {
      session.status = 'completed';
      session.endTime = new Date();
      session.result = data.result;
    }
  }

  handleWorkflowError(workflowId, sessionId, data) {
    const session = this.workflowSessions.get(sessionId);
    if (session) {
      session.status = 'error';
      session.error = data.error;
      session.endTime = new Date();
    }
  }

  handleAgentUpdate(workflowId, sessionId, data) {
    const session = this.workflowSessions.get(sessionId);
    if (session) {
      if (!session.agents) session.agents = {};
      session.agents[data.agentId] = {
        ...session.agents[data.agentId],
        ...data
      };
    }
  }

  handleQualityAssessment(workflowId, sessionId, data) {
    const session = this.workflowSessions.get(sessionId);
    if (session) {
      if (!session.qualityHistory) session.qualityHistory = [];
      session.qualityHistory.push({
        timestamp: new Date(),
        scores: data.scores,
        iteration: data.iteration
      });
    }
  }

  handleHeartbeat(data) {
    // Respond to server heartbeat
    this.send({
      command_type: 'ping',
      session_id: null,
      workflow_id: null,
      data: {
        timestamp: new Date().toISOString()
      }
    });
  }

  handlePong(data) {
    // Server responded to our ping
    console.log('Received pong from server:', data);

    // Adaptive timeout: Increase heartbeat interval on successful ping/pong
    // This allows for longer-running LLM operations
    const newInterval = Math.min(
      this.heartbeatInterval * this.heartbeatScaleFactor,
      this.maxHeartbeatInterval
    );

    if (newInterval !== this.heartbeatInterval) {
      console.log(`📈 Adaptive timeout: Increasing heartbeat interval from ${this.heartbeatInterval}ms to ${newInterval}ms`);
      this.heartbeatInterval = newInterval;

      // Restart heartbeat with new interval
      this.stopHeartbeat();
      this.startHeartbeat();
    }
  }

  handleConnectionEstablished(workflowId, sessionId, data) {
    console.log('WebSocket connection established:', data);
    // Set connection as established and update UI if needed
    this.isConnected = true;

    // Store server capabilities if provided
    if (data && data.capabilities) {
      this.serverCapabilities = data.capabilities;
    }

    // Update session info
    if (sessionId && data) {
      this.serverSessionId = sessionId;
      this.serverInfo = {
        sessionId: sessionId,
        serverTime: data.server_time,
        capabilities: data.capabilities || []
      };
    }
  }

  handleServerError(workflowId, sessionId, data) {
    console.error('Server error received:', data);

    // Create error object
    const error = {
      workflowId,
      sessionId,
      message: data?.error || data?.message || 'Unknown server error',
      code: data?.code,
      timestamp: new Date(),
      data
    };

    // Store in error history for debugging
    if (!this.errorHistory) {
      this.errorHistory = [];
    }
    this.errorHistory.push(error);

    // Keep only last 10 errors
    if (this.errorHistory.length > 10) {
      this.errorHistory = this.errorHistory.slice(-10);
    }

    // Emit error event for UI handling
    this.emit('server_error', error);
  }

  // WebSocket workflows are now started via HTTP POST endpoints
  // This method creates a session ID for tracking WebSocket updates
  createWorkflowSession(workflowId) {
    const sessionData = {
      workflowId,
      status: 'pending',
      startTime: new Date(),
      steps: [],
      currentStep: 0
    };

    this.workflowSessions.set(workflowId, sessionData);
    return workflowId;
  }

  pauseWorkflow(sessionId) {
    this.send({
      command_type: 'pause_workflow',
      session_id: sessionId,
      workflow_id: sessionId,
      data: {
        timestamp: new Date().toISOString()
      }
    });
  }

  resumeWorkflow(sessionId) {
    this.send({
      command_type: 'resume_workflow',
      session_id: sessionId,
      workflow_id: sessionId,
      data: {
        timestamp: new Date().toISOString()
      }
    });
  }

  stopWorkflow(sessionId) {
    this.send({
      command_type: 'stop_workflow',
      session_id: sessionId,
      workflow_id: sessionId,
      data: {
        timestamp: new Date().toISOString()
      }
    });

    // Clean up local session data
    this.workflowSessions.delete(sessionId);
  }

  updateWorkflowConfig(sessionId, config) {
    this.send({
      command_type: 'update_config',
      session_id: sessionId,
      workflow_id: sessionId,
      data: {
        config,
        timestamp: new Date().toISOString()
      }
    });
  }

  send(message) {
    if (this.isConnected && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      // Queue message for when connection is restored
      this.messageQueue.push(message);
    }
  }

  subscribe(eventType, callback) {
    if (!this.subscribers.has(eventType)) {
      this.subscribers.set(eventType, new Set());
    }
    this.subscribers.get(eventType).add(callback);

    // Return unsubscribe function
    return () => {
      const subscribers = this.subscribers.get(eventType);
      if (subscribers) {
        subscribers.delete(callback);
        if (subscribers.size === 0) {
          this.subscribers.delete(eventType);
        }
      }
    };
  }

  emit(eventType, data) {
    const subscribers = this.subscribers.get(eventType);
    if (subscribers) {
      subscribers.forEach(callback => {
        try {
          callback(data);
        } catch (error) {
          console.error('Error in WebSocket event handler:', error);
        }
      });
    }
  }

  getWorkflowSession(sessionId) {
    return this.workflowSessions.get(sessionId);
  }

  getAllWorkflowSessions() {
    return Array.from(this.workflowSessions.values());
  }

  startHeartbeat() {
    this.heartbeatTimer = setInterval(() => {
      if (this.isConnected) {
        this.send({
          command_type: 'ping',
          session_id: null,
          workflow_id: null,
          data: {
            timestamp: new Date().toISOString()
          }
        });
      }
    }, this.heartbeatInterval);
  }

  stopHeartbeat() {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  resetHeartbeatInterval() {
    if (this.heartbeatInterval !== this.baseHeartbeatInterval) {
      console.log(`🔄 Resetting heartbeat interval from ${this.heartbeatInterval}ms to ${this.baseHeartbeatInterval}ms`);
      this.heartbeatInterval = this.baseHeartbeatInterval;
    }
  }

  scheduleReconnect() {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      console.log(`Attempting to reconnect... (${this.reconnectAttempts}/${this.maxReconnectAttempts})`);

      setTimeout(() => {
        this.connect();
      }, this.reconnectInterval * this.reconnectAttempts);
    } else {
      console.error('Max reconnection attempts reached. Giving up.');
      this.emit('connection_failed', { attempts: this.reconnectAttempts });
    }
  }

  flushMessageQueue() {
    while (this.messageQueue.length > 0) {
      const message = this.messageQueue.shift();
      this.send(message);
    }
  }

  generateSessionId() {
    return 'session_' + Math.random().toString(36).substr(2, 9) + '_' + Date.now();
  }

  disconnect() {
    this.stopHeartbeat();
    if (this.ws) {
      this.ws.close();
    }
    this.subscribers.clear();
    this.workflowSessions.clear();
  }

  // Connection status
  getConnectionStatus() {
    return {
      connected: this.isConnected,
      reconnectAttempts: this.reconnectAttempts,
      activeSessions: this.workflowSessions.size,
      subscribers: Array.from(this.subscribers.keys()),
      serverInfo: this.serverInfo || null,
      serverCapabilities: this.serverCapabilities || [],
      errorHistory: this.errorHistory || []
    };
  }
}

// Export for use in examples
window.TerraphimWebSocketClient = TerraphimWebSocketClient;
