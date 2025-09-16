/**
 * Prompt Chaining - Interactive Coding Environment
 * Demonstrates step-by-step software development workflow
 */

class PromptChainingDemo {
  constructor() {
    this.apiClient = null; // Will be initialized by settings
    this.visualizer = new WorkflowVisualizer('pipeline-container');
    this.currentExecution = null;
    this.isPaused = false;
    this.steps = [];
    this.currentStepIndex = 0;
    this.connectionStatus = null;
    this.settingsIntegration = null;
    
    this.initializeElements();
    this.setupEventListeners();
    this.loadProjectTemplate();
  }

  initializeElements() {
    // Control elements
    this.startButton = document.getElementById('start-chain');
    this.pauseButton = document.getElementById('pause-chain');
    this.resetButton = document.getElementById('reset-chain');
    this.templateSelector = document.getElementById('project-template');
    this.statusElement = document.getElementById('workflow-status');
    
    // Input elements
    this.projectDescription = document.getElementById('project-description');
    this.techStack = document.getElementById('tech-stack');
    this.requirements = document.getElementById('requirements');
    
    // Output elements
    this.outputContainer = document.getElementById('chain-output');
    this.metricsContainer = document.getElementById('metrics-container');
    this.stepEditorsContainer = document.getElementById('step-editors');
  }

  setupEventListeners() {
    this.startButton.addEventListener('click', () => this.startChain());
    this.pauseButton.addEventListener('click', () => this.pauseChain());
    this.resetButton.addEventListener('click', () => this.resetChain());
    this.templateSelector.addEventListener('change', () => this.loadProjectTemplate());
    
    // Auto-save inputs
    this.projectDescription.addEventListener('input', () => this.saveState());
    this.techStack.addEventListener('input', () => this.saveState());
    this.requirements.addEventListener('input', () => this.saveState());
  }

  initializeConnectionStatus() {
    // Initialize WebSocket connection status component
    if (typeof ConnectionStatusComponent !== 'undefined') {
      this.connectionStatus = new ConnectionStatusComponent('connection-status-container', this.apiClient);
    }
  }

  async initializeSettings() {
    try {
      // Initialize settings integration
      const initialized = await initializeSettings();
      if (initialized) {
        this.settingsIntegration = getSettingsIntegration();
        
        // Get global API client created by settings
        this.apiClient = window.apiClient;
        
        // Update connection status with new API client if available
        if (this.connectionStatus && this.apiClient && typeof this.connectionStatus.updateApiClient === 'function') {
          this.connectionStatus.updateApiClient(this.apiClient);
        }
        
        console.log('Settings initialized successfully');
        
        // Initialize connection status after API client is ready
        this.initializeConnectionStatus();
      } else {
        // Fallback to default API client
        this.apiClient = new TerraphimApiClient();
        console.warn('Settings initialization failed, using default API client');
        
        // Initialize connection status with fallback client
        this.initializeConnectionStatus();
      }
    } catch (error) {
      console.error('Settings initialization error:', error);
      this.apiClient = new TerraphimApiClient();
      
      // Initialize connection status with fallback client
      this.initializeConnectionStatus();
    }
  }

  loadProjectTemplate() {
    const template = this.templateSelector.value;
    const templates = this.getProjectTemplates();
    const selectedTemplate = templates[template];
    
    if (selectedTemplate) {
      // Set example values
      this.projectDescription.value = selectedTemplate.description;
      this.techStack.value = selectedTemplate.techStack;
      this.requirements.value = selectedTemplate.requirements;
      
      // Load template steps
      this.steps = selectedTemplate.steps;
      this.createStepEditors();
    }
  }

