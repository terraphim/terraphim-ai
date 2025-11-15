# GitHub Actions Fix Plan for v1.0.0 Release

## Current Status Analysis

### Failed Workflows
1. ❌ **Package Release** - Exit code 2
2. ❌ **Publish Tauri with Auto-Update** - Build errors

### Queued Workflows (Waiting)
3. ⏳ **Earthly CI/CD** - Queued
4. ⏳ **CI Native** - Queued
5. ⏳ **CI Optimized** - Queued
6. ⏳ **Comprehensive Release** - Queued

## Root Causes Identified

### Issue 1: Missing .cargo/config.toml
**Workflow**: Package Release
**Error**: `sed: can't read .cargo/config.toml: No such file or directory`
**Line**: Step "Temporarily disable panic abort for building"
**Impact**: Workflow fails at line 52 before building

**Fix**:
```bash
# Option A: Create .cargo/config.toml (if needed)
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[profile.release]
panic = "abort"
opt-level = 3
lto = true
codegen-units = 1
EOF

# Option B: Make workflow step conditional
# Modify workflow to check if file exists first
```

### Issue 2: Svelte Accessibility Warnings (Tauri Build)
**Workflow**: Publish Tauri with Auto-Update
**Errors**: Multiple accessibility warnings in Svelte components
**Impact**: Build may treat warnings as errors

**Files Needing Fixes**:
1. `desktop/src/lib/ConfigWizard.svelte:642` - Label without associated control
2. `desktop/src/lib/ConfigWizard.svelte:901` - Self-closing textarea tag
3. `desktop/src/lib/Chat/SessionList.svelte:219` - Button missing aria-label
4. `desktop/src/lib/Chat/ContextEditModal.svelte:187` - Label without control
5. `desktop/src/lib/Search/KGSearchModal.svelte:708` - Div with keydown needs ARIA role

**Fixes**:
```html
<!-- Fix 1: Label association -->
<label for="config-input">Setting</label>
<input id="config-input" />

<!-- Fix 2: Textarea closing tag -->
<textarea></textarea>  <!-- Not <textarea /> -->

<!-- Fix 3: Button aria-label -->
<button aria-label="Delete session">🗑️</button>

<!-- Fix 4: Div with keyboard handler -->
<div role="button" tabindex="0" on:keydown={handler}>...</div>
```

### Issue 3: Package.json Dependency Conflicts
**Warnings**: Multiple collisions between optionalDependencies and devDependencies
**Impact**: May cause npm/yarn resolution issues

**Conflicting Packages**:
- @tauri-apps/cli: ^1.6.3 vs ^1.5.11
- @testing-library/jest-dom: ^6.8.0 vs ^6.9.1
- @testing-library/svelte: ^5.2.8 vs ^4.0.0
- dotenv: ^17.2.3 vs ^16.4.5
- jsdom: ^24.1.0 vs ^25.0.1
- sass: 1.92.1 vs ^1.83.0
- selenium-webdriver: ^4.34.0 vs ^4.21.0
- svelte-typeahead: ^5.0.1 vs ^4.4.1

## Fix Priority Matrix

### Priority 1: Critical (Blocks Release)
1. **Create/Fix .cargo/config.toml issue** (5 min)
2. **Fix Svelte accessibility errors** (30 min)
3. **Clean up package.json dependencies** (15 min)

### Priority 2: Important (CI Health)
4. **Update workflow to handle missing files gracefully** (10 min)
5. **Add build flag to treat warnings as non-fatal** (5 min)

### Priority 3: Nice to Have
6. **Optimize queued workflows** (defer)
7. **Add pre-commit hooks for accessibility** (defer)

## Step-by-Step Fix Plan

### Step 1: Create .cargo/config.toml (If Needed)
```bash
# Check if this file should exist
git log --all --full-history -- .cargo/config.toml

# If it was deleted, restore it
# If it never existed, workflow needs fixing
```

### Step 2: Fix package-release.yml Workflow
```yaml
# Make the sed step conditional
- name: Temporarily disable panic abort for building
  run: |
    if [ -f .cargo/config.toml ]; then
      sed -i 's/panic = "abort"/# panic = "abort\"/' .cargo/config.toml
    else
      echo "No .cargo/config.toml found, skipping panic abort modification"
    fi
```

### Step 3: Fix Svelte Accessibility Issues
Create a batch fix script:
```bash
#!/bin/bash
# Fix accessibility issues in Svelte files

# Fix 1: ConfigWizard.svelte line 642 (label)
# Manual fix needed - associate label with input

# Fix 2: ConfigWizard.svelte line 901 (textarea)
sed -i 's/<textarea \([^>]*\)\/>/     <textarea \1><\/textarea>/g' desktop/src/lib/ConfigWizard.svelte

# Fix 3: SessionList.svelte line 219 (button)
# Manual fix needed - add aria-label

# Fix 4: ContextEditModal.svelte line 187 (label)
# Manual fix needed - associate label with control

# Fix 5: KGSearchModal.svelte line 708 (div keydown)
# Manual fix needed - add role and tabindex
```

### Step 4: Clean package.json Dependencies
```bash
cd desktop
# Remove duplicates from optionalDependencies that are in devDependencies
jq 'del(.optionalDependencies["@tauri-apps/cli"],
        .optionalDependencies["@testing-library/jest-dom"],
        .optionalDependencies["@testing-library/svelte"],
        .optionalDependencies["dotenv"],
        .optionalDependencies["jsdom"],
        .optionalDependencies["sass"],
        .optionalDependencies["selenium-webdriver"],
        .optionalDependencies["svelte-typeahead"])' package.json > package.json.tmp
mv package.json.tmp package.json
```

