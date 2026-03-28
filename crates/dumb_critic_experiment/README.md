# Dumb Critic Experiment

Validation of the hypothesis that smaller/cheaper LLM models produce higher-quality plan reviews than larger/smarter models.

## Hypothesis

> Smaller models produce better plan reviews than larger models. Smarter models "paper over problems with intelligence and intuition." A review by Haiku or GPT-5 Nano will surface more actionable defects than Opus.

Source: Hrishi (Anthropic), "Agentic Programming Patterns: 1"

## Quick Start

```bash
# 1. Set up OpenRouter API key
export OPENROUTER_API_KEY=your_key_here

# 2. Run full pipeline
cargo run -p dumb_critic_experiment -- full --plans-dir plans/

# Or run steps individually:

# Generate ground truth
cargo run -p dumb_critic_experiment -- generate-ground-truth --plans-dir plans/

# Run experiment (reviews all plans with all models)
cargo run -p dumb_critic_experiment -- run

# Score results
cargo run -p dumb_critic_experiment -- score

# Generate report
cargo run -p dumb_critic_experiment -- report
```

## Commands

- `generate-ground-truth` - Parse plan files and create ground truth manifest
- `run` - Execute reviews across all models
- `score` - Compare model outputs to ground truth
- `report` - Generate markdown report with findings
- `full` - Complete pipeline in one command
- `health-check` - Verify API connectivity

## Architecture

```
crates/dumb_critic_experiment/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── types.rs         # Ground truth data structures
│   ├── models.rs        # LLM tier definitions
│   ├── llm_client.rs    # OpenRouter API client
│   ├── ground_truth.rs  # Ground truth generation
│   ├── scoring.rs       # Review scoring logic
│   └── runner.rs        # Experiment orchestration
```

## Defect Categories

Plans should contain defects of these types (marked with HTML comments):

- `missing_prerequisite` - Missing dependency or prerequisite step
- `ambiguous_acceptance_criteria` - Unclear or subjective success criteria
- `wrong_ordering` - Steps in incorrect sequence
- `scope_creep` - Tasks exceeding stated scope
- `missing_rollback` - No failure recovery strategy
- `contradictory_statements` - Logically conflicting instructions
- `stale_reference` - References to outdated components

### Marking Defects in Plans

Add defects to plan markdown files using HTML comments:

```markdown
<!-- DEFECT: type=missing_prerequisite, description=Database schema not created first -->
1. Start the application server

<!-- DEFECT: type=ambiguous_acceptance_criteria, description="Fast" is not defined -->
- API should respond fast
```

## Models Under Test

| Tier | Model | Cost Rank | Intelligence Rank |
|------|-------|-----------|-------------------|
| Nano | GPT-4o Mini | 1 | 1 |
| Small | MiniMax M2.5 | 2 | 2 |
| Medium | Kimi K2.5 | 3 | 3 |
| Large | Claude Sonnet 3.5 | 4 | 4 |
| Oracle | Claude Opus 3 | 5 | 5 |

## Metrics

- **Recall**: fraction of ground truth defects found
- **Precision**: true positives / total reported
- **Actionability**: is the fix clear from the description?
- **Praise contamination**: does model praise despite instructions?
- **Cost-effectiveness**: defects found per dollar
- **Latency**: wall-clock time per review

## Success Criteria

- **Confirmed**: At least one model smaller than Sonnet achieves higher recall than Opus AND cost-effectiveness >3x Opus
- **Refuted**: Opus has strictly highest recall AND precision
- **Nuanced**: Different models excel at different defect types

## ADF Impact if Confirmed

1. meta-coordinator: Switch review from Opus to smaller model (~$2/run saving)
2. compound_review: Small model for plan pass, large for code pass
3. spec-validator: Currently Opus, potential downgrade
4. CJE judge: Add plan-review profile using smaller tier
5. Encode "dumb critic, smart builder" as ADF design rule
