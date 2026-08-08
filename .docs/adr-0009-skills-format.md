# ADR-0009: TinyClaw Skills Format — `#[derive]` structs + SKILL.md Coexistence

**Status**: ACCEPTED (2026-08-08)
**Deciders**: Hermes Agent (Wave 6 of Hermes parity arc)
**Fleet mandate**: `terraphim-skills` catalog, prompt-template format
**Issue**: terraphim-ai#3167

## Context

Hermes Agent uses **SKILL.md** files for prompt templates — a
human-readable markdown format with YAML frontmatter describing the
skill's inputs, outputs, and steps. This is great for prompt engineers
and lets non-Rust contributors author skills.

TinyClaw currently uses **`#[derive]` structs** in Rust source for
skill definitions — type-safe, code-reviewable, but requires Rust
expertise to author.

## Decision

**TinyClaw supports BOTH formats. They serve different purposes:**

1. **Rust structs** (`crates/terraphim_tinyclaw/src/skills/`):
   - Skill implementation (the executable steps).
   - Tool registration (the function signatures).
   - Internal Rust APIs.

2. **SKILL.md files** (future: `skills/*.md`):
   - Prompt templates (the natural-language steps).
   - Documentation (for users + future skill authors).
   - Frontmatter describing skill metadata.

A future wave will add SKILL.md loading, where the markdown template
is parsed into the existing `SkillStep` enum (or a new wrapper). Until
then, structs are the source of truth.

## Rationale

1. **Existing surface area preserved**: all current tinyclaw skills
   work as Rust structs. No break-the-world.
2. **Future-proof for prompt engineers**: SKILL.md will let non-Rust
   authors contribute.
3. **Hermes parity**: Hermes does both — Rust modules + SKILL.md. TinyClaw
   follows the same pattern.
4. **Minimal scope**: SKILL.md loading can be a separate future wave.

## Rejected Alternatives

### Force-only SKILL.md (no Rust structs)

- **Pros**: Single format, easier learning curve.
- **Cons**: Loses type safety; harder to test; can't enforce input/output
  contracts at compile time.
- **Verdict**: Rejected. Rust structs are the executable backbone.

### Force-only Rust structs (no SKILL.md)

- **Pros**: Pure code, single format.
- **Cons**: Excludes prompt engineers; harder to onboard contributors.
- **Verdict**: Rejected as a permanent state, but acceptable as the
  **current state**. SKILL.md is future work.

### Single mega-format (YAML for everything)

- **Pros**: One parser, one validator.
- **Cons**: Can't represent Rust functions in YAML; would need a
  separate codegen step.
- **Verdict**: Rejected. The two-format pattern matches Hermes.

## Consequences

- **Positive**: Skills are authored in Rust today; can add SKILL.md later
  without breaking existing code.
- **Positive**: SKILL.md frontmatter is a known, documented format
  (see Hermes SKILL.md spec).
- **Negative**: Two formats to document for skill authors.
- **Negative**: Until SKILL.md loading exists, the markdown format is
  inert (just docs).

## Future Trigger

When a user asks "can I write a skill without knowing Rust?", that's
the signal to add SKILL.md loading. The path forward is:

1. Add `serde_yaml` (or similar) for frontmatter parsing.
2. Define `SkillTemplate` enum mirroring the existing `SkillStep` enum.
3. Map markdown sections → `SkillTemplate` variants.
4. Add `Loader::load_from_dir(path)` to find all SKILL.md in a tree.
5. Wire into the skills registry alongside Rust structs.

## References

- Hermes `skills/` and `SKILL.md` format
- `crates/terraphim_tinyclaw/src/skills/` — current Rust struct skills
- `terraphim/terraphim-ai/.docs/adr-0009-skills-format.md` (this file)
- Wave 6 spec: gitea/terraphim-ai#3167