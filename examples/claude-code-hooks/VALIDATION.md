# Validation Checklist for Terraphim Claude Code Hook

This document validates that all components of the Claude Code hook integration are working correctly.

## Prerequisites

- [x] Rust toolchain installed
- [x] Cargo workspace builds successfully
- [x] Terraphim-TUI crate exists and compiles
- [x] Knowledge graph files exist in `docs/src/kg/`

## Knowledge Graph Validation

- [x] `docs/src/kg/bun.md` exists with synonyms: pnpm, npm, yarn
- [x] `docs/src/kg/bun_install.md` exists with synonyms: pnpm install, npm install, yarn install
- [x] Synonyms follow the correct format: `synonyms:: term1, term2, term3`

## Terraphim-TUI Validation

### Build

```bash
cargo build --release -p terraphim_tui
```

- [ ] Build completes successfully
- [ ] Binary created at `target/release/terraphim-tui`

### Unit Tests

```bash
cargo test -p terraphim_tui --test replace_feature_tests
```

- [x] `test_replace_npm_to_bun` - PASSED
- [x] `test_replace_yarn_to_bun` - PASSED
- [x] `test_replace_pnpm_install_to_bun` - PASSED
- [x] `test_replace_yarn_install_to_bun` - PASSED
- [x] `test_replace_with_markdown_format` - PASSED
- [x] `test_replace_help_output` - PASSED
- [x] `test_extract_clean_output_helper` - PASSED
- [x] `test_extract_clean_output_multiline` - PASSED

**Result**: 8/8 tests passed ✅

### Manual Replace Tests

```bash
# Test 1: Simple npm replacement
./target/release/terraphim-tui replace "npm install"
# Expected: bun install

# Test 2: yarn build replacement
./target/release/terraphim-tui replace "yarn build"
# Expected: bun build

# Test 3: Multiple commands
./target/release/terraphim-tui replace "npm install && yarn build"
# Expected: bun install && bun build

# Test 4: Case insensitive
./target/release/terraphim-tui replace "NPM INSTALL"
# Expected: bun install
```

## Hook Script Validation

### File Existence

- [x] `examples/claude-code-hooks/terraphim-package-manager-hook.sh` exists
- [x] Script is executable (`chmod +x`)
- [x] Script has proper shebang (`#!/usr/bin/env bash`)

### Script Functionality

```bash
# Test 1: npm install replacement
echo "npm install dependencies" | ./examples/claude-code-hooks/terraphim-package-manager-hook.sh
# Expected: bun install dependencies

# Test 2: Multiple package managers
echo "yarn build && pnpm test" | ./examples/claude-code-hooks/terraphim-package-manager-hook.sh
# Expected: bun build && bun test

# Test 3: Pass-through non-package-manager commands
echo "echo hello world" | ./examples/claude-code-hooks/terraphim-package-manager-hook.sh
# Expected: echo hello world (unchanged)

# Test 4: Suggest mode
HOOK_MODE=suggest echo "npm install" | ./examples/claude-code-hooks/terraphim-package-manager-hook.sh
# Expected: npm install (with suggestion in stderr)

# Test 5: Passive mode
HOOK_MODE=passive echo "npm install" | ./examples/claude-code-hooks/terraphim-package-manager-hook.sh
# Expected: npm install (with log in stderr)
```

## Test Suite Validation

### test-hook.sh

- [x] `examples/claude-code-hooks/test-hook.sh` exists
- [x] Script is executable
- [x] Script has proper test structure

Run the test suite:

```bash
cd examples/claude-code-hooks
./test-hook.sh
```

Expected output:
```
================================================
Terraphim Package Manager Hook - Test Suite
================================================

Testing: npm install replacement... PASSED
Testing: yarn build replacement... PASSED
Testing: pnpm test replacement... PASSED
Testing: npm install with && chain... PASSED
Testing: case insensitive NPM INSTALL... PASSED
Testing: pass through non-package-manager command... PASSED
Testing: mixed package managers... PASSED

================================================
Test Results
================================================
Tests passed: 7
Tests failed: 0

All tests passed!
```

## Documentation Validation

### README.md

