# CLI Login Setup Guide

Configure Claude Code and Codex CLI to use Terraphim LLM Proxy for intelligent routing.

---

## Prerequisites

1. **Terraphim LLM Proxy** running (default: `http://127.0.0.1:3456`)
2. **Provider API keys** configured in proxy (Groq, Cerebras, DeepSeek, etc.)
3. **CLI tools installed**:
   - Claude Code: `claude --version`
   - Codex CLI: `codex --version`

---

## Claude Code Setup

### Step 1: Verify Installation

```bash
claude --version
```

**Expected output:**
```
2.1.29 (Claude Code)
```

### Step 2: Create Proxy Settings File

Create `~/.claude/settings_proxy.json`:

```bash
cat > ~/.claude/settings_proxy.json << 'EOF'
{
  "env": {
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:3456",
    "ANTHROPIC_API_KEY": "sk_proxy_key_minimum_32_characters",
    "API_TIMEOUT_MS": "300000",
    "ANTHROPIC_MODEL": "auto",
    "ANTHROPIC_SMALL_FAST_MODEL": "auto"
  }
}
EOF
```

**Verify file created:**
```bash
cat ~/.claude/settings_proxy.json | jq '.env.ANTHROPIC_BASE_URL'
```

**Expected output:**
```
"http://127.0.0.1:3456"
```

### Step 3: Launch Claude Code with Proxy

```bash
claude --settings ~/.claude/settings_proxy.json
```

**Verify connection:**
```bash
# In Claude Code, type:
/doctor
```

Or test directly:
```bash
claude --settings ~/.claude/settings_proxy.json -p "Say hello"
```

### Step 4: (Optional) Set as Default

To always use proxy, add to `~/.claude/settings.json`:

```bash
# Backup existing settings
cp ~/.claude/settings.json ~/.claude/settings.json.backup

# Merge proxy settings
cat ~/.claude/settings.json | jq '. * {"env": {"ANTHROPIC_BASE_URL": "http://127.0.0.1:3456", "ANTHROPIC_API_KEY": "sk_proxy_key_minimum_32_characters"}}' > ~/.claude/settings.json.tmp && mv ~/.claude/settings.json.tmp ~/.claude/settings.json
```

**Verify merge:**
```bash
cat ~/.claude/settings.json | jq '.env'
```

---

## Codex CLI Setup

### Step 1: Verify Installation

```bash
codex --version
```

**Expected output:**
```
codex-cli 0.93.0
```

### Step 2: Check Current Login Status

```bash
codex login status
```

**Possible outputs:**
- `Logged in using ChatGPT` (OAuth via ChatGPT Plus)
- `Logged in using API key` (direct API key)
- `Not logged in`

### Step 3: Configure API Key for Proxy

**Option A: Use Environment Variable**

```bash
export OPENAI_API_KEY="sk_proxy_key_minimum_32_characters"
export OPENAI_BASE_URL="http://127.0.0.1:3456/v1"
```

**Verify:**
```bash
echo $OPENAI_BASE_URL
```

**Expected output:**
```
http://127.0.0.1:3456/v1
```

**Option B: Login with API Key**

```bash
echo "sk_proxy_key_minimum_32_characters" | codex login --with-api-key
```

**Verify:**
```bash
codex login status
```

**Expected output:**
```
Logged in using API key
```

### Step 4: Configure Codex for Proxy

Edit `~/.codex/config.toml`:

```bash
cat >> ~/.codex/config.toml << 'EOF'

# Terraphim LLM Proxy Configuration
[api]
base_url = "http://127.0.0.1:3456/v1"
EOF
```

**Verify config:**
```bash
grep -A2 "\[api\]" ~/.codex/config.toml
```

### Step 5: Test Codex with Proxy

```bash
codex exec "What is 2+2?"
```

---

## Quick Test Commands

### Test Claude Code

```bash
# Single request test
claude --settings ~/.claude/settings_proxy.json -p "What is 2+2?" 2>&1 | head -5
```

### Test Codex CLI

```bash
# Single request test
OPENAI_BASE_URL="http://127.0.0.1:3456/v1" codex exec "What is 2+2?" 2>&1 | head -5
```

