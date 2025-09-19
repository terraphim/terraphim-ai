/**
 * AI Orchestrator-Workers - Data Science Pipeline
 * Demonstrates hierarchical task decomposition with specialized workers
 */

class OrchestratorWorkersDemo {
  constructor() {
    this.apiClient = null; // Will be initialized with settings
    this.visualizer = new WorkflowVisualizer('pipeline-container');
    this.settingsIntegration = null;
    this.selectedSources = new Set(['arxiv', 'pubmed', 'semantic_scholar']);
    this.isRunning = false;
    this.currentStage = 0;
    this.stageResults = new Map();
    this.knowledgeGraph = new Map();
    
    // Define available data sources
    this.dataSources = [
      {
        id: 'arxiv',
        name: 'arXiv',
        icon: '📚',
        description: 'Research papers and preprints'
      },
      {
        id: 'pubmed',
        name: 'PubMed',
        icon: '🔬',
        description: 'Medical and life sciences'
      },
      {
        id: 'semantic_scholar',
        name: 'Semantic Scholar',
        icon: '🎓',
        description: 'AI-powered research database'
      },
      {
        id: 'google_scholar',
        name: 'Google Scholar',
        icon: '🔍',
        description: 'Academic search engine'
      },
      {
        id: 'research_gate',
        name: 'ResearchGate',
        icon: '👨‍🔬',
        description: 'Scientific publications network'
      },
      {
        id: 'ieee',
        name: 'IEEE Xplore',
        icon: '⚡',
        description: 'Engineering and technology'
      }
    ];

    // Define specialized worker types
    this.workers = [
      {
        id: 'data_collector',
        name: 'Data Collector',
        icon: '📥',
        specialty: 'Paper retrieval and initial filtering',
        status: 'idle'
      },
      {
        id: 'content_analyzer',
        name: 'Content Analyzer',
        icon: '🔍',
        specialty: 'Abstract and content analysis',
        status: 'idle'
      },
      {
        id: 'methodology_expert',
        name: 'Methodology Expert',
        icon: '🧪',
        specialty: 'Research methods and validation',
        status: 'idle'
      },
      {
        id: 'knowledge_mapper',
        name: 'Knowledge Mapper',
        icon: '🗺️',
        specialty: 'Concept extraction and relationships',
        status: 'idle'
      },
      {
        id: 'synthesis_specialist',
        name: 'Synthesis Specialist',
        icon: '🧩',
        specialty: 'Result aggregation and insights',
        status: 'idle'
      },
      {
        id: 'graph_builder',
        name: 'Graph Builder',
        icon: '🕸️',
        specialty: 'Knowledge graph construction',
        status: 'idle'
      }
    ];

    // Define pipeline stages
    this.pipelineStages = [
      {
        id: 'data_ingestion',
        title: 'Data Ingestion & Collection',
        icon: '📥',
        description: 'Collect research papers and documents from selected data sources based on the research query.',
        workers: ['data_collector'],
        duration: 3000
      },
      {
        id: 'content_analysis',
        title: 'Content Analysis & Processing',
        icon: '🔍',
        description: 'Analyze paper abstracts, extract key concepts, and identify relevant methodologies.',
        workers: ['content_analyzer', 'methodology_expert'],
        duration: 4000
      },
      {
        id: 'knowledge_extraction',
        title: 'Knowledge Extraction & Mapping',
        icon: '🗺️',
        description: 'Extract concepts, relationships, and build semantic mappings from processed content.',
        workers: ['knowledge_mapper'],
        duration: 3500
      },
      {
        id: 'graph_construction',
        title: 'Knowledge Graph Construction',
        icon: '🕸️',
        description: 'Build comprehensive knowledge graph with nodes, edges, and semantic relationships.',
        workers: ['graph_builder'],
        duration: 4500
      },
      {
        id: 'synthesis_insights',
        title: 'Synthesis & Insights Generation',
        icon: '🧩',
        description: 'Aggregate findings, generate insights, and produce comprehensive research summary.',
        workers: ['synthesis_specialist'],
        duration: 3000
      }
    ];
    this.analysisResults = new Map();
    this.isPaused = false;
    this.isRunning = false;
    this.agentConfigManager = null;

    this.init();
  }

