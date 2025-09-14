/**
 * AI Agent Workflows - Visualization Components
 * Shared visualization utilities for workflow examples
 */

class WorkflowVisualizer {
  constructor(containerId) {
    this.container = document.getElementById(containerId);
    if (!this.container) {
      throw new Error(`Container with id '${containerId}' not found`);
    }
    this.currentWorkflow = null;
    this.progressCallbacks = new Set();
  }

  // Create workflow pipeline visualization
  createPipeline(steps, containerId = null) {
    const container = containerId ? document.getElementById(containerId) : this.container;
    
    const pipeline = document.createElement('div');
    pipeline.className = 'workflow-pipeline';
    pipeline.setAttribute('role', 'progressbar');
    pipeline.setAttribute('aria-label', 'Workflow progress');
    
    steps.forEach((step, index) => {
      // Create step node
      const node = document.createElement('div');
      node.className = 'workflow-node pending';
      node.id = `step-${step.id}`;
      node.innerHTML = `
        <div class="node-title">${step.name}</div>
        <div class="node-status">Pending</div>
      `;
      node.setAttribute('aria-label', `Step ${index + 1}: ${step.name}`);
      
      pipeline.appendChild(node);
      
      // Add arrow between steps (except for last step)
      if (index < steps.length - 1) {
        const arrow = document.createElement('div');
        arrow.className = 'workflow-arrow';
        arrow.innerHTML = '→';
        arrow.setAttribute('aria-hidden', 'true');
        pipeline.appendChild(arrow);
      }
    });
    
    container.appendChild(pipeline);
    return pipeline;
  }

  // Update step status
  updateStepStatus(stepId, status, data = {}) {
    const step = document.getElementById(`step-${stepId}`);
    if (!step) return;

    // Update visual status
    step.className = `workflow-node ${status}`;
    
    const statusElement = step.querySelector('.node-status');
    const statusText = {
      'pending': 'Pending',
      'active': 'Processing...',
      'completed': 'Completed',
      'error': 'Error'
    }[status] || status;
    
    statusElement.textContent = statusText;
    
    // Update aria-label for accessibility
    step.setAttribute('aria-label', `${step.querySelector('.node-title').textContent}: ${statusText}`);
    
    // Add duration if completed
    if (status === 'completed' && data.duration) {
      const duration = document.createElement('div');
      duration.className = 'node-duration';
      duration.textContent = `${(data.duration / 1000).toFixed(1)}s`;
      step.appendChild(duration);
    }
    
    // Activate arrow to next step if this step is active
    if (status === 'active') {
      const nextArrow = step.nextElementSibling;
      if (nextArrow && nextArrow.classList.contains('workflow-arrow')) {
        nextArrow.classList.add('active');
      }
    }
  }

  // Create progress bar
  createProgressBar(containerId = null) {
    const container = containerId ? document.getElementById(containerId) : this.container;
    
    const progressContainer = document.createElement('div');
    progressContainer.className = 'progress-container';
    progressContainer.innerHTML = `
      <div class="progress-label">
        <span class="progress-text">Starting...</span>
        <span class="progress-percentage">0%</span>
      </div>
      <div class="progress-bar">
        <div class="progress-fill" style="width: 0%"></div>
      </div>
    `;
    
    container.appendChild(progressContainer);
    return progressContainer;
  }

  // Update progress bar
  updateProgress(percentage, text = null) {
    const progressFill = this.container.querySelector('.progress-fill');
    const progressText = this.container.querySelector('.progress-text');
    const progressPercentage = this.container.querySelector('.progress-percentage');
    
    if (progressFill) {
      progressFill.style.width = `${Math.min(100, Math.max(0, percentage))}%`;
    }
    
    if (progressPercentage) {
      progressPercentage.textContent = `${Math.round(percentage)}%`;
    }
    
    if (text && progressText) {
      progressText.textContent = text;
    }

    // Notify progress callbacks
    this.progressCallbacks.forEach(callback => {
      try {
        callback({ percentage, text });
      } catch (error) {
        console.error('Progress callback error:', error);
      }
    });
  }

