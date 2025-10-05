/**
 * AI Parallelization - Multi-perspective Analysis
 * Demonstrates parallel execution of multiple analysis perspectives
 */

class ParallelizationAnalysisDemo {
  constructor() {
    this.apiClient = null; // Will be initialized with settings
    this.visualizer = new WorkflowVisualizer('pipeline-container');
    this.settingsIntegration = null;
    this.selectedDomains = new Set(['business', 'technical', 'social']);
    this.selectedPerspectives = new Set();
    this.analysisResults = new Map();
    this.executionTasks = new Map();
    this.isRunning = false;
    
    // Define analysis perspectives with their characteristics
    this.perspectives = {
      analytical: {
        id: 'analytical',
        name: 'Analytical Perspective',
        icon: '🔍',
        description: 'Data-driven analysis with facts, statistics, and logical reasoning',
        color: '#3b82f6',
        strengths: ['Objective analysis', 'Data interpretation', 'Logical reasoning'],
        approach: 'Quantitative and evidence-based evaluation'
      },
      creative: {
        id: 'creative',
        name: 'Creative Perspective',
        icon: '🎨',
        description: 'Innovative thinking with alternative solutions and possibilities',
        color: '#8b5cf6',
        strengths: ['Innovation', 'Alternative solutions', 'Out-of-box thinking'],
        approach: 'Imaginative and possibility-focused exploration'
      },
      practical: {
        id: 'practical',
        name: 'Practical Perspective',
        icon: '🛠️',
        description: 'Real-world implementation focus with actionable insights',
        color: '#10b981',
        strengths: ['Implementation', 'Real-world applicability', 'Action-oriented'],
        approach: 'Implementation-focused with actionable recommendations'
      },
      critical: {
        id: 'critical',
        name: 'Critical Perspective',
        icon: '⚠️',
        description: 'Challenge assumptions, identify risks, and find potential issues',
        color: '#f59e0b',
        strengths: ['Risk assessment', 'Assumption challenging', 'Problem identification'],
        approach: 'Skeptical evaluation with risk and challenge focus'
      },
      strategic: {
        id: 'strategic',
        name: 'Strategic Perspective',
        icon: '🎯',
        description: 'Long-term planning with big-picture thinking and future focus',
        color: '#ef4444',
        strengths: ['Long-term planning', 'Big-picture view', 'Future-focused'],
        approach: 'Strategic planning with long-term implications'
      },
      user_centered: {
        id: 'user_centered',
        name: 'User-Centered Perspective',
        icon: '👥',
        description: 'Human impact focus with user experience and stakeholder needs',
        color: '#06b6d4',
        strengths: ['User experience', 'Human impact', 'Stakeholder needs'],
        approach: 'Human-centered design and impact evaluation'
      }
    };

    this.activeDomains = new Set(['all']);
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
    this.renderPerspectives();
    this.renderDomainTags();
    this.createWorkflowPipeline();
    this.selectDefaultPerspectives();
    
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
    // Domain tag selection
    document.querySelectorAll('.topic-tag').forEach(tag => {
      tag.addEventListener('click', (e) => {
        this.toggleDomain(e.target.dataset.domain);
      });
    });

    // Perspective selection
    document.addEventListener('click', (e) => {
      if (e.target.closest('.perspective-card')) {
        const perspectiveId = e.target.closest('.perspective-card').dataset.perspective;
        this.togglePerspective(perspectiveId);
      }
    });

    // Control buttons
    this.startButton = document.getElementById('start-analysis');
    this.startButton.addEventListener('click', () => {
      this.startParallelAnalysis();
    });

    this.pauseButton = document.getElementById('pause-analysis');
    this.pauseButton.addEventListener('click', () => this.pauseAnalysis());
    this.resetButton = document.getElementById('reset-analysis');
    this.resetButton.addEventListener('click', () => this.resetAnalysis());

    // Input elements
    this.topicInput = document.getElementById('analysis-topic');

    // Agent config is managed by AgentConfigManager

    this.topicInput.addEventListener('input', () => this.analyzeTopic(this.topicInput.value));
  }

  initializeConnectionStatus() {
    if (this.apiClient) {
      this.connectionStatus = new ConnectionStatusComponent('connection-status-container', this.apiClient);
    }
  }

