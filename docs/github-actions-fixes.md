# GitHub Actions Issues and Proposed Fixes

## 🚨 **Current Issues**

### **1. Permission Denied Errors**
**Error**: `EACCES: permission denied, unlink '/home/alex/actions-runner-2/_work/terraphim-ai/terraphim-ai/target/.rustc_info.json'`

**Root Cause**: GitHub Actions runner doesn't have permission to remove files from target directory between jobs.

**Affected Workflows**:
- `ci-native.yml` (lint-and-format job)
- `ci-optimized.yml` (Earthly CI/CD)

### **2. Cache Key Generation Issues**
**Error**: Cache key generation failing due to workspace structure complexity.

**Root Cause**: Complex workspace dependencies causing inconsistent cache keys.

---

## 🔧 **Proposed Fixes**

### **Fix 1: Target Directory Permissions**

**File**: `.github/workflows/ci-native.yml`

**Current Problematic Code**:
```yaml
lint-and-format:
  runs-on: [self-hosted, linux, x64, repository, terraphim-ai, linux-self-hosted]
  timeout-minutes: 15  # Reduced timeout with faster runner
```

**Proposed Solution**:
```yaml
lint-and-format:
  runs-on: [self-hosted, linux, x64, repository, terraphim-ai, linux-self-hosted]
  timeout-minutes: 15

  # Add cleanup step to prevent permission issues
  steps:
    - name: Checkout code
      uses: actions/checkout@v5

    - name: Clean target directory
      run: |
        rm -rf target || true
        mkdir -p target

    - name: Cache Cargo dependencies
      uses: actions/cache@v4
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ needs.setup.outputs.cache-key }}-cargo-lint-${{ hashFiles('**/Cargo.lock') }}
        restore-keys: |
          ${{ needs.setup.outputs.cache-key }}-cargo-lint-
```

### **Fix 2: Simplified Cache Key Strategy**

**File**: `.github/workflows/ci-native.yml`

**Current Problematic Code**:
```yaml
- name: Generate cache key
  id: cache
  run: |
    echo "key=${{ env.CACHE_KEY }}" >> $GITHUB_OUTPUT
```

**Proposed Solution**:
```yaml
- name: Generate cache key
  id: cache
  run: |
    # Use workspace root Cargo.lock for consistent hashing
    CACHE_KEY="v1-${{ hashFiles('Cargo.lock') }}"
    echo "key=$CACHE_KEY" >> $GITHUB_OUTPUT
```

### **Fix 3: Earthly CI/CD Improvements**

**File**: `.github/workflows/ci-optimized.yml`

**Current Issues**:
- Complex target handling
- Permission errors with target directories

**Proposed Solution**:
```yaml
build-frontend:
  runs-on: ubuntu-latest
  steps:
    - name: Checkout code
      uses: actions/checkout@v5

    - name: Setup Node.js
      uses: actions/setup-node@v4
      with:
        node-version: '20'

    - name: Cache node modules
      uses: actions/cache@v4
      with:
        path: ~/.npm
        key: ${{ runner.os }}-node-${{ hashFiles('**/package-lock.json') }}

    - name: Install dependencies
      run: npm ci

    - name: Build frontend
      run: npm run build:ci

    - name: Upload frontend artifacts
      uses: actions/upload-artifact@v4
      with:
        name: frontend-dist
        path: desktop/dist/
```

---

## 🎯 **Implementation Priority**

### **High Priority (Immediate)**
1. **Fix target directory cleanup** - Add explicit cleanup steps
2. **Simplify cache key generation** - Use consistent hashing strategy
3. **Add error handling** - Graceful degradation for permission issues

### **Medium Priority (Next Sprint)**
1. **Optimize Earthly workflows** - Separate concerns, improve caching
2. **Add workflow status reporting** - Better visibility into CI health
3. **Implement workflow dispatch** - Manual trigger capabilities

---

## 📋 **Testing Strategy**

### **Before Deployment**
1. **Test in fork** - Create test branch to validate fixes
2. **Dry run validation** - Use `gh workflow run` for testing
3. **Incremental rollout** - Merge fixes one at a time

### **Validation Steps**
1. **Cache key consistency** - Verify same inputs produce same outputs
2. **Permission handling** - Test cleanup steps work correctly
3. **Cross-platform compatibility** - Test on Linux runners

---

## 🚀 **Expected Outcomes**

### **Immediate Improvements**
- ✅ Eliminate permission denied errors
- ✅ Consistent cache behavior across runs
- ✅ Faster CI times with better caching
- ✅ More reliable build process

### **Long-term Benefits**
- 📊 Improved CI/CD reliability
- 🔧 Better debugging capabilities
- 📈 Faster development feedback loops
- 💰 Reduced resource waste

---

## 🔄 **Rollback Plan**

If issues arise:
1. **Revert to current working state** - Use `git revert`
2. **Isolate problematic changes** - Test fixes individually
3. **Gradual implementation** - Deploy incrementally
4. **Monitor closely** - Watch for regression patterns

---

## 📞 **Success Metrics**

### **Target KPIs**
- **CI Success Rate**: Target >95% (currently ~70%)
- **Build Time**: Target <10 minutes (currently 15+ minutes)
- **Cache Hit Rate**: Target >80% (currently inconsistent)
- **Permission Errors**: Target 0 (currently 2-3 per run)

### **Measurement Plan**
- Track workflow run times
- Monitor cache effectiveness
- Document error patterns
- Measure developer satisfaction

---

## 🎯 **Next Steps**

1. **Create fix branch**: `fix/ci-permission-and-cache-issues`
2. **Implement changes**: Apply proposed solutions
3. **Test thoroughly**: Validate in isolated environment
4. **Deploy incrementally**: Merge with careful monitoring
5. **Document learnings**: Update CI/CD best practices

This comprehensive plan addresses the root causes of GitHub Actions failures and provides a clear path to reliable CI/CD infrastructure.
