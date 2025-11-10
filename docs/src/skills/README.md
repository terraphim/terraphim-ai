# Claude Skills for Terraphim

This folder contains Claude Skills–style specifications that LLM agents can execute autonomously. Skills define inputs, tools, repeatable steps, and success criteria. Use these to let LLMs run an agentic loop locally against linters and other validators.

- Authoring workflow:
  - Start from `template.skill.yaml`
  - Fill in `name`, `summary`, `inputs`, `tools`, and `steps`
  - Prefer idempotent operations and strict success criteria
  - Include an iteration cap and clear error handling

- Execution mindset for LLMs:
  - Read the skill YAML
  - Render commands by substituting input values
  - Execute steps in order; parse JSON output when specified
  - Propose minimal file edits, then re-run the validator until clean or the iteration limit is reached

See `kg-schema-lint.skill.yaml` for a complete example that integrates with the new markdown KG linter.
