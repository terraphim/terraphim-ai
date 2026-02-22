from pathlib import Path


def main() -> None:
    p = Path('/etc/terraphim-llm-proxy/config.toml')
    text = p.read_text()
    text = text.replace(
        'long_context = "openrouter,google/gemini-2.5-flash-preview-09-2025"',
        'long_context = "openai-codex,gpt-5.2"',
    )
    text = text.replace(
        'web_search = "openrouter,perplexity/llama-3.1-sonar-large-128k-online"',
        'web_search = "openai-codex,gpt-5.2"',
    )
    text = text.replace(
        'image = "openrouter,anthropic/claude-sonnet-4.5"',
        'image = "zai,claude-sonnet-4.5"',
    )
    p.write_text(text)
    print('updated routes to avoid openrouter')


if __name__ == '__main__':
    main()
