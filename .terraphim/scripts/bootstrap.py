#!/usr/bin/env python3
"""Bootstrap .terraphim for this clone.

- Regenerates thesaurus-<shortname>.json from kg-<shortname>/*.md when that
  fleet-style directory exists
- Normalizes accidental absolute/{REPO} haystack paths back to repo-relative
- Does NOT rewrite legacy kg/<role>/ paths (e.g. .terraphim/kg/rust-engineer)
- NEVER writes machine-absolute paths into tracked config.json

Idempotent. Run after clone and after editing concept files.
"""
from __future__ import annotations

import glob
import json
import os
import subprocess

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
    if loc in ("{REPO}", ".", ""):
        return "."
    if loc.startswith("{REPO}/"):
        return loc[len("{REPO}/") :] or "."
    if loc == REPO or loc.rstrip("/") == REPO:
        return "."
    if loc.startswith(REPO + os.sep):
        return loc[len(REPO) + 1 :] or "."
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
    return loc


cfg = json.load(open(CFG))
changed = False
for role in cfg.get("roles", {}).values():
    short = role.get("shortname") or role.get("name", "role").lower().replace(" ", "-")
    fleet_kg = os.path.join(TD, f"kg-{short}")
    kg = role.get("kg", {}).get("knowledge_graph_local")
    # Only force fleet path when fleet kg-<short> dir exists; leave legacy kg/<name>
    if kg and "path" in kg and os.path.isdir(fleet_kg):
        rel = f".terraphim/kg-{short}"
        if kg["path"] != rel:
            kg["path"] = rel
            changed = True
    elif kg and "path" in kg:
        # normalize absolute/{REPO} on legacy paths without changing layout
        p = kg["path"]
        if p.startswith("{REPO}/"):
            kg["path"] = p[len("{REPO}/") :]
            changed = True
        elif p.startswith(REPO + os.sep):
            kg["path"] = p[len(REPO) + 1 :]
            changed = True
    for h in role.get("haystacks", []):
        new = to_repo_relative(h.get("location", "."))
        if h.get("location") != new:
            h["location"] = new
            changed = True
    if os.path.isdir(fleet_kg):
        compile_thesaurus(
            fleet_kg,
            os.path.join(TD, f"thesaurus-{short}.json"),
            role.get("name", short),
        )

if changed:
    json.dump(cfg, open(CFG, "w"), indent=2)
    print("  config.json normalized (portable relative paths)")
else:
    print("  config.json already portable (no path rewrite)")
print("done.")
