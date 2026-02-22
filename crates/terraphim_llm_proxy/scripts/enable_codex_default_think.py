from pathlib import Path


def main() -> None:
    p = Path("/etc/terraphim-llm-proxy/config.toml")
    text = p.read_text()

    text = text.replace(
        'default = "openrouter,anthropic/claude-sonnet-4.5"',
        'default = "openai-codex,gpt-5.2-codex"',
    )
    text = text.replace(
        'think = "zai,claude-sonnet-4.5"',
        'think = "openai-codex,gpt-5.2"',
    )
    text = text.replace(
        'from = "thinking"\nto = "zai,claude-sonnet-4.5"',
        'from = "thinking"\nto = "openai-codex,gpt-5.2"',
    )

    p.write_text(text)
    print("enabled openai-codex for default and think scenarios")


if __name__ == "__main__":
    main()