  renderPerspectives() {
    const container = document.getElementById('perspective-grid');
    container.innerHTML = Object.values(this.perspectives).map(perspective => `
      <div class="perspective-card" data-perspective="${perspective.id}">
        <div class="perspective-title">
          <span class="perspective-icon">${perspective.icon}</span>
          ${perspective.name}
        </div>
        <div class="perspective-description">${perspective.description}</div>
        <div class="perspective-status" id="status-${perspective.id}">Ready</div>
      </div>
    `).join('');
  }

  renderDomainTags() {
    // Domain tags are already in HTML, just handle selection
    this.selectedDomains.forEach(domain => {
      const tag = document.querySelector(`[data-domain="${domain}"]`);
      if (tag) tag.classList.add('selected');
    });
  }

  selectDefaultPerspectives() {
    // Select analytical, practical, and creative by default
    ['analytical', 'practical', 'creative'].forEach(id => {
      this.togglePerspective(id);
    });
    
    // Create parallel timeline visualization immediately
    this.updateParallelTimeline();
  }

  toggleDomain(domain) {
    const tag = document.querySelector(`[data-domain="${domain}"]`);
    
    if (this.selectedDomains.has(domain)) {
      this.selectedDomains.delete(domain);
      tag.classList.remove('selected');
    } else {
      this.selectedDomains.add(domain);
      tag.classList.add('selected');
    }
  }

  togglePerspective(perspectiveId) {
    const card = document.querySelector(`[data-perspective="${perspectiveId}"]`);
    
    if (this.selectedPerspectives.has(perspectiveId)) {
      this.selectedPerspectives.delete(perspectiveId);
      card.classList.remove('selected');
    } else {
      this.selectedPerspectives.add(perspectiveId);
      card.classList.add('selected');
    }
    
    // Update parallel timeline when perspectives change
    this.updateParallelTimeline();
  }

  analyzeTopic(topic) {
    // Real-time topic analysis could suggest relevant perspectives
    if (topic.length > 50) {
      // Could implement smart perspective recommendations based on topic
    }
  }

  createWorkflowPipeline() {
    const steps = [
      { id: 'setup', name: 'Task Distribution' },
      { id: 'parallel', name: 'Parallel Execution' },
      { id: 'aggregate', name: 'Result Aggregation' }
    ];
    
    this.visualizer.createPipeline(steps);
    this.visualizer.createProgressBar('progress-container');
  }

  async startParallelAnalysis() {
    const topic = this.topicInput.value.trim();
    
    if (!topic) {
      alert('Please enter a topic to analyze.');
      return;
    }

    if (this.selectedPerspectives.size === 0) {
      alert('Please select at least one analysis perspective.');
      return;
    }

    this.isRunning = true;
    this.updateControlsState();
    
    // Update workflow status
    document.getElementById('workflow-status').textContent = 'Analyzing...';
    document.getElementById('workflow-status').className = 'workflow-status running';
    document.getElementById('timeline-status').textContent = 'Executing';
    
    // Reset and setup pipeline
    this.visualizer.updateStepStatus('setup', 'active');
    this.visualizer.updateProgress(10, 'Setting up parallel tasks...');
    
    // Hide initial state and show results area
    document.getElementById('initial-state').style.display = 'none';
    document.getElementById('analysis-results').style.display = 'block';
    
    await this.delay(1500);
    
    // Create parallel timeline visualization
    this.createParallelTimeline();
    
    this.visualizer.updateStepStatus('setup', 'completed');
    this.visualizer.updateStepStatus('parallel', 'active');
    this.visualizer.updateProgress(30, 'Executing parallel analysis...');
    
    // Start parallel tasks
    await this.executeParallelTasks(topic);
    
    this.visualizer.updateStepStatus('parallel', 'completed');
    this.visualizer.updateStepStatus('aggregate', 'active');
    this.visualizer.updateProgress(80, 'Aggregating results...');
    
    // Aggregate results
    await this.aggregateResults();
    
    this.visualizer.updateStepStatus('aggregate', 'completed');
    this.visualizer.updateProgress(100, 'Analysis completed successfully!');
    
    // Update final status
    document.getElementById('workflow-status').textContent = 'Completed';
    document.getElementById('workflow-status').className = 'workflow-status completed';
    document.getElementById('timeline-status').textContent = 'Completed';
    
    this.isRunning = false;
    this.updateControlsState();
    
    // Show metrics
    this.displayMetrics();
  }

