# Post-LLM Checklist Validation

Use this skill to validate LLM outputs against domain checklists. Ensures outputs meet required standards before acceptance.

## When to Use

- After receiving LLM-generated code or content
- To verify domain compliance (security, code review, etc.)
- As a quality gate before committing AI-generated changes
- To ensure AI outputs follow project standards

## Available Checklists

### code_review
Validates code-related outputs for:
- ✓ tests - Test coverage mentioned/included
- ✓ documentation - Docs or comments present
- ✓ error_handling - Error handling addressed
- ✓ security - Security considerations noted
- ✓ performance - Performance implications considered

### security
Validates security-related outputs for:
- ✓ authentication - Auth mechanisms present
- ✓ authorization - Access control addressed
- ✓ input_validation - Input sanitization included
- ✓ encryption - Data protection considered
- ✓ logging - Audit logging mentioned

## Validation Commands

### Validate Against Code Review Checklist

```bash
terraphim-agent validate --checklist code_review --json "LLM output text"
```

**Output:**
```json
{
  "checklist_name": "code_review",
  "passed": false,
  "total_items": 5,
  "satisfied": ["tests", "documentation"],
  "missing": ["error_handling", "security", "performance"]
}
```

### Validate Against Security Checklist

```bash
terraphim-agent validate --checklist security --json "LLM output text"
```

## Workflow Example

```bash
# After receiving LLM output
LLM_OUTPUT="$1"

# Validate against code review checklist
RESULT=$(terraphim-agent validate --checklist code_review --json "$LLM_OUTPUT")
PASSED=$(echo "$RESULT" | jq -r '.passed')

if [ "$PASSED" = "false" ]; then
    echo "Post-LLM validation FAILED"
    echo "Missing items:"
    echo "$RESULT" | jq -r '.missing[]' | while read item; do
        echo "  - $item"
    done

    # Optionally request revision
    echo "Consider asking the LLM to address missing items."
else
    echo "Post-LLM validation PASSED"
fi
```

## Integration with Hooks

Automate via post-tool-use hook:

```bash
terraphim-agent hook --hook-type post-tool-use --input "$JSON"
```

## Best Practices

1. **Choose Appropriate Checklist**: Match checklist to output type
2. **Iterative Refinement**: If validation fails, prompt LLM for specific improvements
3. **Combine Checklists**: Run multiple validations for critical outputs
4. **Document Exceptions**: When skipping validation, document the reason

## Custom Checklists

Add custom checklists in `docs/src/kg/checklists/`:

```markdown
# my_checklist

Description of this checklist.

synonyms:: checklist name aliases
checklist:: item1, item2, item3

## Checklist Items

### item1
Description of item1.

synonyms:: item1 alias, another alias
```

Then use: `terraphim-agent validate --checklist my_checklist "text"`