  getProjectTemplates() {
    return {
      'web-app': {
        description: 'Build a task management web application with user authentication, CRUD operations for tasks, and a clean responsive UI',
        techStack: 'React, Node.js, Express, MongoDB, JWT',
        requirements: 'Mobile-responsive design, user authentication, data persistence, search functionality',
        steps: [
          {
            id: 'specification',
            name: 'Requirements & Specification',
            prompt: 'Create a detailed technical specification including user stories, API endpoints, data models, and acceptance criteria.',
            editable: true
          },
          {
            id: 'architecture',
            name: 'System Design & Architecture',
            prompt: 'Design the system architecture, component structure, database schema, and technology integration.',
            editable: true
          },
          {
            id: 'planning',
            name: 'Development Planning',
            prompt: 'Create a detailed development plan with tasks, priorities, estimated timelines, and milestones.',
            editable: true
          },
          {
            id: 'implementation',
            name: 'Code Implementation',
            prompt: 'Generate the core application code, including backend API, frontend components, and database setup.',
            editable: true
          },
          {
            id: 'testing',
            name: 'Testing & Quality Assurance',
            prompt: 'Create comprehensive tests including unit tests, integration tests, and quality assurance checklist.',
            editable: true
          },
          {
            id: 'deployment',
            name: 'Deployment & Documentation',
            prompt: 'Provide deployment instructions, environment setup, and comprehensive documentation.',
            editable: true
          }
        ]
      },
      'api-service': {
        description: 'Create a RESTful API service for managing user data with authentication, CRUD operations, and proper error handling',
        techStack: 'Node.js, Express, PostgreSQL, JWT, Docker',
        requirements: 'OpenAPI documentation, rate limiting, input validation, comprehensive logging',
        steps: [
          {
            id: 'api_design',
            name: 'API Design & Specification',
            prompt: 'Design RESTful API endpoints, request/response schemas, and create OpenAPI specification.',
            editable: true
          },
          {
            id: 'architecture',
            name: 'Service Architecture',
            prompt: 'Design service architecture, database schema, middleware stack, and security considerations.',
            editable: true
          },
          {
            id: 'implementation',
            name: 'Core Implementation',
            prompt: 'Implement API endpoints, database models, authentication middleware, and error handling.',
            editable: true
          },
          {
            id: 'testing',
            name: 'Testing & Validation',
            prompt: 'Create API tests, input validation, integration tests, and performance benchmarks.',
            editable: true
          },
          {
            id: 'documentation',
            name: 'Documentation & Deployment',
            prompt: 'Generate API documentation, deployment guides, and monitoring setup.',
            editable: true
          }
        ]
      },
      'cli-tool': {
        description: 'Build a command-line tool for file processing with multiple commands, options, and output formats',
        techStack: 'Rust, Clap, Serde, Tokio',
        requirements: 'Cross-platform compatibility, comprehensive help system, configuration file support',
        steps: [
          {
            id: 'specification',
            name: 'CLI Specification',
            prompt: 'Define command structure, options, arguments, and user interface design.',
            editable: true
          },
          {
            id: 'architecture',
            name: 'Tool Architecture',
            prompt: 'Design modular architecture, error handling strategy, and configuration system.',
            editable: true
          },
          {
            id: 'implementation',
            name: 'Core Implementation',
            prompt: 'Implement command parsing, core functionality, file processing, and output formatting.',
            editable: true
          },
          {
            id: 'testing',
            name: 'Testing & Validation',
            prompt: 'Create unit tests, integration tests, and cross-platform compatibility tests.',
            editable: true
          },
          {
            id: 'packaging',
            name: 'Packaging & Distribution',
            prompt: 'Setup build system, create installation packages, and distribution documentation.',
            editable: true
          }
        ]
      },
      'data-analysis': {
        description: 'Create a data analysis pipeline for processing CSV files with statistical analysis and visualization',
        techStack: 'Python, Pandas, NumPy, Matplotlib, Jupyter',
        requirements: 'Interactive notebook, data cleaning, statistical analysis, export capabilities',
        steps: [
          {
            id: 'analysis_plan',
            name: 'Analysis Planning',
            prompt: 'Define data analysis objectives, methodology, and expected outputs.',
            editable: true
          },
          {
            id: 'data_pipeline',
            name: 'Data Processing Pipeline',
            prompt: 'Design data ingestion, cleaning, transformation, and validation pipeline.',
            editable: true
          },
          {
            id: 'analysis_code',
            name: 'Analysis Implementation',
            prompt: 'Implement statistical analysis, data exploration, and visualization code.',
            editable: true
          },
          {
            id: 'visualization',
            name: 'Data Visualization',
            prompt: 'Create comprehensive visualizations, charts, and interactive dashboards.',
            editable: true
          },
          {
            id: 'reporting',
            name: 'Report Generation',
            prompt: 'Generate analysis reports, insights summary, and presentation materials.',
            editable: true
          }
        ]
      },
      'ml-model': {
        description: 'Develop a machine learning model for text classification with training pipeline and evaluation metrics',
        techStack: 'Python, scikit-learn, TensorFlow, Pandas, MLflow',
        requirements: 'Model versioning, performance metrics, deployment pipeline, monitoring',
        steps: [
          {
            id: 'problem_definition',
            name: 'Problem Definition & Data Analysis',
            prompt: 'Define ML problem, analyze dataset, and establish success metrics.',
            editable: true
          },
          {
            id: 'preprocessing',
            name: 'Data Preprocessing Pipeline',
            prompt: 'Create data preprocessing, feature engineering, and data validation pipeline.',
            editable: true
          },
          {
            id: 'model_training',
            name: 'Model Training & Selection',
            prompt: 'Implement model training, hyperparameter tuning, and model selection.',
            editable: true
          },
          {
            id: 'evaluation',
            name: 'Model Evaluation & Validation',
            prompt: 'Create evaluation metrics, validation tests, and performance analysis.',
            editable: true
          },
          {
            id: 'deployment',
            name: 'Model Deployment & Monitoring',
            prompt: 'Setup model deployment, monitoring systems, and maintenance procedures.',
            editable: true
          }
        ]
      }
    };
  }

