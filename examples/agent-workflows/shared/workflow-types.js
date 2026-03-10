/**
 * Workflow Types - Shared type definitions for agent workflow examples
 * Provides consistent types across all 5 workflow patterns
 */

/**
 * Workflow pattern identifiers
 * @readonly
 * @enum {string}
 */
const WorkflowPatterns = {
  PROMPT_CHAIN: 'prompt_chain',
  ROUTING: 'routing',
  PARALLEL: 'parallel',
  ORCHESTRATION: 'orchestration',
  OPTIMIZATION: 'optimization'
};

/**
 * Workflow execution status
 * @readonly
 * @enum {string}
 */
const WorkflowStatus = {
  PENDING: 'pending',
  STARTING: 'starting',
  RUNNING: 'running',
  COMPLETED: 'completed',
  FAILED: 'failed',
  CANCELLED: 'cancelled'
};

/**
 * WebSocket message types
 * @readonly
 * @enum {string}
 */
const WebSocketMessageTypes = {
  STATUS: 'status',
  PROGRESS: 'progress',
  RESULT: 'result',
  ERROR: 'error',
  CONNECTED: 'connected',
  DISCONNECTED: 'disconnected'
};

/**
 * @typedef {Object} WorkflowRequest
 * @property {string} prompt - The main prompt/task
 * @property {string} [role] - Agent role to use
 * @property {string} [overall_role] - Overall coordinator role
 * @property {WorkflowStep[]} [steps] - Custom workflow steps
 * @property {LlmConfig} [llm_config] - LLM configuration
 */

/**
 * @typedef {Object} WorkflowStep
 * @property {string} id - Step identifier
 * @property {string} name - Step name
 * @property {string} prompt - Step prompt
 * @property {string} [role] - Step-specific role
 * @property {Object} [result] - Step result (populated after execution)
 */

/**
 * @typedef {Object} LlmConfig
 * @property {string} [provider] - LLM provider (ollama, openrouter)
 * @property {string} [model] - Model name
 * @property {string} [base_url] - Base URL for provider
 * @property {number} [temperature] - Temperature (0-1)
 * @property {number} [max_tokens] - Maximum tokens
 * @property {Object} [extra] - Additional provider-specific options
 */

/**
 * @typedef {Object} WorkflowResponse
 * @property {string} workflow_id - Unique workflow ID
 * @property {boolean} success - Whether workflow succeeded
 * @property {Object} [result] - Workflow result data
 * @property {string} [error] - Error message if failed
 * @property {WorkflowMetadata} metadata - Execution metadata
 */

/**
 * @typedef {Object} WorkflowMetadata
 * @property {number} execution_time_ms - Execution time in milliseconds
 * @property {string} pattern - Workflow pattern used
 * @property {number} steps - Number of steps executed
 * @property {string} role - Role used
 * @property {string} overall_role - Overall coordinator role
 */

/**
 * @typedef {Object} WorkflowStatusResponse
 * @property {string} workflow_id - Workflow ID
 * @property {string} status - Current status (from WorkflowStatus)
 * @property {Object} [result] - Result if completed
 * @property {string} [error] - Error if failed
 * @property {number} progress - Progress percentage (0-100)
 * @property {string} [current_step] - Current step name if running
 */

/**
 * @typedef {Object} WebSocketMessage
 * @property {string} type - Message type (from WebSocketMessageTypes)
 * @property {string} workflow_id - Associated workflow ID
 * @property {string} status - Workflow status
 * @property {Object} [data] - Additional data
 * @property {string} timestamp - ISO timestamp
 */

/**
 * @typedef {Object} RoutingDecision
 * @property {string} model - Selected model
 * @property {string} complexity - Task complexity level
 * @property {number} confidence - Routing confidence (0-1)
 * @property {string} reason - Routing explanation
 */

/**
 * @typedef {Object} ParallelPerspective
 * @property {string} id - Perspective identifier
 * @property {string} name - Perspective name
 * @property {string} status - Execution status
 * @property {string} [result] - Analysis result
 * @property {number} [progress] - Progress percentage
 */

/**
 * @typedef {Object} OptimizationIteration
 * @property {number} iteration - Iteration number
 * @property {string} content - Generated content
 * @property {number} quality_score - Quality score (0-1)
 * @property {Object} [criteria_scores] - Individual criterion scores
 * @property {string} [feedback] - Evaluator feedback
 */

/**
 * @typedef {Object} WorkerStatus
 * @property {string} id - Worker identifier
 * @property {string} name - Worker name
 * @property {string} status - Current status (idle, working, completed, error)
 * @property {string} [task] - Current task description
 * @property {number} [progress] - Progress percentage
 */

