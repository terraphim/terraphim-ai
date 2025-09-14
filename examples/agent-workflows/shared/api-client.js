/**
 * AI Agent Workflows - API Client
 * Handles communication with terraphim backend services
 */

class TerraphimApiClient {
  constructor(baseUrl = 'http://localhost:3000') {
    this.baseUrl = baseUrl;
    this.headers = {
      'Content-Type': 'application/json',
    };
  }

  // Generic request method
  async request(endpoint, options = {}) {
    const url = `${this.baseUrl}${endpoint}`;
    const config = {
      headers: this.headers,
      ...options,
    };

    try {
      const response = await fetch(url, config);
      
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.message || `HTTP ${response.status}: ${response.statusText}`);
      }

      const contentType = response.headers.get('content-type');
      if (contentType && contentType.includes('application/json')) {
        return await response.json();
      }
      
      return await response.text();
    } catch (error) {
      console.error(`API Error [${endpoint}]:`, error);
      throw error;
    }
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

  // Workflow execution endpoints (to be implemented)
  async executePromptChain(input) {
    return this.request('/workflows/prompt-chain', {
      method: 'POST',
      body: JSON.stringify(input),
    });
  }

  async executeRouting(input) {
    return this.request('/workflows/route', {
      method: 'POST',
      body: JSON.stringify(input),
    });
  }

  async executeParallel(input) {
    return this.request('/workflows/parallel', {
      method: 'POST',
      body: JSON.stringify(input),
    });
  }

  async executeOrchestration(input) {
    return this.request('/workflows/orchestrate', {
      method: 'POST',
      body: JSON.stringify(input),
    });
  }

  async executeOptimization(input) {
    return this.request('/workflows/optimize', {
      method: 'POST',
      body: JSON.stringify(input),
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
}

// WebSocket client for real-time updates
class WorkflowWebSocket {
  constructor(url = 'ws://localhost:3000/ws') {
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