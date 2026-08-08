#!/usr/bin/env python3
"""Bootstrap .terraphim for this clone: materialize {REPO} placeholders,
rebase foreign absolute paths, keep relative paths, regenerate thesauri
from kg-<shortname>/ markdown (and skip roles whose kg dir is missing).

Idempotent. Run from anywhere inside the repo after cloning, and after
adding/changing concept files.
"""
import os, json, glob, subprocess, sys

REPO = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
TD = os.path.join(REPO, ".terraphim")
CFG = os.path.join(TD, "config.json")

def compile_thesaurus(kg_dir, out_path, role_name):
    if not os.path.isdir(kg_dir):
        print(f"  skip thesaurus (missing {os.path.relpath(kg_dir, REPO)})")
        return
    data, cid = {}, 100
    for f in sorted(glob.glob(os.path.join(kg_dir, "*.md"))):
        nterm = os.path.splitext(os.path.basename(f))[0]
        txt = open(f, encoding="utf-8").read()
        syn = next((l.split("::", 1)[1] for l in txt.splitlines()
                    if l.strip().lower().startswith("synonyms::")), "")
        for t in dict.fromkeys([nterm] + [s.strip().lower() for s in syn.split(",") if s.strip()]):
            data.setdefault(t.lower(), {"id": cid, "nterm": nterm})
        cid += 1
    json.dump({"name": role_name, "data": data}, open(out_path, "w"), indent=2)
    print(f"  wrote {os.path.relpath(out_path, REPO)} ({len(data)} terms)")

def materialize_loc(loc: str) -> str:
    if loc == "{REPO}" or loc.startswith("{REPO}/"):
        return REPO + loc[len("{REPO}"):]
    if loc.startswith("/") and REPO not in loc:
        for suffix in ("/rust/crates", "/crates", "/memory", "/docs", "/.docs", "/.terraphim"):
            if loc.endswith(suffix):
                return REPO + suffix
        return loc  # unknown absolute — leave
    # relative or already this-repo absolute
    return loc

cfg = json.load(open(CFG))
for role in cfg.get("roles", {}).values():
    short = role.get("shortname") or role.get("name", "").lower().replace(" ", "-")
    kg = role.get("kg", {}).get("knowledge_graph_local")
    if kg and "path" in kg:
        p = kg["path"]
        if p == "{REPO}" or p.startswith("{REPO}/"):
            kg["path"] = REPO + p[len("{REPO}"):]
        elif p.startswith(".terraphim/") or p.startswith("./"):
            kg["path"] = os.path.join(REPO, p[2:] if p.startswith("./") else p)
        elif not p.startswith("/"):
            # bare relative like "kg/foo" under .terraphim or repo
            cand = os.path.join(TD, p) if not p.startswith("kg") else os.path.join(TD, p)
            # prefer existing kg-<short> fleet layout when shortname set
            fleet = os.path.join(TD, f"kg-{short}")
            if os.path.isdir(fleet):
                kg["path"] = fleet
            elif os.path.isdir(os.path.join(REPO, p)):
                kg["path"] = os.path.join(REPO, p)
            elif os.path.isdir(os.path.join(TD, p)):
                kg["path"] = os.path.join(TD, p)
            else:
                kg["path"] = os.path.join(REPO, p) if not p.startswith(".") else os.path.join(REPO, p)
        elif p.startswith("/") and REPO not in p:
            fleet = os.path.join(TD, f"kg-{short}")
            if os.path.isdir(fleet):
                kg["path"] = fleet
    for h in role.get("haystacks", []):
        h["location"] = materialize_loc(h.get("location", ""))
    # only compile fleet-style kg-<short> dirs (do not clobber legacy kg/<role> thesauri generators)
    fleet_kg = os.path.join(TD, f"kg-{short}")
    if os.path.isdir(fleet_kg):
        compile_thesaurus(fleet_kg, os.path.join(TD, f"thesaurus-{short}.json"), role.get("name", short))

json.dump(cfg, open(CFG, "w"), indent=2)
print(f"  config.json paths rebased to {REPO}")
print("done.")
