from pathlib import Path


def main() -> None:
    p = Path("/etc/terraphim-llm-proxy/config.toml")
    lines = p.read_text().splitlines()

    out = []
    i = 0
    while i < len(lines):
        if (
            lines[i].strip() == "[[router.model_mappings]]"
            and i + 2 < len(lines)
            and lines[i + 1].strip() == 'from = "fastest"'
        ):
            i += 3
            continue
        out.append(lines[i])
        i += 1

    p.write_text("\n".join(out) + "\n")
    print("removed fastest mapping block")


if __name__ == "__main__":
    main()