  // Create metrics display
  createMetricsGrid(metrics, containerId = null) {
    const container = containerId ? document.getElementById(containerId) : this.container;
    
    const metricsGrid = document.createElement('div');
    metricsGrid.className = 'metrics-grid';
    
    Object.entries(metrics).forEach(([key, value]) => {
      const metricCard = document.createElement('div');
      metricCard.className = 'metric-card';
      metricCard.innerHTML = `
        <span class="metric-value">${this.formatMetricValue(value)}</span>
        <span class="metric-label">${this.formatMetricLabel(key)}</span>
      `;
      metricsGrid.appendChild(metricCard);
    });
    
    container.appendChild(metricsGrid);
    return metricsGrid;
  }

  // Create results display
  createResultsDisplay(results, containerId = null) {
    const container = containerId ? document.getElementById(containerId) : this.container;
    
    const resultsContainer = document.createElement('div');
    resultsContainer.className = 'results-container';
    resultsContainer.innerHTML = `
      <div class="results-header">
        <h3>Execution Results</h3>
      </div>
      <div class="results-content" id="results-content">
        <!-- Results will be populated here -->
      </div>
    `;
    
    const resultsContent = resultsContainer.querySelector('#results-content');
    
    if (Array.isArray(results)) {
      results.forEach((result, index) => {
        this.addResultItem(resultsContent, `Step ${index + 1}`, result);
      });
    } else {
      Object.entries(results).forEach(([key, value]) => {
        this.addResultItem(resultsContent, key, value);
      });
    }
    
    container.appendChild(resultsContainer);
    return resultsContainer;
  }

  // Add individual result item
  addResultItem(container, label, value) {
    const resultItem = document.createElement('div');
    resultItem.className = 'result-item';
    resultItem.innerHTML = `
      <div class="result-label">${this.formatMetricLabel(label)}</div>
      <div class="result-value">${this.formatResultValue(value)}</div>
    `;
    container.appendChild(resultItem);
  }

  // Create network diagram for routing visualization
  createRoutingNetwork(routes, selectedRoute, containerId = null) {
    const container = containerId ? document.getElementById(containerId) : this.container;
    
    const networkContainer = document.createElement('div');
    networkContainer.className = 'routing-network';
    networkContainer.style.cssText = `
      display: grid;
      grid-template-columns: 1fr 2fr 1fr;
      gap: 2rem;
      align-items: center;
      padding: 2rem;
      background: var(--surface);
      border-radius: var(--radius-lg);
      margin: 1rem 0;
    `;
    
    // Input node
    const inputNode = document.createElement('div');
    inputNode.className = 'network-node input-node';
    inputNode.innerHTML = `
      <div class="node-title">Input Task</div>
      <div class="node-description">Analyzing complexity...</div>
    `;
    inputNode.style.cssText = `
      padding: 1rem;
      border: 2px solid var(--primary);
      border-radius: var(--radius-md);
      background: #dbeafe;
      text-align: center;
    `;
    
    // Router section
    const routerSection = document.createElement('div');
    routerSection.className = 'router-section';
    routerSection.innerHTML = `
      <div class="router-title">Intelligent Router</div>
      <div class="route-options"></div>
    `;
    routerSection.style.cssText = `
      display: flex;
      flex-direction: column;
      gap: 1rem;
    `;
    
    const routeOptionsContainer = routerSection.querySelector('.route-options');
    
    routes.forEach(route => {
      const routeOption = document.createElement('div');
      routeOption.className = `route-option ${route.id === selectedRoute?.routeId ? 'selected' : ''}`;
      routeOption.innerHTML = `
        <div class="route-name">${route.name}</div>
        <div class="route-details">Cost: $${route.cost} | Speed: ${route.speed}</div>
        ${route.id === selectedRoute?.routeId ? '<div class="route-selected">✓ Selected</div>' : ''}
      `;
      
      const isSelected = route.id === selectedRoute?.routeId;
      routeOption.style.cssText = `
        padding: 0.75rem;
        border: 2px solid ${isSelected ? 'var(--success)' : 'var(--border)'};
        border-radius: var(--radius-md);
        background: ${isSelected ? '#d1fae5' : 'var(--surface)'};
        margin-bottom: 0.5rem;
        transition: var(--transition);
      `;
      
      routeOptionsContainer.appendChild(routeOption);
    });
    
    // Output node
    const outputNode = document.createElement('div');
    outputNode.className = 'network-node output-node';
    outputNode.innerHTML = `
      <div class="node-title">Selected Model</div>
      <div class="node-description">${selectedRoute?.name || 'Processing...'}</div>
    `;
    outputNode.style.cssText = `
      padding: 1rem;
      border: 2px solid var(--success);
      border-radius: var(--radius-md);
      background: #d1fae5;
      text-align: center;
    `;
    
    networkContainer.appendChild(inputNode);
    networkContainer.appendChild(routerSection);
    networkContainer.appendChild(outputNode);
    
    container.appendChild(networkContainer);
    return networkContainer;
  }