  async init() {
    // Initialize settings system first
    await this.initializeSettings();
    
    this.setupEventListeners();
    this.renderDataSources();
    this.renderWorkers();
    this.renderPipelineStages();
    this.createWorkflowPipeline();
    
    // Auto-save functionality
    this.loadSavedState();
    setInterval(() => this.saveState(), 5000);
  }

  async initializeSettings() {
    try {
      const initialized = await initializeSettings();
      if (initialized) {
        this.settingsIntegration = getSettingsIntegration();
        this.apiClient = window.apiClient;
        this.initializeConnectionStatus();
        this.agentConfigManager = new AgentConfigManager({
          apiClient: this.apiClient,
          roleSelectorId: 'role-selector',
          systemPromptId: 'system-prompt',
          onStateChange: () => this.saveState()
        });
        await this.agentConfigManager.initialize();
        this.loadSavedState();
      } else {
        // Fallback to default API client
        console.warn('Settings integration failed, using default configuration');
        this.apiClient = new TerraphimApiClient();
        this.initializeConnectionStatus();
      }
    } catch (error) {
      console.error('Failed to initialize settings:', error);
      // Fallback to default API client
      this.apiClient = new TerraphimApiClient();
      this.initializeConnectionStatus();
    }
  }

  setupEventListeners() {
    // Data source selection
    document.addEventListener('click', (e) => {
      if (e.target.closest('.source-card')) {
        const sourceId = e.target.closest('.source-card').dataset.sourceId;
        this.toggleDataSource(sourceId);
      }
    });

    // Input elements
    this.queryInput = document.getElementById('analysis-query');

    // Agent config is managed by AgentConfigManager

    // Control buttons
    this.startButton = document.getElementById('start-pipeline');
    this.pauseButton = document.getElementById('pause-pipeline');
    this.resetButton = document.getElementById('reset-pipeline');

    this.startButton.addEventListener('click', () => this.startOrchestration());
    this.pauseButton.addEventListener('click', () => this.pauseOrchestration());
    this.resetButton.addEventListener('click', () => this.resetOrchestration());

    this.queryInput.addEventListener('input', () => this.analyzeQuery(this.queryInput.value));
  }

  initializeConnectionStatus() {
    if (this.apiClient) {
      this.connectionStatus = new ConnectionStatusComponent('connection-status-container', this.apiClient);
    }
  }

  async initializeRoles() {
    if (!this.apiClient) return;

    try {
      const config = await this.apiClient.getConfig();
      if (config && config.roles) {
        this.roles = config.roles;
        this.populateRoleSelector();
        this.loadSavedState();
      }
    } catch (error) {
      console.error('Failed to load roles:', error);
    }
  }

  populateRoleSelector() {
    this.roleSelector.innerHTML = '';
    for (const roleName in this.roles) {
      const option = document.createElement('option');
      option.value = roleName;
      option.textContent = this.roles[roleName].name || roleName;
      this.roleSelector.appendChild(option);
    }
    this.onRoleChange();
  }

  onRoleChange() {
    const selectedRoleName = this.roleSelector.value;
    const role = this.roles[selectedRoleName];

    if (role && role.extra && role.extra.system_prompt) {
      this.systemPrompt.value = role.extra.system_prompt;
    } else {
      this.systemPrompt.value = 'This role has no default system prompt. You can define one here.';
    }
    this.saveState();
  }

  renderDataSources() {
    const container = document.getElementById('source-grid');
    container.innerHTML = this.dataSources.map(source => `
      <div class="source-card ${this.selectedSources.has(source.id) ? 'selected' : ''}" data-source-id="${source.id}">
        <span class="source-icon">${source.icon}</span>
        <div class="source-details">
          <div class="source-name">${source.name}</div>
          <div class="source-description">${source.description}</div>
        </div>
      </div>
    `).join('');
  }

