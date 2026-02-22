from pathlib import Path


def main() -> None:
    p = Path("/etc/terraphim-llm-proxy/config.toml")
    text = p.read_text()
    text = text.replace("long_context_threshold = 60000", "long_context_threshold = 12000")
    p.write_text(text)
    print("set long_context_threshold=12000")


if __name__ == "__main__":
    main()