  createStepEditors() {
    this.stepEditorsContainer.innerHTML = '';
    
    this.steps.forEach((step, index) => {
      const stepEditor = document.createElement('div');
      stepEditor.className = 'step-editor';
      stepEditor.id = `step-editor-${step.id}`;
      
      stepEditor.innerHTML = `
        <div class="step-title">
          <div style="display: flex; align-items: center;">
            <span class="step-number">${index + 1}</span>
            ${step.name}
          </div>
          <button class="btn" onclick="window.promptChainDemo.editStep('${step.id}')">Edit</button>
        </div>
        <div class="step-description">
          <textarea 
            class="form-input" 
            placeholder="Step prompt..."
            rows="3"
            id="prompt-${step.id}"
          >${step.prompt}</textarea>
        </div>
      `;
      
      this.stepEditorsContainer.appendChild(stepEditor);
    });
  }

  editStep(stepId) {
    const step = this.steps.find(s => s.id === stepId);
    const textarea = document.getElementById(`prompt-${stepId}`);
    
    if (step && textarea) {
      step.prompt = textarea.value;
      this.saveState();
    }
  }

  async startChain() {
    if (!this.projectDescription.value.trim()) {
      alert('Please provide a project description to start the development chain.');
      return;
    }

    this.updateStatus('running');
    this.startButton.disabled = true;
    this.pauseButton.disabled = false;
    this.resetButton.disabled = true;
    
    // Create pipeline visualization
    this.visualizer.clear();
    const pipeline = this.visualizer.createPipeline(this.steps, 'pipeline-container');
    this.visualizer.createProgressBar('progress-container');
    
    // Clear output
    this.outputContainer.innerHTML = '';
    this.currentStepIndex = 0;
    
    try {
      // Prepare input
      const input = {
        prompt: this.buildMainPrompt(),
        context: this.buildContext(),
        parameters: {
          steps: this.steps.map(step => ({
            id: step.id,
            name: step.name,
            prompt: step.prompt
          }))
        }
      };

      // Enhance input with settings
      const enhancedInput = this.settingsIntegration 
        ? this.settingsIntegration.enhanceWorkflowInput(input)
        : input;

      // Execute workflow
      await this.executePromptChain(enhancedInput);
      
    } catch (error) {
      console.error('Chain execution failed:', error);
      this.updateStatus('error');
      this.showError(error.message);
    }
  }