  // Create parallel execution timeline
  createParallelTimeline(tasks, containerId = null) {
    const container = containerId ? document.getElementById(containerId) : this.container;
    
    const timeline = document.createElement('div');
    timeline.className = 'parallel-timeline';
    timeline.style.cssText = `
      display: grid;
      grid-template-columns: 150px 1fr;
      gap: 1rem;
      background: var(--surface);
      border-radius: var(--radius-lg);
      padding: 1.5rem;
      margin: 1rem 0;
    `;
    
    const taskLabels = document.createElement('div');
    taskLabels.className = 'task-labels';
    
    const taskTimelines = document.createElement('div');
    taskTimelines.className = 'task-timelines';
    taskTimelines.style.position = 'relative';
    
    tasks.forEach((task, index) => {
      // Task label
      const label = document.createElement('div');
      label.className = 'task-label';
      label.textContent = task.name;
      label.style.cssText = `
        padding: 0.5rem;
        margin-bottom: 1rem;
        font-weight: 500;
        color: var(--text);
      `;
      taskLabels.appendChild(label);
      
      // Task timeline bar
      const timelineBar = document.createElement('div');
      timelineBar.className = 'timeline-bar';
      timelineBar.id = `timeline-${task.id}`;
      timelineBar.style.cssText = `
        height: 30px;
        background: var(--surface-2);
        border-radius: var(--radius-sm);
        margin-bottom: 1rem;
        position: relative;
        overflow: hidden;
      `;
      
      const progressBar = document.createElement('div');
      progressBar.className = 'timeline-progress';
      progressBar.style.cssText = `
        height: 100%;
        width: 0%;
        background: linear-gradient(90deg, var(--primary), var(--secondary));
        border-radius: var(--radius-sm);
        transition: width 0.3s ease;
      `;
      
      timelineBar.appendChild(progressBar);
      taskTimelines.appendChild(timelineBar);
    });
    
    timeline.appendChild(taskLabels);
    timeline.appendChild(taskTimelines);
    container.appendChild(timeline);
    
    return timeline;
  }

  // Update parallel task progress
  updateParallelTask(taskId, percentage) {
    const progressBar = document.querySelector(`#timeline-${taskId} .timeline-progress`);
    if (progressBar) {
      progressBar.style.width = `${Math.min(100, Math.max(0, percentage))}%`;
    }
  }

