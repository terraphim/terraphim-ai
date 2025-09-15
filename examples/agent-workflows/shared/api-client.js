/**
 * AI Agent Workflows - API Client
 * Handles communication with terraphim backend services
 */

class TerraphimApiClient {
  constructor(baseUrl = 'http://localhost:8000', options = {}) {
    this.baseUrl = baseUrl;
    this.headers = {
      'Content-Type': 'application/json',
    };
    
    // Configuration options
    this.options = {
      timeout: options.timeout || 30000,
      maxRetries: options.maxRetries || 3,
      retryDelay: options.retryDelay || 1000,
      enableWebSocket: options.enableWebSocket !== false,
      autoReconnect: options.autoReconnect !== false,
      ...options
    };
    
    // WebSocket integration
    this.wsClient = null;
    
    if (this.options.enableWebSocket && typeof TerraphimWebSocketClient !== 'undefined') {
      this.initializeWebSocket();
    }
  }

  initializeWebSocket() {
    try {
      this.wsClient = new TerraphimWebSocketClient({
        url: this.getWebSocketUrl(),
        reconnectInterval: 3000,
        maxReconnectAttempts: 10
      });
      
      // Set up event handlers for workflow updates
      this.wsClient.subscribe('connected', (data) => {
        console.log('WebSocket connected at:', data.timestamp);
      });
      
      this.wsClient.subscribe('disconnected', (data) => {
        console.log('WebSocket disconnected:', data.reason);
      });
      
    } catch (error) {
      console.warn('Failed to initialize WebSocket client:', error);
      this.enableWebSocket = false;
    }
  }

  getWebSocketUrl() {
    const protocol = this.baseUrl.startsWith('https') ? 'wss:' : 'ws:';
    const url = new URL(this.baseUrl);
    return `${protocol}//${url.host}/ws`;
  }

  // Configuration management methods
  updateConfiguration(newConfig) {
    const oldBaseUrl = this.baseUrl;
    const oldOptions = { ...this.options };
    
    if (newConfig.baseUrl && newConfig.baseUrl !== this.baseUrl) {
      this.baseUrl = newConfig.baseUrl;
    }
    
    this.options = { ...this.options, ...newConfig };
    
    // Reinitialize WebSocket if URL changed or WebSocket settings changed
    if (newConfig.baseUrl !== oldBaseUrl || 
        newConfig.enableWebSocket !== oldOptions.enableWebSocket ||
        newConfig.autoReconnect !== oldOptions.autoReconnect) {
      this.reinitializeWebSocket();
    }
    
    return { oldBaseUrl, oldOptions };
  }

  getConfiguration() {
    return {
      baseUrl: this.baseUrl,
      wsUrl: this.getWebSocketUrl(),
      ...this.options
    };
  }

  reinitializeWebSocket() {
    // Cleanup existing WebSocket
    if (this.wsClient) {
      this.wsClient.disconnect();
      this.wsClient = null;
    }
    
    // Initialize new WebSocket if enabled
    if (this.options.enableWebSocket && typeof TerraphimWebSocketClient !== 'undefined') {
      this.initializeWebSocket();
    }
  }

  // Generic request method with retry logic
  async request(endpoint, options = {}) {
    const url = `${this.baseUrl}${endpoint}`;
    const config = {
      headers: this.headers,
      ...options,
    };

    // Add timeout
    if (this.options.timeout) {
      config.signal = AbortSignal.timeout(this.options.timeout);
    }

    let lastError;
    let attempts = 0;
    const maxAttempts = this.options.maxRetries + 1;

    while (attempts < maxAttempts) {
      try {
        attempts++;
        const response = await fetch(url, config);
        
        if (!response.ok) {
          const errorData = await response.json().catch(() => ({}));
          const error = new Error(errorData.message || `HTTP ${response.status}: ${response.statusText}`);
          
          // Only retry on server errors (5xx) or network errors
          if (response.status >= 500 && attempts < maxAttempts) {
            lastError = error;
            await this.delay(this.options.retryDelay * attempts);
            continue;
          }
          
          throw error;
        }

        const contentType = response.headers.get('content-type');
        if (contentType && contentType.includes('application/json')) {
          return await response.json();
        }
        
        return await response.text();
      } catch (error) {
        lastError = error;
        
        // Don't retry on abort/timeout errors unless it's a network error
        if (error.name === 'AbortError' || error.name === 'TimeoutError') {
          break;
        }
        
        // Retry on network errors
        if (attempts < maxAttempts && (error.code === 'NETWORK_ERROR' || error.message.includes('fetch'))) {
          await this.delay(this.options.retryDelay * attempts);
          continue;
        }
        
        break;
      }
    }

    console.error(`API Error [${endpoint}] after ${attempts} attempts:`, lastError);
    throw lastError;
  }

