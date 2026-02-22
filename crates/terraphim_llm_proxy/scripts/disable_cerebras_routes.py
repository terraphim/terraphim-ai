from pathlib import Path


def main() -> None:
    p = Path('/etc/terraphim-llm-proxy/config.toml')
    text = p.read_text()
    text = text.replace(
        'background = "cerebras,llama3.1-8b"',
        'background = "openai-codex,gpt-5.2-codex"',
    )
    text = text.replace(
        'from = "cheapest"\nto = "cerebras,llama3.1-8b"',
        'from = "cheapest"\nto = "openai-codex,gpt-5.2-codex"',
    )
    p.write_text(text)
    print('updated routes to avoid cerebras')


if __name__ == '__main__':
    main()