### Test Proxy Directly

```bash
# Anthropic API format (Claude Code)
curl -s http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk_proxy_key_minimum_32_characters" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "auto",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "What is 2+2?"}]
  }' | jq '.content[0].text'
```

```bash
# OpenAI API format (Codex CLI)
curl -s http://127.0.0.1:3456/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer sk_proxy_key_minimum_32_characters" \
  -d '{
    "model": "auto",
    "messages": [{"role": "user", "content": "What is 2+2?"}]
  }' | jq '.choices[0].message.content'
```

---

## Environment Variables Reference

### Claude Code

| Variable | Description | Example |
|----------|-------------|---------|
| `ANTHROPIC_BASE_URL` | Proxy URL | `http://127.0.0.1:3456` |
| `ANTHROPIC_API_KEY` | Proxy API key | `sk_proxy_key...` |
| `ANTHROPIC_MODEL` | Default model | `auto` or `claude-sonnet-4-5` |
| `API_TIMEOUT_MS` | Request timeout | `300000` (5 min) |

### Codex CLI

| Variable | Description | Example |
|----------|-------------|---------|
| `OPENAI_BASE_URL` | Proxy URL | `http://127.0.0.1:3456/v1` |
| `OPENAI_API_KEY` | Proxy API key | `sk_proxy_key...` |

---

## Troubleshooting

### Claude Code: Connection Refused

```bash
# Check proxy is running
curl -s http://127.0.0.1:3456/health | jq '.'
```

**Fix:** Start the proxy:
```bash
./target/release/terraphim-llm-proxy -c config.toml
```

### Claude Code: 401 Unauthorized

```bash
# Verify API key matches proxy config
grep "api_key" config.toml
```

**Fix:** Ensure `ANTHROPIC_API_KEY` matches `api_key` in proxy config.

### Codex CLI: Invalid API Key

```bash
# Check environment variable
echo $OPENAI_API_KEY | head -c 20
```

**Fix:** Set correct API key:
```bash
export OPENAI_API_KEY="sk_proxy_key_minimum_32_characters"
```

### Codex CLI: Wrong Endpoint

```bash
# Check base URL includes /v1
echo $OPENAI_BASE_URL
```

**Fix:** Codex needs `/v1` suffix:
```bash
export OPENAI_BASE_URL="http://127.0.0.1:3456/v1"
```

---

## Multiple Configurations

### Claude Code: Switch Between Profiles

```bash
# Use proxy
claude --settings ~/.claude/settings_proxy.json

# Use direct Anthropic API
claude --settings ~/.claude/settings.json

# Create alias
alias claude-proxy='claude --settings ~/.claude/settings_proxy.json'
```

### Codex CLI: Switch Between Profiles

```bash
# Use proxy
OPENAI_BASE_URL="http://127.0.0.1:3456/v1" codex

# Use direct OpenAI API
unset OPENAI_BASE_URL && codex

# Create function
codex-proxy() {
  OPENAI_BASE_URL="http://127.0.0.1:3456/v1" codex "$@"
}
```

---

## Verified Commands Summary

| Command | Purpose | Status |
|---------|---------|--------|
| `claude --version` | Check Claude Code version | Verified |
| `claude --settings FILE` | Use custom settings | Verified |
| `claude -p "prompt"` | Non-interactive mode | Verified |
| `codex --version` | Check Codex version | Verified |
| `codex login status` | Check login status | Verified |
| `codex login --with-api-key` | Login with API key | Verified |
| `codex exec "prompt"` | Non-interactive mode | Verified |

---

## Security Notes

1. **Never commit API keys** - Use environment variables or 1Password references
2. **Use 1Password integration** for production:
   ```bash
   export ANTHROPIC_API_KEY=$(op read "op://Vault/Item/field")
   ```
3. **Proxy API key is separate** from provider keys - proxy handles provider auth internally

---

## Next Steps

1. [Cost Savings Estimate](COST_SAVINGS_ESTIMATE.md) - See potential savings
2. [Multi-Client Integration](MULTI_CLIENT_INTEGRATION.md) - Advanced setup
3. [Sponsorship](SPONSORSHIP.md) - Get repository access
