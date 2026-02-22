# Default Routing

The default routing scenario handles general-purpose requests when no specialized routing rules apply. This is the fallback model used for standard interactions that don't match any specific routing criteria such as background tasks, thinking mode, long context, or web search.

In claude-code-router configuration, the default routing is specified as:
```json
"Router": {
  "default": "provider_name,model_name"
}
```

route:: deepseek, deepseek-chat

This routing scenario is used for:
- Regular chat interactions
- Standard code generation requests
- General-purpose AI assistance
- Any request that doesn't trigger specialized routing logic

synonyms:: default model, fallback routing, general routing, standard model, primary model, baseline routing
