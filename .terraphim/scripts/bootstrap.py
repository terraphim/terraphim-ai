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
        for term in dict.fromkeys(
            [nterm] + [s.strip().lower() for s in syn.split(",") if s.strip()]
        ):
            key = term.lower()
            if key in data and data[key]["nterm"] != nterm:
                print(
                    f"  warn: synonym {key!r} already maps to {data[key]['nterm']!r}; "
                    f"ignoring mapping to {nterm!r} ({os.path.basename(f)})"
                )
                continue
            data.setdefault(key, {"id": cid, "nterm": nterm})
        cid += 1
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump({"name": role_name, "data": data}, fh, indent=2)
        fh.write("\n")
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
        # Only rewrite absolute paths that resolve INSIDE this repo.
        # Never rewrite foreign paths that merely share a directory suffix
        # (e.g. /mnt/shared/docs must not become "docs").
        try:
            real_loc = os.path.realpath(loc)
            real_repo = os.path.realpath(REPO)
        except OSError:
            return loc
        if real_loc == real_repo:
            return "."
        prefix = real_repo + os.sep
        if real_loc.startswith(prefix):
            return real_loc[len(prefix):] or "."
        # foreign absolute path — leave untouched
        return loc
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
        # legacy kg/<role>/ — heal via same relative normalizer (no layout change)
        new_p = to_repo_relative(kg["path"])
        if new_p == ".":
            new_p = kg["path"]  # don't collapse a kg path to repo root
        if kg["path"] != new_p:
            kg["path"] = new_p
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
    with open(CFG, "w", encoding="utf-8") as fh:
        json.dump(cfg, fh, indent=2)
        fh.write("\n")
    print("  config.json normalized (portable relative paths)")
else:
    print("  config.json already portable (no path rewrite)")
print("done.")
