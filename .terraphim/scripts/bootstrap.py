#!/usr/bin/env python3
"""Bootstrap .terraphim for this clone.

- Regenerates thesaurus-<shortname>.json from kg-<shortname>/*.md
- Normalizes role KG paths to repo-relative `.terraphim/kg-<short>` form
- Keeps haystack locations repo-relative (`.`, `crates`, …)
- NEVER writes machine-absolute paths into the tracked config.json
  (fleet standard §8 portability)

Idempotent. Run after clone and after editing concept files.
"""
from __future__ import annotations

import glob
import json
import os
import subprocess
import sys

REPO = subprocess.check_output(
    ["git", "rev-parse", "--show-toplevel"], text=True
).strip()
TD = os.path.join(REPO, ".terraphim")
CFG = os.path.join(TD, "config.json")


def compile_thesaurus(kg_dir: str, out_path: str, role_name: str) -> None:
    if not os.path.isdir(kg_dir):
        print(f"  skip thesaurus (missing {os.path.relpath(kg_dir, REPO)})")
        return
    data: dict = {}
    cid = 100
    for f in sorted(glob.glob(os.path.join(kg_dir, "*.md"))):
        nterm = os.path.splitext(os.path.basename(f))[0]
        txt = open(f, encoding="utf-8").read()
        syn = next(
            (
                l.split("::", 1)[1]
                for l in txt.splitlines()
                if l.strip().lower().startswith("synonyms::")
            ),
            "",
        )
        for t in dict.fromkeys(
            [nterm] + [s.strip().lower() for s in syn.split(",") if s.strip()]
        ):
            data.setdefault(t.lower(), {"id": cid, "nterm": nterm})
        cid += 1
    json.dump({"name": role_name, "data": data}, open(out_path, "w"), indent=2)
    print(f"  wrote {os.path.relpath(out_path, REPO)} ({len(data)} terms)")


def to_repo_relative(loc: str) -> str:
    """Map absolute/{REPO}/foreign paths back to repo-relative form."""
    if loc in ("{REPO}", ".", ""):
        return "."
    if loc.startswith("{REPO}/"):
        return loc[len("{REPO}/") :] or "."
    if loc == REPO or loc.rstrip("/") == REPO:
        return "."
    if loc.startswith(REPO + os.sep):
        return loc[len(REPO) + 1 :] or "."
    # foreign absolute: try known suffixes, else basename==repo basename → root
    if loc.startswith("/"):
        for suffix in (
            "/rust/crates",
            "/crates",
            "/memory",
            "/docs",
            "/.docs",
            "/.terraphim",
        ):
            if loc.endswith(suffix):
                return suffix.lstrip("/")
        if os.path.basename(loc.rstrip("/")) == os.path.basename(REPO):
            return "."
    return loc  # already relative or unknown — leave


cfg = json.load(open(CFG))
changed = False
for role in cfg.get("roles", {}).values():
    short = role.get("shortname") or role.get("name", "role").lower().replace(" ", "-")
    kg = role.get("kg", {}).get("knowledge_graph_local")
    if kg and "path" in kg:
        rel = f".terraphim/kg-{short}"
        if kg["path"] != rel:
            kg["path"] = rel
            changed = True
    for h in role.get("haystacks", []):
        new = to_repo_relative(h.get("location", "."))
        if h.get("location") != new:
            h["location"] = new
            changed = True
    compile_thesaurus(
        os.path.join(TD, f"kg-{short}"),
        os.path.join(TD, f"thesaurus-{short}.json"),
        role.get("name", short),
    )

if changed:
    json.dump(cfg, open(CFG, "w"), indent=2)
    print("  config.json normalized to repo-relative paths (portable)")
else:
    print("  config.json already portable (no path rewrite)")
print("done.")