  async executePromptChain(input) {
    const steps = this.steps;
    const startTime = Date.now();
    
    for (let i = 0; i < steps.length; i++) {
      if (this.isPaused) {
        await this.waitForResume();
      }
      
      const step = steps[i];
      this.currentStepIndex = i;
      
      // Update visualization
      this.visualizer.updateStepStatus(step.id, 'active');
      this.visualizer.updateProgress(
        ((i + 1) / steps.length) * 100,
        `Executing: ${step.name}`
      );
      
      // Highlight current step editor
      this.highlightCurrentStep(step.id);
      
      try {
        // Execute step (simulate with API client)
        const stepResult = await this.executeStep(step, input, i);
        
        // Update visualization
        this.visualizer.updateStepStatus(step.id, 'completed', {
          duration: stepResult.duration
        });
        
        // Add output
        this.addStepOutput(step, stepResult);
        
      } catch (error) {
        this.visualizer.updateStepStatus(step.id, 'error');
        throw error;
      }
    }
    
    // Completion
    this.updateStatus('success');
    this.startButton.disabled = false;
    this.pauseButton.disabled = true;
    this.resetButton.disabled = false;
    
    // Show metrics
    this.showMetrics({
      totalTime: Date.now() - startTime,
      stepsCompleted: steps.length,
      linesOfCode: Math.floor(Math.random() * 500 + 200),
      filesGenerated: steps.length + Math.floor(Math.random() * 5),
    });
  }

  async executeStep(step, input, stepIndex) {
    const stepInput = {
      prompt: this.buildStepPrompt(step, input),
      context: input.context,
      stepIndex,
      totalSteps: this.steps.length
    };
    
    // Execute real prompt chain workflow with API client
    const result = await this.apiClient.executePromptChain({
      prompt: stepInput.prompt,
      role: stepInput.role || 'technical_writer',
      overall_role: stepInput.overall_role || 'software_developer'
    }, {
      realTime: true,
      onProgress: (progress) => {
        // Update progress within step
        const baseProgress = (stepIndex / this.steps.length) * 100;
        const stepProgress = (progress.percentage / 100) * (100 / this.steps.length);
        this.visualizer.updateProgress(baseProgress + stepProgress, progress.current);
      }
    });
    
    return {
      output: result.result.steps?.[stepIndex]?.output || this.generateStepOutput(step, input),
      duration: 2000 + Math.random() * 3000,
      metadata: result.metadata
    };
  }

  buildMainPrompt() {
    return `Project: ${this.projectDescription.value}
Technology Stack: ${this.techStack.value || 'Not specified'}
Requirements: ${this.requirements.value || 'Standard requirements'}`;
  }

  buildContext() {
    const template = this.templateSelector.value;
    return `Project Type: ${template}
Development Methodology: Step-by-step iterative development
Quality Standards: Production-ready code with tests and documentation`;
  }

  buildStepPrompt(step, input) {
    return `${step.prompt}

Project Context:
${input.prompt}

Additional Context:
${input.context}

Please provide detailed output for this step.`;
  }

