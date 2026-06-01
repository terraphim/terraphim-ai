# ADF Local Project Skills

`adf --local --agent NAME` supports project-local skills stored under:

```text
.terraphim/
  adf.toml
  skills/
    my-skill/
      SKILL.md
```

When local ADF runs an opencode or Claude agent, it exposes `.terraphim/skills/` through that tool's project skill directory instead of appending skill content to the task prompt.

## Behaviour

- Missing `.terraphim/skills/` is a no-op.
- Unsupported CLI tools keep their normal spawn behaviour.
- The original agent task is not modified.
- Global skills remain managed by the underlying CLI tool.

## Native Bridges

| CLI | Project bridge |
|-----|----------------|
| opencode | `.opencode/skill` points at `.terraphim/skills` when absent |
| Claude | `.claude/skills` points at `.terraphim/skills` when absent |

Existing native skill directories are not overwritten. If a project already has `.opencode/skill` or `.claude/skills`, keep that directory as the source of truth or manually link it to `.terraphim/skills`.