  // Create evaluation cycle visualization
  createEvaluationCycle(iterations, containerId = null) {
    const container = containerId ? document.getElementById(containerId) : this.container;
    
    const cycle = document.createElement('div');
    cycle.className = 'evaluation-cycle';
    cycle.style.cssText = `
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 2rem;
      padding: 2rem;
      background: var(--surface);
      border-radius: var(--radius-lg);
      margin: 1rem 0;
    `;
    
    const cycleTitle = document.createElement('h3');
    cycleTitle.textContent = 'Generation-Evaluation-Optimization Cycle';
    cycleTitle.style.color = 'var(--primary)';
    
    const iterationsContainer = document.createElement('div');
    iterationsContainer.className = 'iterations-container';
    iterationsContainer.style.cssText = `
      display: flex;
      gap: 2rem;
      align-items: center;
      flex-wrap: wrap;
      justify-content: center;
    `;
    
    iterations.forEach((iteration, index) => {
      const iterationNode = document.createElement('div');
      iterationNode.className = `iteration-node iteration-${index}`;
      iterationNode.innerHTML = `
        <div class="iteration-number">Iteration ${iteration.number}</div>
        <div class="quality-score">Quality: ${(iteration.quality * 100).toFixed(0)}%</div>
        <div class="iteration-status">${iteration.status}</div>
      `;
      
      const qualityColor = iteration.quality >= 0.8 ? 'var(--success)' : 
                          iteration.quality >= 0.6 ? 'var(--warning)' : 'var(--danger)';
      
      iterationNode.style.cssText = `
        padding: 1rem;
        border: 2px solid ${qualityColor};
        border-radius: var(--radius-md);
        background: ${iteration.quality >= 0.8 ? '#d1fae5' : '#fef3c7'};
        text-align: center;
        min-width: 120px;
      `;
      
      iterationsContainer.appendChild(iterationNode);
      
      // Add arrow between iterations
      if (index < iterations.length - 1) {
        const arrow = document.createElement('div');
        arrow.className = 'cycle-arrow';
        arrow.innerHTML = '→';
        arrow.style.cssText = `
          font-size: 1.5rem;
          color: var(--primary);
          font-weight: bold;
        `;
        iterationsContainer.appendChild(arrow);
      }
    });
    
    cycle.appendChild(cycleTitle);
    cycle.appendChild(iterationsContainer);
    container.appendChild(cycle);
    
    return cycle;
  }

  // Utility methods
  formatMetricValue(value) {
    if (typeof value === 'number') {
      if (value > 1000) {
        return `${(value / 1000).toFixed(1)}k`;
      }
      if (value < 1) {
        return value.toFixed(3);
      }
      return value.toFixed(1);
    }
    return value;
  }

  formatMetricLabel(label) {
    return label
      .replace(/_/g, ' ')
      .replace(/([A-Z])/g, ' $1')
      .replace(/^./, str => str.toUpperCase())
      .trim();
  }

  formatResultValue(value) {
    if (typeof value === 'string' && value.length > 200) {
      return value.substring(0, 200) + '...';
    }
    if (typeof value === 'object') {
      return JSON.stringify(value, null, 2);
    }
    return value;
  }

  // Event handling
  onProgress(callback) {
    this.progressCallbacks.add(callback);
  }

  offProgress(callback) {
    this.progressCallbacks.delete(callback);
  }

  // Clear all visualizations
  clear() {
    if (this.container) {
      this.container.innerHTML = '';
    }
    this.progressCallbacks.clear();
  }
}

// Utility functions for animations
class AnimationUtils {
  static fadeIn(element, duration = 300) {
    element.style.opacity = '0';
    element.style.transition = `opacity ${duration}ms ease`;
    
    // Force reflow
    element.offsetHeight;
    
    element.style.opacity = '1';
  }

  static slideIn(element, direction = 'left', duration = 300) {
    const transforms = {
      left: 'translateX(-100%)',
      right: 'translateX(100%)',
      up: 'translateY(-100%)',
      down: 'translateY(100%)'
    };
    
    element.style.transform = transforms[direction];
    element.style.transition = `transform ${duration}ms ease`;
    
    // Force reflow
    element.offsetHeight;
    
    element.style.transform = 'translate(0)';
  }

  static pulse(element, duration = 1000) {
    element.style.animation = `pulse ${duration}ms ease-in-out infinite`;
  }

  static stopAnimation(element) {
    element.style.animation = '';
    element.style.transition = '';
    element.style.transform = '';
    element.style.opacity = '';
  }
}

// Export for use in examples
window.WorkflowVisualizer = WorkflowVisualizer;
window.AnimationUtils = AnimationUtils;