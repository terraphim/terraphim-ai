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
    const { type, workflowId, sessionId, data } = message;
    
    switch (type) {
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
        
      default:
        console.warn('Unknown message type:', type);
    }
    
    // Emit to all subscribers
    this.emit(type, { workflowId, sessionId, data, timestamp: new Date() });
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
      type: 'heartbeat_response',
      timestamp: new Date().toISOString()
    });
  }

  startWorkflow(workflowType, config) {
    const sessionId = this.generateSessionId();
    const message = {
      type: 'start_workflow',
      workflowType,
      sessionId,
      config,
      timestamp: new Date().toISOString()
    };
    
    this.send(message);
    return sessionId;
  }

  pauseWorkflow(sessionId) {
    this.send({
      type: 'pause_workflow',
      sessionId,
      timestamp: new Date().toISOString()
    });
  }

  resumeWorkflow(sessionId) {
    this.send({
      type: 'resume_workflow',
      sessionId,
      timestamp: new Date().toISOString()
    });
  }

  stopWorkflow(sessionId) {
    this.send({
      type: 'stop_workflow',
      sessionId,
      timestamp: new Date().toISOString()
    });
    
    // Clean up local session data
    this.workflowSessions.delete(sessionId);
  }

  updateWorkflowConfig(sessionId, config) {
    this.send({
      type: 'update_config',
      sessionId,
      config,
      timestamp: new Date().toISOString()
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
          type: 'heartbeat',
          timestamp: new Date().toISOString()
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
      subscribers: Array.from(this.subscribers.keys())
    };
  }
}

// Export for use in examples
window.TerraphimWebSocketClient = TerraphimWebSocketClient;