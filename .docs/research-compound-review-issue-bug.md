# Research Document: Compound Review Issue Serialisation Bug

## 1. Problem Restatement and Scope

**Problem:** When compound review finds a `ReviewFinding`, the `finding.finding` field contains raw JSON (from a tool call response) that gets embedded directly into the Gitea issue title and body without proper escaping. This results in malformed issues showing raw JSON visible instead of properly formatted markdown.

**IN SCOPE:**
- The code path that creates Gitea issues from ReviewFinding objects
- How the `finding.finding` string is populated by review agents
- Title and body escaping in `file_finding_issue` method in `lib.rs`
- JSON handling in review agent outputs

**OUT OF SCOPE:**
- Gitea API label handling (already noted as separate issue)
- The underlying cause of JSON being in `finding.finding` (that's agent output)
- Previous labels field removal

## 2. User & Business Outcomes

- Users see unreadable issues in Gitea with raw JSON in titles
- Automated finding tracking is broken - issues cannot be triaged
- Compound review automated workflows are compromised

## 3. System Elements and Dependencies

| Component | Location | Role |
|-----------|----------|------|
| `file_finding_issue` method | `crates/terraphim_orchestrator/src/lib.rs:1305-1395` | Creates Gitea issues from findings |
| `ReviewFinding` struct | `crates/terraphim_symphony/src/runner/protocol.rs:191` | Data structure containing finding text |
| `create_issue` method | `crates/terraphim_tracker/src/gitea.rs:241` | Gitea API call |
| Review agents | Various agents in orchestrator | Populate `ReviewFinding.finding` field |

**Data Flow:**
1. Review agent runs and produces `ReviewAgentOutput` with `Vec<ReviewFinding>`
2. Each `ReviewFinding` is passed to `file_finding_issue`
3. Title constructed from `finding.finding` substring (line 1349-1353)
4. Body constructed from `finding.finding` directly (line 1374)
5. `create_issue` called with unescaped title and body

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Solution Impact |
|------------|----------------|------------------|
| Issue title length limits | Gitea likely has title length limits | Need to truncate carefully |
| JSON in finding text | Raw JSON breaks markdown rendering | Need to escape or strip JSON formatting |
| Dedup search relies on title keywords | Finding text used for matching | Cannot completely sanitise without breaking dedup |
| Existing code assumes finding text is readable | No current escaping logic | Need to add escaping layer |

## 5. Risks, Unknowns, and Assumptions

**Assumptions:**
1. The `finding.finding` field contains JSON from tool call serialisation because review agents return tool output verbatim
2. The issue is NOT with the Gitea API returning JSON - it's the input being malformed

**Risks:**
- **Risk 1:** Escaping could break dedup search if keywords change significantly
- **Risk 2:** Multiple levels of JSON nesting could make escaping complex
- **Risk 3:** Other callers of `file_finding_issue` might have different expectations

**Unknowns:**
1. Whether ALL findings contain JSON or just certain agent types
2. The exact Gitea API title length limit
3. Whether body escaping is needed or just title

## 6. Context Complexity vs. Simplicity Opportunities

**Complexity Sources:**
- Two separate fields (title, body) need different escaping strategies
- Dedup logic depends on finding text - need to preserve keywords
- Need to understand why JSON appears in finding field

**Simplification Strategies:**
- Create a dedicated escaping utility function for issue content
- Add a "clean finding text" transformation that strips JSON but preserves readable content
- Consider separate handling for title (strict) vs body (can be more lenient)

## 7. Questions for Human Reviewer

1. Is it acceptable to strip JSON syntax (`{}`, `[]`, `"`) from finding text, or should it be escaped to preserve the original?
2. Should the dedup keyword be based on cleaned text or original text?
3. Do we need to handle this at the review agent level (before it reaches orchestrator) or at the issue filing level?
4. Should we also escape for other special characters (backticks, etc.)?
5. What's the priority - is issue #276 blocking operations?