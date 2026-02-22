# Web Search Routing

Web search routing handles requests that require real-time web search capabilities or access to current information beyond the model's training data. This routing scenario activates when web search tools are detected in the request.

In claude-code-router, web search routing is triggered when:
- The request includes tools with `type` starting with "web_search"
- Real-time information retrieval is required
- Claude Code requests current data or live information

Configuration example:
```json
"Router": {
  "webSearch": "gemini,gemini-2.5-flash"
}
```

route:: openrouter, perplexity/llama-3.1-sonar-large-128k-online

Requirements:
- Model must natively support web search functionality
- For OpenRouter, append `:online` suffix to model name (e.g., "anthropic/claude-3.5-sonnet:online")
- Provider must have web search API integration

Use cases:
- Current documentation lookup
- Real-time API reference checking
- Latest framework version information
- Current best practices research
- Live package and library information

synonyms:: web search model, online search, internet search routing, live data, current information, real-time search
