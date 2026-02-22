# Long Context Routing

Long context routing handles requests with large token counts that exceed the standard context window. This routing scenario automatically switches to models with extended context capabilities when token thresholds are exceeded.

In claude-code-router, long context routing is triggered when:
- Token count exceeds `longContextThreshold` (default: 60,000 tokens)
- Last message usage had high input tokens (>60K) and current request >20K tokens
- Large file operations or repository-wide analysis is performed

Configuration example:
```json
"Router": {
  "longContext": "openrouter,google/gemini-2.5-pro-preview",
  "longContextThreshold": 60000
}
```

route:: openrouter, google/gemini-2.0-flash-exp

Token counting includes:
- All message content (user and assistant)
- System prompts and context
- Tool definitions and schemas
- Previous conversation history

Use cases:
- Whole codebase analysis
- Large documentation processing
- Multi-file refactoring
- Repository-wide search and replace
- Extended conversation contexts

synonyms:: long context model, extended context, large context window, high token routing, context window routing, extended window
