/**
 * AI Evaluator-Optimizer - Content Generation Studio
 * Demonstrates iterative content improvement through evaluation and optimization cycles
 */

class EvaluatorOptimizerDemo {
  constructor() {
    this.apiClient = null; // Will be initialized with settings
    this.visualizer = new WorkflowVisualizer('pipeline-container');
    this.settingsIntegration = null;
    this.isOptimizing = false;
    this.currentIteration = 0;
    this.maxIterations = 5;
    this.qualityThreshold = 85;
    this.learningRate = 0.3;
    this.contentVersions = [];
    this.qualityHistory = [];
    this.bestVersion = null;
    
    // Terraphim role configuration - will be updated by agent config manager
    this.terraphimConfig = {
      overallRole: 'TechnicalWriter', // Default overall role for the workflow
      agentRoles: {
        generator: 'TechnicalWriter',
        evaluator: 'QAEngineer', 
        optimizer: 'QAEngineer'
      },
      availableRoles: [
        'content_creator',
        'creative_writer', 
        'content_critic',
        'content_editor',
        'technical_writer',
        'marketing_specialist',
        'academic_researcher',
        'copy_editor'
      ]
    };
    
    // Define quality criteria with weights
    this.qualityCriteria = [
      {
        id: 'clarity',
        name: 'Clarity',
        weight: 25,
        description: 'How clear and understandable the content is',
        enabled: true
      },
      {
        id: 'engagement',
        name: 'Engagement', 
        weight: 20,
        description: 'How engaging and interesting for target audience',
        enabled: true
      },
      {
        id: 'accuracy',
        name: 'Accuracy',
        weight: 20,
        description: 'Factual correctness and reliable information',
        enabled: true
      },
      {
        id: 'structure',
        name: 'Structure',
        weight: 15,
        description: 'Logical organization and flow',
        enabled: true
      },
      {
        id: 'tone',
        name: 'Tone',
        weight: 10,
        description: 'Appropriate voice and style for audience',
        enabled: true
      },
      {
        id: 'completeness',
        name: 'Completeness',
        weight: 10,
        description: 'Coverage of all required topics',
        enabled: true
      }
    ];
  }

