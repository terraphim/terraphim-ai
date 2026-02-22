# Think Routing

Think routing (also known as reasoning mode) is used for complex problem-solving tasks that require deep reasoning, step-by-step analysis, or extended chain-of-thought processing. This routing scenario activates when Claude Code enters "Plan Mode" or when explicit thinking is required.

In claude-code-router, think routing is triggered when:
- The request includes a `thinking` parameter
- Plan Mode is activated in Claude Code
- Complex reasoning tasks are detected

Configuration example:
```json
"Router": {
  "think": "openai-codex,gpt-5.2"
}
```

route:: openai-codex, gpt-5.2

Optimal for:
- Architecture design and system planning
- Complex debugging and root cause analysis
- Multi-step problem decomposition
- Strategic decision-making
- Plan Mode interactions in Claude Code

synonyms:: think, plan, reason, analyze, break down, step by step, think through, reason through, consider carefully, work through, design thinking, systematic thinking, logical reasoning, critical thinking, problem solving, strategic planning, chain-of-thought, deep reasoning, thinking routing, reasoning routing, reasoning mode, plan mode, think model
