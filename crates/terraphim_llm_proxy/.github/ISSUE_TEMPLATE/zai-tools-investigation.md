## Problem
ZAI GLM-4.7 and GLM-4.5 models return 422 error when requests include OpenAI tools format:
```
422 Failed to deserialize the JSON body into the target type: tools[0]: missing field 'name'
```

## Current Behavior
- ZaiClient passes tools directly to ZAI API
- GLM models don't support OpenAI tools format
- clawdbot requests fail when using 'thinking' keyword routed to ZAI

## Expected Behavior
ZAI should either:
1. Support tools properly, or
2. Gracefully ignore tools (like other providers), or
3. Proxy should strip tools for ZAI requests

## Temporary Fix
Modified ZaiClient to strip tools from requests (commented out tools passing in zai_client.rs)

## Configuration
```
[[providers]]
name = "zai"
api_base_url = "https://api.z.ai/api/coding/paas/v4"
api_key = "$ZAI_API_KEY"
models = ["glm-4.7", "glm-4.5"]
transformers = ["openai"]
```

## Testing
- [x] Test ZAI with simple chat (no tools) - WORKS
- [ ] Test ZAI with tools - FAILS with 422
- [ ] Verify fix works for clawdbot integration

## Related
- clawdbot WhatsApp integration
- Keyword routing: 'thinking' -> ZAI
