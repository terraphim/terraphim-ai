<h3>Summary</h3>

Reviewer accidentally posted the draft before filling in the confidence
score. The Inline Findings section and footer are present but the score
header is missing.

<h3>Important Files Changed</h3>

| Filename | Overview |
|----------|----------|
| `crates/terraphim_orchestrator/src/scope.rs` | Adds `scope_from_labels` helper. |

<h3>Inline Findings</h3>

**P2 crates/terraphim_orchestrator/src/scope.rs, line 18**: **Missing docstring**

The new helper lacks a docstring explaining the label precedence rules.

<sub>Last reviewed commit: 6becb2f7 | Reviews (1)</sub>
