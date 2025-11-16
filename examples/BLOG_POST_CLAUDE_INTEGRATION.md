# Introducing Terraphim's Claude Code Integration: Deterministic AI Quality Gates for Your Codebase

**TL;DR**: We've built a complete framework for integrating Terraphim's knowledge graph capabilities with Claude Code, enabling automatic text replacement, conversational skills, and objective codebase quality evaluation. All deterministic, privacy-first, and running locally.

---

## The Challenge: Can We Trust AI-Generated Code?

AI coding assistants like Claude Code, GitHub Copilot, and autonomous agents are revolutionizing software development. But they raise a critical question:

**How do we know if AI changes actually improve our codebase—or make it worse?**

Traditional approaches rely on subjective code review and hope that tests catch problems. But what if we could evaluate AI-generated changes objectively, deterministically, and automatically?

That's exactly what we've built with Terraphim's new Claude Code integration.

## Three Pillars of Integration

Our integration provides three complementary capabilities:

### 1. 🪝 Claude Code Hooks: Automatic Text Replacement

Hooks intercept user prompts before Claude sees them, enabling transparent, automatic replacements:

```bash
# In your Claude Code settings
{
  "hooks": {
    "user-prompt-submit": {
      "command": "bash",
      "args": ["/path/to/terraphim-package-manager-hook.sh"],
      "enabled": true
    }
  }
}
```

**Example: Package Manager Enforcement**

Want to enforce `bun` over `npm`, `yarn`, or `pnpm`? Create knowledge graph entries:

```markdown
# docs/src/kg/bun.md
# Bun

Bun is a modern JavaScript runtime and package manager.

synonyms:: pnpm, npm, yarn
```

```markdown
# docs/src/kg/bun_install.md
# bun install

Fast package installation with Bun.

synonyms:: pnpm install, npm install, yarn install
```

Now when you (or Claude) write "npm install", it automatically becomes "bun install". Deterministically. Every time.

**How It Works:**
1. User types prompt mentioning "npm install"
2. Hook intercepts the prompt
3. Terraphim uses Aho-Corasick automata to find matches in knowledge graph
4. Prompt is modified: "npm install" → "bun install"
5. Claude receives the modified prompt
6. Claude generates code using bun instead of npm

**Real Output:**
```bash
$ echo "npm install && yarn test" | terraphim-tui replace
bun_install && bun test
```

### 2. 🎯 Claude Skills: Context-Aware Assistance

Skills provide conversational, explanatory integration that works across all Claude platforms:

```yaml
---
name: terraphim-package-manager
description: Automatically replace package manager commands with bun
---

When the user mentions npm, yarn, or pnpm, suggest using bun instead...
```

Unlike hooks, skills:
- Explain *why* they're making suggestions
- Provide context about the replacements
- Work on web, mobile, and desktop Claude interfaces
- Use progressive disclosure (metadata → instructions → resources)

**Example Interaction:**
```
You: "Let's add a new package with npm install express"

Claude: "I notice you mentioned npm. Based on Terraphim's knowledge graph,
I can suggest using bun instead for faster installation. Would you like me
to proceed with 'bun install express'?

Here's why bun is preferred:
- 10-100x faster than npm
- Drop-in replacement
- Better caching

Shall I use bun?"
```

### 3. 📊 Codebase Quality Evaluation: The Game Changer

This is where things get really interesting. We've built a complete framework for **objectively evaluating whether AI agents improve or deteriorate your codebase**.

#### The Problem

You run Claude Code on your project. It makes 50 changes across 20 files. Is your code better or worse now?

Without objective measurement, you're relying on:
- Manual code review (slow, subjective)
- Hoping tests catch issues (reactive, incomplete)
- Gut feeling (unreliable)

#### Our Solution: Deterministic Evaluation

Terraphim evaluates code quality using:

1. **Knowledge Graphs** - Define what "good" and "bad" code looks like
2. **Aho-Corasick Automata** - Fast, deterministic pattern matching
3. **Quantifiable Metrics** - Objective scores you can track over time
4. **Multi-Dimensional Analysis** - Security, performance, quality perspectives

#### How It Works

**Step 1: Define Evaluation Perspectives**

Create knowledge graph files defining quality patterns:

```markdown
# code-quality.md
# Code Quality

synonyms:: code smell, technical debt, refactoring opportunity, bad practice
```

```markdown
# bug-patterns.md
# Bug Patterns

synonyms:: null pointer, memory leak, race condition, unhandled exception
```

```markdown
# security.md
# Security Vulnerability

synonyms:: SQL injection, XSS, CSRF, authentication flaw, command injection
```

**Step 2: Baseline Evaluation**

Before AI makes changes:

```bash
./scripts/baseline-evaluation.sh /path/to/codebase "Code Reviewer"

# Collects:
# - Clippy warnings: 15
# - Anti-patterns (unwrap, panic): 23
# - TODOs/FIXMEs: 47
# - Knowledge graph matches: 12 code smells detected
```

**Step 3: Apply AI Changes**

Let Claude Code (or any AI agent) modify your codebase.

**Step 4: Post-Change Evaluation**

After AI changes:

```bash
./scripts/post-evaluation.sh /path/to/codebase "Code Reviewer"

# Collects same metrics:
# - Clippy warnings: 8  (↓ 7)
# - Anti-patterns: 18   (↓ 5)
# - TODOs/FIXMEs: 45    (↓ 2)
# - Code smells: 9      (↓ 3)
```

**Step 5: Generate Verdict**

```bash
./scripts/compare-evaluations.sh

# Generates:
# ✅ IMPROVEMENT: The AI agent improved the codebase quality.
#
# - Improved metrics: 4
# - Deteriorated metrics: 0
# - Neutral metrics: 1
#
# Recommendations:
# - Review remaining 8 clippy warnings
# - No critical issues found
```

The script exits with code 1 if quality deteriorates, making it perfect for CI/CD quality gates.

## Real-World Use Case: PR Evaluation in CI/CD

Here's how to use this in GitHub Actions:

```yaml
name: AI Agent Quality Check

on:
  pull_request:
    types: [opened, synchronize]

jobs:
  evaluate-ai-changes:
    runs-on: ubuntu-latest

    steps:
      - name: Checkout baseline (main)
        uses: actions/checkout@v3
        with:
          ref: main
          path: baseline

      - name: Checkout PR changes
        uses: actions/checkout@v3
        with:
          path: pr-changes

      - name: Install Terraphim
        run: |
          curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/install.sh | bash

      - name: Baseline evaluation
        run: |
          cd baseline
          ../scripts/baseline-evaluation.sh . "Code Reviewer"

      - name: Post-change evaluation
        run: |
          cd pr-changes
          ../scripts/post-evaluation.sh . "Code Reviewer"

      - name: Generate verdict
        id: verdict
        run: |
          ./scripts/compare-evaluations.sh
          # Exits with code 1 on deterioration

      - name: Post verdict as comment
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const verdict = fs.readFileSync('./evaluation-results/verdict.md', 'utf8');

            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: verdict
            });
```

**Result**: Automatic quality gates that block PRs if AI changes deteriorate code quality.

## Multi-Role Evaluation: Security, Performance, Quality

You're not limited to one perspective. Evaluate from multiple angles:

```bash
# Security audit
./scripts/evaluate-ai-agent.sh ./codebase claude-code "Security Auditor"

# Performance analysis
./scripts/evaluate-ai-agent.sh ./codebase claude-code "Performance Analyst"

# Code quality review
./scripts/evaluate-ai-agent.sh ./codebase claude-code "Code Reviewer"

# Documentation check
./scripts/evaluate-ai-agent.sh ./codebase claude-code "Documentation Quality"
```

