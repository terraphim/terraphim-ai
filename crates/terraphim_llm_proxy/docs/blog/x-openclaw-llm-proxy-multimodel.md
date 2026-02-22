# OpenClaw + Terraphim LLM Proxy: Multi-Model Routing That Actually Works

If you run OpenClaw, you can point it at one local proxy endpoint and get:

- OpenAI Codex (`gpt-5.2`)
- Z.ai (`glm-5`)
- MiniMax (`MiniMax-M2.5`)
- intelligent keyword routing
- automatic fallback when a provider goes down

Here is the exact pattern.

## Why this setup

Most agent stacks fail at provider outages and model sprawl. A single proxy with explicit route chains keeps clients stable while you switch providers underneath.

## Proxy config pattern

Use route chains in `/etc/terraphim-llm-proxy/config.toml`:

```toml
[router]
default = "openai-codex,gpt-5.2-codex|zai,glm-5"
think = "openai-codex,gpt-5.2|minimax,MiniMax-M2.5|zai,glm-5"
long_context = "openai-codex,gpt-5.2|zai,glm-5"
web_search = "openai-codex,gpt-5.2|zai,glm-5"
strategy = "fill_first"

[[providers]]
name = "openai-codex"
api_base_url = "https://api.openai.com/v1"
api_key = "oauth-token-managed-internally"
models = ["gpt-5.2", "gpt-5.2-codex", "gpt-5.3", "gpt-4o"]
transformers = ["openai"]

[[providers]]
name = "zai"
api_base_url = "https://api.z.ai/api/paas/v4"
api_key = "$ZAI_API_KEY"
models = ["glm-5", "glm-4.7", "glm-4.6", "glm-4.5"]
transformers = ["openai"]

[[providers]]
name = "minimax"
api_base_url = "https://api.minimax.io/anthropic"
api_key = "$MINIMAX_API_KEY"
models = ["MiniMax-M2.5", "MiniMax-M2.1"]
transformers = ["anthropic"]
```

Keep secrets in env, never inline.

## OpenClaw config pattern

In both:

- `/home/alex/.openclaw/openclaw.json`
- `/home/alex/.openclaw/clawdbot.json`

set Terraphim provider:

- `baseUrl`: `http://127.0.0.1:3456/v1`
- `api`: `openai-completions`
- model ids include:
  - `openai-codex,gpt-5.2`
  - `zai,glm-5`
  - `minimax,MiniMax-M2.5`

## Intelligent routing example

Add taxonomy file:

`/etc/terraphim-llm-proxy/taxonomy/routing_scenarios/minimax_keyword_routing.md`

```md
route:: minimax, MiniMax-M2.5
priority:: 100
synonyms:: minimax, m2.5, minimax keyword, minimax route
```

Now a normal request containing a minimax keyword can route to MiniMax even if requested model is generic.

## Validation commands

Direct provider checks:

```bash
curl -sS -X POST http://127.0.0.1:3456/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -H 'x-api-key: <PROXY_API_KEY>' \
  -d '{"model":"openai-codex,gpt-5.2","messages":[{"role":"user","content":"Reply exactly: openai-ok"}],"stream":false}'

curl -sS -X POST http://127.0.0.1:3456/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -H 'x-api-key: <PROXY_API_KEY>' \
  -d '{"model":"zai,glm-5","messages":[{"role":"user","content":"Reply exactly: zai-ok"}],"stream":false}'

curl -sS -X POST http://127.0.0.1:3456/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -H 'x-api-key: <PROXY_API_KEY>' \
  -d '{"model":"minimax,MiniMax-M2.5","messages":[{"role":"user","content":"Reply exactly: minimax-ok"}],"stream":false}'
```

Fallback proof (simulate Codex outage):

```bash
sudo cp /etc/hosts /tmp/hosts.bak
echo '127.0.0.1 chatgpt.com' | sudo tee -a /etc/hosts >/dev/null

curl -sS -X POST http://127.0.0.1:3456/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -H 'x-api-key: <PROXY_API_KEY>' \
  -d '{"model":"gpt-5.2","messages":[{"role":"user","content":"Reply exactly: fallback-ok"}],"stream":false}'

sudo cp /tmp/hosts.bak /etc/hosts
```

Look for fallback logs:

```bash
sudo journalctl -u terraphim-llm-proxy -n 120 --no-pager | rg 'Primary target failed, attempting fallback target|next_provider='
```

## Key lesson

Reliable multi-model routing is mostly configuration discipline:

- explicit route chains
- provider-specific endpoint handling where needed
- deterministic fallback order
- logs that prove routing decisions

This keeps OpenClaw simple and makes provider outages routine instead of incidents.
