# Droid Configuration - Lessons Learned

## Overview
Configuration of Factory Droid to work with Claude Max, Codex, and Antigravity models via CLIProxyAPI.

## Key Lessons

### 1. Port Configuration
- **CLIProxyAPI runs on port 8317** (not 8318)
- Initially configured port 8317 correctly, then accidentally changed to 8318
- Always verify which port the proxy server is actually listening on using `lsof -i :<port>`

### 2. Model Discovery
- CLIProxyAPI provides a `/v1/models` endpoint to list all available models
- Use `curl http://127.0.0.1:8317/v1/models | jq` to see all available models
- Not all models listed in the API work with authentication

### 3. Authentication Methods
- **Claude**: Uses `/v1/messages` endpoint with `x-api-key` header and dummy key
- **GPT/Codex/Gemini**: Uses `/v1/chat/completions` endpoint with `Authorization: Bearer` header and dummy key
- CLIProxyAPI handles OAuth token management automatically when configured with `--claude-login` or similar auth commands

### 4. Model Testing Strategy
Test models individually before adding to config:
```bash
# Claude models
curl -X POST http://127.0.0.1:8317/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: dummy" \
  -H "anthropic-version: 2023-06-01" \
  -d '{"model": "claude-opus-4-5-20251101", "max_tokens": 10, "messages": [{"role": "user", "content": "hi"}]}'

# GPT/Codex/Gemini models
curl -X POST http://127.0.0.1:8317/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer dummy" \
  -d '{"model": "gpt-5.2", "messages": [{"role": "user", "content": "hi"}], "max_tokens": 10}'
```

### 5. Working Models (as of 2026-01-01)

#### Claude Models (Anthropic)
- `claude-opus-4-5-20251101` ✓
- `claude-sonnet-4-5-20250929` ✓
- `claude-haiku-4-5-20251001` ✓
- `claude-3-7-sonnet-20250219` ✓
- `claude-opus-4-1-20250805` ✓

#### GPT/Codex Models (OpenAI)
- `gpt-5.2` ✓
- `gpt-5.2-codex` ✓
- `gpt-5.1-codex-max` ✓
- `gpt-5-codex-mini` ✓
- `gpt-5-mini` ✓
- `gpt-4.1` ✓

#### Gemini/Antigravity Models
- `gemini-2.5-flash` ✓
- `gemini-claude-opus-4-5-thinking` ✓

#### Other Models
- `grok-code-fast-1` ✓

### 6. Non-Working Models
- `antigravity-1` ✗ Unknown provider (token exists but not mapped)
- `antigravity-1-preview` ✗ Unknown provider
- `gemini-3-pro` ✗ Model not supported
- `raptor-mini` ✗ Model not supported
- All `-high`, `-medium`, `-low` variants ✗ Unknown provider

### 7. Droid Configuration File
Location: `~/.factory/config.json`

Example configuration:
```json
{
    "custom_models": [
        {
            "model": "claude-opus-4-5-20251101",
            "base_url": "http://127.0.0.1:8317",
            "api_key": "sk-dummy",
            "provider": "anthropic"
        },
        {
            "model": "gpt-5.2",
            "base_url": "http://127.0.0.1:8317/v1",
            "api_key": "sk-dummy",
            "provider": "openai"
        }
    ]
}
```

Important notes:
- Claude models use provider `"anthropic"` without `/v1` in base_url
- All other models use provider `"openai"` with `/v1` in base_url
- `api_key` is required by Factory but ignored by CLIProxyAPI (use `"sk-dummy"`)
- Validate JSON with `python3 -m json.tool ~/.factory/config.json`

### 8. Common Issues

#### Issue: Models not showing in Droid
**Solution**: Restart Droid completely (not just tmux session). Droid caches config on startup.

#### Issue: "402 status code (no body)" error
**Solution**: This is a Factory billing error, not a configuration issue. Check your Factory.ai subscription.

#### Issue: "Unknown provider for model" error
**Solution**: The model name is not mapped to a provider in CLIProxyAPI authentication. Check available models via `/v1/models` endpoint.

#### Issue: "OAuth token has expired" error
**Solution**: Re-authenticate with CLIProxyAPI:
```bash
/Applications/VibeProxy.app/Contents/Resources/cli-proxy-api-plus --claude-login
```

### 9. Testing Workflow
1. List available models from API
2. Test each model individually via curl
3. Add only working models to config.json
4. Validate JSON syntax
5. Restart Droid completely
6. Use `/model` command to select model in Droid

### 10. CLIProxyAPI Management
- Process name: `cli-proxy-api-plus`
- Running from: `/Applications/VibeProxy.app/Contents/Resources/`
- Auth tokens stored in: `~/.cli-proxy-api/`
- Config file: `/Applications/VibeProxy.app/Contents/Resources/config.yaml`

View running processes:
```bash
ps aux | grep cli-proxy
lsof -i :8317
```

## Verification Commands
```bash
# Check proxy is running
lsof -i :8317

# List all available models
curl -s http://127.0.0.1:8317/v1/models | python3 -m json.tool

# Validate config JSON
python3 -m json.tool ~/.factory/config.json > /dev/null

# Test a model
curl -s -X POST http://127.0.0.1:8317/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer dummy" \
  -d '{"model": "gpt-5.2", "messages": [{"role": "user", "content": "test"}], "max_tokens": 5}'
```

## References
- CLIProxyAPI GitHub: https://github.com/luispater/CLIProxyAPI
- Factory Droid: https://factory.ai/
- VibeProxy (CLIProxyAPI wrapper): /Applications/VibeProxy.app/