Each role uses its own knowledge graph to focus on different aspects:

- **Security Auditor**: SQL injection, XSS, authentication flaws
- **Performance Analyst**: O(n²) algorithms, blocking operations, bottlenecks
- **Code Reviewer**: Technical debt, code smells, refactoring opportunities
- **Documentation Quality**: Missing docstrings, unclear APIs

## Why This Matters: The Bigger Picture

As AI agents become more autonomous, we need **objective quality gates** to ensure they're helping, not hurting.

**Traditional Approach:**
```
AI makes changes → Manual review → Hope for the best → Ship
```

**Terraphim Approach:**
```
AI makes changes → Automatic evaluation → Objective verdict → Block if worse → Ship confidently
```

### Key Benefits

**1. Objectivity**
- Quantifiable metrics over subjective opinions
- Consistent evaluation across all changes
- No human bias in assessment

**2. Speed**
- Evaluations run in seconds
- No waiting for manual code review
- Immediate feedback in CI/CD

**3. Privacy**
- Everything runs locally
- No code sent to external APIs
- Your codebase stays private

**4. Determinism**
- Same input → same output
- Aho-Corasick automata are deterministic
- Repeatable across environments

**5. Transparency**
- See exactly why quality improved/deteriorated
- Detailed metrics and explanations
- Audit trail for quality changes

## The Technology: How It Works Under the Hood

### Knowledge Graphs

At the core is Terraphim's knowledge graph system. You define semantic relationships in markdown:

```markdown
# Performance Bottleneck

synonyms:: slow code, inefficient algorithm, O(n^2) complexity,
          blocking operation, performance issue
```

This creates a thesaurus mapping synonyms to normalized concepts.

### Aho-Corasick Automata

Terraphim builds an Aho-Corasick automaton from the knowledge graph:

```rust
let ac = AhoCorasick::builder()
    .match_kind(MatchKind::LeftmostLongest)
    .ascii_case_insensitive(true)
    .build(patterns)?;

let result = ac.replace_all_bytes(text.as_bytes(), &replace_with);
```

**Why Aho-Corasick?**
- **O(n + m) complexity**: Linear time in text length + pattern count
- **Multiple patterns simultaneously**: Search for thousands of patterns at once
- **Deterministic**: Same input always produces same output
- **Fast**: Ideal for real-time text processing

### Role-Based Evaluation

Each evaluation role has:
- **Name**: "Code Reviewer", "Security Auditor", etc.
- **Knowledge Graph**: Domain-specific patterns
- **Haystack**: The codebase to search
- **Relevance Function**: TerraphimGraph for semantic ranking

```json
{
  "name": "Code Reviewer",
  "relevance_function": "terraphim-graph",
  "kg": {
    "knowledge_graph_local": {
      "input_type": "markdown",
      "path": "docs/src/kg/code-quality"
    }
  }
}
```

## Getting Started: Try It Today

### 1. Install Terraphim

```bash
# Docker (easiest)
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/docker-run.sh | bash

# Or binary installation
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/install.sh | bash

# Or from source
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
cargo build --release -p terraphim_tui --features repl-full
```

### 2. Set Up Text Replacement

**For Hooks:**
```bash
# Copy hook script
cp examples/claude-code-hooks/terraphim-package-manager-hook.sh ~/.claude/hooks/

# Configure Claude Code settings
# See: examples/claude-code-hooks/README.md
```

**For Skills:**
```bash
# Copy skill to Claude skills directory
cp -r examples/claude-skills/terraphim-package-manager ~/.claude/skills/

# Restart Claude to load the skill
```

### 3. Try Codebase Evaluation

```bash
# Clone examples
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai/examples/codebase-evaluation

# Run evaluation on your project
./scripts/evaluate-ai-agent.sh /path/to/your/codebase

# The script will:
# 1. Capture baseline metrics
# 2. Prompt you to make AI changes
# 3. Re-evaluate after changes
# 4. Generate verdict report
```

