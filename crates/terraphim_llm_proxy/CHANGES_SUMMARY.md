## Summary of Changes

### Current Status

✅ **Non-streaming requests** (`stream: false`) to `/v1/chat/completions` now return proper OpenAI format:
```json
{
  "id": "...",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "glm-4.7",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "..."
    },
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 12,
    "completion_tokens": 30,
    "total_tokens": 42
  }
}
```

⚠️ **Streaming requests** (`stream: true`) still use internal format and need separate conversion.

### Key Changes in `src/server.rs`:

1. Added `ChatResponse::to_openai_format()` method to convert internal response to OpenAI format
2. Added `handle_chat_completions_non_streaming()` function for non-streaming requests
3. Modified `handle_chat_completions()` to route streaming vs non-streaming appropriately

### Testing

Non-streaming test:
```bash
curl -X POST http://127.0.0.1:3456/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer YOUR_API_KEY' \
  -d '{"model": "thinking", "messages": [{"role": "user", "content": "Hello"}], "stream": false}'
```

### Next Steps

- [ ] Implement OpenAI format conversion for streaming (SSE) responses
- [ ] Update clawdbot to use `stream: false` if possible
- [ ] Test with various providers (Groq, ZAI, OpenRouter)

### Issue

The proxy returns OpenAI format for non-streaming, but clawdbot uses streaming by default which still returns internal format, causing "Cannot read properties of undefined" errors.
