# Intelligent Multi-Model Proxy

Most multi-provider AI stacks are fragile because we built them like a spreadsheet, not a system.

One SDK for OpenAI. Another for Anthropic. A custom adapter for Z.ai. A sidecar for routing. A separate fallback script that fails silently at 2 a.m. Then we call this platform engineering. It is not. It is dependency roulette with good branding.

The failure mode is predictable: one provider hiccups, your product stalls, and your team starts debugging glue code instead of shipping features. The stack is flexible right up until it is not.

We took a different path in `terraphim-llm-proxy`: one proxy, one endpoint, explicit route chains, deterministic fallback.

## 1) The thesis: one endpoint, explicit route chains, deterministic fallback

If your app talks to more than one model provider, your app should still talk to exactly one API endpoint.

That endpoint should be your control plane, not someone else's SDK abstraction.

Terraphim LLM Proxy gives us that control plane. OpenClaw points to a single endpoint and uses OpenAI completions mode. Routing decisions happen in the proxy, where they belong. Fallback rules are expressed as route chains, not buried in app code. We use:

`provider,model|provider,model`

That tiny delimiter does the work most teams overengineer into orchestration pipelines. First target. If it fails, next target. Deterministic. Observable. Boring in the best way.

And yes, boring wins in production.

## 2) Practical config snippet

OpenClaw is configured once. It does not need to know five providers or ten auth strategies. It needs one endpoint.

```yaml
# OpenClaw (openai-completions mode)
api:
  base_url: "http://127.0.0.1:3456/v1"
  mode: "openai-completions"
```

Route chains live in proxy config, where provider differences can be handled centrally.

```txt
default = openai-codex,gpt-5.2-codex|zai,glm-5
think = openai-codex,gpt-5.2|minimax,MiniMax-M2.5|zai,glm-5
```

Taxonomy-aware routing is explicit too.

```txt
minimax_keyword_routing -> minimax,MiniMax-M2.5
```

No provider-specific branching in product code.

## 3) Validation evidence: direct checks, fallback, intelligent routing

We validated this as a system, not a demo.

Direct checks through the proxy succeeded for:

- `openai-codex,gpt-5.2`
- `zai,glm-5`
- `minimax,MiniMax-M2.5`

Runtime fallback was proven by blocking Codex upstream and sending the same request path. Primary leg failed, fallback switched to `zai,glm-5`, request succeeded. No client change. No retry logic in app code.

Intelligent routing also worked. Taxonomy keyword route `minimax_keyword_routing` steered request selection to MiniMax when intent matched.

One important detail: MiniMax uses Anthropic compatibility and needed provider-specific endpoint handling inside the proxy. Compatibility labels are marketing. Endpoint behavior is engineering.

## 4) What to stop doing

- Stop embedding fallback logic across app services.
- Stop treating SDK wrappers as architecture.
- Stop shipping model selection through undocumented env folklore.
- Stop assuming compatible APIs are identical protocols.
- Stop optimizing for provider-switching demos instead of runtime behavior.

## 5) What to do instead

- Put one proxy endpoint between apps and all providers.
- Define route chains explicitly and version them.
- Implement fallback as proxy policy, not business-logic retries.
- Use taxonomy-based routing where intent changes model choice.
- Isolate provider-specific endpoint quirks in one place.

## 6) Closing

Reliable multi-model systems are not built by adding moving parts. They are built by removing ambiguity.

One endpoint. Explicit chains. Deterministic fallback. Provider quirks isolated behind the proxy.

That is what `terraphim-llm-proxy` gives you.