/**
 * Default LLM configuration for local Ollama
 * @returns {LlmConfig}
 */
function getDefaultLlmConfig() {
  return {
    provider: 'ollama',
    model: 'gemma3:270m',
    base_url: 'http://127.0.0.1:11434',
    temperature: 0.7,
    max_tokens: 2048
  };
}

/**
 * Create a workflow request with defaults
 * @param {string} prompt - The main prompt
 * @param {Object} [options] - Optional overrides
 * @returns {WorkflowRequest}
 */
function createWorkflowRequest(prompt, options = {}) {
  return {
    prompt,
    role: options.role || 'DevelopmentAgent',
    overall_role: options.overallRole || options.role || 'DevelopmentAgent',
    steps: options.steps || null,
    llm_config: options.llmConfig || getDefaultLlmConfig()
  };
}

/**
 * Validate a workflow request
 * @param {WorkflowRequest} request
 * @returns {Object} { valid: boolean, errors: string[] }
 */
function validateWorkflowRequest(request) {
  const errors = [];

  if (!request || typeof request !== 'object') {
    errors.push('Request must be an object');
    return { valid: false, errors };
  }

  if (!request.prompt || typeof request.prompt !== 'string') {
    errors.push('Request must have a non-empty prompt string');
  }

  if (request.llm_config) {
    const llm = request.llm_config;
    if (llm.temperature !== undefined && (llm.temperature < 0 || llm.temperature > 1)) {
      errors.push('LLM temperature must be between 0 and 1');
    }
    if (llm.max_tokens !== undefined && llm.max_tokens < 1) {
      errors.push('LLM max_tokens must be positive');
    }
  }

  return {
    valid: errors.length === 0,
    errors
  };
}

/**
 * Check if a status is terminal (completed, failed, or cancelled)
 * @param {string} status
 * @returns {boolean}
 */
function isTerminalStatus(status) {
  return [
    WorkflowStatus.COMPLETED,
    WorkflowStatus.FAILED,
    WorkflowStatus.CANCELLED
  ].includes(status);
}

/**
 * Get FontAwesome icon class for workflow status
 * @param {string} status
 * @returns {string}
 */
function getStatusIconClass(status) {
  const iconMap = {
    [WorkflowStatus.PENDING]: 'fas fa-clock',
    [WorkflowStatus.STARTING]: 'fas fa-play-circle',
    [WorkflowStatus.RUNNING]: 'fas fa-spinner fa-spin',
    [WorkflowStatus.COMPLETED]: 'fas fa-check-circle',
    [WorkflowStatus.FAILED]: 'fas fa-times-circle',
    [WorkflowStatus.CANCELLED]: 'fas fa-ban'
  };
  return iconMap[status] || 'fas fa-question-circle';
}

/**
 * Get CSS class for workflow status
 * @param {string} status
 * @returns {string}
 */
function getStatusCssClass(status) {
  const cssMap = {
    [WorkflowStatus.PENDING]: 'status-pending',
    [WorkflowStatus.STARTING]: 'status-starting',
    [WorkflowStatus.RUNNING]: 'status-running',
    [WorkflowStatus.COMPLETED]: 'status-completed',
    [WorkflowStatus.FAILED]: 'status-failed',
    [WorkflowStatus.CANCELLED]: 'status-cancelled'
  };
  return cssMap[status] || 'status-unknown';
}

// Export for both module and script tag usage
const WorkflowTypes = {
  WorkflowPatterns,
  WorkflowStatus,
  WebSocketMessageTypes,
  getDefaultLlmConfig,
  createWorkflowRequest,
  validateWorkflowRequest,
  isTerminalStatus,
  getStatusIconClass,
  getStatusCssClass
};

// Browser global export
if (typeof window !== 'undefined') {
  window.WorkflowTypes = WorkflowTypes;
  window.WorkflowPatterns = WorkflowPatterns;
  window.WorkflowStatus = WorkflowStatus;
  window.WebSocketMessageTypes = WebSocketMessageTypes;
  window.getDefaultLlmConfig = getDefaultLlmConfig;
  window.createWorkflowRequest = createWorkflowRequest;
  window.validateWorkflowRequest = validateWorkflowRequest;
  window.isTerminalStatus = isTerminalStatus;
  window.getStatusIconClass = getStatusIconClass;
  window.getStatusCssClass = getStatusCssClass;
}

// Node.js module export
if (typeof module !== 'undefined' && module.exports) {
  module.exports = WorkflowTypes;
}