  // Health check
  async health() {
    return this.request('/health');
  }

  // Configuration endpoints
  async getConfig() {
    return this.request('/config');
  }

  async updateConfig(config) {
    return this.request('/config', {
      method: 'POST',
      body: JSON.stringify(config),
    });
  }

  // Document search
  async searchDocuments(query) {
    const searchParams = new URLSearchParams(query);
    return this.request(`/documents/search?${searchParams}`);
  }

  async searchDocumentsPost(query) {
    return this.request('/documents/search', {
      method: 'POST',
      body: JSON.stringify(query),
    });
  }

  // Chat completion
  async chatCompletion(messages, options = {}) {
    return this.request('/chat', {
      method: 'POST',
      body: JSON.stringify({ messages, ...options }),
    });
  }

  // Workflow execution endpoints with WebSocket support
  async executePromptChain(input, options = {}) {
    if (this.wsClient && options.realTime) {
      return this.executeWorkflowWithWebSocket('prompt-chain', input, options);
    }
    return this.request('/workflows/prompt-chain', {
      method: 'POST',
      body: JSON.stringify(input),
    });
  }

  async executeRouting(input, options = {}) {
    if (this.wsClient && options.realTime) {
      return this.executeWorkflowWithWebSocket('routing', input, options);
    }
    return this.request('/workflows/route', {
      method: 'POST',
      body: JSON.stringify(input),
    });
  }

  async executeParallel(input, options = {}) {
    if (this.wsClient && options.realTime) {
      return this.executeWorkflowWithWebSocket('parallel', input, options);
    }
    return this.request('/workflows/parallel', {
      method: 'POST',
      body: JSON.stringify(input),
    });
  }

  async executeOrchestration(input, options = {}) {
    if (this.wsClient && options.realTime) {
      return this.executeWorkflowWithWebSocket('orchestration', input, options);
    }
    return this.request('/workflows/orchestrate', {
      method: 'POST',
      body: JSON.stringify(input),
    });
  }

  async executeOptimization(input, options = {}) {
    if (this.wsClient && options.realTime) {
      return this.executeWorkflowWithWebSocket('optimization', input, options);
    }
    return this.request('/workflows/optimize', {
      method: 'POST',
      body: JSON.stringify(input),
    });
  }