  renderWorkers() {
    const container = document.getElementById('worker-grid');
    container.innerHTML = this.workers.map(worker => `
      <div class="worker-card" id="worker-${worker.id}">
        <div class="worker-header">
          <span class="worker-icon">${worker.icon}</span>
          <div class="worker-name">${worker.name}</div>
        </div>
        <div class="worker-status" id="status-${worker.id}">Idle</div>
        <div class="worker-progress">
          <div class="worker-progress-fill" id="progress-${worker.id}"></div>
        </div>
      </div>
    `).join('');
  }

  renderPipelineStages() {
    const container = document.getElementById('pipeline-stages');
    container.innerHTML = this.pipelineStages.map((stage, index) => `
      <div class="pipeline-stage" id="stage-${stage.id}">
        <div class="stage-header">
          <div class="stage-title">
            <span class="stage-icon">${stage.icon}</span>
            ${stage.title}
          </div>
          <div class="stage-status pending" id="stage-status-${stage.id}">Pending</div>
        </div>
        <div class="stage-content">
          <div class="stage-description">${stage.description}</div>
          <div class="stage-workers">
            ${stage.workers.map(workerId => {
              const worker = this.workers.find(w => w.id === workerId);
              return `
                <div class="assigned-worker">
                  <div class="worker-task">${worker.icon} ${worker.name}</div>
                  <div class="worker-role">${worker.specialty}</div>
                </div>
              `;
            }).join('')}
          </div>
          <div class="stage-results" id="results-${stage.id}">
            <div class="results-summary" id="summary-${stage.id}"></div>
            <div class="results-details" id="details-${stage.id}"></div>
          </div>
        </div>
      </div>
    `).join('');
  }

  toggleDataSource(sourceId) {
    const card = document.querySelector(`[data-source-id="${sourceId}"]`);
    
    if (this.selectedSources.has(sourceId)) {
      this.selectedSources.delete(sourceId);
      card.classList.remove('selected');
    } else {
      this.selectedSources.add(sourceId);
      card.classList.add('selected');
    }
  }

  analyzeQuery(query) {
    // Real-time query analysis could suggest optimal data sources
    if (query.toLowerCase().includes('medical') || query.toLowerCase().includes('health')) {
      // Could highlight medical sources like PubMed
    }
  }

  createWorkflowPipeline() {
    const steps = [
      { id: 'orchestrate', name: 'Task Orchestration' },
      { id: 'execute', name: 'Worker Execution' },
      { id: 'aggregate', name: 'Result Aggregation' }
    ];
    
    this.visualizer.createPipeline(steps);
    this.visualizer.createProgressBar('progress-container');
  }

  async startOrchestration() {
    const query = this.queryInput.value.trim();
    
    if (!query) {
      alert('Please enter a research query.');
      return;
    }

    if (this.selectedSources.size === 0) {
      alert('Please select at least one data source.');
      return;
    }

    this.isRunning = true;
    this.currentStage = 0;
    this.updateControlsState();
    
    // Update workflow status
    document.getElementById('workflow-status').textContent = 'Orchestrating...';
    document.getElementById('workflow-status').className = 'workflow-status running';
    
    // Hide initial state and show pipeline stages
    document.getElementById('initial-state').style.display = 'none';
    document.getElementById('pipeline-stages').style.display = 'block';
    
    // Reset and setup pipeline
    this.visualizer.updateStepStatus('orchestrate', 'active');
    this.visualizer.updateProgress(10, 'Analyzing research query and planning tasks...');
    
    await this.delay(2000);
    
    // Task orchestration phase
    await this.orchestrateTasks();
    
    this.visualizer.updateStepStatus('orchestrate', 'completed');
    this.visualizer.updateStepStatus('execute', 'active');
    this.visualizer.updateProgress(20, 'Executing pipeline stages with specialized workers...');
    
    // Execute pipeline stages sequentially
    for (let i = 0; i < this.pipelineStages.length; i++) {
      this.currentStage = i;
      await this.executePipelineStage(this.pipelineStages[i]);
      
      const progress = 20 + (i + 1) * (60 / this.pipelineStages.length);
      this.visualizer.updateProgress(progress, `Completed ${this.pipelineStages[i].title}`);
    }
    
    this.visualizer.updateStepStatus('execute', 'completed');
    this.visualizer.updateStepStatus('aggregate', 'active');
    this.visualizer.updateProgress(85, 'Aggregating results and building knowledge graph...');
    
    // Final aggregation and knowledge graph construction
    await this.aggregateResults();
    
    this.visualizer.updateStepStatus('aggregate', 'completed');
    this.visualizer.updateProgress(100, 'Pipeline completed successfully!');
    
    // Update final status
    document.getElementById('workflow-status').textContent = 'Completed';
    document.getElementById('workflow-status').className = 'workflow-status completed';
    
    this.isRunning = false;
    this.updateControlsState();
    
    // Show results and knowledge graph
    this.displayResults();
  }

