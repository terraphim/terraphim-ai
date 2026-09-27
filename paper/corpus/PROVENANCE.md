# Corpus: terraphim-docs

- Source: terraphim-ai/docs/src/ @ commit 2d363b8af0f528f3e9a6b06808aad6ac45084f89
- Frozen: 2026-09-26T11:30:51Z
- Files: 130 markdown files
- Corpus hash (SHA256 of SHA256SUMS): ec54d56b241ad7ed7d6be826e0c299455ea4a29db91fd66203cd6c18cf77ff66
- Verification: cd corpus/terraphim-docs && shasum -a 256 -c ../SHA256SUMS

## Regeneration

The corpus files are a derived artefact and are NOT tracked in git (they
contain verbatim documentation content, incl. placeholder-looking key
examples that trip secret scanners). Regenerate from the pinned commit:

    git checkout 2d363b8af0f528f3e9a6b06808aad6ac45084f89
    rsync -a --include='*/' --include='*.md' --exclude='*' \
        docs/src/ paper/corpus/terraphim-docs/
    cd paper/corpus/terraphim-docs
    find . -name '*.md' -type f -print0 | sort -z | \
        xargs -0 shasum -a 256 > ../SHA256SUMS
    shasum -a 256 -c ../SHA256SUMS --quiet   # must print nothing (OK)
