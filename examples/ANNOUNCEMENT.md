# 🚀 Announcing Terraphim's Claude Code Integration

**Three powerful ways to integrate Terraphim's knowledge graph capabilities with Claude Code**

---

## What's New?

We're excited to announce a comprehensive integration between Terraphim and Claude Code, providing three complementary capabilities for AI-assisted development:

### 1. 🪝 Claude Code Hooks
**Automatic, transparent text replacement**

Intercept user prompts before Claude sees them to enforce standards and preferences:

```bash
# Example: Automatically enforce bun over npm/yarn/pnpm
echo "npm install && yarn test" | terraphim-tui replace
# Output: bun_install && bun test
```

**Use Cases:**
- Package manager enforcement
- Coding standard compliance
- Attribution replacement
- Domain-specific terminology

### 2. 🎯 Claude Skills
**Context-aware, conversational assistance**

Progressive disclosure of Terraphim capabilities across all Claude platforms:

- Works on web, mobile, and desktop
- Provides explanations and reasoning
- Learns when to apply replacements
- Fully conversational interface

### 3. 📊 Codebase Quality Evaluation
**Objective assessment of AI-generated changes**

Deterministic framework for evaluating whether AI agents improve or deteriorate your codebase:

```bash
./scripts/evaluate-ai-agent.sh /path/to/codebase

# Generates verdict:
# ✅ IMPROVEMENT: The AI agent improved the codebase quality.
# - Improved metrics: 4
# - Deteriorated metrics: 0
```

**Key Features:**
- ✅ Deterministic (Aho-Corasick automata)
- ✅ Privacy-first (runs locally)
- ✅ Multi-dimensional (security, performance, quality)
- ✅ CI/CD ready (exit codes for automation)

---

## Why This Matters

As AI coding assistants become more prevalent, we need **objective quality gates** to ensure they help rather than hurt our codebases.

**Traditional Approach:**
```
AI changes → Manual review → Hope → Ship
```

**Terraphim Approach:**
```
AI changes → Automatic evaluation → Objective verdict → Block if worse → Ship confidently
```

---

## Quick Start

### Installation

```bash
# Docker (easiest)
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/docker-run.sh | bash

# Or binary installation
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/install.sh | bash
```

### Try Evaluation

```bash
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai/examples/codebase-evaluation
./scripts/evaluate-ai-agent.sh /path/to/your/codebase
```

### Set Up Hooks

```bash
# Copy hook script
cp examples/claude-code-hooks/terraphim-package-manager-hook.sh ~/.claude/hooks/

# Configure Claude Code settings
# See: examples/claude-code-hooks/README.md
```

### Set Up Skills

```bash
# Copy skill
cp -r examples/claude-skills/terraphim-package-manager ~/.claude/skills/

# Restart Claude
```

---

## Example: CI/CD Quality Gate

```yaml
name: AI Quality Check

on: pull_request

jobs:
  evaluate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Terraphim
        run: curl -fsSL https://[...]/install.sh | bash
      - name: Baseline evaluation
        run: ./scripts/baseline-evaluation.sh .
      - name: Post-change evaluation
        run: ./scripts/post-evaluation.sh .
      - name: Generate verdict (fails on deterioration)
        run: ./scripts/compare-evaluations.sh
```

---

## Documentation

**Complete Guides:**
- [Integration Guide](https://github.com/terraphim/terraphim-ai/blob/main/examples/TERRAPHIM_CLAUDE_INTEGRATION.md)
- [Evaluation Design](https://github.com/terraphim/terraphim-ai/blob/main/examples/CODEBASE_EVALUATION_DESIGN.md)
- [Quick Start](https://github.com/terraphim/terraphim-ai/blob/main/examples/codebase-evaluation/README.md)

**Examples:**
- [Hooks](https://github.com/terraphim/terraphim-ai/tree/main/examples/claude-code-hooks)
- [Skills](https://github.com/terraphim/terraphim-ai/tree/main/examples/claude-skills)
- [Evaluation Scripts](https://github.com/terraphim/terraphim-ai/tree/main/examples/codebase-evaluation/scripts)

---

## Use Cases

### 1. Package Manager Enforcement
Automatically replace npm/yarn/pnpm with bun in all prompts and code.

### 2. PR Quality Gates
Block PRs from AI agents if they deteriorate code quality.

### 3. Security Auditing
Evaluate AI changes for security vulnerabilities before merge.

### 4. Performance Analysis
Assess whether AI optimizations actually improve performance.

### 5. Trend Monitoring
Track codebase quality evolution over time across AI changes.

---

## Key Benefits

**Objectivity** - Quantifiable metrics over subjective opinions
**Speed** - Evaluations in seconds, not hours
**Privacy** - Everything runs locally, no external APIs
**Determinism** - Same input → same output
**Transparency** - See exactly why quality changed

---

## Technical Highlights

**Aho-Corasick Automata**
- O(n + m) complexity for pattern matching
- Search thousands of patterns simultaneously
- Deterministic and fast

**Knowledge Graphs**
- Define semantic relationships in markdown
- Build thesauri for concept mapping
- Extensible for any domain

**Role-Based Evaluation**
- Security Auditor: Vulnerabilities and attack vectors
- Performance Analyst: Bottlenecks and efficiency
- Code Reviewer: Quality and maintainability
- Documentation Quality: Completeness and clarity

---

## Community

**Connect:**
- GitHub: [terraphim/terraphim-ai](https://github.com/terraphim/terraphim-ai)
- Discord: https://discord.gg/VPJXB6BGuY
- Discourse: https://terraphim.discourse.group

**Contribute:**
- Share knowledge graph templates
- Report useful evaluation patterns
- Submit PRs with improvements
- Write about your experiences

---

## What's Next

**Short Term:**
- Visual dashboard for evaluation trends
- More built-in evaluation roles
- Language-specific KG templates

**Medium Term:**
- ML-enhanced pattern detection
- Automatic KG expansion
- Real-time evaluation

**Long Term:**
- Distributed team evaluation
- Knowledge graph marketplace
- Advanced analytics

---

## Try It Today

All code is Apache 2.0 licensed. Privacy-first. Runs locally. No external dependencies.

**Get Started:** https://github.com/terraphim/terraphim-ai

---

*Terraphim: Privacy-first AI assistant for trusted AI-assisted development*