  async orchestrateTasks() {
    await this.delay(1500);
    
    // Simulate task analysis and worker assignment optimization
    const query = this.queryInput.value;
    const complexity = this.analyzeQueryComplexity(query);

    const agentState = this.agentConfigManager.getState();

    const input = {
      prompt: query,
      dataSources: Array.from(this.selectedSources),
      complexity,
      role: agentState.selectedRole,
      config: {
        system_prompt_override: agentState.systemPrompt
      }
    };

    const enhancedInput = this.settingsIntegration
      ? this.settingsIntegration.enhanceWorkflowInput(input)
      : input;
    
    // Update orchestrator status
    console.log(`Orchestrating tasks for complexity level: ${complexity}`);
  }

  analyzeQueryComplexity(query) {
    // Simple complexity analysis based on query characteristics
    let complexity = 0.5; // base complexity
    
    if (query.length > 100) complexity += 0.2;
    if (query.toLowerCase().includes('machine learning') || query.toLowerCase().includes('ai')) complexity += 0.2;
    if (query.toLowerCase().includes('meta-analysis') || query.toLowerCase().includes('systematic')) complexity += 0.3;
    
    return Math.min(1.0, complexity);
  }

  async executePipelineStage(stage) {
    // Update stage status to active
    document.getElementById(`stage-status-${stage.id}`).textContent = 'Active';
    document.getElementById(`stage-status-${stage.id}`).className = 'stage-status active';
    
    // Activate assigned workers
    stage.workers.forEach(workerId => {
      this.updateWorkerStatus(workerId, 'active', 'Processing...');
    });
    
    // Execute real orchestration workflow with API client
    try {
      const result = await this.apiClient.executeOrchestration({
        prompt: `Execute ${stage.name} stage with workers: ${stage.workers.join(', ')}`,
        role: this.agentConfigManager ? this.agentConfigManager.getState().selectedRole : 'OrchestratorAgent',
        overall_role: 'data_science_pipeline_coordinator',
        config: {
          stage: stage.id,
          workers: stage.workers,
          dataSources: Array.from(this.selectedSources)
        }
      }, {
        realTime: true,
        onProgress: (progress) => {
          stage.workers.forEach(workerId => {
            this.updateWorkerProgress(workerId, progress.percentage || 0);
          });
        }
      });
      
      // Store API result for later use
      this.stageResults.set(stage.id, result.result || result);
    } catch (error) {
      console.error(`Stage ${stage.id} execution failed:`, error);
      // Fallback to basic completion for demo purposes
    }
    
    // Complete workers
    stage.workers.forEach(workerId => {
      this.updateWorkerStatus(workerId, 'completed', 'Completed');
      this.updateWorkerProgress(workerId, 100);
    });
    
    // Update stage status to completed
    document.getElementById(`stage-status-${stage.id}`).textContent = 'Completed';
    document.getElementById(`stage-status-${stage.id}`).className = 'stage-status completed';
    
    // Generate and display stage results
    const results = this.generateStageResults(stage);
    this.displayStageResults(stage.id, results);
    this.stageResults.set(stage.id, results);
    
    // Add delay between stages
    await this.delay(500);
  }