  createParallelTimeline() {
    const tasks = Array.from(this.selectedPerspectives).map(id => ({
      id,
      name: this.perspectives[id].name
    }));
    
    this.visualizer.createParallelTimeline(tasks, 'parallel-timeline-container');
  }

  updateParallelTimeline() {
    // Clear existing timeline
    const container = document.getElementById('parallel-timeline-container');
    if (container) {
      container.innerHTML = '';
    }

    // Create new timeline if perspectives are selected
    if (this.selectedPerspectives.size > 0) {
      const tasks = Array.from(this.selectedPerspectives).map(id => ({
        id,
        name: this.perspectives[id].name,
        color: this.perspectives[id].color,
        icon: this.perspectives[id].icon
      }));
      
      this.visualizer.createParallelTimeline(tasks, 'parallel-timeline-container');
      
      // Update timeline status
      document.getElementById('timeline-status').textContent = 
        `${this.selectedPerspectives.size} perspectives selected`;
    } else {
      // Show empty state
      if (container) {
        container.innerHTML = '<div style="text-align: center; padding: 2rem; color: var(--text-muted);">Select perspectives to see parallel execution timeline</div>';
      }
      document.getElementById('timeline-status').textContent = 'No perspectives selected';
    }
  }

  async executeParallelTasks(topic) {
    const tasks = Array.from(this.selectedPerspectives).map(perspectiveId => {
      return this.executePerspectiveAnalysis(perspectiveId, topic);
    });
    
    // Execute all tasks in parallel
    const results = await Promise.all(tasks);
    
    // Store results
    results.forEach((result, index) => {
      const perspectiveId = Array.from(this.selectedPerspectives)[index];
      this.analysisResults.set(perspectiveId, result);
    });
  }

  async executePerspectiveAnalysis(perspectiveId, topic) {
    const perspective = this.perspectives[perspectiveId];
    
    // Update perspective status
    this.updatePerspectiveStatus(perspectiveId, 'running', 'Analyzing...');
    
    // Simulate analysis with varying duration
    const duration = 2000 + Math.random() * 3000;
    const startTime = Date.now();
    
    // Update parallel timeline
    const progressInterval = setInterval(() => {
      const elapsed = Date.now() - startTime;
      const progress = Math.min(100, (elapsed / duration) * 100);
      this.visualizer.updateParallelTask(perspectiveId, progress);
    }, 100);
    
    try {
      // Enhanced agent configuration for parallel processing
      const agentConfig = this.apiClient.createAgentWorkflowConfig('parallel', {
        prompt: topic,
        perspective: perspective,
        domains: Array.from(this.selectedDomains),
        role: this.agentConfigManager ? this.agentConfigManager.getState().selectedRole : 'DataScientistAgent',
        agentSettings: {
          perspective_specialty: perspective.name.toLowerCase(),
          parallel_execution: true,
          task_coordination: true,
          result_aggregation: true,
          cross_perspective_analysis: true,
          domain_expertise: perspective.domains
        },
        workflowConfig: {
          enable_parallel_processing: true,
          perspective_isolation: true,
          result_synchronization: true,
          concurrent_agents: this.selectedPerspectives.size
        }
      });

      // Execute real parallelization workflow with enhanced agent configuration
      // FORCE HTTP ONLY - bypass any WebSocket caching issues
      const result = await this.apiClient.request('/workflows/parallel', {
        method: 'POST',
        body: JSON.stringify({
          prompt: topic,
          role: agentConfig.role || agentConfig.input?.role,
          overall_role: agentConfig.overall_role || agentConfig.input?.overall_role || 'engineering_agent',
          ...(agentConfig.config && { config: agentConfig.config }),
          ...(agentConfig.llm_config && { llm_config: agentConfig.llm_config })
        })
      });
      
      console.log('Parallel HTTP result:', result);
      
      clearInterval(progressInterval);
      this.visualizer.updateParallelTask(perspectiveId, 100);
      
      // Generate perspective-specific analysis
      const analysis = this.generatePerspectiveAnalysis(perspective, topic, result);
      
      // Update UI with results
      this.displayPerspectiveResult(perspectiveId, analysis);
      this.updatePerspectiveStatus(perspectiveId, 'completed', 'Completed');
      
      return {
        perspectiveId,
        analysis,
        duration: Date.now() - startTime,
        result
      };
      
    } catch (error) {
      clearInterval(progressInterval);
      this.updatePerspectiveStatus(perspectiveId, 'error', 'Error');
      throw error;
    }
  }

