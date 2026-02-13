
````markdown
## UBS Quick Reference for AI Agents

UBS stands for "Ultimate Bug Scanner": **The AI Coding Agent's Secret Weapon: Flagging Likely Bugs for Fixing Early On**

**Install:** `curl -sSL https://raw.githubusercontent.com/Dicklesworthstone/ultimate_bug_scanner/master/install.sh | bash`

**Golden Rule:** `ubs --only=<lang> <changed-files>` before every commit. Exit 0 = safe. Exit >0 = fix & re-run.
Always specify language with `--only` to avoid false positives from cross-language scanning.

**Why --only is Critical:**
UBS auto-detects all 8 languages (js, python, cpp, rust, golang, java, ruby, swift) and scans every file with every scanner. Without `--only`:
- Python scanner reports "invalid-syntax" errors on Rust files
- JavaScript scanner flags "loose equality" in Rust code (false critical)
- Wasted time (8x slower) scanning files with wrong language parsers

**Always specify target language to avoid noise and false positives.**

**Commands:**
```bash
# Language-specific scanning (RECOMMENDED)
ubs --only=rust crates/terraphim_automata/src/lib.rs     # Rust files only
ubs --only=python test_*.py                               # Python files only
ubs --only=js desktop/src/lib/*.ts                      # JavaScript/TypeScript files only
ubs --only=js,python src/                               # Multiple languages

# General commands
ubs file.ts file2.py                                      # Specific files (use --only instead)
ubs $(git diff --name-only --cached)                      # Staged files — before commit
ubs --ci --fail-on-warning .                              # CI mode — before PR
ubs --help                                                # Full command reference
ubs sessions --entries 1                                  # Tail the latest install session log
ubs .                                                     # Whole project (slow, avoid)
```

**Language Flags Quick Reference:**
| Flag | File Extensions | Use For |
|------|----------------|---------|
| `--only=rust` | .rs | Rust source files |
| `--only=python` | .py, .pyi | Python scripts and tests |
| `--only=js` | .js, .ts, .jsx, .tsx | JavaScript/TypeScript files |
| `--only=cpp` | .c, .cpp, .h, .hpp | C/C++ files |
| `--only=golang` | .go | Go source files |
| `--only=java` | .java | Java source files |
| `--only=ruby` | .rb | Ruby scripts |
| `--only=swift` | .swift | Swift source files |

**Output Format:**
```
⚠️  Category (N errors)
    file.ts:42:5 – Issue description
    💡 Suggested fix
Exit code: 1
```
Parse: `file:line:col` → location | 💡 → how to fix | Exit 0/1 → pass/fail

**Fix Workflow:**
1. Read finding → category + fix suggestion
2. Navigate `file:line:col` → view context
3. Verify real issue (not false positive)
4. Fix root cause (not symptom)
5. Re-run `ubs <file>` → exit 0
6. Commit

**Speed Critical:** Scope to changed files. `ubs src/file.ts` (< 1s) vs `ubs .` (30s). Never full scan for small edits.

**Bug Severity:**
- **Critical** (always fix): Null safety, XSS/injection, async/await, memory leaks
- **Important** (production): Type narrowing, division-by-zero, resource leaks
- **Contextual** (judgment): TODO/FIXME, console logs

**Anti-Patterns:**
- ❌ Ignore findings → ✅ Investigate each
- ❌ Full scan per edit → ✅ Scope to file
- ❌ Fix symptom (`if (x) { x.y }`) → ✅ Root cause (`x?.y`)
- ❌ Scan without `--only` → ✅ Always specify target language
````
