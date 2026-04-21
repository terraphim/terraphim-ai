# Research Document: EDM Scanner -- terraphim_negative_contribution Crate

## 1. Problem Restatement and Scope

**Problem**: LLM-generated code frequently contains deferral macros (`todo!()`, `unimplemented!()`, panic stubs) as scaffolding. These "negative contributions" slip through code review and accumulate as latent bugs. A zero-LLM static scanner can detect these before expensive LLM-based review runs.

**In scope** (#626 only):
- New crate `terraphim_negative_contribution` with Tier 1 pattern matching
- Aho-Corasick automaton for multi-pattern byte scanning
- File exclusion logic (tests, examples, benches, build.rs, inline test modules)
- Inline suppression via `// terraphim: allow(stub)`
- Output as `Vec<ReviewFinding>` compatible with existing compound review pipeline
- Unit tests covering all specified cases

**Out of scope** (deferred to later steps):
- Compound review wiring (`run_static_precheck()` in orchestrator) -- #627
- Integration test baseline against codebase -- #628
- LSP server crate (`terraphim_lsp`) -- #629
- Tier 2 patterns (`// FIXME`, `// HACK`)
- CoverageSignal analysis (eliminated by via-negativa)

## 2. User & Business Outcomes

- Compound review detects stubs before spending LLM tokens on them
- Review agents spend time on real issues, not scaffolding detection
- Foundational crate enables future LSP integration for real-time editor feedback
- Zero LLM cost for this class of detection

## 3. System Elements and Dependencies

### New Crate
| Element | Location | Role |
|---------|----------|------|
| `terraphim_negative_contribution` | `crates/terraphim_negative_contribution/` | Core EDM scanner |

### Existing Dependencies (incoming)
| Crate | Version | What We Use |
|-------|---------|-------------|
| `terraphim_types` | 1.15+ (workspace) | `ReviewFinding`, `FindingSeverity`, `FindingCategory`, `ReviewAgentOutput`, `deduplicate_findings()` |
| `aho-corasick` | 1.0.2 (via terraphim_automata) | Multi-pattern byte scanning. Direct dep needed since we don't use Thesaurus/find_matches. |

### Downstream Consumers (outgoing, future)
| Crate | How They'll Use It |
|-------|-------------------|
| `terraphim_orchestrator` | `Arc<NegativeContributionScanner>` in compound review pre-check (#627) |
| `terraphim_lsp` | Scanner reuse for editor diagnostics (#629) |

### Key Type Mappings
```
aho_corasick::AhoCorasick -> built once at new(), shared via &self
pattern index -> NEGATIVE_PATTERNS_TIER1[index] -> pattern name
byte offset -> count \n before offset -> line number
line content -> contains "terraphim: allow(stub)" -> suppress
file path -> is_non_production() -> skip entirely
```

### Review Types (from terraphim_types::review)
```rust
ReviewFinding {
    file: String,           // relative path
    line: u32,              // 1-indexed line number
    severity: FindingSeverity,  // High for todo!/unimplemented!()
    category: FindingCategory,  // Quality
    finding: String,        // human-readable description
    suggestion: Option<String>, // "Replace with implementation"
    confidence: f64,        // 0.95 for exact macro match
}

ReviewAgentOutput {
    agent: String,          // "edm-scanner"
    findings: Vec<ReviewFinding>,
    summary: String,
    pass: bool,             // true if 0 findings
}
```

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implication |
|------------|----------------|-------------|
| Aho-Corasick built once at `new()` | Avoids rebuilding per file | Scanner must be `Clone` + `Arc`-safe, no interior mutability |
| `is_non_production` checks full content | `#[test]` can appear anywhere, not just top 20 lines | Must scan entire file content for test markers -- acceptable since we're already scanning for patterns |
| Tier 1 patterns only | Tier 2 (`// FIXME`) has too many false positives | Only exact macro invocations: `todo!()`, `unimplemented!()`, panic variants |
| Inline suppression | Some stubs are intentional (e.g., trait defaults) | Check line content for `// terraphim: allow(stub)` before creating finding |
| No `diff.rs` | Use `diffy` crate | Don't reinvent diff parsing. But for Step 1, we scan file content, not diffs |
| Edition 2024 | Workspace standard | `use` statements must follow 2024 edition rules |
| Must not depend on `terraphim_automata` Thesaurus | Our patterns are fixed strings, not user-configurable thesaurus entries | Use `aho-corasick` directly, not `find_matches()` |

## 5. Risks, Unknowns, and Assumptions

### Risks
| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| False positives in macro-heavy crates | Medium | Low | Inline suppression mechanism |
| `aho-corasick` version conflict with `terraphim_automata` | Build failure | Low | Pin same version (1.0.2) |
| Scanning large files slowly | Performance | Low | Aho-Corasick is O(n); single pass |

### Unknowns
- Whether `diffy` is needed for Step 1 (likely no -- we scan full file content, not diffs)
- Whether `ReviewAgentOutput` needs to be produced by the scanner itself or by the orchestrator wrapper

### Assumptions
- The scanner operates on file content strings, not git diffs (diff-based scanning is future work)
- `ReviewAgentOutput` is constructed by the caller, not the scanner itself
- Confidence for exact macro matches is 0.95 (near-certain)
- File paths are relative to repo root

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity
- Two exclusion mechanisms (path-based AND content-based)
- Suppression syntax parsing

### Simplifications
1. **Direct `aho-corasick` instead of `terraphim_automata`**: Our patterns are fixed compile-time constants. No need for Thesaurus builder, NormalizedTerm, or any KG machinery.
2. **Single-pass scanning**: Aho-Corasick scans for all patterns simultaneously. No need for multiple passes.
3. **Separate `is_non_production` from scanning**: Clean separation of concerns. Scanner only scans; caller decides what files to feed it.

## 7. Questions for Human Reviewer

1. **Should `diffy` be a dependency of this crate, or only added when diff-based scanning is needed in Steps 2-3?** I recommend omitting it for Step 1 since we scan full file content.

2. **Should the scanner return `Vec<ReviewFinding>` directly, or a custom `NegativeContributionSignal` type that the caller converts?** I recommend `Vec<ReviewFinding>` directly for simplicity and immediate compatibility.

3. **Is `confidence: 0.95` appropriate for exact `todo!()` macro matches?** These are deterministic string matches, not probabilistic.

4. **Should `is_non_production()` be a public function or internal to the scanner?** Making it public allows the orchestrator to pre-filter file lists before calling the scanner.