## Documentation and Resources

**Complete Guides:**
- [Terraphim-Claude Integration Guide](https://github.com/terraphim/terraphim-ai/blob/main/examples/TERRAPHIM_CLAUDE_INTEGRATION.md)
- [Codebase Evaluation Design](https://github.com/terraphim/terraphim-ai/blob/main/examples/CODEBASE_EVALUATION_DESIGN.md)
- [Evaluation Quick Start](https://github.com/terraphim/terraphim-ai/blob/main/examples/codebase-evaluation/README.md)

**Examples:**
- [Hook Implementation](https://github.com/terraphim/terraphim-ai/tree/main/examples/claude-code-hooks)
- [Skills Implementation](https://github.com/terraphim/terraphim-ai/tree/main/examples/claude-skills)
- [Evaluation Scripts](https://github.com/terraphim/terraphim-ai/tree/main/examples/codebase-evaluation/scripts)

**Knowledge Graph Templates:**
- [Code Quality Patterns](https://github.com/terraphim/terraphim-ai/blob/main/examples/codebase-evaluation/kg-templates/code-quality.md)
- [Bug Patterns](https://github.com/terraphim/terraphim-ai/blob/main/examples/codebase-evaluation/kg-templates/bug-patterns.md)
- [Security Patterns](https://github.com/terraphim/terraphim-ai/blob/main/examples/codebase-evaluation/kg-templates/security.md)
- [Performance Patterns](https://github.com/terraphim/terraphim-ai/blob/main/examples/codebase-evaluation/kg-templates/performance.md)

## What's Next: Future Enhancements

We're continuously improving the integration. Upcoming features:

**Short Term:**
- Visual dashboard for evaluation trends
- More built-in evaluation roles
- Language-specific knowledge graph templates
- Integration with more AI coding tools

**Medium Term:**
- Machine learning-enhanced pattern detection
- Automatic knowledge graph expansion from codebases
- Real-time evaluation during AI generation
- Multi-language support for knowledge graphs

**Long Term:**
- Distributed evaluation across teams
- Knowledge graph marketplace
- Advanced analytics and reporting
- Integration with code quality platforms

## Join the Community

We'd love to hear about your use cases and experiences!

**Connect:**
- **GitHub**: [terraphim/terraphim-ai](https://github.com/terraphim/terraphim-ai)
- **Discord**: [Join our Discord](https://discord.gg/VPJXB6BGuY)
- **Discourse**: [Terraphim Forum](https://terraphim.discourse.group)

**Contribute:**
- Share your knowledge graph templates
- Report evaluation patterns you find useful
- Submit PRs with improvements
- Write about your experiences

## Conclusion: The Future of AI-Assisted Development

AI coding assistants are here to stay. The question isn't *if* we'll use them—it's *how* we ensure they improve rather than deteriorate our codebases.

Terraphim's Claude Code integration provides:

✅ **Automatic text replacement** for enforcing standards
✅ **Conversational skills** for guided assistance
✅ **Objective quality evaluation** for AI-generated changes
✅ **Deterministic, privacy-first** assessment you can trust
✅ **CI/CD integration** for automated quality gates

All running locally, with no external dependencies, using proven algorithms like Aho-Corasick for fast, deterministic pattern matching.

**Try it today** and join us in building the future of trusted AI-assisted development.

---

*Terraphim is a privacy-first AI assistant that works for you under your complete control. All code is Apache 2.0 licensed.*

**Links:**
- [Main Repository](https://github.com/terraphim/terraphim-ai)
- [Integration Examples](https://github.com/terraphim/terraphim-ai/tree/main/examples)
- [Installation Guide](https://github.com/terraphim/terraphim-ai/blob/main/release/v0.2.3/README.md)
- [Contributing](https://github.com/terraphim/terraphim-ai/blob/main/CONTRIBUTING.md)
