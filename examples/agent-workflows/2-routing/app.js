/**
 * AI Routing - Smart Prototyping Environment
 * Demonstrates intelligent model selection based on task complexity
 */

class RoutingPrototypingDemo {
  constructor() {
    this.apiClient = null; // Will be initialized with settings
    this.visualizer = new WorkflowVisualizer('pipeline-container');
    this.settingsIntegration = null;
    this.currentTemplate = 'landing-page';
    this.selectedModel = null;
    this.currentComplexity = 0;
    this.generationResult = null;
    this.agentConfigManager = null;

    // Element references
    this.promptInput = null;
    this.generateButton = null;
    this.analyzeButton = null;
    this.refineButton = null;

    // Available AI models with capabilities and costs
    this.models = [
      {
        id: 'openai_gpt35',
        name: 'GPT-3.5 Turbo',
        speed: 'Fast',
        capability: 'Balanced',
        cost: 0.002,
        costLabel: '$0.002/1k tokens',
        maxComplexity: 0.6,
        description: 'Great for simple to moderate complexity tasks',
        color: '#10b981'
      },
      {
        id: 'openai_gpt4',
        name: 'GPT-4',
        speed: 'Medium',
        capability: 'Advanced',
        cost: 0.03,
        costLabel: '$0.03/1k tokens',
        maxComplexity: 0.9,
        description: 'Perfect for complex reasoning and detailed work',
        color: '#3b82f6'
      },
      {
        id: 'claude_opus',
        name: 'Claude 3 Opus',
        speed: 'Slow',
        capability: 'Expert',
        cost: 0.075,
        costLabel: '$0.075/1k tokens',
        maxComplexity: 1.0,
        description: 'Best for highly complex and creative tasks',
        color: '#8b5cf6'
      }
    ];

    // Prototype templates with complexity indicators
    this.templates = {
      'landing-page': {
        name: 'Landing Page',
        baseComplexity: 0.2,
        features: ['Hero section', 'Navigation', 'Call-to-action', 'Footer'],
        example: 'Simple marketing site with clear messaging'
      },
      'dashboard': {
        name: 'Dashboard',
        baseComplexity: 0.5,
        features: ['Data visualization', 'Charts', 'Metrics', 'Interactive elements'],
        example: 'Analytics dashboard with charts and KPIs'
      },
      'ecommerce': {
        name: 'E-commerce',
        baseComplexity: 0.6,
        features: ['Product catalog', 'Shopping cart', 'Checkout', 'User accounts'],
        example: 'Online store with complete shopping experience'
      },
      'saas-app': {
        name: 'SaaS Application',
        baseComplexity: 0.8,
        features: ['Complex UI', 'User management', 'API integration', 'Advanced features'],
        example: 'Feature-rich application with multiple workflows'
      },
      'portfolio': {
        name: 'Portfolio',
        baseComplexity: 0.3,
        features: ['Gallery', 'About section', 'Contact form', 'Project showcase'],
        example: 'Creative showcase with portfolio pieces'
      }
    };
  }