  // WebSocket-enabled workflow execution
  async executeWorkflowWithWebSocket(workflowType, input, options = {}) {
    return new Promise((resolve, reject) => {
      const sessionId = this.wsClient.startWorkflow(workflowType, {
        input,
        role: input.role || 'default',
        overallRole: input.overallRole || 'content_creator',
        ...options
      });

      let workflowResult = null;
      let progressCallback = options.onProgress;
      let agentUpdateCallback = options.onAgentUpdate;
      let qualityCallback = options.onQualityUpdate;

      // Set up event listeners for this session
      const cleanupListeners = [];

      // Progress updates
      const progressUnsub = this.wsClient.subscribe('workflow_progress', (data) => {
        if (data.sessionId === sessionId && progressCallback) {
          progressCallback({
            step: data.data.currentStep,
            total: data.data.totalSteps,
            current: data.data.currentTask,
            percentage: data.data.progress
          });
        }
      });
      cleanupListeners.push(progressUnsub);

      // Agent updates
      const agentUnsub = this.wsClient.subscribe('agent_update', (data) => {
        if (data.sessionId === sessionId && agentUpdateCallback) {
          agentUpdateCallback(data.data);
        }
      });
      cleanupListeners.push(agentUnsub);

      // Quality assessment updates
      const qualityUnsub = this.wsClient.subscribe('quality_assessment', (data) => {
        if (data.sessionId === sessionId && qualityCallback) {
          qualityCallback(data.data);
        }
      });
      cleanupListeners.push(qualityUnsub);

      // Completion handler
      const completionUnsub = this.wsClient.subscribe('workflow_completed', (data) => {
        if (data.sessionId === sessionId) {
          workflowResult = data.data;
          
          // Cleanup listeners
          cleanupListeners.forEach(unsub => unsub());
          
          resolve({
            sessionId,
            success: true,
            result: workflowResult.result,
            metadata: {
              executionTime: workflowResult.executionTime,
              pattern: workflowType,
              steps: workflowResult.steps?.length || 1,
              sessionInfo: this.wsClient.getWorkflowSession(sessionId)
            }
          });
        }
      });
      cleanupListeners.push(completionUnsub);

      // Error handler
      const errorUnsub = this.wsClient.subscribe('workflow_error', (data) => {
        if (data.sessionId === sessionId) {
          // Cleanup listeners
          cleanupListeners.forEach(unsub => unsub());
          
          reject(new Error(data.data.error || 'Workflow execution failed'));
        }
      });
      cleanupListeners.push(errorUnsub);

      // Set a timeout for the workflow
      const timeout = options.timeout || 300000; // 5 minutes default
      setTimeout(() => {
        if (!workflowResult) {
          cleanupListeners.forEach(unsub => unsub());
          this.wsClient.stopWorkflow(sessionId);
          reject(new Error('Workflow execution timeout'));
        }
      }, timeout);
    });
  }

  // Workflow status monitoring
  async getWorkflowStatus(workflowId) {
    return this.request(`/workflows/${workflowId}/status`);
  }

  async getExecutionTrace(workflowId) {
    return this.request(`/workflows/${workflowId}/trace`);
  }

  // Knowledge graph endpoints
  async getRoleGraph() {
    return this.request('/rolegraph');
  }

  async getThesaurus(roleName) {
    return this.request(`/thesaurus/${roleName}`);
  }

  // Utility methods for workflow demos
  
