# clawpatch

Open-source automated code review CLI from openclaw (https://github.com/openclaw/clawpatch) that maps a repo into semantic feature slices, reviews each slice with a pluggable coding-agent provider, persists findings in `.clawpatch/`, and runs an explicit per-finding fix loop. Providers supported include `codex`, `claude`, `cursor`, `grok`, `opencode`, `pi`, and any ACP-compatible agent via `acpx`. The `fix` command never auto-commits or opens PRs by itself; `open-pr` is a separate, explicit step. Workflow: `init → map → review → report → next → show → triage → fix → open-pr → revalidate`. Distributed via pnpm; state is project-local under `.clawpatch/`.

Relevance to ADF and terraphim-ai: clawpatch is a structural review/patching orchestrator that complements ADF (which dispatches long-running coding agents on bigbox). It would slot in as a deterministic pre/post-merge review layer on top of ADF outputs, using subscription-only providers (claude-code, opencode, pi) and feeding findings back as Gitea issues. Compared to the existing roborev/structural-pr-review skill it adds: semantic feature slicing, persistent finding state across runs, and a constrained fix loop with revalidation.

synonyms:: clawpatch CLI, openclaw clawpatch, claw-patch, automated code review CLI