  async init() {
    // Initialize element references
    this.promptInput = document.getElementById('prototype-prompt');
    this.generateButton = document.getElementById('generate-btn');
    this.analyzeButton = document.getElementById('analyze-btn');
    this.refineButton = document.getElementById('refine-btn');
    this.outputFrame = document.getElementById('output-frame');

    // Initialize settings system first
    await this.initializeSettings();

    this.setupEventListeners();
    this.renderModels();
    this.renderTemplateCards();
    this.createWorkflowPipeline();
    this.selectDefaultModel();

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

  initializeConnectionStatus() {
    if (typeof ConnectionStatusComponent !== 'undefined' && this.apiClient) {
      this.connectionStatus = new ConnectionStatusComponent('connection-status-container', this.apiClient);
    }
  }

  // Helper methods to manage both button sets
  setGenerateButtonState(disabled) {
    if (this.generateButton) {
      this.generateButton.disabled = disabled;
    }
    const sidebarGenerateBtn = document.getElementById('sidebar-generate-btn');
    if (sidebarGenerateBtn) {
      sidebarGenerateBtn.disabled = disabled;
    }
  }

  setRefineButtonState(disabled) {
    if (this.refineButton) {
      this.refineButton.disabled = disabled;
    }
    const sidebarRefineBtn = document.getElementById('sidebar-refine-btn');
    if (sidebarRefineBtn) {
      sidebarRefineBtn.disabled = disabled;
    }
  }

  setupEventListeners() {
    // Main canvas button event listeners
    if (this.analyzeButton) {
      this.analyzeButton.addEventListener('click', () => this.analyzeTask());
    }

    if (this.generateButton) {
      this.generateButton.addEventListener('click', () => this.generatePrototype());
    }

    if (this.refineButton) {
      this.refineButton.addEventListener('click', () => this.refinePrototype());
    }

    // Sidebar button event listeners (duplicates)
    const sidebarAnalyzeBtn = document.getElementById('sidebar-analyze-btn');
    if (sidebarAnalyzeBtn) {
      sidebarAnalyzeBtn.addEventListener('click', () => this.analyzeTask());
    }

    const sidebarGenerateBtn = document.getElementById('sidebar-generate-btn');
    if (sidebarGenerateBtn) {
      sidebarGenerateBtn.addEventListener('click', () => this.generatePrototype());
    }

    const sidebarRefineBtn = document.getElementById('sidebar-refine-btn');
    if (sidebarRefineBtn) {
      sidebarRefineBtn.addEventListener('click', () => this.refinePrototype());
    }

    // Template selection event listeners
    document.querySelectorAll('.template-card').forEach(card => {
      card.addEventListener('click', () => {
        const template = card.dataset.template;
        this.selectTemplate(template);
      });
    });

    // Model selection event listeners
    document.addEventListener('click', (e) => {
      if (e.target.closest('.model-option')) {
        const modelId = e.target.closest('.model-option').dataset.modelId;
        this.selectModel(modelId);
      }
    });

    // Real-time complexity analysis
    if (this.promptInput) {
      this.promptInput.addEventListener('input', () => {
        this.analyzeComplexityRealTime(this.promptInput.value);
      });
    }
  }

  getModels() {
    return [
      {
        id: 'ollama_gemma3_270m',
        name: 'Gemma3 270M (Local)',
        speed: 'Very Fast',
        capability: 'Basic',
        cost: 0.0,
        costLabel: 'Free (Local)',
        maxComplexity: 0.3,
        description: 'Ultra-fast local model for simple tasks',
        color: '#16a34a'
      },
      {
        id: 'ollama_llama32_3b',
        name: 'Llama3.2 3B (Local)',
        speed: 'Fast',
        capability: 'Balanced',
        cost: 0.0,
        costLabel: 'Free (Local)',
        maxComplexity: 0.7,
        description: 'Balanced local model for moderate complexity tasks',
        color: '#059669'
      },
      {
        id: 'openai_gpt35',
        name: 'GPT-3.5 Turbo',
        speed: 'Fast',
        capability: 'Balanced',
        cost: 0.002,
        costLabel: '$0.002/1k tokens',
        maxComplexity: 0.6,
        description: 'Great for simple to moderate complexity tasks',
        color: '#10b981'
      },
      {
        id: 'openai_gpt4',
        name: 'GPT-4',
        speed: 'Medium',
        capability: 'Advanced',
        cost: 0.03,
        costLabel: '$0.03/1k tokens',
        maxComplexity: 0.9,
        description: 'Perfect for complex reasoning and detailed work',
        color: '#3b82f6'
      },
      {
        id: 'claude_opus',
        name: 'Claude 3 Opus',
        speed: 'Slow',
        capability: 'Expert',
        cost: 0.075,
        costLabel: '$0.075/1k tokens',
        maxComplexity: 1.0,
        description: 'Best for highly complex and creative tasks',
        color: '#8b5cf6'
      }
    ];
  }

  renderModels() {
    const container = document.getElementById('models-list');
    container.innerHTML = this.models.map(model => `
      <div class="model-option" data-model-id="${model.id}">
        <div class="model-details">
          <div class="model-name">${model.name}</div>
          <div class="model-specs">${model.speed} • ${model.capability}</div>
        </div>
        <div class="model-cost">${model.costLabel}</div>
      </div>
    `).join('');
  }

  renderTemplateCards() {
    // Templates are already in HTML, just add click handlers
    this.selectTemplate('landing-page'); // Default selection
  }

  selectTemplate(templateId) {
    this.currentTemplate = templateId;

    // Update UI
    document.querySelectorAll('.template-card').forEach(card => {
      card.classList.remove('selected');
    });
    document.querySelector(`[data-template="${templateId}"]`).classList.add('selected');

    // Update complexity based on template
    this.updateComplexityForTemplate();
  }

  selectModel(modelId) {
    this.selectedModel = this.models.find(m => m.id === modelId);

    // Update model selection display
    const display = document.getElementById('selected-model-display');
    if (this.selectedModel) {
      display.innerHTML = `
        <div class="model-details">
          <div class="model-name">${this.selectedModel.name}</div>
          <div class="model-specs">${this.selectedModel.speed} • ${this.selectedModel.capability}</div>
          <div class="model-cost">${this.selectedModel.costLabel}</div>
        </div>
      `;
    }

    // Update model options styling
    document.querySelectorAll('.model-option').forEach(option => {
      option.classList.remove('selected');
      if (option.dataset.modelId === modelId) {
        option.classList.add('selected');
      }
    });
  }

  selectDefaultModel() {
    this.selectModel('ollama_gemma3_270m');
  }

  analyzeComplexityRealTime(prompt) {
    const complexity = this.calculateComplexity(prompt);
    this.updateComplexityDisplay(complexity);
    this.recommendModel(complexity);
  }

  calculateComplexity(prompt) {
    const template = this.templates[this.currentTemplate];
    let complexity = template.baseComplexity;

    // Add complexity based on prompt characteristics
    const wordCount = prompt.split(/\s+/).length;
    const sentenceCount = prompt.split(/[.!?]+/).length;

    // Length complexity
    if (wordCount > 100) complexity += 0.2;
    if (wordCount > 200) complexity += 0.2;

    // Feature complexity keywords
    const complexFeatures = [
      'authentication', 'payment', 'database', 'api', 'real-time',
      'machine learning', 'ai', 'complex', 'advanced', 'enterprise',
      'integration', 'workflow', 'automation', 'dashboard', 'analytics'
    ];

    const featureMatches = complexFeatures.filter(feature =>
      prompt.toLowerCase().includes(feature)
    ).length;

    complexity += featureMatches * 0.1;

    // Technical requirements
    if (prompt.toLowerCase().includes('responsive')) complexity += 0.1;
    if (prompt.toLowerCase().includes('mobile')) complexity += 0.1;
    if (prompt.toLowerCase().includes('interactive')) complexity += 0.15;

    return Math.min(1.0, Math.max(0.1, complexity));
  }

  updateComplexityDisplay(complexity) {
    this.currentComplexity = complexity;

    const fill = document.getElementById('complexity-fill');
    const label = document.getElementById('complexity-label');
    const factors = document.getElementById('complexity-factors');

    fill.style.width = `${complexity * 100}%`;

    let complexityLevel = 'Simple';
    if (complexity > 0.7) complexityLevel = 'Complex';
    else if (complexity > 0.4) complexityLevel = 'Moderate';

    label.textContent = complexityLevel;

    // Show complexity factors
    const template = this.templates[this.currentTemplate];
    factors.innerHTML = `
      Template: ${template.name} (${Math.round(template.baseComplexity * 100)}%)
      <br>Content Analysis: +${Math.round((complexity - template.baseComplexity) * 100)}%
    `;
  }

  recommendModel(complexity) {
    // Find best model for complexity
    let recommendedModel = this.models[0]; // Default to cheapest

    for (const model of this.models) {
      if (complexity <= model.maxComplexity) {
        recommendedModel = model;
        break;
      }
    }

    // Update model recommendations in UI
    document.querySelectorAll('.model-option').forEach(option => {
      option.classList.remove('recommended');
      if (option.dataset.modelId === recommendedModel.id) {
        option.classList.add('recommended');
      }
    });

    return recommendedModel;
  }

  updateComplexityForTemplate() {
    const prompt = document.getElementById('prototype-prompt').value;
    this.analyzeComplexityRealTime(prompt);
  }

  createWorkflowPipeline() {
    const steps = [
      { id: 'analyze', name: 'Task Analysis' },
      { id: 'route', name: 'Model Selection' },
      { id: 'generate', name: 'Content Generation' }
    ];

    this.visualizer.createPipeline(steps);
    this.visualizer.createProgressBar('progress-container');
    return this.visualizer;
  }

  async analyzeTask() {
    this.setGenerateButtonState(true);

    const pipeline = this.createWorkflowPipeline();
    pipeline.updateStepStatus('analyze', 'active');

    // Simulate analysis delay
    await this.delay(500);

    const prompt = this.promptInput.value;
    const complexity = this.calculateComplexity(prompt);
    this.currentComplexity = complexity;

    this.updateComplexityDisplay(complexity);

    const recommendedModel = this.recommendModel(complexity);
    this.selectModel(recommendedModel.id);

    pipeline.updateStepStatus('analyze', 'completed');
    pipeline.updateStepStatus('route', 'active');

    // Simulate routing delay
    await this.delay(300);

    this.createRoutingVisualization(this.selectedModel, complexity);
    pipeline.updateStepStatus('route', 'completed');
    pipeline.updateStepStatus('generate', 'pending');

    this.setGenerateButtonState(false);
  }

  createRoutingVisualization(selectedModel, complexity) {
    const visualizer = new WorkflowVisualizer('routing-visualization');
    visualizer.clear();

    const routes = this.getModels().map(model => ({
      id: model.id,
      name: model.name,
      active: model.id === selectedModel.id
    }));

    visualizer.createRoutingNetwork(routes, selectedModel.id);
  }

  async generatePrototype() {
    this.generateButton.disabled = true;
    this.refineButton.disabled = true;

    const pipeline = this.createWorkflowPipeline();
    pipeline.updateStepStatus('generate', 'active');

    const agentState = this.agentConfigManager.getState();

    // Prepare workflow input
    const input = {
      prompt: this.promptInput.value,
      template: this.selectedTemplate,
      complexity: this.currentComplexity,
      model: this.selectedModel.id,
      role: agentState.selectedRole,
      config: {
        system_prompt_override: agentState.systemPrompt
      }
    };

    const enhancedInput = this.settingsIntegration
      ? this.settingsIntegration.enhanceWorkflowInput(input)
      : input;

    try {
      // FORCE HTTP ONLY - bypass any WebSocket caching issues
      const result = await this.apiClient.request('/workflows/route', {
        method: 'POST',
        body: JSON.stringify({
          prompt: enhancedInput.prompt,
          role: enhancedInput.role || enhancedInput.input?.role,
          overall_role: enhancedInput.overall_role || enhancedInput.input?.overall_role || 'engineering_agent',
          ...(enhancedInput.config && { config: enhancedInput.config }),
          ...(enhancedInput.llm_config && { llm_config: enhancedInput.llm_config })
        })
      });

      console.log('Routing HTTP result:', result);

      this.generationResult = result;
      this.renderPrototypeResult(result);
      this.displayGenerationResults(result);

      pipeline.updateStepStatus('generate', 'completed');
      this.setRefineButtonState(false);

    } catch (error) {
      console.error('Generation failed:', error);
      pipeline.updateStepStatus('generate', 'error');
    } finally {
      this.setGenerateButtonState(false);
    }
  }

  async renderPrototypeResult(result) {
    const htmlContent = this.generateMockHTML(this.selectedTemplate, result);
    this.outputFrame.srcdoc = htmlContent;
  }

  generateMockHTML(template, result) {
    // This is a simplified mock HTML generator
    const title = (result.result && result.result.title) || "Generated Prototype";
    const body = (result.result && result.result.body) || "<p>Could not generate content.</p>";

    return `
      <html>
        <head>
          <title>${title}</title>
          <style>
            body { font-family: sans-serif; line-height: 1.6; padding: 20px; }
            .container { max-width: 800px; margin: 0 auto; }
            .header { background: #f0f0f0; padding: 20px; border-radius: 8px; margin-bottom: 20px; }
            .content { border: 1px solid #ddd; padding: 20px; border-radius: 8px; }
          </style>
        </head>
        <body>
          <div class="container">
            <div class="header"><h1>${title}</h1></div>
            <div class="content">${body}</div>
          </div>
        </body>
      </html>
    `;
  }

  displayGenerationResults(result) {
    const container = document.getElementById('results-container');
    container.innerHTML = '';

    const visualizer = new WorkflowVisualizer('results-container');
    visualizer.createResultsDisplay({
      'Selected Model': this.selectedModel.name,
      'Task Complexity': `${(this.currentComplexity * 100).toFixed(0)}%`,
      'Estimated Cost': `$${(Math.random() * 0.1).toFixed(4)}`,
      'Execution Time': `${(result.duration || 2500 / 1000).toFixed(2)}s`
    });
  }

  async refinePrototype() {
    alert('Refinement functionality would be implemented here.');
  }

  // Utility methods
  delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  saveState() {
    const agentState = this.agentConfigManager ? this.agentConfigManager.getState() : {};
    const state = {
      prompt: this.promptInput ? this.promptInput.value : '',
      template: this.selectedTemplate,
      model: this.selectedModel ? this.selectedModel.id : null,
      ...agentState
    };
    localStorage.setItem('routing-demo-state', JSON.stringify(state));
  }

  loadSavedState() {
    const saved = localStorage.getItem('routing-demo-state');
    if (saved) {
      const savedState = JSON.parse(saved);
      if (this.promptInput) {
        this.promptInput.value = savedState.prompt || '';
        this.analyzeComplexityRealTime(this.promptInput.value);
      }
      this.selectTemplate(savedState.template || 'landing-page');
      this.selectModel(savedState.model || 'openai_gpt35');

      if (this.agentConfigManager) {
        this.agentConfigManager.applyState(savedState);
      }
    }
  }
}

// Initialize the demo when DOM is loaded
document.addEventListener('DOMContentLoaded', async () => {
  try {
    const demo = new RoutingPrototypingDemo();
    window.demo = demo; // Make it globally accessible for debugging
    await demo.init();
    console.log('Routing demo initialized successfully');
  } catch (error) {
    console.error('Failed to initialize routing demo:', error);
  }
});