  updateWorkerStatus(workerId, status, statusText) {
    const workerCard = document.getElementById(`worker-${workerId}`);
    const statusElement = document.getElementById(`status-${workerId}`);
    
    if (workerCard) {
      workerCard.className = `worker-card ${status}`;
    }
    
    if (statusElement) {
      statusElement.textContent = statusText;
    }
  }

  updateWorkerProgress(workerId, progress) {
    const progressFill = document.getElementById(`progress-${workerId}`);
    if (progressFill) {
      progressFill.style.width = `${progress}%`;
    }
  }

  generateStageResults(stage) {
    const mockResults = {
      'data_ingestion': {
        summary: 'Successfully collected 247 research papers',
        details: 'Retrieved papers from arXiv (89), PubMed (126), and Semantic Scholar (32). Applied initial filtering based on relevance scores and publication dates. Average relevance: 0.78.'
      },
      'content_analysis': {
        summary: 'Analyzed content and extracted 156 key methodologies',
        details: 'Processed abstracts and identified machine learning approaches (67%), statistical methods (24%), and experimental designs (9%). Extracted 342 unique concepts and 89 research themes.'
      },
      'knowledge_extraction': {
        summary: 'Mapped 284 concept relationships and semantic connections',
        details: 'Built conceptual mappings between research themes, methodologies, and outcomes. Identified 45 core concepts with high centrality scores and 127 secondary concept clusters.'
      },
      'graph_construction': {
        summary: 'Constructed knowledge graph with 312 nodes and 567 edges',
        details: 'Created comprehensive knowledge graph structure with weighted relationships. Applied graph algorithms for community detection and identified 12 major research clusters.'
      },
      'synthesis_insights': {
        summary: 'Generated comprehensive insights and research gaps analysis',
        details: 'Synthesized findings across all pipeline stages. Identified 8 key research trends, 15 promising methodologies, and 23 potential research opportunities for future investigation.'
      }
    };
    
    return mockResults[stage.id] || {
      summary: `Completed ${stage.title}`,
      details: 'Stage execution completed successfully with detailed analysis.'
    };
  }

  displayStageResults(stageId, results) {
    const resultsContainer = document.getElementById(`results-${stageId}`);
    const summaryElement = document.getElementById(`summary-${stageId}`);
    const detailsElement = document.getElementById(`details-${stageId}`);
    
    if (summaryElement) summaryElement.textContent = results.summary;
    if (detailsElement) detailsElement.textContent = results.details;
    if (resultsContainer) resultsContainer.classList.add('visible');
  }

  async aggregateResults() {
    await this.delay(2000);
    
    // Build knowledge graph visualization
    this.buildKnowledgeGraph();
    
    // Show knowledge graph section
    document.getElementById('knowledge-graph').style.display = 'block';
  }

  buildKnowledgeGraph() {
    const graphContainer = document.getElementById('graph-visualization');
    
    // Mock knowledge graph nodes and connections
    const nodes = [
      'Machine Learning', 'Healthcare Outcomes', 'Clinical Trials', 'Predictive Models',
      'Data Analysis', 'Patient Care', 'Diagnostic Accuracy', 'Treatment Efficacy'
    ];
    
    const connections = [
      {
        type: 'Applied to',
        description: 'Machine Learning → Healthcare Outcomes'
      },
      {
        type: 'Validated through',
        description: 'Predictive Models → Clinical Trials'
      },
      {
        type: 'Improves',
        description: 'Data Analysis → Diagnostic Accuracy'
      },
      {
        type: 'Enhances',
        description: 'Healthcare Outcomes → Patient Care'
      }
    ];
    
    graphContainer.innerHTML = `
      <div class="graph-nodes">
        ${nodes.map(node => `<div class="graph-node">${node}</div>`).join('')}
      </div>
      <div class="graph-connections">
        ${connections.map(conn => `
          <div class="connection-item">
            <div class="connection-type">${conn.type}</div>
            <div class="connection-description">${conn.description}</div>
          </div>
        `).join('')}
      </div>
    `;
    
    // Store in knowledge graph map
    nodes.forEach(node => {
      this.knowledgeGraph.set(node, {
        connections: connections.filter(c => c.description.includes(node)),
        weight: 0.8 + Math.random() * 0.2
      });
    });
  }

