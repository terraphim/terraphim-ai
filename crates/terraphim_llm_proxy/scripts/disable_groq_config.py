from pathlib import Path


def main() -> None:
    p = Path("/etc/terraphim-llm-proxy/config.toml")
    text = p.read_text()

    text = text.replace(
        'default = "groq,llama-3.3-70b-versatile"',
        'default = "zai,claude-sonnet-4.5"',
    )
    text = text.replace(
        'long_context = "groq,llama-3.3-70b-versatile"',
        'long_context = "zai,claude-sonnet-4.5"',
    )
    text = text.replace(
        'to = "groq,llama-3.3-70b-versatile"',
        'to = "zai,claude-sonnet-4.5"',
    )
    text = text.replace(
        'to = "groq,llama-3.1-8b-instant"',
        'to = "zai,claude-sonnet-4.5"',
    )
    text = text.replace('name = "groq"', 'name = "groq_disabled"')

    p.write_text(text)
    print("config updated")


if __name__ == "__main__":
    main()
