# Plan Implementation Routing

Plan implementation routing distinguishes between **strategic planning** (high-level architecture and design) and **tactical implementation** (writing code, building features, creating deliverables).

## Overview

This routing scenario activates when a query contains both "plan" and implementation-related keywords. It routes to smaller, faster models suitable for coding and implementation tasks rather than strategic reasoning models.

## Triggers

Plan implementation routing is triggered when:
- The query contains "plan" AND
- One or more implementation keywords:
  - implementation
  - implement  
  - build
  - code
  - develop
  - write
  - create

## Examples

### Triggers Plan Implementation
- "plan implementation of the authentication system"
- "plan to build the user dashboard"
- "plan code structure for the API"
- "plan and develop the frontend components"
- "create a plan to implement caching"

### Triggers Think Routing (Strategic)
- "plan the system architecture"
- "design a plan for the product"
- "create a strategic plan"
- "plan the approach" (without implementation keywords)

## Configuration

```toml
[router]
# Strategic planning - high-performance models
think = "openrouter,anthropic/claude-3-opus"

# Tactical implementation - smaller, faster models
plan_implementation = "openrouter,anthropic/claude-3.5-haiku"
```

## Route

Primary route: `openrouter, anthropic/claude-3.5-haiku`

Alternative routes by provider:
- **OpenRouter**: `anthropic/claude-3.5-haiku` (fast, cost-effective)
- **Groq**: `llama-3.1-8b-instant` (very fast inference)
- **OpenAI**: `gpt-4o-mini` (efficient for coding)

## Use Cases

Optimal for:
- Writing code based on an existing plan
- Implementing specific features
- Building prototypes and MVPs
- Creating implementation details
- Coding tasks and script writing
- Writing tests based on requirements
- Building UI components
- Implementing API endpoints
- Writing documentation code examples

## Model Characteristics

Plan implementation models should be:
- **Fast**: Low latency for iterative coding
- **Cost-effective**: Cheaper per token for large code generation
- **Code-capable**: Strong coding abilities
- **Context-efficient**: Good at working with existing code context

Not suitable for:
- Architecture design (use think routing)
- Complex reasoning (use think routing)
- Research and analysis (use think or web_search routing)

## Priority

Plan implementation routing has priority over think routing. When both "plan" and implementation keywords are present, the query is routed to the plan_implementation provider rather than the think provider.

## Related Scenarios

- **Think Routing**: For strategic planning without implementation
- **Background Routing**: For batch/low-priority implementation tasks
- **Default Routing**: Falls back to default if plan_implementation not configured

## Performance

Typical response times:
- Claude 3.5 Haiku: ~500-800ms
- Llama 3.1 8B: ~200-400ms (Groq)
- GPT-4o Mini: ~300-600ms

## Cost Comparison

Compared to think routing models:
- ~10x cheaper than Claude 3 Opus
- ~5x cheaper than GPT-4 Turbo
- ~3x cheaper than Claude 3.5 Sonnet

## Configuration Examples

### Cost-Optimized Setup
```toml
[router]
think = "openrouter,anthropic/claude-3-opus"
plan_implementation = "groq,llama-3.1-8b-instant"
```

### Quality-Optimized Setup  
```toml
[router]
think = "openrouter,anthropic/claude-3-opus"
plan_implementation = "openrouter,anthropic/claude-3.5-sonnet"
```

### Balanced Setup
```toml
[router]
think = "openrouter,anthropic/claude-3.5-sonnet"
plan_implementation = "openrouter,anthropic/claude-3.5-haiku"
```

## Monitoring

Log indicators:
```
INFO: Plan implementation detected - routing to smaller model
Provider: openrouter, Model: anthropic/claude-3.5-haiku
Scenario: plan_implementation
```

## Testing

Test queries:
```bash
# Should route to plan_implementation
curl -X POST http://localhost:3456/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model": "claude-3-opus", "messages": [{"role": "user", "content": "plan implementation of login"}]}'

# Should route to think
curl -X POST http://localhost:3456/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model": "claude-3-opus", "messages": [{"role": "user", "content": "plan the architecture"}]}'
```

## History

Added: January 2026
Purpose: Distinguish strategic vs tactical AI coding assistance
Requested by: User requirement for intelligent model selection based on task type
