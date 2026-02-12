# TinyClaw Skill Examples

This directory contains example skill definitions for TinyClaw.

## What are Skills?

Skills are JSON-defined workflows that combine multiple steps (tool calls, LLM prompts, shell commands) into reusable automation scripts.

## Available Examples

### analyze-repo
Analyze a git repository structure and provide insights.

```bash
terraphim-tinyclaw skill save examples/skills/analyze-repo.json
terraphim-tinyclaw skill run analyze-repo repo_path=/path/to/repo analysis_type=structure
```

**Inputs:**
- `repo_path` (required): Path to the git repository
- `analysis_type` (optional): Type of analysis - structure, dependencies, or complexity

**Steps:**
1. Get recent git commits
2. List repository structure
3. LLM analysis with context

### research-topic
Research a topic using web search and summarize findings.

```bash
terraphim-tinyclaw skill save examples/skills/research-topic.json
terraphim-tinyclaw skill run research-topic topic="Rust programming" num_results=5
```

**Inputs:**
- `topic` (required): Topic to research
- `num_results` (optional): Number of search results to analyze (default: 5)

**Steps:**
1. Web search for topic
2. LLM analysis of results
3. Log completion timestamp

### code-review
Perform automated code review on a file or directory.

```bash
terraphim-tinyclaw skill save examples/skills/code-review.json
terraphim-tinyclaw skill run code-review target_path=./src/main.rs language=rust
```

**Inputs:**
- `target_path` (required): Path to file or directory to review
- `language` (optional): Programming language (default: auto-detect)

**Steps:**
1. Read target file(s)
2. LLM code review with quality assessment

### generate-docs
Generate documentation for a codebase.

```bash
terraphim-tinyclaw skill save examples/skills/generate-docs.json
terraphim-tinyclaw skill run generate-docs source_path=./src output_path=./docs
```

**Inputs:**
- `source_path` (required): Path to source code directory
- `output_path` (optional): Where to save documentation (default: ./docs/generated)

**Steps:**
1. List source directory structure
2. LLM generates documentation outline
3. Create output directory
4. Write README.md

### security-scan
Basic security scan of files for common vulnerabilities.

```bash
terraphim-tinyclaw skill save examples/skills/security-scan.json
terraphim-tinyclaw skill run security-scan scan_path=./src severity=medium
```

**Inputs:**
- `scan_path` (required): Path to scan for security issues
- `severity` (optional): Minimum severity to report (default: medium)

**Steps:**
1. Find source files
2. Search for potential secrets/credentials
3. LLM security analysis

## Creating Your Own Skills

### Skill Structure

```json
{
  "name": "skill-name",
  "version": "1.0.0",
  "description": "What this skill does",
  "author": "Your Name",
  "inputs": [
    {
      "name": "input_name",
      "description": "What this input is for",
      "required": true,
      "default": null
    }
  ],
  "steps": [
    {
      "type": "tool|llm|shell",
      ...
    }
  ]
}
```

### Step Types

#### Tool Step
```json
{
  "type": "tool",
  "tool": "tool_name",
  "args": {
    "param": "value"
  }
}
```

Available tools:
- `shell`: Execute shell commands
- `filesystem`: Read/write/list files
- `web_search`: Search the web
- `web_fetch`: Fetch web pages
- `edit`: Edit files with search/replace

#### LLM Step
```json
{
  "type": "llm",
  "prompt": "Your prompt here with {input_variable}",
  "use_context": true
}
```

#### Shell Step
```json
{
  "type": "shell",
  "command": "echo {variable}",
  "working_dir": "/optional/path"
}
```

### Variable Substitution

Use `{variable_name}` syntax to substitute input values:

```json
{
  "inputs": [
    {"name": "project_name", "required": true}
  ],
  "steps": [
    {
      "type": "shell",
      "command": "mkdir {project_name}"
    }
  ]
}
```

### Using Skills

```bash
# Save a skill
terraphim-tinyclaw skill save path/to/skill.json

# List all skills
terraphim-tinyclaw skill list

# Load and view a skill
terraphim-tinyclaw skill load skill-name

# Run a skill with inputs
terraphim-tinyclaw skill run skill-name key1=value1 key2=value2

# Cancel a running skill
terraphim-tinyclaw skill cancel
```

## Storage Location

Skills are stored in:
- Linux/macOS: `~/.config/terraphim/skills/`
- Windows: `%APPDATA%\terraphim\skills\`

## More Information

See the [TinyClaw documentation](../../docs/) for detailed usage instructions.
