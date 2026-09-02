#!/usr/bin/env python3 -u
"""Benchmark cheap/high-throughput OpenRouter models for terraphim-grep RLM fallback."""
import argparse
import json
import os
import subprocess
import sys
import time
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

DEFAULT_CANDIDATES = [
    "deepseek/deepseek-v4-flash",
    "amazon/nova-micro-v1",
    "amazon/nova-lite-v1",
    "mistralai/mistral-nemo",
    "meta-llama/llama-3.2-1b-instruct",
    "meta-llama/llama-3.2-3b-instruct",
    "google/gemma-3-4b-it",
    "qwen/qwen3-coder:free",
]


def fetch_models():
    print("Fetching OpenRouter model metadata...", flush=True)
    req = urllib.request.Request("https://openrouter.ai/api/v1/models")
    with urllib.request.urlopen(req, timeout=30) as resp:
        return {m["id"]: m for m in json.loads(resp.read().decode())["data"]}


def run_grep(query, paths, haystack, model_id, timeout):
    env = os.environ.copy()
    env["OPENROUTER_MODEL"] = model_id
    start = time.perf_counter()
    try:
        proc = subprocess.run(
            [
                "terraphim-grep",
                query,
                "--haystack",
                haystack,
                "--paths",
                paths,
                "--answer",
                "--json",
            ],
            cwd=os.getcwd(),
            env=env,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return {"error": "timeout", "wall_ms": round((time.perf_counter() - start) * 1000, 1)}
    wall = time.perf_counter() - start
    if proc.returncode != 0:
        err = proc.stderr.strip().splitlines()[-1] if proc.stderr else "unknown error"
        return {"error": err, "wall_ms": round(wall * 1000, 1)}
    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {"error": "invalid JSON", "wall_ms": round(wall * 1000, 1)}
    answer = data.get("answer") or {}
    stats = data.get("stats", {})
    return {
        "rlm_latency_ms": stats.get("rlm_latency_ms"),
        "search_latency_ms": stats.get("search_latency_ms"),
        "wall_ms": round(wall * 1000, 1),
        "sufficiency": data.get("sufficiency"),
        "answer_text": answer.get("answer") if isinstance(answer, dict) else None,
        "confidence": answer.get("confidence") if isinstance(answer, dict) else None,
    }


def classify_answer(text):
    if text is None:
        return "null"
    text = text.strip()
    if not text:
        return "empty"
    lowered = text.lower()
    if "[insert" in lowered or "placeholder" in lowered or "synthesised answer" in lowered:
        return "placeholder"
    return "ok"


def main():
    parser = argparse.ArgumentParser(
        description="Benchmark OpenRouter models for terraphim-grep RLM fallback"
    )
    parser.add_argument("--query", default="fn main")
    parser.add_argument("--paths", default="crates/terraphim_rlm/src")
    parser.add_argument("--haystack", default="code")
    parser.add_argument("--timeout", type=int, default=45)
    parser.add_argument("--output", default="scripts/benchmark_openrouter_tradeoff")
    parser.add_argument("--models", nargs="+", default=DEFAULT_CANDIDATES)
    args = parser.parse_args()

    models = fetch_models()
    results = []
    for model_id in args.models:
        info = models.get(model_id)
        print(f"Testing {model_id}...", flush=True)
        res = run_grep(args.query, args.paths, args.haystack, model_id, args.timeout)
        pricing = info.get("pricing", {}) if info else {}
        ctx = info.get("context_length") if info else None
        results.append({
            "model": model_id,
            "prompt_price": pricing.get("prompt"),
            "completion_price": pricing.get("completion"),
            "context_length": ctx,
            **res,
        })
        status = res.get("error") or classify_answer(res.get("answer_text"))
        print(f"  -> {status} (rlm={res.get('rlm_latency_ms')} ms wall={res.get('wall_ms')} ms)", flush=True)

    out_dir = Path(args.output)
    out_dir.mkdir(parents=True, exist_ok=True)
    date_stamp = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    results_file = out_dir / f"results-{date_stamp}.json"
    results_file.write_text(json.dumps(results, indent=2))
    print(f"\nRaw results saved to: {results_file}", flush=True)

    print("\n### OpenRouter model trade-off for terraphim-grep RLM fallback\n", flush=True)
    print("| Model | Prompt $/M | Completion $/M | Context | RLM latency | Wall | Answer |")
    print("|---|---|---|---|---|---|---|")
    for r in sorted(
        results,
        key=lambda x: (
            x.get("error") is not None,
            x.get("rlm_latency_ms") if x.get("rlm_latency_ms") is not None else 1e9,
        ),
    ):
        prompt = r["prompt_price"]
        comp = r["completion_price"]
        ctx = r["context_length"]
        lat = r.get("rlm_latency_ms")
        wall = r.get("wall_ms")
        ans = r.get("error") or classify_answer(r.get("answer_text"))
        print(
            f"| `{r['model']}` | {f'${float(prompt)*1e6:.3f}' if prompt is not None else 'n/a'} | "
            f"{f'${float(comp)*1e6:.3f}' if comp is not None else 'n/a'} | "
            f"{ctx if ctx is not None else 'n/a'} | "
            f"{f'{lat:.0f} ms' if lat is not None else 'n/a'} | "
            f"{f'{wall:.0f} ms' if wall is not None else 'n/a'} | {ans} |"
        )


if __name__ == "__main__":
    main()
