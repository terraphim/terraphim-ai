# Ripgrep Tag Filtering Documentation

This document explains how to use tag filtering with Ripgrep haystacks in Terraphim AI, including configuration through the wizard UI and expected behavior.

## Overview

Tag filtering allows you to restrict search results to documents that contain specific hashtags (e.g., `#rust`, `#docs`, `#test`) in addition to your search terms. This feature is particularly useful for organizing and filtering content in knowledge bases.

## How It Works

When you configure a tag filter like `#rust`, the system generates a ripgrep command that requires **both** your search term and the specified tag to be present in the same file:

```bash
rg --json --trim -C3 --ignore-case -tmarkdown --all-match -e "your_search" -e "#rust" /path/to/haystack
```

The `--all-match` flag ensures that all specified patterns must be found for a document to be included in results.

## Configuration via Wizard UI

### Step 1: Create or Edit a Role

1. Open the Configuration Wizard at `/config/wizard`
2. Navigate to Step 2 (Roles)
3. Add a new role or edit an existing one
4. Add a haystack to the role

### Step 2: Configure Ripgrep Haystack

1. Set the **Service Type** to "Ripgrep (File Search)"
2. Set the **Directory Path** to your document directory
3. In the **Extra Parameters** section, you'll see tag filtering options

### Step 3: Set Up Tag Filtering

#### Option A: Use Preset Tags
- Use the "Presets" dropdown to select common tags:
  - `#rust` - Rust-related content
  - `#docs` - Documentation
  - `#test` - Testing-related content
  - `#todo` - TODO items

#### Option B: Manual Tag Entry
- Enter a custom tag in the "Hashtag" field (e.g., `#custom`, `#project-name`)
- Multiple tags can be separated by commas or spaces

### Step 4: Additional Parameters (Optional)

You can also configure other filtering parameters:

- **Max Results** (`max_count`): Limit the number of results per file
- **Custom Parameters**: Add other ripgrep options like `glob` patterns

## Configuration JSON Structure

When saved, the configuration includes the tag filter in the `extra_parameters` field:

```json
{
  "roles": {
    "Your Role Name": {
      "haystacks": [
        {
          "location": "/path/to/your/documents",
          "service": "Ripgrep",
          "read_only": false,
          "extra_parameters": {
            "tag": "#rust",
            "max_count": "10"
          }
        }
      ]
    }
  }
}
```

## Example Use Cases

### 1. Rust Development Team

Filter search results to only show Rust-related documentation:

```json
{
  "extra_parameters": {
    "tag": "#rust",
    "type": "md"
  }
}
```

**Generated command:**
```bash
rg --json --trim -C3 --ignore-case -tmarkdown --all-match -t md -e "async" -e "#rust" ./docs/
```

### 2. Documentation Search

Find only documentation files with specific tags:

```json
{
  "extra_parameters": {
    "tag": "#docs",
    "context": "5",
    "max_count": "15"
  }
}
```

**Generated command:**
```bash
rg --json --trim -C5 --ignore-case -tmarkdown --all-match --max-count 15 -e "api" -e "#docs" ./docs/
```

### 3. Testing Focus

Search only test-related documentation:

```json
{
  "extra_parameters": {
    "tag": "#test",
    "case_sensitive": "true"
  }
}
```

**Generated command:**
```bash
rg --json --trim -C3 --case-sensitive -tmarkdown --all-match -e "integration" -e "#test" ./docs/
```

## Supported Extra Parameters

| Parameter | Description | Example | Ripgrep Flag |
|-----------|-------------|---------|--------------|
| `tag` | Filter by hashtags | `"#rust"` | `-e "#rust" --all-match` |
| `glob` | File pattern filter | `"*.md"` | `--glob "*.md"` |
| `type` | File type filter | `"md"` | `-t md` |
| `max_count` | Max results per file | `"10"` | `--max-count 10` |
| `context` | Context lines | `"5"` | `-C 5` |
| `case_sensitive` | Case-sensitive search | `"true"` | `--case-sensitive` |

## Document Preparation

To use tag filtering effectively, your documents should include hashtags:

```markdown
# Rust Memory Management #rust

This document explains Rust's ownership system and memory safety features.

## Borrowing #rust #advanced

Understanding how borrowing works...

## Testing Your Code #rust #test

Here's how to write tests in Rust...
```

## Expected Behavior

### With Tag Filter `#rust`:
- **Search:** "memory"
- **Results:** Only files containing BOTH "memory" AND "#rust"
- **Excluded:** Files with "memory" but no "#rust" tag

### Without Tag Filter:
- **Search:** "memory"
- **Results:** All files containing "memory" regardless of tags

## Troubleshooting

### No Results Found

1. **Check tag syntax:** Ensure tags include the `#` symbol
2. **Verify document tags:** Confirm your documents actually contain the specified tags
3. **Case sensitivity:** By default, searches are case-insensitive
4. **File types:** Make sure you're searching the right file types (default is markdown)

### Too Many/Few Results

1. **Adjust `max_count`:** Limit results per file
2. **Add more specific tags:** Use multiple tags for better filtering
3. **Use `glob` patterns:** Filter by file paths or names

### Debug Information

Set `LOG_LEVEL=debug` to see detailed logging:

```bash
LOG_LEVEL=debug cargo run
```

Look for log messages like:
```
[INFO] 🏷️ Processing tag filter: '#rust'
[INFO] Added tag pattern: #rust
[INFO] 🚀 Executing: rg --json --trim -C3 --ignore-case -tmarkdown --all-match -e memory -e #rust /path/to/docs
```

## Testing

### Manual Testing

1. Create test files with and without tags
2. Configure a role with tag filtering
3. Perform searches and verify only tagged content appears

### Automated Testing

Run the validation script:

```bash
./scripts/validate_ripgrep_tag_filtering.sh
```

Or run the E2E tests:

```bash
cd desktop
npm test -- tests/e2e/ripgrep-tag-filtering.spec.ts
```

### Direct Command Testing

Test ripgrep commands directly:

```bash
# With tag filtering
rg --json --trim -C3 --ignore-case -tmarkdown --all-match -e "search_term" -e "#rust" ./docs/

# Without tag filtering (for comparison)
rg --json --trim -C3 --ignore-case -tmarkdown "search_term" ./docs/
```

## Best Practices

### Tag Naming Conventions

- Use descriptive, consistent tags: `#rust`, `#api`, `#tutorial`
- Avoid spaces in tags: `#rust-lang` not `#rust lang`
- Use lowercase for consistency: `#rust` not `#Rust`
- Group related content: `#rust-async`, `#rust-testing`

### Document Organization

- Add tags at the document level and section level
- Use multiple tags for cross-cutting concerns
- Keep tag lists updated as content evolves
- Document your tagging strategy for team members

### Performance Considerations

- Use specific tags to reduce search scope
- Set appropriate `max_count` limits
- Consider using `glob` patterns for path-based filtering
- Monitor search performance with complex tag combinations

## Integration with Other Features

### Knowledge Graphs
Tag filtering works alongside knowledge graph processing. Tagged documents will still contribute to graph relationships while being filtered during search.

### Role-Based Configuration
Different roles can have different tag filtering strategies:
- **Developer Role:** Filter by `#code`, `#api`, `#architecture`
- **Documentation Role:** Filter by `#docs`, `#guide`, `#tutorial`
- **QA Role:** Filter by `#test`, `#bug`, `#validation`

### Multiple Haystacks
Each haystack in a role can have different tag filtering configuration, allowing for granular control over different document sources.

## Future Enhancements

Potential improvements to tag filtering:

1. **Tag Suggestions:** Auto-complete based on existing tags in documents
2. **Tag Analytics:** Show tag usage statistics
3. **Exclude Tags:** Support for excluding certain tags (NOT operator)
4. **Tag Hierarchies:** Support parent/child tag relationships
5. **Visual Tag Management:** UI for browsing and managing tags

## Support and Feedback

For issues related to tag filtering:

1. Check the server logs for detailed error messages
2. Verify your ripgrep installation: `rg --version`
3. Test with direct ripgrep commands to isolate issues
4. Report bugs with configuration details and log output