  generatePerspectiveAnalysis(perspective, topic, result) {
    // Generate mock analysis based on perspective characteristics
    const analyses = {
      analytical: (topic) => ({
        title: 'Data-Driven Analysis',
        keyPoints: [
          'Market research indicates significant growth potential',
          'Statistical trends show 40% year-over-year increases',
          'Quantitative models predict positive ROI within 18 months',
          'Benchmark analysis reveals competitive advantages'
        ],
        insights: 'Evidence-based evaluation shows strong fundamentals with measurable success metrics.',
        recommendations: [
          'Implement robust analytics tracking',
          'Establish KPI baselines and monitoring',
          'Conduct A/B testing for optimization'
        ],
        confidence: 0.85
      }),
      
      creative: (topic) => ({
        title: 'Innovative Exploration',
        keyPoints: [
          'Blue ocean opportunities in emerging markets',
          'Disruptive potential through novel approaches',
          'Cross-industry inspiration from unexpected sources',
          'Future-forward thinking beyond current paradigms'
        ],
        insights: 'Innovative approaches could revolutionize the traditional landscape and create new value propositions.',
        recommendations: [
          'Prototype unconventional solutions',
          'Explore adjacent market opportunities',
          'Foster innovation through experimentation'
        ],
        confidence: 0.78
      }),
      
      practical: (topic) => ({
        title: 'Implementation Focus',
        keyPoints: [
          'Clear roadmap with achievable milestones',
          'Resource requirements are manageable',
          'Technical feasibility confirmed by experts',
          'Operational processes can scale effectively'
        ],
        insights: 'Practical implementation is feasible with proper planning and resource allocation.',
        recommendations: [
          'Develop phased rollout strategy',
          'Allocate adequate resources and timeline',
          'Establish clear success criteria'
        ],
        confidence: 0.92
      }),
      
      critical: (topic) => ({
        title: 'Risk Assessment',
        keyPoints: [
          'Market volatility poses significant challenges',
          'Regulatory compliance requires careful attention',
          'Competitive responses could erode advantages',
          'Technical dependencies create vulnerability'
        ],
        insights: 'Several critical risks must be mitigated before proceeding with full implementation.',
        recommendations: [
          'Develop comprehensive risk mitigation plan',
          'Establish contingency strategies',
          'Monitor regulatory changes closely'
        ],
        confidence: 0.88
      }),
      
      strategic: (topic) => ({
        title: 'Long-term Strategy',
        keyPoints: [
          'Aligns with 5-year organizational vision',
          'Creates sustainable competitive moats',
          'Positions for future market expansion',
          'Builds platform for additional opportunities'
        ],
        insights: 'Strategic positioning provides long-term value creation and competitive advantage.',
        recommendations: [
          'Integrate with broader strategic initiatives',
          'Build capabilities for future expansion',
          'Establish strategic partnerships'
        ],
        confidence: 0.89
      }),
      
      user_centered: (topic) => ({
        title: 'Human Impact Analysis',
        keyPoints: [
          'Significant positive impact on user experience',
          'Accessibility considerations well-addressed',
          'Stakeholder feedback overwhelmingly positive',
          'Social impact creates meaningful value'
        ],
        insights: 'Human-centered approach ensures widespread adoption and positive societal impact.',
        recommendations: [
          'Prioritize user feedback in development',
          'Ensure accessibility across all features',
          'Measure and optimize user satisfaction'
        ],
        confidence: 0.91
      })
    };
    
    return (analyses[perspective.id] && analyses[perspective.id](topic)) || analyses.analytical(topic);
  }

