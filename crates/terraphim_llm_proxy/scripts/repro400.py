import json
import os
import time
import urllib.error
import urllib.request


def run_case(name: str, n_chars: int, with_tool: bool, url: str, key: str) -> None:
    body = {
        "model": "fastest",
        "messages": [
            {
                "role": "user",
                "content": "Investigate and summarize: " + ("X" * n_chars),
            }
        ],
        "stream": True,
        "max_tokens": 32,
    }
    if with_tool:
        body["tools"] = [
            {
                "type": "function",
                "function": {
                    "name": "web_search",
                    "description": "Search the web",
                    "parameters": {
                        "type": "object",
                        "properties": {"query": {"type": "string"}},
                        "required": ["query"],
                    },
                },
            }
        ]
        body["tool_choice"] = "auto"

    req = urllib.request.Request(url, data=json.dumps(body).encode(), method="POST")
    req.add_header("content-type", "application/json")
    req.add_header("authorization", f"Bearer {key}")

    started = time.time()
    try:
        with urllib.request.urlopen(req, timeout=70) as resp:
            sample = resp.read(280)
            print(
                f"{name}: HTTP {resp.status}, ttfb_s={time.time()-started:.2f}, "
                f"sample={sample[:120]!r}",
                flush=True,
            )
    except urllib.error.HTTPError as err:
        detail = err.read(280)
        print(
            f"{name}: HTTP {err.code}, ttfb_s={time.time()-started:.2f}, "
            f"sample={detail[:160]!r}",
            flush=True,
        )
    except Exception as err:  # noqa: BLE001
        print(
            f"{name}: ERROR {type(err).__name__}, ttfb_s={time.time()-started:.2f}, msg={err}",
            flush=True,
        )


def main() -> None:
    url = "http://127.0.0.1:3456/v1/chat/completions"
    key = os.environ.get("PROXY_API_KEY", "")
    cases = [
        ("small_web_search", 2000, True),
        ("large_web_search", 580000, True),
        ("large_no_tools", 580000, False),
    ]
    for name, n_chars, with_tool in cases:
        run_case(name, n_chars, with_tool, url, key)


if __name__ == "__main__":
    main()
