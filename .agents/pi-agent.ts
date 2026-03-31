/**
 * pi-agent - AI coding assistant with kimi-code and z.ai coding plans
 *
 * This agent is configured to use subscription-based coding-focused LLM models:
 * - kimi-for-coding/* (Moonshot Kimi K2 subscription)
 * - zai-coding-plan/* (z.ai GLM-4.5 subscription)
 *
 * Supports advanced coding tasks, code review, and intelligent code generation.
 * 
 * Provider routing configuration:
 * - kimi-for-coding/* - Moonshot subscription models optimized for coding
 * - zai-coding-plan/* - z.ai subscription coding plans
 */

import type { AgentDefinition } from './types/agent-definition'

const definition: AgentDefinition = {
  id: 'pi-agent',
  displayName: 'PI Agent',
  version: '1.0.0',
  publisher: 'terraphim',

  // Primary model: kimi-for-coding subscription (Moonshot Kimi K2.5 optimized for coding)
  // Fallback chain: kimi-for-coding -> zai-coding-plan -> direct models
  model: 'kimi-for-coding/kimi-k2.5',

  // Available model fallbacks for different coding tasks
  reasoningOptions: {
    enabled: true,
    effort: 'high',
  },

  // Comprehensive tool set for coding tasks
  toolNames: [
    // File operations
    'read_files',
    'write_file',
    'str_replace',
    'find_files',
    // Code analysis
    'code_search',
    // Terminal
    'run_terminal_command',
    // Web search for documentation
    'web_search',
    'read_docs',
    // Agent management
    'spawn_agents',
    // Planning
    'think_deeply',
    // Output control
    'set_output',
    'end_turn',
  ],

  // Sub-agents for specialized tasks
  spawnableAgents: [
    'file-explorer',
    'code-reviewer',
    'test-writer',
  ],

  // Input schema for structured prompts
  inputSchema: {
    prompt: {
      type: 'string',
      description: 'The coding task or question to help with',
    },
    params: {
      type: 'object',
      properties: {
        subscription_plan: {
          type: 'string',
          enum: ['kimi-for-coding', 'zai-coding-plan', 'auto'],
          description: 'Preferred coding plan subscription: kimi-for-coding (Moonshot), zai-coding-plan (z.ai), or auto-routing',
        },
        task_type: {
          type: 'string',
          enum: ['code_generation', 'code_review', 'refactoring', 'debugging', 'documentation'],
          description: 'Type of coding task',
        },
        language: {
          type: 'string',
          description: 'Programming language (e.g., rust, typescript, python)',
        },
      },
    },
  },

  // Output structured responses
  outputMode: 'structured_output',
  outputSchema: {
    type: 'object',
    properties: {
      summary: {
        type: 'string',
        description: 'Brief summary of the task completion',
      },
      code_changes: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            file_path: { type: 'string' },
            change_type: { type: 'string', enum: ['create', 'modify', 'delete'] },
            description: { type: 'string' },
          },
        },
      },
      suggestions: {
        type: 'array',
        items: { type: 'string' },
        description: 'Additional improvement suggestions',
      },
    },
  },

  // Spawner prompt for when other agents invoke pi-agent
  spawnerPrompt: `Spawn pi-agent when you need advanced coding assistance with:
- Code generation and implementation
- Code review and quality assessment  
- Refactoring and optimization
- Debugging and troubleshooting
- Technical documentation

PI Agent supports subscription-based coding plans:
- kimi-for-coding/* (Moonshot subscription): Best for complex reasoning and long-context coding
- zai-coding-plan/* (z.ai subscription): Excellent for structured code generation

Specify subscription_plan in params to choose a specific coding plan.`,

  // System prompt with coding context
  systemPrompt: `You are PI Agent, an expert AI coding assistant powered by kimi-code and z.ai coding plans.

Your capabilities include:
- Writing clean, efficient, and well-documented code
- Performing thorough code reviews
- Refactoring and optimizing existing code
- Debugging complex issues
- Explaining technical concepts clearly

When working with code:
1. Always follow language-specific best practices
2. Include appropriate error handling
3. Write comprehensive comments for complex logic
4. Consider performance implications
5. Maintain consistency with existing codebase style

You have access to multiple specialized models optimized for coding tasks.`,

  // Instructions for each interaction
  instructionsPrompt: `Approach each coding task systematically:

1. **Understand the Context**: Read relevant files and understand the codebase structure
2. **Analyze the Task**: Break down complex tasks into manageable steps
3. **Implement Thoughtfully**: Write code that is correct, efficient, and maintainable
4. **Validate**: Check for errors, test edge cases, and ensure quality
5. **Document**: Add clear comments and explanations where needed

For code generation:
- Follow existing patterns in the codebase
- Use idiomatic language features
- Include appropriate imports and dependencies
- Handle errors gracefully

For code review:
- Check for correctness and edge cases
- Identify potential bugs and security issues
- Suggest improvements for readability and performance
- Verify consistency with project conventions

For debugging:
- Reproduce the issue first
- Use systematic debugging approaches
- Check logs and error messages
- Verify fixes thoroughly`,
}

export default definition