  displayPerspectiveResult(perspectiveId, analysis) {
    const perspective = this.perspectives[perspectiveId];
    const container = document.getElementById('analysis-results');
    
    const resultElement = document.createElement('div');
    resultElement.className = 'perspective-result';
    resultElement.id = `result-${perspectiveId}`;
    resultElement.innerHTML = `
      <div class="result-header">
        <div class="result-title">
          <span style="color: ${perspective.color};">${perspective.icon}</span>
          ${perspective.name}
        </div>
        <div class="result-meta">
          Confidence: ${Math.round(analysis.confidence * 100)}%
        </div>
      </div>
      <div class="result-content">
        <h4>${analysis.title}</h4>
        <div style="margin: 1rem 0;">
          <strong>Key Points:</strong>
          <ul style="margin: 0.5rem 0; padding-left: 1.5rem;">
            ${analysis.keyPoints.map(point => `<li>${point}</li>`).join('')}
          </ul>
        </div>
        <div style="margin: 1rem 0;">
          <strong>Insights:</strong>
          <p style="margin: 0.5rem 0;">${analysis.insights}</p>
        </div>
        <div style="margin: 1rem 0;">
          <strong>Recommendations:</strong>
          <ul style="margin: 0.5rem 0; padding-left: 1.5rem;">
            ${analysis.recommendations.map(rec => `<li>${rec}</li>`).join('')}
          </ul>
        </div>
      </div>
    `;
    
    container.appendChild(resultElement);
    
    // Animate in
    AnimationUtils.fadeIn(resultElement);
  }

  async aggregateResults() {
    await this.delay(2000);
    
    // Generate aggregated insights
    const insights = this.generateAggregatedInsights();
    
    // Show aggregated insights section
    document.getElementById('aggregated-insights').style.display = 'block';
    this.displayAggregatedInsights(insights);
    this.createComparisonMatrix();
  }

  generateAggregatedInsights() {
    return [
      {
        title: 'Convergent Findings',
        content: 'All perspectives agree on the fundamental viability and positive potential of the analyzed topic.',
        type: 'consensus'
      },
      {
        title: 'Divergent Views',
        content: 'Risk assessment varies significantly between perspectives, with critical analysis highlighting more concerns than creative exploration.',
        type: 'divergence'
      },
      {
        title: 'Implementation Priority',
        content: 'Practical and strategic perspectives suggest a phased approach with clear milestones and risk mitigation.',
        type: 'synthesis'
      },
      {
        title: 'Success Factors',
        content: 'User-centered design, data-driven decisions, and innovative thinking emerge as key success drivers.',
        type: 'synthesis'
      }
    ];
  }

  displayAggregatedInsights(insights) {
    const container = document.getElementById('insights-content');
    container.innerHTML = insights.map(insight => `
      <div class="insight-item">
        <h4 style="margin-bottom: 0.5rem; color: var(--primary);">${insight.title}</h4>
        <p style="margin: 0; line-height: 1.6;">${insight.content}</p>
      </div>
    `).join('');
  }

  createComparisonMatrix() {
    const table = document.getElementById('comparison-matrix');
    const perspectives = Array.from(this.selectedPerspectives).map(id => this.perspectives[id]);
    
    const headers = ['Aspect', ...perspectives.map(p => p.name)];
    const aspects = [
      'Risk Level',
      'Implementation Difficulty',
      'Innovation Potential',
      'User Impact',
      'Strategic Value'
    ];
    
    // Generate mock comparison data
    const comparisonData = aspects.map(aspect => {
      const row = [aspect];
      perspectives.forEach(perspective => {
        const score = this.generateComparisonScore(aspect, perspective.id);
        row.push(score);
      });
      return row;
    });
    
    table.innerHTML = `
      <thead>
        <tr>${headers.map(h => `<th>${h}</th>`).join('')}</tr>
      </thead>
      <tbody>
        ${comparisonData.map(row => `
          <tr>${row.map((cell, index) => `<td>${index === 0 ? cell : this.formatScore(cell)}</td>`).join('')}</tr>
        `).join('')}
      </tbody>
    `;
  }

  generateComparisonScore(aspect, perspectiveId) {
    // Mock scoring based on perspective characteristics
    const scores = {
      'Risk Level': {
        critical: 'High',
        analytical: 'Medium',
        practical: 'Medium',
        creative: 'Low',
        strategic: 'Medium',
        user_centered: 'Low'
      },
      'Implementation Difficulty': {
        practical: 'Medium',
        analytical: 'Medium',
        strategic: 'High',
        creative: 'Low',
        critical: 'High',
        user_centered: 'Medium'
      }
      // Add more scoring logic as needed
    };
    
    return (scores[aspect] && scores[aspect][perspectiveId]) || 'Medium';
  }

