# Pre-LLM Validation

Use this skill to validate input before sending to LLMs. Ensures semantic coherence and domain relevance using the local knowledge graph.

## When to Use

- Before making LLM API calls with user-provided context
- When validating that input contains relevant domain terms
- To check semantic coherence of multi-concept queries
- Pre-flight validation to reduce wasted LLM tokens

## Validation Steps

### 1. Check Semantic Connectivity

```bash
# Validate that input terms are semantically connected
terraphim-agent validate --connectivity --json "your input text"
```

**Output interpretation:**
- `connected: true` - Input is semantically coherent, proceed with LLM call
- `connected: false` - Input spans unrelated concepts, consider warning user

### 2. Check Domain Coverage

```bash
# Find matched domain terms
terraphim-agent validate --json "your input text"
```

**Output interpretation:**
- High match count - Input is domain-relevant
- Low/zero matches - Input may be off-topic or use non-standard terminology

### 3. Suggest Corrections (Optional)

If terms aren't found, suggest alternatives:

```bash
# Get fuzzy suggestions for potential typos
terraphim-agent suggest --fuzzy "unclear term" --threshold 0.6
```

## Workflow Example

```bash
# 1. Validate connectivity
RESULT=$(terraphim-agent validate --connectivity --json "$INPUT")
CONNECTED=$(echo "$RESULT" | jq -r '.connected')

if [ "$CONNECTED" = "false" ]; then
    echo "Warning: Input spans unrelated concepts"
    TERMS=$(echo "$RESULT" | jq -r '.matched_terms | join(", ")')
    echo "Matched terms: $TERMS"
fi

# 2. Check domain coverage
MATCHES=$(terraphim-agent validate --json "$INPUT" | jq '.matched_count')

if [ "$MATCHES" -lt 2 ]; then
    echo "Warning: Low domain coverage ($MATCHES terms)"
fi

# 3. Proceed with LLM call if validation passes
```

## Integration with Hooks

This skill can be automated via the pre-tool-use hook:

```bash
terraphim-agent hook --hook-type pre-tool-use --input "$JSON"
```

## Configuration

- `--role` - Specify role for domain-specific validation
- `--json` - Machine-readable output for scripting
- `--threshold` - Similarity threshold for fuzzy matching (default: 0.6)

## Best Practices

1. **Advisory Mode**: Use validation results as warnings, not blockers
2. **Role Selection**: Match role to the domain of the query
3. **Threshold Tuning**: Adjust fuzzy threshold based on domain vocabulary
4. **Caching**: Cache validation results for repeated queries