  async init() {
    // Initialize settings system first
    await this.initializeSettings();
    
    this.setupEventListeners();
    this.renderQualityCriteria();
    this.renderCurrentMetrics();
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
    // Generation controls
    document.getElementById('generate-btn').addEventListener('click', () => {
      this.generateInitialContent();
    });

    document.getElementById('optimize-btn').addEventListener('click', () => {
      this.startOptimization();
    });

    document.getElementById('stop-btn').addEventListener('click', () => {
      this.stopOptimization();
    });

    document.getElementById('reset-btn').addEventListener('click', () => {
      this.resetOptimization();
    });

    // Quality criteria toggles
    document.addEventListener('click', (e) => {
      if (e.target.closest('.criterion-card')) {
        const criterionId = e.target.closest('.criterion-card').dataset.criterionId;
        this.toggleCriterion(criterionId);
      }
    });

    // Iteration timeline clicks
    document.addEventListener('click', (e) => {
      if (e.target.closest('.iteration-node')) {
        const iteration = parseInt(e.target.closest('.iteration-node').dataset.iteration);
        this.showIteration(iteration);
      }
    });

    // Real-time prompt analysis
    const promptInput = document.getElementById('content-prompt');
    promptInput.addEventListener('input', () => {
      this.analyzePrompt(promptInput.value);
    });

    // Input elements
    this.promptInput = document.getElementById('generation-prompt');

    // Agent config is managed by AgentConfigManager

    // Control buttons
    this.generateButton = document.getElementById('generate-btn');
    this.optimizeButton = document.getElementById('optimize-btn');
    this.stopButton = document.getElementById('stop-btn');
    this.resetButton = document.getElementById('reset-btn');

    this.promptInput.addEventListener('input', () => this.analyzePrompt(this.promptInput.value));
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

  renderQualityCriteria() {
    const container = document.getElementById('criteria-grid');
    container.innerHTML = this.qualityCriteria.map(criterion => `
      <div class="criterion-card ${criterion.enabled ? 'selected' : ''}" data-criterion-id="${criterion.id}">
        <div class="criterion-header">
          <span class="criterion-name">${criterion.name}</span>
          <span class="criterion-weight">${criterion.weight}%</span>
        </div>
        <div style="font-size: 0.75rem; color: var(--text-muted);">${criterion.description}</div>
      </div>
    `).join('');
  }

  renderCurrentMetrics() {
    const container = document.getElementById('current-quality-metrics');
    const metrics = this.currentIteration > 0 ? 
      this.contentVersions[this.currentIteration - 1] && this.contentVersions[this.currentIteration - 1].qualityScores : 
      this.getDefaultMetrics();
    
    container.innerHTML = Object.entries(metrics).map(([key, value]) => `
      <div class="metric-card">
        <div class="metric-value">${Math.round(value)}%</div>
        <div class="metric-label">${this.formatMetricLabel(key)}</div>
      </div>
    `).join('');
  }

  getDefaultMetrics() {
    const metrics = {};
    this.qualityCriteria.forEach(criterion => {
      metrics[criterion.id] = 0;
    });
    metrics.overall = 0;
    return metrics;
  }

  formatMetricLabel(key) {
    if (key === 'overall') return 'Overall';
    const criterion = this.qualityCriteria.find(c => c.id === key);
    return criterion ? criterion.name : key;
  }

  toggleCriterion(criterionId) {
    const criterion = this.qualityCriteria.find(c => c.id === criterionId);
    if (criterion) {
      criterion.enabled = !criterion.enabled;
      this.renderQualityCriteria();
    }
  }

  analyzePrompt(prompt) {
    // Could provide real-time suggestions for optimization based on prompt
    if (prompt.length > 100) {
      // Suggest relevant quality criteria
    }
  }

  createWorkflowPipeline() {
    const steps = [
      { id: 'generate', name: 'Content Generation' },
      { id: 'evaluate', name: 'Quality Evaluation' },
      { id: 'optimize', name: 'Iterative Optimization' }
    ];
    
    this.visualizer.createPipeline(steps);
    this.visualizer.createProgressBar('progress-container');
  }

  async generateInitialContent() {
    const prompt = document.getElementById('content-prompt').value.trim();
    
    if (!prompt) {
      alert('Please enter a content brief.');
      return;
    }

    // Update workflow status
    document.getElementById('workflow-status').textContent = 'Generating...';
    document.getElementById('workflow-status').className = 'workflow-status running';
    
    // Reset state
    this.currentIteration = 0;
    this.contentVersions = [];
    this.qualityHistory = [];
    this.bestVersion = null;
    
    // Hide initial state
    document.getElementById('initial-state').style.display = 'none';
    
    // Setup pipeline
    this.visualizer.updateStepStatus('generate', 'active');
    this.visualizer.updateProgress(20, 'Generating initial content...');
    
    try {
      // Generate initial content
      const initialContent = await this.generateContent(prompt, null);
      this.currentIteration = 1;
      
      this.visualizer.updateStepStatus('generate', 'completed');
      this.visualizer.updateStepStatus('evaluate', 'active');
      this.visualizer.updateProgress(50, 'Evaluating content quality...');
      
      // Evaluate initial content
      const qualityScores = await this.evaluateContent(initialContent, prompt);
      
      // Store version
      const version = {
        iteration: 1,
        content: initialContent,
        qualityScores: qualityScores,
        feedback: this.generateFeedback(qualityScores),
        timestamp: new Date()
      };
      
      this.contentVersions.push(version);
      this.qualityHistory.push(qualityScores.overall);
      this.bestVersion = version;
      
      this.visualizer.updateStepStatus('evaluate', 'completed');
      this.visualizer.updateProgress(100, 'Initial content generated and evaluated!');
      
      // Update UI
      this.renderContentVersion(version);
      this.renderIterationHistory();
      this.renderCurrentMetrics();
      this.updateBestVersionInfo();
      
      // Enable optimization
      document.getElementById('optimize-btn').disabled = false;
      document.getElementById('workflow-status').textContent = 'Ready to Optimize';
      document.getElementById('workflow-status').className = 'workflow-status ready';
      
    } catch (error) {
      console.error('Generation failed:', error);
      document.getElementById('workflow-status').textContent = 'Error';
      document.getElementById('workflow-status').className = 'workflow-status error';
    }
  }

  async startOptimization() {
    if (this.contentVersions.length === 0) {
      alert('Please generate initial content first.');
      return;
    }

    this.isOptimizing = true;
    this.updateControlsState();
    
    // Update workflow status
    document.getElementById('workflow-status').textContent = 'Optimizing...';
    document.getElementById('workflow-status').className = 'workflow-status running';
    
    this.visualizer.updateStepStatus('optimize', 'active');
    
    try {
      // Run optimization iterations
      while (this.currentIteration < this.maxIterations && this.isOptimizing) {
        const currentBestScore = this.bestVersion.qualityScores.overall;
        
        // Check if quality threshold is met
        if (currentBestScore >= this.qualityThreshold) {
          break;
        }
        
        await this.performOptimizationIteration();
        
        const progress = 70 + (this.currentIteration / this.maxIterations) * 30;
        this.visualizer.updateProgress(progress, `Optimization iteration ${this.currentIteration}...`);
        
        // Brief delay between iterations
        await this.delay(1000);
      }
      
      this.visualizer.updateStepStatus('optimize', 'completed');
      this.visualizer.updateProgress(100, 'Optimization completed!');
      
      // Show results
      this.displayOptimizationResults();
      
    } catch (error) {
      console.error('Optimization failed:', error);
      this.visualizer.updateStepStatus('optimize', 'error');
    } finally {
      this.isOptimizing = false;
      this.updateControlsState();
      
      document.getElementById('workflow-status').textContent = 'Completed';
      document.getElementById('workflow-status').className = 'workflow-status completed';
    }
  }

  async performOptimizationIteration() {
    this.currentIteration++;
    
    // Get previous version and feedback
    const previousVersion = this.contentVersions[this.contentVersions.length - 1];
    const optimizationPrompt = this.buildOptimizationPrompt(previousVersion);
    
    try {
      // Execute real optimization workflow with API client
      // FORCE HTTP ONLY - bypass any WebSocket caching issues
      const result = await this.apiClient.request('/workflows/optimize', {
        method: 'POST',
        body: JSON.stringify({
          prompt: document.getElementById('content-prompt').value,
          role: this.agentConfigManager ? this.agentConfigManager.getState().selectedRole : 'QAEngineer',
          overall_role: this.terraphimConfig.overallRole,
          config: {
            previousContent: previousVersion.content,
            optimizationPrompt: optimizationPrompt,
            iteration: this.currentIteration,
            qualityThreshold: this.qualityThreshold,
            criteria: this.qualityCriteria.filter(c => c.enabled)
          }
        })
      });
      
      console.log('Optimization HTTP result:', result);
      
      // Extract improved content from API result
      const improvedContent = (result.result && result.result.optimized_content) || (result.result && result.result.final_result) || 'Generated improved content';
      const qualityScores = (result.result && result.result.quality_metrics) || await this.evaluateContent(improvedContent, document.getElementById('content-prompt').value);
      
      // Create new version with API results
      const version = {
        iteration: this.currentIteration,
        content: improvedContent,
        qualityScores: qualityScores,
        feedback: this.generateFeedback(qualityScores),
        improvements: this.identifyImprovements(previousVersion.qualityScores, qualityScores),
        apiResult: result.result // Store full API result
      };
      
      this.contentVersions.push(version);
      this.qualityHistory.push(qualityScores.overall);
    } catch (error) {
      console.error('API optimization failed, falling back to simulation:', error);
      
      // Fallback to original simulation if API fails
      const improvedContent = await this.generateContent(
        document.getElementById('content-prompt').value,
        optimizationPrompt
      );
      
      const qualityScores = await this.evaluateContent(
        improvedContent, 
        document.getElementById('content-prompt').value
      );
      
      const version = {
        iteration: this.currentIteration,
        content: improvedContent,
        qualityScores: qualityScores,
        feedback: this.generateFeedback(qualityScores),
        improvements: this.identifyImprovements(previousVersion.qualityScores, qualityScores),
        timestamp: new Date()
      };
    
      this.contentVersions.push(version);
      this.qualityHistory.push(qualityScores.overall);
    }
    
    // Update best version if this is better
    if (qualityScores.overall > this.bestVersion.qualityScores.overall) {
      this.bestVersion = version;
      this.updateBestVersionInfo();
    }
    
    // Update UI
    this.renderContentVersion(version);
    this.renderIterationHistory();
    this.renderCurrentMetrics();
    this.renderOptimizationChart();
  }

  buildOptimizationPrompt(previousVersion) {
    const weakestCriteria = this.identifyWeakestCriteria(previousVersion.qualityScores);
    const feedback = previousVersion.feedback.negative.slice(0, 2); // Top 2 issues
    
    return {
      focusAreas: weakestCriteria,
      specificFeedback: feedback,
      targetImprovements: this.calculateTargetImprovements(previousVersion.qualityScores)
    };
  }

  identifyWeakestCriteria(qualityScores) {
    return this.qualityCriteria
      .filter(c => c.enabled)
      .sort((a, b) => qualityScores[a.id] - qualityScores[b.id])
      .slice(0, 2)
      .map(c => c.id);
  }

  calculateTargetImprovements(qualityScores) {
    const improvements = {};
    this.qualityCriteria.forEach(criterion => {
      if (criterion.enabled) {
        const currentScore = qualityScores[criterion.id];
        const targetImprovement = Math.min(10, (85 - currentScore) * this.learningRate);
        improvements[criterion.id] = currentScore + targetImprovement;
      }
    });
    return improvements;
  }

  async generateContent(prompt, optimizationContext = null) {
    // Use terraphim role for content generation
    const role = optimizationContext ? 
      this.terraphimConfig.agentRoles.optimizer : 
      this.terraphimConfig.agentRoles.generator;
    
    // Simulate API call to terraphim with role configuration
    const terraphimRequest = {
      role: role,
      overallRole: this.terraphimConfig.overallRole,
      prompt: prompt,
      context: optimizationContext,
      workflow: 'evaluator-optimizer'
    };
    
    // Simulate content generation with realistic delay
    await this.delay(2000 + Math.random() * 2000);
    
    // Generate mock content based on prompt and role context
    const baseContent = this.generateMockContentWithRole(prompt, role);
    
    if (optimizationContext) {
      return this.applyOptimizations(baseContent, optimizationContext, role);
    }
    
    return baseContent;
  }

  generateMockContentWithRole(prompt, role) {
    // Generate content based on terraphim role specialization
    const roleSpecializations = {
      'creative_writer': () => this.generateCreativeContent(prompt),
      'content_critic': () => this.generateAnalyticalContent(prompt),
      'content_editor': () => this.generateEditorialContent(prompt),
      'technical_writer': () => this.generateTechnicalContent(prompt),
      'marketing_specialist': () => this.generateMarketingCopy(prompt),
      'academic_researcher': () => this.generateAcademicContent(prompt),
      'copy_editor': () => this.generatePolishedContent(prompt)
    };
    
    const generator = roleSpecializations[role] || (() => this.generateGenericContent(prompt));
    return generator();
  }

  generateMockContent(prompt) {
    // Legacy method - now uses role-based generation
    return this.generateMockContentWithRole(prompt, this.terraphimConfig.agentRoles.generator);
  }

  generateBlogPost(prompt) {
    return `# The Future of Sustainable Technology: Transforming Our World

In an era where environmental consciousness meets technological innovation, sustainable technology emerges as the cornerstone of our planet's future. From renewable energy breakthroughs to smart city initiatives, we're witnessing a revolutionary shift that promises to reshape how we interact with our environment.

## Renewable Energy Innovations

The renewable energy sector is experiencing unprecedented growth, driven by technological advances that make clean energy more efficient and cost-effective than ever before. Solar panel efficiency has increased by 40% in the past decade, while wind turbine technology continues to evolve with larger, more efficient designs.

## Smart Cities: The Urban Revolution

Smart cities represent the convergence of technology and sustainability, creating urban environments that adapt and respond to their inhabitants' needs while minimizing environmental impact. These cities leverage IoT sensors, AI-driven analytics, and sustainable infrastructure to optimize resource usage.

## Green Manufacturing Processes

Manufacturing industries are embracing green processes that reduce waste, conserve energy, and minimize environmental footprint. Advanced materials, circular economy principles, and automated systems are driving this transformation.

## Looking Ahead

The future of sustainable technology holds immense promise. As we continue to innovate and implement these solutions, we move closer to a world where technology and nature coexist in harmony.`;
  }

  generateArticle(prompt) {
    return `Understanding Sustainable Technology in the Modern Era

Sustainable technology represents more than just an environmental initiative—it's a comprehensive approach to innovation that considers long-term ecological impact alongside immediate functionality. This field encompasses renewable energy systems, efficient manufacturing processes, and smart infrastructure designed to minimize resource consumption while maximizing output.

Recent developments in this sector demonstrate significant progress. Solar energy costs have decreased by 70% over the past decade, making renewable energy increasingly competitive with traditional fossil fuels. Similarly, advances in battery technology and energy storage solutions are addressing the intermittency challenges that have historically limited renewable energy adoption.

The integration of artificial intelligence and machine learning into sustainable systems has opened new possibilities for optimization. Smart grids can now predict energy demand patterns and adjust renewable energy distribution accordingly, while AI-powered manufacturing systems can identify and eliminate waste in real-time.

As we look toward the future, the convergence of sustainability and technology promises to address some of humanity's most pressing challenges while creating new opportunities for economic growth and innovation.`;
  }

  generateMarketingCopy(prompt) {
    return `🌱 Embrace the Future with Sustainable Technology

Transform your business with cutting-edge sustainable solutions that deliver results while protecting our planet. Our innovative approach combines environmental responsibility with technological excellence.

✅ Reduce operational costs by up to 40%
✅ Minimize environmental footprint
✅ Future-proof your business operations
✅ Access government incentives and tax benefits

Join thousands of forward-thinking companies making the switch to sustainable technology. The future is green, efficient, and profitable.

Ready to make the change? Contact us today for a free sustainability assessment.`;
  }

  generateGenericContent(prompt) {
    return `Exploring the Topic: ${prompt.split(' ').slice(0, 5).join(' ')}

This comprehensive analysis examines the key aspects and implications of the subject matter, providing insights and perspectives that contribute to a deeper understanding.

The current landscape presents both opportunities and challenges that require careful consideration and strategic thinking. Through systematic analysis and evidence-based evaluation, we can identify patterns and trends that inform decision-making processes.

Key findings suggest that multiple factors contribute to the complexity of this topic, each requiring specialized attention and tailored approaches. The interconnected nature of these elements creates a dynamic environment where change and adaptation are constant requirements.

Moving forward, continued research and development will be essential to address emerging challenges and capitalize on new opportunities. This ongoing process requires collaboration between stakeholders and commitment to evidence-based solutions.`;
  }

  // Role-based content generation methods
  generateCreativeContent(prompt) {
    return `# ${prompt.split(' ').slice(0, 4).join(' ')}: A Creative Exploration

Imagine a world where innovation meets possibility, where every challenge becomes an opportunity for transformation. This isn't just another analysis—it's a journey into the heart of what makes our topic truly extraordinary.

## The Creative Lens

Picture this: You're standing at the crossroads of tradition and revolution. What do you see? Not just data points and statistics, but stories waiting to be told, connections waiting to be made, and futures waiting to be created.

## Reimagining Possibilities

What if we approached this differently? What if instead of asking "how do we solve this," we asked "how do we reimagine this entirely?" This shift in perspective opens doors to solutions that conventional thinking might never discover.

## The Human Connection

At its core, every great innovation speaks to something deeply human. It addresses not just our needs, but our dreams, our aspirations, and our desire to create something meaningful that resonates across time and space.

The future belongs to those who dare to think differently, to those who see possibilities where others see problems, and to those who understand that creativity isn't just about making things beautiful—it's about making them profoundly impactful.`;
  }

  generateAnalyticalContent(prompt) {
    return `## Critical Analysis: ${prompt.split(' ').slice(0, 5).join(' ')}

### Executive Summary
This analytical framework examines the multifaceted dimensions of the subject matter through systematic evaluation and evidence-based assessment.

### Key Performance Indicators
- **Relevance Score**: 8.5/10 - High alignment with current market demands
- **Implementation Feasibility**: 7.2/10 - Moderate complexity with clear pathways
- **Risk Assessment**: Medium - Manageable risks with proper mitigation strategies

### Methodological Approach
Our analysis employs a mixed-methods approach combining quantitative metrics with qualitative insights. Data sources include peer-reviewed research, industry reports, and stakeholder interviews.

### Critical Findings
1. **Primary Factor**: Market dynamics show 23% growth trajectory over 18 months
2. **Secondary Factor**: Technological readiness index indicates 78% preparedness
3. **Risk Factors**: Regulatory uncertainty poses 15% implementation risk

### Recommendations
Based on comprehensive analysis, we recommend a phased implementation approach with continuous monitoring and adaptive strategy refinement.`;
  }

  generateEditorialContent(prompt) {
    return `# ${prompt.split(' ').slice(0, 4).join(' ')}: An Editorial Perspective

In today's rapidly evolving landscape, the question isn't whether change is coming—it's whether we're prepared to embrace it intelligently and purposefully.

## Setting the Context

The conversation around this topic has reached a critical juncture. Stakeholders across industries are recognizing that traditional approaches may no longer suffice in addressing contemporary challenges.

## The Editorial Position

This publication believes that sustainable progress requires both bold vision and practical implementation. We advocate for solutions that balance innovation with responsibility, efficiency with ethics, and growth with sustainability.

## Key Considerations

**First**, we must acknowledge the complexity of the current situation while maintaining focus on actionable outcomes.

**Second**, stakeholder alignment is essential—success requires collaboration across traditional boundaries.

**Third**, measurement and accountability frameworks must be established from the outset to ensure meaningful progress.

## Looking Forward

The path ahead requires thoughtful leadership, strategic investment, and unwavering commitment to excellence. The decisions we make today will shape the landscape for generations to come.

*This editorial reflects our commitment to fostering informed dialogue and evidence-based decision-making in an increasingly complex world.*`;
  }

  generateTechnicalContent(prompt) {
    return `# Technical Documentation: ${prompt.split(' ').slice(0, 5).join(' ')}

## Overview
This technical specification outlines the core components, implementation requirements, and operational parameters for the subject system.

## System Architecture

### Core Components
- **Processing Module**: Handles primary computational tasks with 99.7% uptime SLA
- **Data Layer**: Implements redundant storage with automatic failover capabilities  
- **Interface Layer**: RESTful API with OAuth 2.0 authentication and rate limiting
- **Monitoring System**: Real-time metrics collection and alerting infrastructure

### Technical Requirements
- **CPU**: Minimum 4 cores, 2.4GHz+ recommended
- **Memory**: 16GB RAM minimum, 32GB for production environments
- **Storage**: SSD recommended, minimum 100GB available space
- **Network**: 1Gbps connection with sub-50ms latency requirements

## Implementation Guidelines

### Configuration Parameters
\`\`\`
max_connections: 1000
timeout_threshold: 30s
retry_attempts: 3
backup_frequency: hourly
\`\`\`

### Security Considerations
- All data transmission encrypted using TLS 1.3
- Role-based access control with granular permissions
- Audit logging for all system interactions
- Regular security vulnerability assessments

## Troubleshooting
Common issues and their resolutions are documented in the operational runbook. Contact technical support for escalation procedures.`;
  }

  generateAcademicContent(prompt) {
    return `# Academic Research: ${prompt.split(' ').slice(0, 5).join(' ')}

## Abstract
This research investigates the contemporary implications and theoretical frameworks surrounding the subject matter, contributing to the existing body of knowledge through systematic inquiry and empirical analysis.

## Introduction
The academic literature reveals significant gaps in our understanding of this phenomenon, necessitating rigorous investigation through established research methodologies (Smith et al., 2023; Johnson, 2022).

## Literature Review
Previous studies have established foundational principles (Brown & Davis, 2021), while recent research has expanded our conceptual framework (Wilson et al., 2023). This study builds upon these contributions while addressing identified limitations.

## Methodology
A mixed-methods approach was employed, combining quantitative analysis (n=247) with qualitative interviews (n=15). Data collection followed IRB-approved protocols with informed consent procedures.

## Findings
Statistical analysis revealed significant correlations (p<0.05) between primary variables, while thematic analysis of qualitative data identified three major patterns:

1. **Pattern A**: Structural relationships and dependencies
2. **Pattern B**: Behavioral adaptations and responses  
3. **Pattern C**: Systemic implications and outcomes

## Discussion
These findings contribute to theoretical understanding while providing practical implications for practitioners and policymakers. Future research should explore longitudinal effects and cross-cultural variations.

## References
- Brown, A. & Davis, C. (2021). *Foundational Studies in Contemporary Theory*. Academic Press.
- Johnson, M. (2022). Recent developments and future directions. *Journal of Applied Research*, 45(3), 123-145.
- Smith, R., et al. (2023). Systematic review and meta-analysis. *Research Quarterly*, 78(2), 234-256.`;
  }

  generatePolishedContent(prompt) {
    return `# ${prompt.split(' ').slice(0, 4).join(' ')}: A Comprehensive Guide

In an era defined by rapid transformation and unprecedented opportunity, understanding this subject has never been more crucial for informed decision-making and strategic planning.

## Executive Overview

This comprehensive analysis provides stakeholders with essential insights, practical recommendations, and actionable frameworks designed to navigate complexity while maximizing value creation.

## Key Insights

**Strategic Implications**
The current landscape presents unique opportunities for organizations willing to embrace innovative approaches while maintaining operational excellence.

**Operational Excellence**
Best practices demonstrate that success requires careful attention to both strategic vision and tactical execution, supported by robust measurement systems.

**Future Readiness**
Organizations that invest in building adaptive capabilities today will be best positioned to capitalize on emerging opportunities tomorrow.

## Recommended Actions

1. **Immediate Steps**: Conduct comprehensive assessment and establish baseline metrics
2. **Short-term Initiatives**: Implement pilot programs with clear success criteria  
3. **Long-term Strategy**: Develop sustainable competitive advantages through continuous innovation

## Conclusion

Success in this domain requires thoughtful strategy, disciplined execution, and unwavering commitment to excellence. Organizations that embrace this comprehensive approach will create lasting value for all stakeholders.

*This guide represents best practices compiled from industry leaders and validated through extensive research and practical application.*`;
  }

  // Role-based evaluation method
  evaluateCriterionWithRole(content, originalPrompt, criterion, evaluatorRole) {
    // Enhanced evaluation logic based on evaluator role
    let baseScore = 60 + Math.random() * 25; // Base score 60-85
    
    // Role-specific evaluation adjustments
    if (evaluatorRole === 'content_critic') {
      // More stringent evaluation
      baseScore -= 5;
      
      // Additional criteria for critical evaluation
      if (criterion.id === 'accuracy' && !content.includes('research') && !content.includes('data')) {
        baseScore -= 8;
      }
    } else if (evaluatorRole === 'copy_editor') {
      // Focus on technical writing quality
      if (criterion.id === 'clarity' && content.includes('utilize')) baseScore -= 10;
      if (criterion.id === 'structure' && content.includes('#')) baseScore += 10;
    }
    
    // Original criterion-specific logic
    switch (criterion.id) {
      case 'clarity':
        if (content.includes('utilize') || content.includes('numerous')) baseScore -= 5;
        if (content.includes('clear') || content.includes('simple')) baseScore += 5;
        break;
        
      case 'engagement':
        if (content.includes('?') || content.includes('!')) baseScore += 5;
        if (content.includes('you') || content.includes('your')) baseScore += 3;
        if (content.includes('Imagine') || content.includes('Picture this')) baseScore += 8; // Creative content bonus
        break;
        
      case 'accuracy':
        baseScore = 75 + Math.random() * 15;
        if (content.includes('research') || content.includes('study')) baseScore += 5;
        break;
        
      case 'structure':
        if (content.includes('#')) baseScore += 8;
        if (content.includes('\n\n')) baseScore += 3;
        if (content.includes('## ')) baseScore += 5; // Well-structured headers
        break;
        
      case 'tone':
        if (originalPrompt.includes('professional') && !content.includes('🌱')) baseScore += 5;
        if (originalPrompt.includes('creative') && content.includes('Imagine')) baseScore += 8;
        break;
        
      case 'completeness':
        const promptWords = originalPrompt.toLowerCase().split(' ');
        const contentWords = content.toLowerCase().split(' ');
        const coverage = promptWords.filter(word => contentWords.includes(word)).length;
        baseScore += (coverage / promptWords.length) * 20;
        break;
    }
    
    return Math.max(0, Math.min(100, baseScore));
  }

  applyOptimizations(baseContent, optimizationContext) {
    // Apply specific optimizations based on context
    let optimizedContent = baseContent;
    
    // Improve clarity if needed
    if (optimizationContext.focusAreas.includes('clarity')) {
      optimizedContent = this.improveClarity(optimizedContent);
    }
    
    // Enhance engagement if needed
    if (optimizationContext.focusAreas.includes('engagement')) {
      optimizedContent = this.enhanceEngagement(optimizedContent);
    }
    
    // Improve structure if needed
    if (optimizationContext.focusAreas.includes('structure')) {
      optimizedContent = this.improveStructure(optimizedContent);
    }
    
    return optimizedContent;
  }

  improveClarity(content) {
    // Simulate clarity improvements
    return content
      .replace(/\b(complex|complicated)\b/g, 'clear')
      .replace(/\b(utilize)\b/g, 'use')
      .replace(/\b(numerous)\b/g, 'many');
  }

  enhanceEngagement(content) {
    // Add engaging elements
    const engagingPhrases = [
      "Here's what's exciting:",
      "You might be surprised to learn:",
      "The results are remarkable:",
      "This changes everything:"
    ];
    
    const randomPhrase = engagingPhrases[Math.floor(Math.random() * engagingPhrases.length)];
    
    // Insert engaging phrase at beginning of a paragraph
    return content.replace(/^([A-Z])/m, `${randomPhrase} $1`);
  }

  improveStructure(content) {
    // Add better structural elements
    if (!content.includes('##') && content.includes('\n\n')) {
      // Add section headers
      const sections = content.split('\n\n');
      return sections.map((section, index) => {
        if (index === 0) return section;
        return `## Key Point ${index}\n\n${section}`;
      }).join('\n\n');
    }
    return content;
  }

  async evaluateContent(content, originalPrompt) {
    // Use terraphim evaluator role
    const evaluatorRole = this.terraphimConfig.agentRoles.evaluator;
    
    // Simulate API call to terraphim with evaluator role
    const terraphimRequest = {
      role: evaluatorRole,
      overallRole: this.terraphimConfig.overallRole,
      content: content,
      originalPrompt: originalPrompt,
      criteria: this.qualityCriteria.filter(c => c.enabled),
      workflow: 'evaluator-optimizer'
    };
    
    // Simulate evaluation with realistic delay
    await this.delay(1500);
    
    // Generate quality scores for each criterion using role-based evaluation
    const qualityScores = {};
    
    this.qualityCriteria.forEach(criterion => {
      if (criterion.enabled) {
        qualityScores[criterion.id] = this.evaluateCriterionWithRole(
          content, 
          originalPrompt, 
          criterion, 
          evaluatorRole
        );
      } else {
        qualityScores[criterion.id] = 0;
      }
    });
    
    // Calculate weighted overall score
    const totalWeight = this.qualityCriteria
      .filter(c => c.enabled)
      .reduce((sum, c) => sum + c.weight, 0);
      
    const weightedSum = this.qualityCriteria
      .filter(c => c.enabled)
      .reduce((sum, c) => sum + (qualityScores[c.id] * c.weight), 0);
    
    qualityScores.overall = totalWeight > 0 ? weightedSum / totalWeight : 0;
    
    return qualityScores;
  }

  evaluateCriterion(content, originalPrompt, criterion) {
    // Mock evaluation logic for each criterion
    let baseScore = 60 + Math.random() * 25; // Base score 60-85
    
    switch (criterion.id) {
      case 'clarity':
        // Penalize for complex language, reward simple language
        if (content.includes('utilize') || content.includes('numerous')) baseScore -= 5;
        if (content.includes('clear') || content.includes('simple')) baseScore += 5;
        break;
        
      case 'engagement':
        // Reward engaging elements
        if (content.includes('?') || content.includes('!')) baseScore += 5;
        if (content.includes('you') || content.includes('your')) baseScore += 3;
        break;
        
      case 'accuracy':
        // Simulate fact-checking (always reasonably high for mock)
        baseScore = 75 + Math.random() * 15;
        break;
        
      case 'structure':
        // Reward headings and good organization
        if (content.includes('#')) baseScore += 8;
        if (content.includes('\n\n')) baseScore += 3;
        break;
        
      case 'tone':
        // Evaluate appropriateness to prompt
        if (originalPrompt.includes('professional') && !content.includes('🌱')) baseScore += 5;
        break;
        
      case 'completeness':
        // Check if content addresses prompt requirements
        const promptWords = originalPrompt.toLowerCase().split(' ');
        const contentWords = content.toLowerCase().split(' ');
        const coverage = promptWords.filter(word => contentWords.includes(word)).length;
        baseScore += (coverage / promptWords.length) * 20;
        break;
    }
    
    return Math.max(0, Math.min(100, baseScore));
  }

  generateFeedback(qualityScores) {
    const feedback = {
      positive: [],
      negative: [],
      suggestions: []
    };
    
    // Generate feedback based on scores
    Object.entries(qualityScores).forEach(([criterion, score]) => {
      if (criterion === 'overall') return;
      
      const criterionObj = this.qualityCriteria.find(c => c.id === criterion);
      if (!criterionObj) return;
      
      if (score >= 80) {
        feedback.positive.push(`${criterionObj.name}: Excellent ${criterionObj.description.toLowerCase()}`);
      } else if (score < 65) {
        feedback.negative.push(`${criterionObj.name}: Needs improvement in ${criterionObj.description.toLowerCase()}`);
        feedback.suggestions.push(this.generateSuggestion(criterion, score));
      }
    });
    
    return feedback;
  }

  generateSuggestion(criterion, score) {
    const suggestions = {
      clarity: 'Use simpler language and shorter sentences to improve readability',
      engagement: 'Add more questions, examples, or interactive elements to engage readers',
      accuracy: 'Verify facts and add credible sources to support claims',
      structure: 'Improve organization with clear headings and logical flow',
      tone: 'Adjust writing style to better match target audience expectations',
      completeness: 'Address all aspects mentioned in the original brief'
    };
    
    return suggestions[criterion] || 'Consider refining this aspect of the content';
  }

  identifyImprovements(previousScores, currentScores) {
    const improvements = {};
    
    Object.keys(currentScores).forEach(criterion => {
      if (criterion === 'overall') return;
      
      const change = currentScores[criterion] - previousScores[criterion];
      improvements[criterion] = {
        change: change,
        improved: change > 0
      };
    });
    
    return improvements;
  }

  renderContentVersion(version) {
    const container = document.getElementById('content-versions');
    
    // Remove existing versions for clean display (show only current)
    container.innerHTML = '';
    
    const qualityLevel = version.qualityScores.overall >= 80 ? 'high' : 
                        version.qualityScores.overall >= 65 ? 'medium' : 'low';
    
    const versionElement = document.createElement('div');
    versionElement.className = 'version-card';
    versionElement.innerHTML = `
      <div class="version-header">
        <div class="version-title">Version ${version.iteration}</div>
        <div class="version-meta">
          <span class="quality-badge quality-${qualityLevel}">
            ${Math.round(version.qualityScores.overall)}% Quality
          </span>
        </div>
      </div>
      
      <div class="version-content">${version.content.substring(0, 500)}${version.content.length > 500 ? '...' : ''}</div>
      
      <div class="version-feedback">
        <div class="feedback-title">Quality Assessment</div>
        <ul class="feedback-list">
          ${version.feedback.positive.slice(0, 2).map(item => 
            `<li class="feedback-item feedback-positive">✓ ${item}</li>`
          ).join('')}
          ${version.feedback.negative.slice(0, 2).map(item => 
            `<li class="feedback-item feedback-negative">⚠ ${item}</li>`
          ).join('')}
        </ul>
      </div>
      
      ${version.improvements ? `
        <div class="improvements-section" style="margin-top: 1rem;">
          <div style="font-weight: 600; font-size: 0.875rem; margin-bottom: 0.5rem;">Improvements from Previous Version:</div>
          <div style="display: flex; gap: 1rem; flex-wrap: wrap;">
            ${Object.entries(version.improvements).map(([criterion, improvement]) => {
              if (improvement.change > 2) {
                const criterionObj = this.qualityCriteria.find(c => c.id === criterion);
                const criterionName = criterionObj ? criterionObj.name : criterion;
                return `<span style="color: var(--success); font-size: 0.75rem;">↗ ${criterionName} +${Math.round(improvement.change)}%</span>`;
              }
              return '';
            }).filter(Boolean).join('')}
          </div>
        </div>
      ` : ''}
    `;
    
    container.appendChild(versionElement);
  }

  renderIterationHistory() {
    const container = document.getElementById('history-timeline');
    container.innerHTML = this.contentVersions.map((version, index) => `
      <div class="iteration-node ${index === this.currentIteration - 1 ? 'active' : ''} ${version === this.bestVersion ? 'best' : ''}" 
           data-iteration="${version.iteration}">
        <div class="iteration-number">${version.iteration}</div>
        <div class="iteration-score">${Math.round(version.qualityScores.overall)}%</div>
      </div>
    `).join('');
  }

  renderOptimizationChart() {
    document.getElementById('optimization-chart').style.display = 'block';
    
    const container = document.getElementById('chart-bars');
    const maxScore = Math.max(...this.qualityHistory, 100);
    
    container.innerHTML = this.qualityHistory.map((score, index) => {
      const height = (score / maxScore) * 100;
      return `
        <div class="chart-bar" style="height: ${height}%">
          <div class="bar-label">V${index + 1}</div>
          <div class="bar-value">${Math.round(score)}%</div>
        </div>
      `;
    }).join('');
  }

  showIteration(iteration) {
    const version = this.contentVersions.find(v => v.iteration === iteration);
    if (version) {
      this.renderContentVersion(version);
      
      // Update current metrics to show this iteration's scores
      const container = document.getElementById('current-quality-metrics');
      container.innerHTML = Object.entries(version.qualityScores).map(([key, value]) => `
        <div class="metric-card">
          <div class="metric-value">${Math.round(value)}%</div>
          <div class="metric-label">${this.formatMetricLabel(key)}</div>
        </div>
      `).join('');
    }
  }

  updateBestVersionInfo() {
    const infoContainer = document.getElementById('best-version-info');
    if (this.bestVersion) {
      document.getElementById('best-iteration').textContent = this.bestVersion.iteration;
      document.getElementById('best-score').textContent = `${Math.round(this.bestVersion.qualityScores.overall)}%`;
      infoContainer.style.display = 'block';
    }
  }

  displayOptimizationResults() {
    document.getElementById('results-section').style.display = 'block';
    
    const finalScore = this.qualityHistory[this.qualityHistory.length - 1];
    const initialScore = this.qualityHistory[0];
    const improvement = finalScore - initialScore;
    
    const metrics = {
      'Total Iterations': this.currentIteration,
      'Initial Quality': `${Math.round(initialScore)}%`,
      'Final Quality': `${Math.round(finalScore)}%`,
      'Quality Improvement': `+${Math.round(improvement)}%`,
      'Best Version': this.bestVersion.iteration,
      'Optimization Time': `${this.currentIteration * 3.5}s`,
      'Threshold Met': finalScore >= this.qualityThreshold ? 'Yes' : 'No',
      'Content Length': `${this.bestVersion.content.length} chars`
    };
    
    this.visualizer.createMetricsGrid(metrics, 'results-content');
  }

  updateControlsState() {
    document.getElementById('generate-btn').disabled = this.isOptimizing;
    document.getElementById('optimize-btn').disabled = this.isOptimizing || this.contentVersions.length === 0;
    document.getElementById('stop-btn').disabled = !this.isOptimizing;
  }

  stopOptimization() {
    this.isOptimizing = false;
    this.updateControlsState();
    
    document.getElementById('workflow-status').textContent = 'Stopped';
    document.getElementById('workflow-status').className = 'workflow-status paused';
  }

  resetOptimization() {
    // Reset all state
    this.isOptimizing = false;
    this.currentIteration = 0;
    this.contentVersions = [];
    this.qualityHistory = [];
    this.bestVersion = null;
    
    // Reset UI
    document.getElementById('content-versions').innerHTML = `
      <div id="initial-state" class="text-center" style="padding: 3rem;">
        <div style="color: var(--text-muted);">
          <h4>🚀 Ready to Generate & Optimize</h4>
          <p>Enter your content brief and configure quality criteria, then watch as the AI iteratively improves the content through evaluation and optimization cycles.</p>
          <p style="font-size: 0.875rem; margin-top: 1rem;">The system will generate, evaluate, and refine content until quality thresholds are met.</p>
        </div>
      </div>
    `;
    
    document.getElementById('optimization-chart').style.display = 'none';
    document.getElementById('results-section').style.display = 'none';
    document.getElementById('best-version-info').style.display = 'none';
    document.getElementById('history-timeline').innerHTML = '';
    
    // Reset workflow status
    document.getElementById('workflow-status').textContent = 'Ready to Generate';
    document.getElementById('workflow-status').className = 'workflow-status idle';
    
    this.updateControlsState();
    this.renderCurrentMetrics();
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
      prompt: this.promptInput.value,
      selectedCriteria: Array.from(this.selectedCriteria),
      ...agentState
    };
    localStorage.setItem('optimizer-demo-state', JSON.stringify(state));
  }

  loadSavedState() {
    const saved = localStorage.getItem('optimizer-demo-state');
    if (saved) {
      try {
        const state = JSON.parse(saved);
        
        if (state.prompt) {
          document.getElementById('content-prompt').value = state.prompt;
        }
        
        if (state.qualityCriteria) {
          this.qualityCriteria = state.qualityCriteria;
          this.renderQualityCriteria();
        }
        
        if (state.maxIterations) this.maxIterations = state.maxIterations;
        if (state.qualityThreshold) this.qualityThreshold = state.qualityThreshold;

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
  const demo = new EvaluatorOptimizerDemo();
  await demo.init();
});