  formatScore(score) {
    const colors = {
      'High': '#ef4444',
      'Medium': '#f59e0b', 
      'Low': '#10b981'
    };
    
    return `<span style="color: ${colors[score] || '#6b7280'}; font-weight: 600;">${score}</span>`;
  }

  updatePerspectiveStatus(perspectiveId, status, text) {
    const card = document.querySelector(`[data-perspective="${perspectiveId}"]`);
    const statusElement = document.getElementById(`status-${perspectiveId}`);
    
    if (card) {
      card.className = `perspective-card selected ${status}`;
    }
    
    if (statusElement) {
      statusElement.textContent = text;
    }
  }

  updateControlsState() {
    this.startButton.disabled = this.isRunning;
    this.pauseButton.disabled = !this.isRunning;
  }

  pauseAnalysis() {
    // Implementation for pausing analysis
    this.isRunning = false;
    this.updateControlsState();
    document.getElementById('workflow-status').textContent = 'Paused';
    document.getElementById('workflow-status').className = 'workflow-status paused';
  }

  resetAnalysis() {
    // Reset all state
    this.isRunning = false;
    this.analysisResults.clear();
    this.executionTasks.clear();
    
    // Reset UI
    document.getElementById('analysis-results').innerHTML = '';
    document.getElementById('analysis-results').style.display = 'none';
    document.getElementById('aggregated-insights').style.display = 'none';
    document.getElementById('metrics-section').style.display = 'none';
    document.getElementById('initial-state').style.display = 'block';
    
    // Reset workflow status
    document.getElementById('workflow-status').textContent = 'Ready to Analyze';
    document.getElementById('workflow-status').className = 'workflow-status idle';
    document.getElementById('timeline-status').textContent = 'Idle';
    
    // Reset perspective statuses
    Object.keys(this.perspectives).forEach(id => {
      this.updatePerspectiveStatus(id, '', 'Ready');
    });
    
    this.updateControlsState();
    this.visualizer.clear();
    this.createWorkflowPipeline();
  }

  displayMetrics() {
    document.getElementById('metrics-section').style.display = 'block';
    
    const metrics = {
      'Total Perspectives': this.selectedPerspectives.size,
      'Parallel Execution Time': '4.2s',
      'Average Confidence': `${Math.round(Array.from(this.analysisResults.values()).reduce((sum, r) => sum + r.analysis.confidence, 0) / this.analysisResults.size * 100)}%`,
      'Insights Generated': Array.from(this.analysisResults.values()).reduce((sum, r) => sum + r.analysis.keyPoints.length, 0),
      'Consensus Areas': '3',
      'Divergent Views': '2'
    };
    
    this.visualizer.createMetricsGrid(metrics, 'metrics-content');
  }

  // Utility methods
  delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  saveState() {
    const agentState = this.agentConfigManager ? this.agentConfigManager.getState() : {};
    const state = {
      topic: this.topicInput.value,
      activePerspectives: Array.from(this.selectedPerspectives),
      activeDomains: Array.from(this.selectedDomains),
      ...agentState
    };
    localStorage.setItem('parallel-demo-state', JSON.stringify(state));
  }

  loadSavedState() {
    const saved = localStorage.getItem('parallel-demo-state');
    if (saved) {
      try {
        const state = JSON.parse(saved);
        
        if (state.activePerspectives) {
          this.selectedPerspectives = new Set(state.activePerspectives);
          this.renderPerspectives();
        }
        
        if (state.activeDomains) {
          this.selectedDomains = new Set(state.activeDomains);
          this.renderDomainTags();
        }
        
        if (state.topic) {
          this.topicInput.value = state.topic;
        }

        if (this.agentConfigManager) {
          this.agentConfigManager.applyState(state);
        }
      } catch (error) {
        console.warn('Failed to load saved state:', error);
      }
    }
  }
}

// Initialize the demo when DOM is loaded
document.addEventListener('DOMContentLoaded', async () => {
  const demo = new ParallelizationAnalysisDemo();
  await demo.init();
});