  displayResults() {
    document.getElementById('results-section').style.display = 'block';
    
    const totalPapers = 247;
    const totalConcepts = 342;
    const totalConnections = 567;
    const executionTime = this.pipelineStages.reduce((sum, stage) => sum + stage.duration, 0);
    
    const metrics = {
      'Papers Processed': totalPapers,
      'Concepts Extracted': totalConcepts,
      'Graph Connections': totalConnections,
      'Pipeline Stages': this.pipelineStages.length,
      'Active Workers': this.workers.length,
      'Data Sources': this.selectedSources.size,
      'Execution Time': `${(executionTime / 1000).toFixed(1)}s`,
      'Knowledge Clusters': '12'
    };
    
    this.visualizer.createMetricsGrid(metrics, 'results-content');
  }

  updateControlsState() {
    this.startButton.disabled = this.isRunning;
    this.pauseButton.disabled = !this.isRunning;
  }

  pauseOrchestration() {
    this.isRunning = false;
    this.updateControlsState();
    document.getElementById('workflow-status').textContent = 'Paused';
    document.getElementById('workflow-status').className = 'workflow-status paused';
  }

  resetOrchestration() {
    // Reset all state
    this.isRunning = false;
    this.currentStage = 0;
    this.stageResults.clear();
    this.knowledgeGraph.clear();
    
    // Reset UI
    document.getElementById('pipeline-stages').style.display = 'none';
    document.getElementById('knowledge-graph').style.display = 'none';
    document.getElementById('results-section').style.display = 'none';
    document.getElementById('initial-state').style.display = 'block';
    
    // Reset workflow status
    document.getElementById('workflow-status').textContent = 'Ready to Process';
    document.getElementById('workflow-status').className = 'workflow-status idle';
    
    // Reset all workers
    this.workers.forEach(worker => {
      this.updateWorkerStatus(worker.id, '', 'Idle');
      this.updateWorkerProgress(worker.id, 0);
    });
    
    // Reset all stages
    this.pipelineStages.forEach(stage => {
      const statusElement = document.getElementById(`stage-status-${stage.id}`);
      if (statusElement) {
        statusElement.textContent = 'Pending';
        statusElement.className = 'stage-status pending';
      }
      
      const resultsContainer = document.getElementById(`results-${stage.id}`);
      if (resultsContainer) {
        resultsContainer.classList.remove('visible');
      }
    });
    
    this.updateControlsState();
    this.visualizer.clear();
    this.createWorkflowPipeline();
  }

  // Utility methods
  delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  saveState() {
    const agentState = this.agentConfigManager ? this.agentConfigManager.getState() : {};
    const state = {
      query: this.queryInput.value,
      activeDataSources: Array.from(this.selectedSources),
      ...agentState
    };
    localStorage.setItem('orchestrator-demo-state', JSON.stringify(state));
  }

  loadSavedState() {
    const saved = localStorage.getItem('orchestrator-demo-state');
    if (saved) {
      try {
        const savedState = JSON.parse(saved);
        this.queryInput.value = savedState.query || '';
        this.selectedSources = new Set(savedState.activeDataSources || []);
        this.renderDataSources();
        this.analyzeQuery(this.queryInput.value);

        if (this.agentConfigManager) {
          this.agentConfigManager.applyState(savedState);
        }
      } catch (error) {
        console.error('Failed to load saved state:', error);
      }
    }
  }
}

// Initialize the demo when DOM is loaded
document.addEventListener('DOMContentLoaded', async () => {
  const demo = new OrchestratorWorkersDemo();
  await demo.init();
});