/**
 * Terraphim VM Execution Client for Workflows
 * Provides workflow-level VM execution with success/rollback handling
 */

class VmExecutionClient {
  constructor(apiClient, options = {}) {
    this.apiClient = apiClient;
    this.options = {
      maxRetries: options.maxRetries || 3,
      retryDelay: options.retryDelay || 1000,
      autoRollback: options.autoRollback !== false,
      snapshotOnExecution: options.snapshotOnExecution || false,
      snapshotOnFailure: options.snapshotOnFailure !== false,
      timeout: options.timeout || 30000,
      ...options
    };
    
    this.executionHistory = [];
    this.vmSessions = new Map();
    this.activeSnapshots = new Map();
  }

  async executeCode(request) {
    const {
      language,
      code,
      agentId = 'workflow-agent',
      vmId = null,
      requirements = [],
      workingDir = null,
      metadata = {},
      onProgress = null
    } = request;

    const executionId = this.generateExecutionId();
    const startTime = Date.now();

    if (onProgress) {
      onProgress({ status: 'validating', executionId });
    }

    const validation = this.validateCode(language, code);
    if (!validation.valid) {
      return {
        success: false,
        executionId,
        error: validation.error,
        blocked: true,
        reason: validation.reason
      };
    }

    let snapshotId = null;
    if (this.options.snapshotOnExecution && vmId) {
      if (onProgress) {
        onProgress({ status: 'creating_snapshot', executionId, vmId });
      }
      
      try {
        snapshotId = await this.createSnapshot(vmId, `pre-execution-${executionId}`);
        this.activeSnapshots.set(executionId, {
          snapshotId,
          vmId,
          timestamp: Date.now()
        });
      } catch (error) {
        console.warn('Failed to create pre-execution snapshot:', error);
      }
    }

    const executeRequest = {
      agent_id: agentId,
      language,
      code,
      vm_id: vmId,
      requirements,
      timeout_seconds: Math.floor(this.options.timeout / 1000),
      working_dir: workingDir,
      metadata: {
        ...metadata,
        execution_id: executionId,
        snapshot_id: snapshotId,
        workflow_timestamp: startTime
      }
    };

    if (onProgress) {
      onProgress({ status: 'executing', executionId, vmId });
    }

    let attempts = 0;
    let lastError = null;

    while (attempts < this.options.maxRetries) {
      attempts++;

      try {
        const response = await this.apiClient.request('/api/llm/execute', {
          method: 'POST',
          body: JSON.stringify(executeRequest)
        });

        const result = {
          success: response.exit_code === 0,
          executionId,
          vmId: response.vm_id || vmId,
          exitCode: response.exit_code,
          stdout: response.stdout || '',
          stderr: response.stderr || '',
          duration: Date.now() - startTime,
          attempts,
          snapshotId,
          timestamp: new Date().toISOString()
        };

        this.executionHistory.push(result);

        if (result.success) {
          if (onProgress) {
            onProgress({ status: 'completed', ...result });
          }
          return result;
        } else {
          if (this.options.snapshotOnFailure && result.vmId && !snapshotId) {
            try {
              const failureSnapshotId = await this.createSnapshot(
                result.vmId,
                `failure-${executionId}`
              );
              result.failureSnapshotId = failureSnapshotId;
            } catch (error) {
              console.warn('Failed to create failure snapshot:', error);
            }
          }

          if (this.options.autoRollback && snapshotId) {
            if (onProgress) {
              onProgress({ 
                status: 'rolling_back', 
                executionId, 
                snapshotId 
              });
            }

            try {
              await this.rollbackToSnapshot(result.vmId, snapshotId);
              result.rolledBack = true;
              result.restoredSnapshot = snapshotId;
            } catch (rollbackError) {
              console.error('Rollback failed:', rollbackError);
              result.rollbackError = rollbackError.message;
            }
          }

          if (attempts < this.options.maxRetries) {
            if (onProgress) {
              onProgress({ 
                status: 'retrying', 
                executionId, 
                attempt: attempts + 1,
                maxRetries: this.options.maxRetries
              });
            }
            await this.delay(this.options.retryDelay * attempts);
            continue;
          }

          result.retriesExhausted = true;
          this.executionHistory.push(result);
          
          if (onProgress) {
            onProgress({ status: 'failed', ...result });
          }

          return result;
        }
      } catch (error) {
        lastError = error;
        
        if (attempts < this.options.maxRetries) {
          await this.delay(this.options.retryDelay * attempts);
          continue;
        }
        
        const errorResult = {
          success: false,
          executionId,
          vmId,
          error: error.message,
          duration: Date.now() - startTime,
          attempts,
          snapshotId,
          timestamp: new Date().toISOString()
        };

        this.executionHistory.push(errorResult);
        
        if (onProgress) {
          onProgress({ status: 'error', ...errorResult });
        }

        return errorResult;
      }
    }

    throw lastError || new Error('Execution failed after retries');
  }