### Step 5: Add Vite Build Flag for Non-Fatal Warnings
In `desktop/vite.config.ts` or `desktop/package.json`:
```json
{
  "scripts": {
    "build": "vite build --logLevel warn"
  }
}
```

Or in workflow:
```yaml
- name: Build frontend
  run: yarn build --logLevel warn
  continue-on-error: false
```

## Testing Strategy

### Local Testing Before Push
```bash
# Test 1: Verify .cargo/config.toml approach
cargo build --release --package terraphim_server

# Test 2: Build Tauri locally
cd desktop
yarn install
yarn build  # Should complete without errors
yarn tauri build  # Full build with signing

# Test 3: Check package.json
yarn install  # Should not show warnings
```

### CI Testing Approach
1. Push fixes to `release/v1.0.0` branch
2. Monitor workflows:
   - Package Release should complete
   - Tauri publish should generate artifacts
3. Download and test artifacts locally

## Implementation Commands

### Quick Fix Script
```bash
#!/bin/bash
set -e

echo "=== Fixing GitHub Actions Issues ==="

# Step 1: Handle .cargo/config.toml
echo "Step 1: Checking .cargo/config.toml..."
if [ ! -f .cargo/config.toml ]; then
  echo "Creating .cargo/config.toml"
  mkdir -p .cargo
  cat > .cargo/config.toml << 'CARGO_EOF'
[profile.release]
panic = "abort"
opt-level = 3
lto = true
codegen-units = 1
CARGO_EOF
fi

# Step 2: Fix package.json duplicates
echo "Step 2: Cleaning package.json..."
cd desktop
npm pkg delete \
  "optionalDependencies.@tauri-apps/cli" \
  "optionalDependencies.@testing-library/jest-dom" \
  "optionalDependencies.@testing-library/svelte" \
  "optionalDependencies.dotenv" \
  "optionalDependencies.jsdom" \
  "optionalDependencies.sass" \
  "optionalDependencies.selenium-webdriver" \
  "optionalDependencies.svelte-typeahead" 2>/dev/null || true
cd ..

# Step 3: Git commit
echo "Step 3: Committing fixes..."
git add .cargo/config.toml desktop/package.json
git commit -m "fix(ci): resolve GitHub Actions workflow failures

- Add .cargo/config.toml for package-release workflow
- Remove duplicate dependencies from package.json
- Fix optionalDependencies collisions with devDependencies

Fixes: Package Release and Tauri build workflows
Related: #286"

# Step 4: Push to release branch
echo "Step 4: Pushing to release/v1.0.0..."
git push origin release/v1.0.0

echo "=== Fix script complete ==="
echo "Monitor workflows at: https://github.com/terraphim/terraphim-ai/actions"
```

## Manual Svelte Fixes (Required)

These need manual code review and fixes:

### File: desktop/src/lib/ConfigWizard.svelte
**Line 642**: Add `for` attribute to label
```diff
-<label>Setting Name</label>
+<label for="setting-input">Setting Name</label>
-<input bind:value={setting} />
+<input id="setting-input" bind:value={setting} />
```

**Line 901**: Fix textarea closing
```diff
-<textarea bind:value={content} />
+<textarea bind:value={content}></textarea>
```

### File: desktop/src/lib/Chat/SessionList.svelte
**Line 219**: Add aria-label
```diff
-<button on:click={deleteSession}>🗑️</button>
+<button on:click={deleteSession} aria-label="Delete session">🗑️</button>
```

### File: desktop/src/lib/Chat/ContextEditModal.svelte
**Line 187**: Associate label
```diff
-<label>Context</label>
+<label for="context-input">Context</label>
-<input bind:value={context} />
+<input id="context-input" bind:value={context} />
```

### File: desktop/src/lib/Search/KGSearchModal.svelte
**Line 708**: Add ARIA role
```diff
-<div on:keydown={handleKey}>...</div>
+<div role="button" tabindex="0" on:keydown={handleKey}>...</div>
```

## Success Criteria

✅ **Package Release Workflow**:
- Builds terraphim_server binary
- Builds terraphim-agent binary
- Creates .deb packages
- Creates Arch packages
- Uploads to GitHub release

✅ **Tauri Publish Workflow**:
- Builds frontend without errors
- Compiles Tauri app for all platforms
- Signs with TAURI_PRIVATE_KEY
- Generates .dmg, .msi, .deb, .AppImage
- Creates latest.json for updater

✅ **CI Workflows**:
- All queued workflows complete successfully
- No blocking errors in logs

## Timeline Estimate

- **Quick fixes** (config + package.json): 10 minutes
- **Svelte accessibility fixes**: 30 minutes
- **Testing and validation**: 20 minutes
- **CI execution time**: 30-45 minutes
- **Total**: ~2 hours to complete release

## Rollback Plan

If fixes don't work:
1. Revert commits on release/v1.0.0
2. Delete v1.0.0 tag: `git push --delete origin v1.0.0`
3. Fix issues on main branch first
4. Cherry-pick to new release/v1.0.1 branch
5. Tag as v1.0.1 instead

## Post-Fix Actions

1. Monitor GitHub Actions: https://github.com/terraphim/terraphim-ai/actions
2. Download artifacts when ready
3. Test signed Tauri apps locally
4. Publish draft release (remove draft status)
5. Update meta-issue #286
6. Announce release

---

**Next Action**: Run the quick fix script, then manually fix Svelte files.
**Owner**: @AlexMikhalev
**Target**: Complete within 2 hours