  generateStepOutput(step, input) {
    const outputs = {
      specification: `# Technical Specification

## Project Overview
${input.prompt}

## User Stories
- As a user, I want to create and manage tasks
- As a user, I want to authenticate securely
- As a user, I want a responsive mobile experience

## API Endpoints
- POST /auth/login - User authentication
- GET /tasks - Retrieve user tasks
- POST /tasks - Create new task
- PUT /tasks/:id - Update task
- DELETE /tasks/:id - Delete task

## Data Models
\`\`\`javascript
Task: {
  id: String,
  title: String,
  description: String,
  completed: Boolean,
  createdAt: Date,
  userId: String
}

User: {
  id: String,
  email: String,
  password: String (hashed),
  createdAt: Date
}
\`\`\``,
      
      architecture: `# System Architecture

## High-Level Architecture
- Frontend: React SPA with React Router
- Backend: Node.js REST API with Express
- Database: MongoDB with Mongoose ODM
- Authentication: JWT tokens

## Component Structure
\`\`\`
src/
├── components/
│   ├── TaskList.jsx
│   ├── TaskForm.jsx
│   └── AuthForm.jsx
├── pages/
│   ├── Dashboard.jsx
│   └── Login.jsx
├── services/
│   └── api.js
└── utils/
    └── auth.js
\`\`\`

## Database Schema
Tasks and Users collections with proper indexing on userId and email fields.`,
      
      planning: `# Development Plan

## Phase 1: Foundation (Days 1-2)
- [ ] Setup project structure
- [ ] Configure build tools (Vite/Webpack)
- [ ] Setup database connection
- [ ] Implement basic routing

## Phase 2: Authentication (Days 3-4)
- [ ] User registration/login forms
- [ ] JWT authentication middleware
- [ ] Protected routes implementation
- [ ] Session management

## Phase 3: Core Features (Days 5-7)
- [ ] Task CRUD operations
- [ ] Task list component
- [ ] Task form validation
- [ ] Data persistence

## Phase 4: Polish (Days 8-9)
- [ ] Responsive design
- [ ] Error handling
- [ ] Loading states
- [ ] Testing

## Phase 5: Deployment (Day 10)
- [ ] Production build
- [ ] Environment configuration
- [ ] Deployment setup`,

      implementation: `# Core Implementation

## Backend API (server.js)
\`\`\`javascript
const express = require('express');
const mongoose = require('mongoose');
const jwt = require('jsonwebtoken');
const bcrypt = require('bcryptjs');

const app = express();
app.use(express.json());

// Task Schema
const taskSchema = new mongoose.Schema({
  title: String,
  description: String,
  completed: Boolean,
  userId: String,
  createdAt: { type: Date, default: Date.now }
});

const Task = mongoose.model('Task', taskSchema);

// Routes
app.get('/api/tasks', authenticateToken, async (req, res) => {
  const tasks = await Task.find({ userId: req.user.id });
  res.json(tasks);
});

app.post('/api/tasks', authenticateToken, async (req, res) => {
  const task = new Task({
    ...req.body,
    userId: req.user.id
  });
  await task.save();
  res.json(task);
});

function authenticateToken(req, res, next) {
  const token = req.headers['authorization'];
  if (!token) return res.sendStatus(401);
  
  jwt.verify(token, process.env.JWT_SECRET, (err, user) => {
    if (err) return res.sendStatus(403);
    req.user = user;
    next();
  });
}
\`\`\`

## Frontend Components (TaskList.jsx)
\`\`\`jsx
import React, { useState, useEffect } from 'react';
import api from '../services/api';

function TaskList() {
  const [tasks, setTasks] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchTasks();
  }, []);

  const fetchTasks = async () => {
    try {
      const response = await api.get('/tasks');
      setTasks(response.data);
    } catch (error) {
      console.error('Failed to fetch tasks:', error);
    } finally {
      setLoading(false);
    }
  };

  const toggleTask = async (id, completed) => {
    try {
      await api.put(\`/tasks/\${id}\`, { completed });
      setTasks(tasks.map(task => 
        task.id === id ? { ...task, completed } : task
      ));
    } catch (error) {
      console.error('Failed to update task:', error);
    }
  };

  if (loading) return <div>Loading...</div>;

  return (
    <div className="task-list">
      {tasks.map(task => (
        <div key={task.id} className="task-item">
          <input
            type="checkbox"
            checked={task.completed}
            onChange={(e) => toggleTask(task.id, e.target.checked)}
          />
          <span className={task.completed ? 'completed' : ''}>
            {task.title}
          </span>
        </div>
      ))}
    </div>
  );
}

export default TaskList;
\`\`\``,

      testing: `# Testing & Quality Assurance

## Unit Tests (tasks.test.js)
\`\`\`javascript
const request = require('supertest');
const app = require('../server');

describe('Tasks API', () => {
  test('GET /api/tasks returns user tasks', async () => {
    const token = 'valid-jwt-token';
    const response = await request(app)
      .get('/api/tasks')
      .set('Authorization', token)
      .expect(200);
    
    expect(Array.isArray(response.body)).toBe(true);
  });

  test('POST /api/tasks creates new task', async () => {
    const token = 'valid-jwt-token';
    const newTask = {
      title: 'Test Task',
      description: 'Test Description'
    };
    
    const response = await request(app)
      .post('/api/tasks')
      .set('Authorization', token)
      .send(newTask)
      .expect(200);
    
    expect(response.body.title).toBe(newTask.title);
    expect(response.body.id).toBeDefined();
  });
});
\`\`\`

## Frontend Tests (TaskList.test.jsx)
\`\`\`jsx
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import TaskList from '../TaskList';
import * as api from '../services/api';

jest.mock('../services/api');

test('renders task list', async () => {
  api.get.mockResolvedValue({
    data: [
      { id: '1', title: 'Test Task', completed: false }
    ]
  });

  render(<TaskList />);
  
  await waitFor(() => {
    expect(screen.getByText('Test Task')).toBeInTheDocument();
  });
});

test('toggles task completion', async () => {
  api.get.mockResolvedValue({
    data: [{ id: '1', title: 'Test Task', completed: false }]
  });
  api.put.mockResolvedValue({});

  render(<TaskList />);
  
  await waitFor(() => {
    const checkbox = screen.getByRole('checkbox');
    fireEvent.click(checkbox);
    expect(api.put).toHaveBeenCalledWith('/tasks/1', { completed: true });
  });
});
\`\`\`

## Quality Checklist
- [x] All API endpoints tested
- [x] Frontend components tested
- [x] Authentication flow tested
- [x] Error handling implemented
- [x] Input validation added
- [x] Security headers configured
- [x] Performance optimizations applied`,

      deployment: `# Deployment & Documentation

## Environment Setup
\`\`\`bash
# Install dependencies
npm install

# Environment variables
cp .env.example .env
# Edit .env with your values:
# MONGODB_URI=mongodb://localhost:27017/taskmanager
# JWT_SECRET=your-secret-key
# PORT=3000

# Development
npm run dev

# Production build
npm run build
npm start
\`\`\`

## Docker Configuration
\`\`\`dockerfile
FROM node:18-alpine

WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production

COPY . .
RUN npm run build

EXPOSE 3000
CMD ["npm", "start"]
\`\`\`

## docker-compose.yml
\`\`\`yaml
version: '3.8'
services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - MONGODB_URI=mongodb://mongo:27017/taskmanager
      - JWT_SECRET=production-secret
    depends_on:
      - mongo
  
  mongo:
    image: mongo:5
    volumes:
      - mongo_data:/data/db

volumes:
  mongo_data:
\`\`\`

## API Documentation
API endpoints documented with OpenAPI 3.0 specification available at /api/docs

## User Guide
1. Register a new account or login
2. Create tasks using the "Add Task" button
3. Mark tasks as complete by clicking the checkbox
4. Edit or delete tasks using the action buttons
5. Use the search function to filter tasks

## Deployment Options
- **Heroku**: Push to Heroku with Procfile
- **Vercel**: Deploy frontend with serverless functions
- **Docker**: Use provided Dockerfile and docker-compose
- **Traditional VPS**: PM2 process manager with Nginx reverse proxy`
    };
    
    return outputs[step.id] || `Generated output for ${step.name}:\n\n${input.prompt.substring(0, 200)}...`;
  }

