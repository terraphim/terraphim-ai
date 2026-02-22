import json
from pathlib import Path


def update_file(path: Path) -> None:
    if not path.exists():
        return

    backup = Path(f"{path}.bak-20260213-codex-routing")
    backup.write_text(path.read_text())

    data = json.loads(path.read_text())
    defaults = data.setdefault("agents", {}).setdefault("defaults", {})
    model_defaults = defaults.setdefault("model", {})
    model_defaults["primary"] = "terraphim/thinking"
    model_defaults["fallbacks"] = ["terraphim/fastest", "terraphim/cheapest"]

    data.setdefault("env", {})["TERRAPHIM_API_KEY"] = "$PROXY_API_KEY"

    path.write_text(json.dumps(data, indent=2) + "\n")
    print(f"updated {path}")


def main() -> None:
    update_file(Path("/home/alex/.openclaw/openclaw.json"))
    update_file(Path("/home/alex/.openclaw/clawdbot.json"))


if __name__ == "__main__":
    main()
