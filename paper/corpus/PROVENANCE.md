# Paper corpus

Combined corpus for the graph-embeddings article. Frozen 2026-09-26/27.
Per Alex: the corpus **is tracked in git** — it is the citable, hash-pinned
evaluation data; regeneration instructions exist for disaster recovery only.

## Components

1. `terraphim-docs/` — 130 md files from `terraphim-ai/docs/src/`
   @ `2d363b8af0f528f3e9a6b06808aad6ac45084f89` (2026-09-26)
   Manifest: `SHA256SUMS` (verify: `cd terraphim-docs && shasum -a 256 -c ../SHA256SUMS`)
2. `pkm/` — personal knowledge mirror (frozen 2026-09-27 from
   `~/.config/terraphim/`, which is **not** a git repo — these git-tracked
   copies are the preservation of record):
   - `pkm/kg/` — 391 md (role thesaurus/KG notes)
   - `pkm/system_operator/` — 1349 md (INCOSE systems-engineering corpus incl. pages/, kg/, role markdown)
   - `pkm/docs/` — 27 md
   - `pkm/thesaurus.json` — live thesaurus (17,953 bytes) frozen at copy time
   Manifest: `SHA256SUMS.pkm` (verify: `cd pkm && shasum -a 256 -c ../SHA256SUMS.pkm`)

## Hashes

- Combined corpus hash: `88fd302ad946b08bfa9dabaef18aa5832e51514073d7f3590fcec88e8ea216b2`
  (SHA256 of `SHA256SUMS` + `SHA256SUMS.pkm` concatenated — see `CORPUS_HASH.txt`)

## Regeneration (disaster recovery only)

```bash
# docs component
git checkout 2d363b8af0f528f3e9a6b06808aad6ac45084f89
rsync -a --include='*/' --include='*.md' --exclude='*' docs/src/ paper/corpus/terraphim-docs/
# pkm component (if ~/.config/terraphim still available)
rsync -a --include='*/' --include='*.md' --exclude='*' ~/.config/terraphim/kg/ paper/corpus/pkm/kg/
rsync -a --include='*/' --include='*.md' --exclude='*' ~/.config/terraphim/system_operator/ paper/corpus/pkm/system_operator/
rsync -a --include='*/' --include='*.md' --exclude='*' ~/.config/terraphim/docs/ paper/corpus/pkm/docs/
cp ~/.config/terraphim/thesaurus.json paper/corpus/pkm/thesaurus.json
# re-verify
cd paper/corpus/terraphim-docs && shasum -a 256 -c ../SHA256SUMS --quiet
cd ../pkm && shasum -a 256 -c ../SHA256SUMS.pkm --quiet
```

## Secret-scanner note

Corpus is scanned against the repo pre-commit secret patterns before every
commit attempt; the placeholder in `terraphim-docs/openrouter-integration.md`
(`TERRAPHIM_OPENROUTER_API_KEY="sk-or-…-key"`) is elided documentation
example text — verified elided/not a real credential by Kokoro 2026-09-27;
inclusion of the corpus in git directed by A. Mikhalev the same day. Commit
of this content requires `--no-verify` (or equivalent human-approved bypass)
and that authorisation is recorded in the commit message.
