from pathlib import Path


def main() -> None:
    p = Path("/etc/terraphim-llm-proxy/config.toml")
    text = p.read_text()

    replacements = {
        'default = "zai,claude-sonnet-4.5"': 'default = "openrouter,anthropic/claude-sonnet-4.5"',
        'think = "openai-codex,gpt-5.2"': 'think = "zai,claude-sonnet-4.5"',
        'long_context = "zai,claude-sonnet-4.5"': 'long_context = "openrouter,google/gemini-2.5-flash-preview-09-2025"',
        'web_search = "zai,claude-sonnet-4.5"': 'web_search = "openrouter,perplexity/llama-3.1-sonar-large-128k-online"',
        'image = "openrouter,anthropic/claude-3.5-sonnet"': 'image = "openrouter,anthropic/claude-sonnet-4.5"',
    }

    for old, new in replacements.items():
        text = text.replace(old, new)

    text = text.replace(
        '[[router.model_mappings]]\nfrom = "fastest"\nto = "zai,claude-sonnet-4.5"',
        '[[router.model_mappings]]\nfrom = "fastest"\nto = "openrouter,anthropic/claude-sonnet-4.5"',
    )
    text = text.replace(
        'to = "openai-codex,gpt-5.2"',
        'to = "zai,claude-sonnet-4.5"',
    )
    text = text.replace(
        '[[router.model_mappings]]\nfrom = "cheapest"\nto = "zai,claude-sonnet-4.5"',
        '[[router.model_mappings]]\nfrom = "cheapest"\nto = "cerebras,llama3.1-8b"',
    )

    text = text.replace(
        'models = [\n    "anthropic/claude-3.5-sonnet",\n]',
        'models = [\n    "anthropic/claude-sonnet-4.5",\n    "anthropic/claude-3.5-sonnet",\n    "google/gemini-2.5-flash-preview-09-2025",\n    "perplexity/llama-3.1-sonar-large-128k-online"\n]',
    )

    p.write_text(text)
    print("remote config updated")


if __name__ == "__main__":
    main()