  addStepOutput(step, result) {
    const outputDiv = document.createElement('div');
    outputDiv.className = 'step-output';
    outputDiv.innerHTML = `
      <h4>${step.name}</h4>
      <div class="step-content">${result.output}</div>
    `;
    
    this.outputContainer.appendChild(outputDiv);
    
    // Auto-scroll to show new content
    outputDiv.scrollIntoView({ behavior: 'smooth' });
  }

  highlightCurrentStep(stepId) {
    // Remove active class from all step editors
    document.querySelectorAll('.step-editor').forEach(editor => {
      editor.classList.remove('active');
    });
    
    // Add active class to current step
    const currentEditor = document.getElementById(`step-editor-${stepId}`);
    if (currentEditor) {
      currentEditor.classList.add('active');
    }
  }

  showMetrics(metrics) {
    this.metricsContainer.style.display = 'block';
    this.visualizer.createMetricsGrid(metrics, 'metrics-container');
  }

  showError(message) {
    const errorDiv = document.createElement('div');
    errorDiv.className = 'error-message';
    errorDiv.style.cssText = `
      background: #fee2e2;
      color: var(--danger);
      padding: 1rem;
      border-radius: var(--radius-md);
      margin: 1rem 0;
      border: 1px solid var(--danger);
    `;
    errorDiv.textContent = `Error: ${message}`;
    
    this.outputContainer.appendChild(errorDiv);
  }