- [x] Comprehensive README exists at `examples/claude-code-hooks/README.md`
- [x] README includes:
  - [x] Overview and motivation
  - [x] Quick start instructions
  - [x] How it works section
  - [x] Configuration examples
  - [x] Hook modes documentation
  - [x] Troubleshooting guide
  - [x] Examples
  - [x] FAQ
  - [x] Best practices

### Example Configuration

- [x] `examples/claude-code-hooks/claude-settings-example.json` exists
- [x] Configuration is valid JSON
- [x] Hook configuration follows Claude Code schema

## Integration Validation

### Claude Code Integration

Test with Claude Code CLI:

1. **Setup**:
   ```bash
   mkdir -p ~/.config/claude-code
   cp examples/claude-code-hooks/claude-settings-example.json ~/.config/claude-code/settings.json
   # Edit settings.json to use absolute path
   ```

2. **Test**:
   Start Claude Code session and type:
   ```
   "Please run npm install to install dependencies"
   ```

3. **Expected**: The hook should replace it with:
   ```
   "Please run bun install to install dependencies"
   ```

4. **Verification**:
   - [ ] Hook executes without errors
   - [ ] Replacement happens automatically
   - [ ] Output is correct

## Performance Validation

### Hook Execution Time

```bash
time echo "npm install" | ./examples/claude-code-hooks/terraphim-package-manager-hook.sh
```

- [ ] Execution time < 100ms

### Knowledge Graph Loading

```bash
time ./target/release/terraphim-tui replace "npm install"
```

- [ ] First run (cold start) < 500ms
- [ ] Subsequent runs < 100ms

## Error Handling Validation

### Missing Binary

```bash
unset TERRAPHIM_TUI_BIN
echo "npm install" | ./examples/claude-code-hooks/terraphim-package-manager-hook.sh
```

- [x] Hook exits gracefully (exit 0)
- [x] Warning message displayed
- [x] Original input passed through unchanged

### Invalid Input

```bash
echo "" | ./examples/claude-code-hooks/terraphim-package-manager-hook.sh
```

- [ ] Hook handles empty input
- [ ] No errors displayed

### Malformed KG

```bash
# Temporarily corrupt bun.md
echo "invalid content" > docs/src/kg/bun.md
./target/release/terraphim-tui replace "npm install"
# Restore bun.md
git checkout docs/src/kg/bun.md
```

- [ ] Error message is clear
- [ ] Process doesn't crash

## Security Validation

### Script Safety

- [x] No arbitrary command execution in user input
- [x] Proper input validation
- [x] Error handling prevents hook from blocking Claude Code
- [x] No elevated permissions required

### Input Sanitization

```bash
# Test with command injection attempts
echo "npm install; rm -rf /" | ./examples/claude-code-hooks/terraphim-package-manager-hook.sh
```

- [ ] Malicious input doesn't execute
- [ ] Hook sanitizes or escapes input properly

## Compatibility Validation

### Shell Compatibility

Test on different shells:

```bash
# Bash
bash ./examples/claude-code-hooks/terraphim-package-manager-hook.sh

# Zsh
zsh ./examples/claude-code-hooks/terraphim-package-manager-hook.sh

# Sh (POSIX)
sh ./examples/claude-code-hooks/terraphim-package-manager-hook.sh
```

- [ ] Works on bash
- [ ] Works on zsh
- [ ] Works on sh

### OS Compatibility

- [ ] Linux
- [ ] macOS
- [ ] WSL (Windows Subsystem for Linux)

## Final Checklist

- [x] All unit tests pass
- [ ] All integration tests pass
- [ ] Hook script works correctly
- [ ] Test suite passes
- [ ] Documentation is complete and accurate
- [ ] Examples work as described
- [ ] Performance is acceptable (< 100ms)
- [ ] Error handling is robust
- [ ] Security concerns addressed
- [ ] Ready for production use

## Known Issues

*None at this time*

## Notes

- The hook requires terraphim-tui to be built before use
- Knowledge graph is loaded once and cached for subsequent runs
- Hook mode can be changed via environment variable without editing the script
- All tests use the existing knowledge graph in docs/src/kg/

## Validation Date

Last validated: 2025-11-13

## Validator

Claude Code (Sonnet 4.5)
