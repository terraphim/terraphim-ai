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
    this.complexityScore = 0;
    this.routingResult = null;
    
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
        console.log('Settings integration initialized successfully');
      } else {
        // Fallback to default API client
        console.warn('Settings integration failed, using default configuration');
        this.apiClient = new TerraphimApiClient();
      }
    } catch (error) {
      console.error('Failed to initialize settings:', error);
      // Fallback to default API client
      this.apiClient = new TerraphimApiClient();
    }
  }

  setupEventListeners() {
    // Template selection
    document.querySelectorAll('.template-card').forEach(card => {
      card.addEventListener('click', (e) => {
        this.selectTemplate(e.currentTarget.dataset.template);
      });
    });

    // Button controls
    document.getElementById('analyze-btn').addEventListener('click', () => {
      this.analyzeTask();
    });

    document.getElementById('generate-btn').addEventListener('click', () => {
      this.generatePrototype();
    });

    document.getElementById('refine-btn').addEventListener('click', () => {
      this.refinePrototype();
    });

    // Real-time complexity analysis
    const promptInput = document.getElementById('prototype-prompt');
    promptInput.addEventListener('input', () => {
      this.analyzeComplexityRealTime(promptInput.value);
    });

    // Model selection
    document.addEventListener('click', (e) => {
      if (e.target.closest('.model-option')) {
        const modelId = e.target.closest('.model-option').dataset.modelId;
        this.selectModel(modelId);
      }
    });
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
    this.selectModel('openai_gpt35');
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
    this.complexityScore = complexity;
    
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
  }

  async analyzeTask() {
    const prompt = document.getElementById('prototype-prompt').value.trim();
    
    if (!prompt) {
      alert('Please enter a prototype description first.');
      return;
    }

    // Update workflow status
    document.getElementById('workflow-status').textContent = 'Analyzing...';
    document.getElementById('workflow-status').className = 'workflow-status running';
    
    // Reset pipeline
    this.visualizer.updateStepStatus('analyze', 'active');
    this.visualizer.updateProgress(10, 'Analyzing task complexity...');
    
    // Simulate analysis delay
    await this.delay(1500);
    
    // Calculate final complexity
    const complexity = this.calculateComplexity(prompt);
    this.updateComplexityDisplay(complexity);
    
    // Get recommended model
    const recommendedModel = this.recommendModel(complexity);
    
    this.visualizer.updateStepStatus('analyze', 'completed');
    this.visualizer.updateStepStatus('route', 'active');
    this.visualizer.updateProgress(40, 'Selecting optimal model...');
    
    await this.delay(1000);
    
    // Create routing visualization
    this.createRoutingVisualization(recommendedModel, complexity);
    
    // Auto-select recommended model
    this.selectModel(recommendedModel.id);
    
    this.visualizer.updateStepStatus('route', 'completed');
    this.visualizer.updateProgress(70, 'Model selected. Ready to generate.');
    
    // Enable generation
    document.getElementById('generate-btn').disabled = false;
    document.getElementById('workflow-status').textContent = 'Ready to Generate';
    document.getElementById('workflow-status').className = 'workflow-status ready';
  }

  createRoutingVisualization(selectedModel, complexity) {
    const container = document.getElementById('routing-visualization');
    container.style.display = 'block';
    
    // Create routing network visualization
    const routes = this.models.map(model => ({
      id: model.id,
      name: model.name,
      cost: model.costLabel,
      speed: model.speed,
      suitable: complexity <= model.maxComplexity
    }));
    
    const routingNetwork = this.visualizer.createRoutingNetwork(
      routes, 
      { routeId: selectedModel.id, name: selectedModel.name },
      'routing-visualization'
    );
  }

  async generatePrototype() {
    const prompt = document.getElementById('prototype-prompt').value.trim();
    
    if (!this.selectedModel) {
      alert('Please select a model first.');
      return;
    }

    // Update workflow status
    document.getElementById('workflow-status').textContent = 'Generating...';
    document.getElementById('workflow-status').className = 'workflow-status running';
    
    this.visualizer.updateStepStatus('generate', 'active');
    this.visualizer.updateProgress(80, `Generating with ${this.selectedModel.name}...`);
    
    try {
      // Execute real routing workflow with API client
      const result = await this.apiClient.executeRouting({
        prompt: prompt,
        role: this.selectedModel?.id || 'content_creator',
        overall_role: 'technical_specialist',
        config: {
          template: this.currentTemplate,
          complexity: this.complexityScore
        }
      }, {
        realTime: true,
        onProgress: (progress) => {
          this.visualizer.updateProgress(80 + (progress.percentage * 0.2), progress.current);
        }
      });
      
      this.routingResult = result;
      
      // Generate actual prototype content
      await this.renderPrototypeResult(result);
      
      this.visualizer.updateStepStatus('generate', 'completed');
      this.visualizer.updateProgress(100, 'Prototype generated successfully!');
      
      document.getElementById('workflow-status').textContent = 'Completed';
      document.getElementById('workflow-status').className = 'workflow-status completed';
      document.getElementById('refine-btn').disabled = false;
      
      // Show results section
      document.getElementById('results-section').style.display = 'block';
      this.displayGenerationResults(result);
      
    } catch (error) {
      console.error('Generation failed:', error);
      this.visualizer.updateStepStatus('generate', 'error');
      document.getElementById('workflow-status').textContent = 'Error';
      document.getElementById('workflow-status').className = 'workflow-status error';
    }
  }

  async renderPrototypeResult(result) {
    const preview = document.getElementById('prototype-preview');
    const template = this.templates[this.currentTemplate];
    
    // Generate mock HTML based on template and complexity
    const htmlContent = this.generateMockHTML(template, result);
    
    preview.innerHTML = `
      <div class="prototype-content">
        <div style="margin-bottom: 1rem; padding: 1rem; background: #f3f4f6; border-radius: 6px;">
          <strong>Generated with:</strong> ${this.selectedModel.name} 
          <span style="color: #6b7280;">(Complexity: ${Math.round(this.complexityScore * 100)}%)</span>
        </div>
        ${htmlContent}
      </div>
    `;
  }

  generateMockHTML(template, result) {
    const complexityLevel = this.complexityScore;
    
    const templates = {
      'landing-page': () => `
        <div style="text-align: center; padding: 2rem;">
          <h1 style="color: #1f2937; margin-bottom: 1rem;">Revolutionary SaaS Platform</h1>
          <p style="color: #6b7280; margin-bottom: 2rem;">Transform your workflow with AI-powered collaboration tools</p>
          <button style="background: #3b82f6; color: white; padding: 0.75rem 2rem; border: none; border-radius: 6px; margin-right: 1rem;">Get Started Free</button>
          <button style="background: transparent; color: #3b82f6; padding: 0.75rem 2rem; border: 2px solid #3b82f6; border-radius: 6px;">Learn More</button>
        </div>
        ${complexityLevel > 0.4 ? `
        <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 2rem; padding: 2rem;">
          <div style="text-align: center; padding: 1.5rem; border: 1px solid #e5e7eb; border-radius: 8px;">
            <h3>⚡ Fast Setup</h3>
            <p style="color: #6b7280;">Get started in minutes</p>
          </div>
          <div style="text-align: center; padding: 1.5rem; border: 1px solid #e5e7eb; border-radius: 8px;">
            <h3>🤝 Team Collaboration</h3>
            <p style="color: #6b7280;">Work together seamlessly</p>
          </div>
          <div style="text-align: center; padding: 1.5rem; border: 1px solid #e5e7eb; border-radius: 8px;">
            <h3>📊 Advanced Analytics</h3>
            <p style="color: #6b7280;">Track your progress</p>
          </div>
        </div>
        ` : ''}
      `,
      
      'dashboard': () => `
        <div style="display: grid; grid-template-columns: repeat(${complexityLevel > 0.6 ? '4' : '2'}, 1fr); gap: 1rem; margin-bottom: 2rem;">
          <div style="background: #f3f4f6; padding: 1rem; border-radius: 6px; text-align: center;">
            <div style="font-size: 2rem; font-weight: bold; color: #10b981;">1,234</div>
            <div style="color: #6b7280;">Active Users</div>
          </div>
          <div style="background: #f3f4f6; padding: 1rem; border-radius: 6px; text-align: center;">
            <div style="font-size: 2rem; font-weight: bold; color: #3b82f6;">$12,345</div>
            <div style="color: #6b7280;">Revenue</div>
          </div>
          ${complexityLevel > 0.6 ? `
          <div style="background: #f3f4f6; padding: 1rem; border-radius: 6px; text-align: center;">
            <div style="font-size: 2rem; font-weight: bold; color: #f59e0b;">89%</div>
            <div style="color: #6b7280;">Conversion</div>
          </div>
          <div style="background: #f3f4f6; padding: 1rem; border-radius: 6px; text-align: center;">
            <div style="font-size: 2rem; font-weight: bold; color: #8b5cf6;">456</div>
            <div style="color: #6b7280;">New Leads</div>
          </div>
          ` : ''}
        </div>
        <div style="background: #f9fafb; padding: 2rem; border-radius: 8px; text-align: center;">
          <h3>📈 Performance Chart</h3>
          <div style="height: 200px; background: white; border-radius: 4px; display: flex; align-items: center; justify-content: center; color: #6b7280;">
            Interactive chart would be rendered here
          </div>
        </div>
      `,
      
      'ecommerce': () => `
        <div style="display: grid; grid-template-columns: repeat(${complexityLevel > 0.7 ? '3' : '2'}, 1fr); gap: 1.5rem;">
          <div style="border: 1px solid #e5e7eb; border-radius: 8px; overflow: hidden;">
            <div style="height: 150px; background: #f3f4f6; display: flex; align-items: center; justify-content: center; color: #6b7280;">Product Image</div>
            <div style="padding: 1rem;">
              <h4>Premium Widget</h4>
              <p style="color: #6b7280; font-size: 0.875rem;">High-quality product description</p>
              <div style="font-weight: bold; color: #10b981;">$99.99</div>
              <button style="width: 100%; background: #3b82f6; color: white; padding: 0.5rem; border: none; border-radius: 4px; margin-top: 0.5rem;">Add to Cart</button>
            </div>
          </div>
          <div style="border: 1px solid #e5e7eb; border-radius: 8px; overflow: hidden;">
            <div style="height: 150px; background: #f3f4f6; display: flex; align-items: center; justify-content: center; color: #6b7280;">Product Image</div>
            <div style="padding: 1rem;">
              <h4>Deluxe Package</h4>
              <p style="color: #6b7280; font-size: 0.875rem;">Complete solution bundle</p>
              <div style="font-weight: bold; color: #10b981;">$199.99</div>
              <button style="width: 100%; background: #3b82f6; color: white; padding: 0.5rem; border: none; border-radius: 4px; margin-top: 0.5rem;">Add to Cart</button>
            </div>
          </div>
          ${complexityLevel > 0.7 ? `
          <div style="border: 1px solid #e5e7eb; border-radius: 8px; overflow: hidden;">
            <div style="height: 150px; background: #f3f4f6; display: flex; align-items: center; justify-content: center; color: #6b7280;">Product Image</div>
            <div style="padding: 1rem;">
              <h4>Enterprise Suite</h4>
              <p style="color: #6b7280; font-size: 0.875rem;">Full enterprise solution</p>
              <div style="font-weight: bold; color: #10b981;">$499.99</div>
              <button style="width: 100%; background: #3b82f6; color: white; padding: 0.5rem; border: none; border-radius: 4px; margin-top: 0.5rem;">Add to Cart</button>
            </div>
          </div>
          ` : ''}
        </div>
      `
    };
    
    return templates[this.currentTemplate]?.() || templates['landing-page']();
  }

  displayGenerationResults(result) {
    const container = document.getElementById('results-content');
    
    const metrics = {
      'Model Used': this.selectedModel.name,
      'Task Complexity': `${Math.round(this.complexityScore * 100)}%`,
      'Estimated Cost': this.selectedModel.costLabel,
      'Generation Time': `${(result.metadata.executionTime / 1000).toFixed(1)}s`,
      'Quality Score': '92%'
    };
    
    this.visualizer.createMetricsGrid(metrics, 'results-content');
  }

  async refinePrototype() {
    document.getElementById('workflow-status').textContent = 'Refining...';
    document.getElementById('workflow-status').className = 'workflow-status running';
    
    // Simulate refinement
    await this.delay(2000);
    
    document.getElementById('workflow-status').textContent = 'Refined';
    document.getElementById('workflow-status').className = 'workflow-status completed';
  }

  // Utility methods
  delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  saveState() {
    const state = {
      currentTemplate: this.currentTemplate,
      selectedModel: this.selectedModel?.id,
      prompt: document.getElementById('prototype-prompt').value,
      complexity: this.complexityScore
    };
    localStorage.setItem('routing-demo-state', JSON.stringify(state));
  }

  loadSavedState() {
    const saved = localStorage.getItem('routing-demo-state');
    if (saved) {
      try {
        const state = JSON.parse(saved);
        if (state.currentTemplate) this.selectTemplate(state.currentTemplate);
        if (state.selectedModel) this.selectModel(state.selectedModel);
        if (state.prompt) {
          document.getElementById('prototype-prompt').value = state.prompt;
          this.analyzeComplexityRealTime(state.prompt);
        }
      } catch (error) {
        console.warn('Failed to load saved state:', error);
      }
    }
  }
}

// Initialize the demo when DOM is loaded
document.addEventListener('DOMContentLoaded', async () => {
  const demo = new RoutingPrototypingDemo();
  await demo.init();
});