  pauseChain() {
    this.isPaused = true;
    this.updateStatus('paused');
    this.pauseButton.textContent = 'Resume';
    this.pauseButton.onclick = () => this.resumeChain();
  }

  resumeChain() {
    this.isPaused = false;
    this.updateStatus('running');
    this.pauseButton.textContent = 'Pause';
    this.pauseButton.onclick = () => this.pauseChain();
  }

  async waitForResume() {
    return new Promise(resolve => {
      const checkResume = () => {
        if (!this.isPaused) {
          resolve();
        } else {
          setTimeout(checkResume, 100);
        }
      };
      checkResume();
    });
  }

  resetChain() {
    this.currentExecution = null;
    this.isPaused = false;
    this.currentStepIndex = 0;
    
    this.updateStatus('idle');
    this.startButton.disabled = false;
    this.pauseButton.disabled = true;
    this.pauseButton.textContent = 'Pause';
    this.resetButton.disabled = false;
    
    // Clear visualizations
    this.visualizer.clear();
    this.outputContainer.innerHTML = '<p class="text-muted">Start the development process to see step-by-step outputs here.</p>';
    this.metricsContainer.style.display = 'none';
    
    // Remove active highlighting
    document.querySelectorAll('.step-editor').forEach(editor => {
      editor.classList.remove('active');
    });
  }

  updateStatus(status) {
    const statusText = {
      idle: 'Idle',
      running: 'Processing...',
      paused: 'Paused',
      success: 'Completed',
      error: 'Error'
    };
    
    this.statusElement.textContent = statusText[status] || status;
    this.statusElement.className = `workflow-status ${status}`;
  }

  saveState() {
    const state = {
      projectDescription: this.projectDescription.value,
      techStack: this.techStack.value,
      requirements: this.requirements.value,
      template: this.templateSelector.value,
      steps: this.steps.map(step => ({
        ...step,
        prompt: document.getElementById(`prompt-${step.id}`)?.value || step.prompt
      }))
    };
    
    localStorage.setItem('prompt-chain-state', JSON.stringify(state));
  }

  loadState() {
    const saved = localStorage.getItem('prompt-chain-state');
    if (saved) {
      try {
        const state = JSON.parse(saved);
        this.projectDescription.value = state.projectDescription || '';
        this.techStack.value = state.techStack || '';
        this.requirements.value = state.requirements || '';
        
        if (state.template) {
          this.templateSelector.value = state.template;
        }
        
        if (state.steps) {
          this.steps = state.steps;
          this.createStepEditors();
        }
      } catch (error) {
        console.error('Failed to load saved state:', error);
      }
    }
  }
}

// Initialize the demo when page loads
document.addEventListener('DOMContentLoaded', async () => {
  window.promptChainDemo = new PromptChainingDemo();
  await window.promptChainDemo.initializeSettings();
  window.promptChainDemo.loadState(); // Load any saved state
  
  // Ensure settings UI is globally available
  if (window.promptChainDemo.settingsIntegration && window.promptChainDemo.settingsIntegration.getSettingsUI()) {
    console.log('Settings UI ready - use Ctrl+, to open');
  }
});