  // Simulate workflow execution with progress updates
  async simulateWorkflow(workflowType, input, onProgress) {
    const startTime = Date.now();
    const workflowId = `workflow_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    // Simulate different workflow patterns
    const workflows = {
      'prompt-chain': () => this.simulatePromptChain(input, onProgress),
      'routing': () => this.simulateRouting(input, onProgress),
      'parallel': () => this.simulateParallelization(input, onProgress),
      'orchestration': () => this.simulateOrchestration(input, onProgress),
      'optimization': () => this.simulateOptimization(input, onProgress),
    };

    if (!workflows[workflowType]) {
      throw new Error(`Unknown workflow type: ${workflowType}`);
    }

    try {
      const result = await workflows[workflowType]();
      const executionTime = Date.now() - startTime;
      
      return {
        workflowId,
        success: true,
        result,
        metadata: {
          executionTime,
          pattern: workflowType,
          steps: result.steps?.length || 1,
        },
      };
    } catch (error) {
      return {
        workflowId,
        success: false,
        error: error.message,
        metadata: {
          executionTime: Date.now() - startTime,
          pattern: workflowType,
        },
      };
    }
  }

  // Workflow simulation methods
  async simulatePromptChain(input, onProgress) {
    const steps = [
      { id: 'understand_task', name: 'Understanding Task', duration: 2000 },
      { id: 'generate_spec', name: 'Generating Specification', duration: 3000 },
      { id: 'create_design', name: 'Creating Design', duration: 2500 },
      { id: 'plan_tasks', name: 'Planning Tasks', duration: 2000 },
      { id: 'implement', name: 'Implementation', duration: 4000 },
    ];

    const results = [];
    
    for (let i = 0; i < steps.length; i++) {
      const step = steps[i];
      
      onProgress?.({
        step: i + 1,
        total: steps.length,
        current: step.name,
        percentage: ((i + 1) / steps.length) * 100,
      });

      // Simulate processing time
      await this.delay(step.duration);
      
      // Simulate step result
      const stepResult = {
        stepId: step.id,
        name: step.name,
        output: this.generateMockOutput(step.id, input),
        duration: step.duration,
        success: true,
      };
      
      results.push(stepResult);
    }

    return {
      pattern: 'prompt_chaining',
      steps: results,
      finalResult: results[results.length - 1].output,
    };
  }

  async simulateRouting(input, onProgress) {
    onProgress?.({ step: 1, total: 3, current: 'Analyzing Task Complexity', percentage: 33 });
    await this.delay(1500);

    onProgress?.({ step: 2, total: 3, current: 'Selecting Optimal Model', percentage: 66 });
    await this.delay(2000);

    onProgress?.({ step: 3, total: 3, current: 'Executing Task', percentage: 100 });
    await this.delay(3000);

    const complexity = input.prompt.length > 500 ? 'complex' : 'simple';
    const selectedModel = complexity === 'complex' ? 'openai_gpt4' : 'openai_gpt35';
    
    return {
      pattern: 'routing',
      taskAnalysis: { complexity, estimatedCost: complexity === 'complex' ? 0.08 : 0.02 },
      selectedRoute: { 
        routeId: selectedModel, 
        reasoning: `Selected ${selectedModel} for ${complexity} task`,
        confidence: complexity === 'complex' ? 0.95 : 0.85,
      },
      result: this.generateMockOutput('routing', input),
    };
  }

  async simulateParallelization(input, onProgress) {
    const perspectives = ['Analysis', 'Practical', 'Creative'];
    const tasks = perspectives.map((p, i) => ({
      id: `perspective_${i}`,
      name: `${p} Perspective`,
      duration: 2000 + Math.random() * 2000,
    }));

    // Simulate parallel execution
    const taskPromises = tasks.map(async (task, index) => {
      const startProgress = (index / tasks.length) * 50;
      const endProgress = ((index + 1) / tasks.length) * 50;
      
      onProgress?.({
        step: index + 1,
        total: tasks.length,
        current: `Processing ${task.name}`,
        percentage: startProgress,
      });
      
      await this.delay(task.duration);
      
      onProgress?.({
        step: index + 1,
        total: tasks.length,
        current: `Completed ${task.name}`,
        percentage: endProgress,
      });

      return {
        taskId: task.id,
        perspective: task.name,
        result: this.generateMockOutput(task.id, input),
        duration: task.duration,
      };
    });

    const parallelResults = await Promise.all(taskPromises);

    onProgress?.({ step: 4, total: 4, current: 'Aggregating Results', percentage: 75 });
    await this.delay(1500);

    onProgress?.({ step: 4, total: 4, current: 'Final Analysis', percentage: 100 });

    return {
      pattern: 'parallelization',
      parallelTasks: parallelResults,
      aggregatedResult: this.generateMockOutput('aggregation', input),
      totalDuration: Math.max(...parallelResults.map(r => r.duration)),
    };
  }

  async simulateOrchestration(input, onProgress) {
    onProgress?.({ step: 1, total: 5, current: 'Planning Tasks', percentage: 20 });
    await this.delay(2000);

    onProgress?.({ step: 2, total: 5, current: 'Data Ingestion', percentage: 40 });
    await this.delay(2500);

    onProgress?.({ step: 3, total: 5, current: 'Analysis Phase', percentage: 60 });
    await this.delay(3000);

    onProgress?.({ step: 4, total: 5, current: 'Knowledge Graph Enrichment', percentage: 80 });
    await this.delay(2000);

    onProgress?.({ step: 5, total: 5, current: 'Generating Insights', percentage: 100 });
    await this.delay(1500);

    return {
      pattern: 'orchestrator_workers',
      orchestrationPlan: {
        totalTasks: 5,
        workerAssignments: ['data_worker', 'analysis_worker', 'graph_worker'],
      },
      workerResults: [
        { workerId: 'data_worker', task: 'Data Ingestion', result: 'Processed 1,240 documents' },
        { workerId: 'analysis_worker', task: 'Analysis', result: 'Extracted 847 key insights' },
        { workerId: 'graph_worker', task: 'Graph Enrichment', result: 'Added 312 new connections' },
      ],
      finalResult: this.generateMockOutput('orchestration', input),
    };
  }

  async simulateOptimization(input, onProgress) {
    const iterations = 3;
    const results = [];

    for (let i = 0; i < iterations; i++) {
      onProgress?.({
        step: i * 2 + 1,
        total: iterations * 2,
        current: `Generation Iteration ${i + 1}`,
        percentage: ((i * 2 + 1) / (iterations * 2)) * 100,
      });
      await this.delay(2500);

      const generated = this.generateMockOutput(`generation_${i}`, input);
      const quality = 0.6 + (i * 0.15); // Improving quality each iteration

      onProgress?.({
        step: i * 2 + 2,
        total: iterations * 2,
        current: `Evaluation Iteration ${i + 1}`,
        percentage: ((i * 2 + 2) / (iterations * 2)) * 100,
      });
      await this.delay(1500);

      results.push({
        iteration: i + 1,
        generated,
        qualityScore: quality,
        feedback: quality < 0.8 ? 'Needs improvement in clarity and structure' : 'Meets quality threshold',
      });

      if (quality >= 0.8) break; // Early stopping
    }

    return {
      pattern: 'evaluator_optimizer',
      iterations: results,
      finalResult: results[results.length - 1].generated,
      finalQuality: results[results.length - 1].qualityScore,
      optimizationSteps: results.length,
    };
  }

  // Helper methods
  delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  generateMockOutput(stepId, input) {
    const mockOutputs = {
      understand_task: `Task Analysis: ${input.prompt.substring(0, 100)}...`,
      generate_spec: `Specification: Detailed requirements and acceptance criteria for "${input.prompt.split(' ').slice(0, 5).join(' ')}"`,
      create_design: `Design: System architecture and component breakdown with UML diagrams`,
      plan_tasks: `Task Plan: 1. Setup environment 2. Implement core logic 3. Add tests 4. Documentation`,
      implement: `Implementation: Complete working solution with ${Math.floor(Math.random() * 500 + 200)} lines of code`,
      routing: `Routed execution result for task: ${input.prompt.substring(0, 50)}...`,
      aggregation: `Aggregated insights from multiple perspectives on: ${input.prompt.substring(0, 50)}...`,
      orchestration: `Orchestrated pipeline result with knowledge graph enrichment: ${input.prompt.substring(0, 50)}...`,
      analysis_perspective: `Analysis: Detailed examination of core concepts and relationships`,
      practical_perspective: `Practical: Real-world implementation considerations and best practices`,
      creative_perspective: `Creative: Innovative approaches and alternative solutions`,
      generation_0: `Initial draft: Basic response to "${input.prompt.substring(0, 30)}..." (Quality: 60%)`,
      generation_1: `Improved version: Enhanced structure and clarity (Quality: 75%)`,
      generation_2: `Optimized result: Professional quality output with excellent structure (Quality: 90%)`,
    };

    return mockOutputs[stepId] || `Generated output for ${stepId}: ${input.prompt.substring(0, 100)}...`;
  }

  // WebSocket utility methods
  isWebSocketEnabled() {
    return this.enableWebSocket && this.wsClient && this.wsClient.isConnected;
  }

  getWebSocketStatus() {
    return this.wsClient ? this.wsClient.getConnectionStatus() : { connected: false };
  }

  subscribeToWorkflowEvents(eventType, callback) {
    if (this.wsClient) {
      return this.wsClient.subscribe(eventType, callback);
    }
    return () => {}; // no-op unsubscribe function
  }

  getActiveWorkflowSessions() {
    return this.wsClient ? this.wsClient.getAllWorkflowSessions() : [];
  }

  pauseWorkflowSession(sessionId) {
    if (this.wsClient) {
      this.wsClient.pauseWorkflow(sessionId);
    }
  }

  resumeWorkflowSession(sessionId) {
    if (this.wsClient) {
      this.wsClient.resumeWorkflow(sessionId);
    }
  }

  stopWorkflowSession(sessionId) {
    if (this.wsClient) {
      this.wsClient.stopWorkflow(sessionId);
    }
  }

  updateWorkflowConfiguration(sessionId, config) {
    if (this.wsClient) {
      this.wsClient.updateWorkflowConfig(sessionId, config);
    }
  }

  // Real-time workflow monitoring
  monitorWorkflow(sessionId, callbacks = {}) {
    const unsubscribeFunctions = [];

    if (callbacks.onProgress) {
      const unsub = this.subscribeToWorkflowEvents('workflow_progress', (data) => {
        if (data.sessionId === sessionId) {
          callbacks.onProgress(data.data);
        }
      });
      unsubscribeFunctions.push(unsub);
    }

    if (callbacks.onAgentUpdate) {
      const unsub = this.subscribeToWorkflowEvents('agent_update', (data) => {
        if (data.sessionId === sessionId) {
          callbacks.onAgentUpdate(data.data);
        }
      });
      unsubscribeFunctions.push(unsub);
    }

    if (callbacks.onQualityUpdate) {
      const unsub = this.subscribeToWorkflowEvents('quality_assessment', (data) => {
        if (data.sessionId === sessionId) {
          callbacks.onQualityUpdate(data.data);
        }
      });
      unsubscribeFunctions.push(unsub);
    }

    if (callbacks.onComplete) {
      const unsub = this.subscribeToWorkflowEvents('workflow_completed', (data) => {
        if (data.sessionId === sessionId) {
          callbacks.onComplete(data.data);
          // Auto-cleanup on completion
          unsubscribeFunctions.forEach(fn => fn());
        }
      });
      unsubscribeFunctions.push(unsub);
    }

    if (callbacks.onError) {
      const unsub = this.subscribeToWorkflowEvents('workflow_error', (data) => {
        if (data.sessionId === sessionId) {
          callbacks.onError(data.data);
          // Auto-cleanup on error
          unsubscribeFunctions.forEach(fn => fn());
        }
      });
      unsubscribeFunctions.push(unsub);
    }

    // Return cleanup function
    return () => {
      unsubscribeFunctions.forEach(fn => fn());
    };
  }

  // Cleanup method
  disconnect() {
    if (this.wsClient) {
      this.wsClient.disconnect();
      this.wsClient = null;
    }
  }
}

// WebSocket client for real-time updates
class WorkflowWebSocket {
  constructor(url = 'ws://localhost:8000/ws') {
    this.url = url;
    this.ws = null;
    this.listeners = new Map();
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 5;
    this.reconnectDelay = 1000;
  }

  connect() {
    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.url);
        
        this.ws.onopen = () => {
          console.log('WebSocket connected');
          this.reconnectAttempts = 0;
          resolve();
        };

        this.ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);
            this.handleMessage(data);
          } catch (error) {
            console.error('Failed to parse WebSocket message:', error);
          }
        };

        this.ws.onclose = () => {
          console.log('WebSocket disconnected');
          this.attemptReconnect();
        };

        this.ws.onerror = (error) => {
          console.error('WebSocket error:', error);
          reject(error);
        };
      } catch (error) {
        reject(error);
      }
    });
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  send(message) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  subscribe(event, callback) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event).add(callback);
  }

  unsubscribe(event, callback) {
    if (this.listeners.has(event)) {
      this.listeners.get(event).delete(callback);
    }
  }

  handleMessage(data) {
    const { event, payload } = data;
    if (this.listeners.has(event)) {
      this.listeners.get(event).forEach(callback => {
        try {
          callback(payload);
        } catch (error) {
          console.error(`Error in WebSocket event handler for ${event}:`, error);
        }
      });
    }
  }

  attemptReconnect() {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);
      
      console.log(`Attempting to reconnect in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
      
      setTimeout(() => {
        this.connect().catch(error => {
          console.error('Reconnection failed:', error);
        });
      }, delay);
    }
  }
}

// Export for use in examples
window.TerraphimApiClient = TerraphimApiClient;
window.WorkflowWebSocket = WorkflowWebSocket;