  async parseAndExecute(text, options = {}) {
    const codeBlocks = this.extractCodeBlocks(text);
    
    if (codeBlocks.length === 0) {
      return {
        success: false,
        error: 'No code blocks found in text',
        noCodeFound: true
      };
    }

    const results = [];

    for (const block of codeBlocks) {
      const result = await this.executeCode({
        language: block.language,
        code: block.code,
        ...options,
        metadata: {
          ...options.metadata,
          blockIndex: block.index,
          totalBlocks: codeBlocks.length
        }
      });

      results.push(result);

      if (!result.success && options.stopOnFailure !== false) {
        break;
      }
    }

    return {
      success: results.every(r => r.success),
      results,
      totalBlocks: codeBlocks.length,
      successfulBlocks: results.filter(r => r.success).length
    };
  }

  extractCodeBlocks(text) {
    const codeBlockRegex = /```(\w+)?\n([\s\S]*?)```/g;
    const blocks = [];
    let match;
    let index = 0;

    while ((match = codeBlockRegex.exec(text)) !== null) {
      blocks.push({
        index,
        language: match[1] || 'python',
        code: match[2].trim(),
        raw: match[0]
      });
      index++;
    }

    return blocks;
  }

  validateCode(language, code) {
    const supportedLanguages = ['python', 'javascript', 'bash', 'rust', 'go'];
    
    if (!supportedLanguages.includes(language)) {
      return {
        valid: false,
        error: `Unsupported language: ${language}`,
        reason: `Supported languages: ${supportedLanguages.join(', ')}`
      };
    }

    if (code.length > 10000) {
      return {
        valid: false,
        error: 'Code exceeds maximum length of 10,000 characters',
        reason: 'Code too long'
      };
    }

    const dangerousPatterns = [
      { pattern: /rm\s+-rf\s+\//, reason: 'Dangerous file deletion' },
      { pattern: /curl.*\|\s*sh/, reason: 'Remote code execution' },
      { pattern: /eval\s*\(/, reason: 'Dynamic code evaluation' },
      { pattern: /exec\s*\(/, reason: 'Code execution function' },
      { pattern: /__import__\s*\(\s*['"]os['"]\s*\)/, reason: 'OS module import' }
    ];

    for (const { pattern, reason } of dangerousPatterns) {
      if (pattern.test(code)) {
        return {
          valid: false,
          error: `Security validation failed: ${reason}`,
          reason
        };
      }
    }

    return { valid: true };
  }

  async createSnapshot(vmId, snapshotName) {
    try {
      const response = await this.apiClient.request(
        `/api/vms/${vmId}/snapshots`,
        {
          method: 'POST',
          body: JSON.stringify({
            name: snapshotName,
            description: `Workflow snapshot: ${snapshotName}`
          })
        }
      );

      return response.snapshot_id;
    } catch (error) {
      console.error('Failed to create snapshot:', error);
      throw error;
    }
  }

  async rollbackToSnapshot(vmId, snapshotId) {
    try {
      const response = await this.apiClient.request(
        `/api/vms/${vmId}/rollback/${snapshotId}`,
        {
          method: 'POST'
        }
      );

      return {
        success: true,
        vmId,
        snapshotId,
        message: response.message
      };
    } catch (error) {
      console.error('Rollback failed:', error);
      throw error;
    }
  }

  async getExecutionHistory(vmId = null) {
    if (vmId) {
      try {
        const response = await this.apiClient.request(
          `/api/vms/${vmId}/history`
        );
        return response.history || [];
      } catch (error) {
        console.error('Failed to fetch history:', error);
        return this.executionHistory.filter(h => h.vmId === vmId);
      }
    }

    return this.executionHistory;
  }

  async rollbackToLastSuccess(vmId, agentId) {
    const history = await this.getExecutionHistory(vmId);
    const lastSuccess = history
      .filter(h => h.success && h.snapshotId)
      .sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp))[0];

    if (!lastSuccess) {
      throw new Error('No successful snapshot found for rollback');
    }

    return this.rollbackToSnapshot(vmId, lastSuccess.snapshotId);
  }

  getActiveSnapshots() {
    return Array.from(this.activeSnapshots.values());
  }

  clearHistory() {
    this.executionHistory = [];
    this.activeSnapshots.clear();
  }

  generateExecutionId() {
    return `exec-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }

  delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = VmExecutionClient;
}
