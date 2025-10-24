# CI/Testing Infrastructure Enhancement Plan

## Current State Analysis

### Existing CI Infrastructure
- **GitHub Actions**: Multiple workflows (ci-native.yml, vm-execution-tests.yml, test-matrix.yml)
- **Self-hosted runner**: bigbox with Linux environment
- **Pre-commit hooks**: Comprehensive setup with cargo fmt/clippy, biome, secret detection
- **VM execution testing**: Experimental firecracker-rust integration (gitignored)

### Identified Gaps
1. **No unified test strategy** across different components
2. **Limited VM testing** due to gitignored experimental code
3. **No performance benchmarking** in CI
4. **Missing integration tests** for VM execution API
5. **No security testing automation** beyond basic secret detection

## Enhanced CI/Testing Infrastructure Design

### 1. Unified Test Strategy

#### 1.1 Test Categories

```yaml
# Test hierarchy and priorities
test_categories:
  unit_tests:
    priority: "critical"
    timeout: "5m"
    coverage_target: "80%"
    components:
      - rust_crates
      - javascript_typescript
      - api_endpoints

  integration_tests:
    priority: "high"
    timeout: "15m"
    components:
      - vm_execution_api
      - agent_system
      - llm_proxy_integration

  security_tests:
    priority: "critical"
    timeout: "10m"
    components:
      - vulnerability_scanning
      - penetration_testing
      - dependency_audit

  performance_tests:
    priority: "medium"
    timeout: "20m"
    components:
      - vm_boot_time
      - memory_usage
      - api_response_time

  e2e_tests:
    priority: "high"
    timeout: "30m"
    components:
      - full_workflow_execution
      - multi_agent_scenarios
      - disaster_recovery
```

#### 1.2 Test Matrix Strategy

```yaml
# .github/workflows/test-matrix-enhanced.yml
name: Enhanced Test Matrix

on:
  push:
    branches: [main, develop, agent_system]
  pull_request:
    branches: [main, develop]
  schedule:
    - cron: '0 2 * * *'  # Daily nightly tests

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1
  RUST_LOG: info

jobs:
  determine-scope:
    runs-on: ubuntu-latest
    outputs:
      run-unit: ${{ steps.changes.outputs.unit }}
      run-integration: ${{ steps.changes.outputs.integration }}
      run-security: ${{ steps.changes.outputs.security }}
      run-performance: ${{ steps.changes.outputs.performance }}
      run-e2e: ${{ steps.changes.outputs.e2e }}
    steps:
      - uses: actions/checkout@v4
      - uses: dorny/paths-filter@v3
        id: changes
        with:
          filters: |
            unit:
              - 'crates/**/*.rs'
              - 'desktop/src/**/*.{js,ts,tsx}'
              - 'Cargo.toml'
              - 'Cargo.lock'
            integration:
              - 'crates/terraphim_multi_agent/**'
              - 'scripts/test-*.sh'
              - '.github/workflows/*test*.yml'
            security:
              - '**/security/**'
              - 'crates/terraphim_multi_agent/**'
              - 'scratchpad/firecracker-rust/**'
            performance:
              - 'benchmarks/**'
              - 'firecracker-rust/**'
            e2e:
              - 'examples/**'
              - 'docs/**'
              - 'docker-compose.yml'

  unit-tests:
    needs: determine-scope
    if: needs.determine-scope.outputs.run-unit == 'true'
    strategy:
      matrix:
        component: [rust-crates, javascript-typescript, api-endpoints]
    runs-on: [self-hosted, linux, bigbox]
    timeout-minutes: 10

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Rust
        if: matrix.component == 'rust-crates'
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy

      - name: Setup Node.js
        if: matrix.component == 'javascript-typescript'
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: 'desktop/package-lock.json'

      - name: Run Rust unit tests
        if: matrix.component == 'rust-crates'
        run: |
          cargo fmt --all -- --check
          cargo clippy --workspace --all-targets --all-features -- -D warnings
          cargo test --workspace --lib --bins

      - name: Run JavaScript/TypeScript tests
        if: matrix.component == 'javascript-typescript'
        run: |
          cd desktop
          npm ci
          npm run test
          npm run lint
          npm run type-check

      - name: Run API endpoint tests
        if: matrix.component == 'api-endpoints'
        run: |
          # Start test server
          cargo run --bin terraphim_server &
          SERVER_PID=$!
          sleep 10

          # Run API tests
          cargo test -p terraphim_multi_agent --test api_tests

          # Cleanup
          kill $SERVER_PID || true

  integration-tests:
    needs: determine-scope
    if: needs.determine-scope.outputs.run-integration == 'true'
    runs-on: [self-hosted, linux, bigbox]
    timeout-minutes: 20

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup environment
        run: |
          source ~/.cargo/env
          mkdir -p test-data/integration

      - name: Test VM Execution API
        run: |
          if [ -d "scratchpad/firecracker-rust/fcctl-web" ]; then
            cd scratchpad/firecracker-rust/fcctl-web
            cargo test --test integration_tests -- --nocapture
          else
            echo "⚠️ VM execution tests skipped (firecracker-rust not present)"
          fi

      - name: Test Agent System Integration
        run: |
          cargo test -p terraphim_multi_agent --test integration_tests

      - name: Test LLM Proxy Integration
        run: |
          # Mock LLM proxy for testing
          export LLM_PROXY_URL="http://localhost:8081"
          cargo test -p terraphim_multi_agent --test llm_proxy_integration

  security-tests:
    needs: determine-scope
    if: needs.determine-scope.outputs.run-security == 'true'
    runs-on: [self-hosted, linux, bigbox]
    timeout-minutes: 15

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Run security audit
        run: |
          cargo audit
          cargo deny check

      - name: Run security tests
        run: |
          cargo test -p terraphim_multi_agent --test security_tests

      - name: Run penetration tests
        run: |
          if [ -f "scripts/security-penetration-test.sh" ]; then
            ./scripts/security-penetration-test.sh
          fi

      - name: Check for secrets
        run: |
          detect-secrets --baseline .secrets.baseline --scan-all

  performance-tests:
    needs: determine-scope
    if: needs.determine-scope.outputs.run-performance == 'true'
    runs-on: [self-hosted, linux, bigbox]
    timeout-minutes: 25

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Run performance benchmarks
        run: |
          if [ -d "benchmarks" ]; then
            cd benchmarks
            cargo bench --all
          fi

      - name: Test VM performance
        run: |
          if [ -d "scratchpad/firecracker-rust" ]; then
            ./scripts/test-vm-performance.sh
          fi

      - name: Generate performance report
        run: |
          python3 scripts/generate-performance-report.py

  e2e-tests:
    needs: [unit-tests, integration-tests, security-tests]
    if: needs.determine-scope.outputs.run-e2e == 'true'
    runs-on: [self-hosted, linux, bigbox]
    timeout-minutes: 35

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup test environment
        run: |
          docker-compose -f docker-compose.test.yml up -d
          sleep 30

      - name: Run E2E tests
        run: |
          cargo test -p terraphim_multi_agent --test e2e_tests

      - name: Test complete workflows
        run: |
          ./scripts/test-complete-workflows.sh

      - name: Cleanup
        run: |
          docker-compose -f docker-compose.test.yml down -v
```

### 2. Enhanced Pre-commit Hooks

#### 2.1 Smart Pre-commit Configuration

```yaml
# .pre-commit-config-enhanced.yaml
default_language_version:
  python: python3.9
  rust: 1.70.0

repos:
  # Fast checks (always run)
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v6.0.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
      - id: check-yaml
      - id: check-toml
      - id: check-json
      - id: check-case-conflict
      - id: check-merge-conflict
      - id: check-added-large-files
        args: ['--maxkb=1000']

  # Rust formatting and linting
  - repo: local
    hooks:
      - id: cargo-fmt
        name: Cargo format
        entry: cargo fmt --all
        language: system
        types: [rust]
        pass_filenames: false

      - id: cargo-clippy
        name: Cargo clippy
        entry: bash -c 'cargo clippy --workspace --all-targets --all-features -- -D warnings || (echo "⚠️ Clippy failed - run \"cargo clippy --fix\" to auto-fix" && exit 1)'
        language: system
        types: [rust]
        pass_filenames: false

  # JavaScript/TypeScript
  - repo: local
    hooks:
      - id: biome-check
        name: Biome check
        entry: bash -c 'cd desktop && npx @biomejs/biome check --no-errors-on-unmatched || (echo "⚠️ Biome failed - run \"npm run lint:fix\" to auto-fix" && exit 1)'
        language: system
        files: 'desktop/.*\.(js|ts|tsx|jsx|json)$'

  # Security scanning
  - repo: https://github.com/Yelp/detect-secrets
    rev: v1.5.0
    hooks:
      - id: detect-secrets
        args: ['--baseline', '.secrets.baseline']
        exclude: |
          (?x)^(
            .*\.rs$|
            .*\.js$|
            .*\.ts$|
            tests/.*|
            target/.*|
            node_modules/.*
          )$

  # Custom security checks
  - repo: local
    hooks:
      - id: security-pattern-scan
        name: Security pattern scan
        entry: python3 scripts/security-pattern-scan.py
        language: system
        files: '\.(rs|js|ts)$'

      - id: vm-execution-validation
        name: VM execution validation
        entry: bash scripts/validate-vm-execution-changes.sh
        language: system
        files: 'scratchpad/firecracker-rust/.*'

  # Performance checks
  - repo: local
    hooks:
      - id: performance-regression-test
        name: Performance regression test
        entry: python3 scripts/performance-regression-check.py
        language: system
        files: '(benchmarks/.*|scratchpad/firecracker-rust/.*)'
        pass_filenames: false
        stages: [manual]

  # Documentation checks
  - repo: local
    hooks:
      - id: markdown-lint
        name: Markdown lint
        entry: markdownlint
        language: system
        files: '\.md$'

      - id: doc-link-check
        name: Documentation link check
        entry: bash scripts/check-doc-links.sh
        language: system
        files: '\.md$'
        pass_filenames: false
        stages: [manual]
```

#### 2.2 Custom Security Scripts

```python
#!/usr/bin/env python3
# scripts/security-pattern-scan.py

import re
import sys
import os
from pathlib import Path

# Security patterns to detect
SECURITY_PATTERNS = {
    'hardcoded_secret': [
        r'password\s*=\s*["\'][^"\']+["\']',
        r'api_key\s*=\s*["\'][^"\']+["\']',
        r'secret\s*=\s*["\'][^"\']+["\']',
        r'token\s*=\s*["\'][^"\']+["\']',
    ],
    'sql_injection': [
        r'format!\s*["\'].*\{.*\}.*["\'].*SELECT',
        r'execute!\s*["\'].*\{.*\}.*["\']',
        r'query!\s*["\'].*\{.*\}.*["\']',
    ],
    'command_injection': [
        r'Command::new\(".*"\)\.arg\(&format!\)',
        r'std::process::Command::new\(".*"\)\.arg\(&format!\)',
        r'exec\(".*\{.*\}.*"\)',
    ],
    'unsafe_deserialization': [
        r'serde_json::from_str.*\.unwrap\(\)',
        r'bincode::deserialize.*\.unwrap\(\)',
        r'toml::from_str.*\.unwrap\(\)',
    ],
    'debug_code': [
        r'println!\s*\(',
        r'dbg!\s*\(',
        r'eprintln!\s*\(',
        r'console\.log\s*\(',
    ]
}

def scan_file(file_path):
    """Scan a single file for security patterns"""
    issues = []

    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
            lines = content.split('\n')
    except UnicodeDecodeError:
        return issues  # Skip binary files

    for category, patterns in SECURITY_PATTERNS.items():
        for pattern in patterns:
            for match in re.finditer(pattern, content, re.IGNORECASE):
                line_num = content[:match.start()].count('\n') + 1
                line_content = lines[line_num - 1].strip()

                issues.append({
                    'file': str(file_path),
                    'line': line_num,
                    'category': category,
                    'pattern': pattern,
                    'content': line_content,
                })

    return issues

def main():
    """Main entry point"""
    if len(sys.argv) != 2:
        print("Usage: python3 security-pattern-scan.py <file>")
        sys.exit(1)

    file_path = Path(sys.argv[1])
    if not file_path.exists():
        print(f"File not found: {file_path}")
        sys.exit(1)

    issues = scan_file(file_path)

    if issues:
        print(f"🚨 Security issues found in {file_path}:")
        for issue in issues:
            print(f"  Line {issue['line']}: {issue['category']}")
            print(f"    Pattern: {issue['pattern']}")
            print(f"    Content: {issue['content']}")
            print()
        sys.exit(1)
    else:
        print(f"✅ No security issues found in {file_path}")
        sys.exit(0)

if __name__ == "__main__":
    main()
```

### 3. VM Execution Testing Infrastructure

#### 3.1 VM Test Environment Setup

```bash
#!/bin/bash
# scripts/setup-vm-test-env.sh

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[SETUP]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running on Linux
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    error "VM execution testing requires Linux (Firecracker limitation)"
    exit 1
fi

# Check if running as root (required for Firecracker)
if [[ $EUID -ne 0 ]]; then
    warn "Some VM operations require root privileges"
    warn "Consider running with sudo for full functionality"
fi

# Install Firecracker if not present
if ! command -v firecracker &> /dev/null; then
    log "Installing Firecracker..."

    # Download latest Firecracker binary
    FIRECRACKER_VERSION=$(curl -s https://api.github.com/repos/firecracker-microvm/firecracker/releases/latest | grep tag_name | cut -d '"' -f 4)
    ARCH=$(uname -m)

    if [[ "$ARCH" == "x86_64" ]]; then
        FC_ARCH="x86_64"
    else
        error "Unsupported architecture: $ARCH"
        exit 1
    fi

    wget "https://github.com/firecracker-microvm/firecracker/releases/download/${FIRECRACKER_VERSION}/firecracker-${FIRECRACKER_VERSION}-${FC_ARCH}.tgz"
    tar -xzf "firecracker-${FIRECRACKER_VERSION}-${FC_ARCH}.tgz"
    sudo mv release-v*/firecracker-${FC_ARCH}-v*/firecracker /usr/local/bin/
    sudo chmod +x /usr/local/bin/firecracker
    rm -rf firecracker-*

    log "Firecracker installed successfully"
else
    log "Firecracker already installed"
fi

# Setup network for VMs
log "Setting up VM network..."
sudo ip link add name fcbr0 type bridge
sudo ip addr add 172.26.0.1/24 dev fcbr0
sudo ip link set fcbr0 up

# Enable IP forwarding
sudo sysctl -w net.ipv4.ip_forward=1

# Setup NAT for VM internet access
sudo iptables -t nat -A POSTROUTING -s 172.26.0.0/24 -j MASQUERADE
sudo iptables -A FORWARD -s 172.26.0.0/24 -j ACCEPT

# Create test directories
mkdir -p test-data/vms
mkdir -p test-data/snapshots
mkdir -p test-data/logs

# Download test root filesystem
if [[ ! -f "test-data/rootfs.ext4" ]]; then
    log "Downloading test root filesystem..."
    wget -O test-data/rootfs.ext4 https://s3.amazonaws.com/spec.ccfc.min/img/ubuntu/jammy/20231010/uvm-rootfs.ext4
fi

# Set proper permissions
sudo chown -R $USER:$USER test-data/
chmod 755 test-data/

log "VM test environment setup complete!"
log "You can now run: ./scripts/test-vm-execution.sh"
```

#### 3.2 Comprehensive VM Test Suite

```bash
#!/bin/bash
# scripts/test-vm-execution-comprehensive.sh

set -euo pipefail

# Test configuration
FCCTL_WEB_URL="http://localhost:8080"
TEST_TIMEOUT=1200  # 20 minutes
PARALLEL_JOBS=2
VM_POOL_SIZE=5

# Test scenarios
declare -a TEST_SCENARIOS=(
    "basic_execution"
    "concurrent_execution"
    "snapshot_rollback"
    "resource_limits"
    "security_isolation"
    "performance_benchmarks"
    "error_handling"
    "network_isolation"
)

run_test_scenario() {
    local scenario=$1
    local test_file="test-scenarios/${scenario}.sh"

    if [[ -f "$test_file" ]]; then
        echo "🧪 Running scenario: $scenario"
        bash "$test_file" || {
            echo "❌ Scenario $scenario failed"
            return 1
        }
        echo "✅ Scenario $scenario passed"
    else
        echo "⚠️ Test scenario file not found: $test_file"
        return 0
    fi
}

# Main test execution
main() {
    echo "🚀 Starting comprehensive VM execution tests"
    echo "Timeout: ${TEST_TIMEOUT}s"
    echo "Parallel jobs: $PARALLEL_JOBS"
    echo ""

    # Setup test environment
    ./scripts/setup-vm-test-env.sh

    # Start fcctl-web if not running
    if ! curl -s "$FCCTL_WEB_URL/health" > /dev/null; then
        echo "🔧 Starting fcctl-web..."
        cd scratchpad/firecracker-rust/fcctl-web
        cargo run -- --host 0.0.0.0 --port 8080 &
        FCCTL_PID=$!
        cd - > /dev/null

        # Wait for startup
        for i in {1..30}; do
            if curl -s "$FCCTL_WEB_URL/health" > /dev/null; then
                echo "✅ fcctl-web is ready"
                break
            fi
            sleep 1
        done
    fi

    # Run test scenarios
    local failed_scenarios=()

    for scenario in "${TEST_SCENARIOS[@]}"; do
        if ! run_test_scenario "$scenario"; then
            failed_scenarios+=("$scenario")
        fi
    done

    # Cleanup
    if [[ -n "${FCCTL_PID:-}" ]]; then
        kill $FCCTL_PID || true
    fi

    # Report results
    echo ""
    echo "📊 Test Results Summary"
    echo "======================"

    if [[ ${#failed_scenarios[@]} -eq 0 ]]; then
        echo "🎉 All test scenarios passed!"
        exit 0
    else
        echo "❌ Failed scenarios:"
        for scenario in "${failed_scenarios[@]}"; do
            echo "  - $scenario"
        done
        exit 1
    fi
}

main "$@"
```

### 4. Performance Monitoring Integration

#### 4.1 Performance Benchmarking

```yaml
# .github/workflows/performance-benchmarks.yml
name: Performance Benchmarks

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]
  schedule:
    - cron: '0 3 * * *'  # Daily at 3 AM

jobs:
  vm-performance:
    runs-on: [self-hosted, linux, bigbox]
    timeout-minutes: 30

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup benchmark environment
        run: |
          ./scripts/setup-vm-test-env.sh
          mkdir -p benchmark-results

      - name: Run VM boot time benchmarks
        run: |
          cd benchmarks
          cargo run --bin vm-boot-time --release > ../benchmark-results/vm-boot-time.json

      - name: Run execution performance benchmarks
        run: |
          cd benchmarks
          cargo run --bin execution-performance --release > ../benchmark-results/execution-performance.json

      - name: Run memory usage benchmarks
        run: |
          cd benchmarks
          cargo run --bin memory-usage --release > ../benchmark-results/memory-usage.json

      - name: Generate performance report
        run: |
          python3 scripts/generate-performance-report.py \
            --input benchmark-results/ \
            --output benchmark-results/report.html \
            --baseline benchmark-results/baseline.json

      - name: Upload benchmark results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: benchmark-results/

      - name: Comment PR with performance changes
        if: github.event_name == 'pull_request'
        uses: actions/github-script@v7
        with:
          script: |
            const fs = require('fs');
            const report = fs.readFileSync('benchmark-results/report.html', 'utf8');

            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: `## 📊 Performance Benchmark Results\n\n${report}`
            });
```

#### 4.2 Performance Regression Detection

```python
#!/usr/bin/env python3
# scripts/performance-regression-check.py

import json
import sys
import os
from pathlib import Path
import statistics

def load_benchmark_results(file_path):
    """Load benchmark results from JSON file"""
    with open(file_path, 'r') as f:
        return json.load(f)

def calculate_regression(current, baseline, threshold=0.1):
    """Calculate performance regression"""
    regressions = []

    for metric in current:
        if metric in baseline:
            current_val = current[metric]
            baseline_val = baseline[metric]

            if baseline_val == 0:
                continue

            change = (current_val - baseline_val) / baseline_val

            if change > threshold:  # Performance got worse
                regressions.append({
                    'metric': metric,
                    'current': current_val,
                    'baseline': baseline_val,
                    'change_percent': change * 100
                })

    return regressions

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 performance-regression-check.py <results-file> [baseline-file]")
        sys.exit(1)

    results_file = sys.argv[1]
    baseline_file = sys.argv[2] if len(sys.argv) > 2 else "benchmark-results/baseline.json"

    if not os.path.exists(results_file):
        print(f"Results file not found: {results_file}")
        sys.exit(0)  # Not an error, just no results to check

    current_results = load_benchmark_results(results_file)

    if not os.path.exists(baseline_file):
        print(f"Baseline file not found: {baseline_file}")
        print("Creating baseline from current results...")
        os.makedirs(os.path.dirname(baseline_file), exist_ok=True)
        with open(baseline_file, 'w') as f:
            json.dump(current_results, f, indent=2)
        sys.exit(0)

    baseline_results = load_benchmark_results(baseline_file)
    regressions = calculate_regression(current_results, baseline_results)

    if regressions:
        print("🚨 Performance regressions detected:")
        for regression in regressions:
            print(f"  {regression['metric']}: {regression['change_percent']:.1f}% "
                  f"({regression['baseline']:.2f} → {regression['current']:.2f})")
        sys.exit(1)
    else:
        print("✅ No performance regressions detected")
        sys.exit(0)

if __name__ == "__main__":
    main()
```

### 5. Security Testing Automation

#### 5.1 Automated Security Scanning

```yaml
# .github/workflows/security-scanning.yml
name: Security Scanning

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]
  schedule:
    - cron: '0 4 * * *'  # Daily at 4 AM

jobs:
  vulnerability-scan:
    runs-on: [self-hosted, linux, bigbox]
    timeout-minutes: 15

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Run Rust security audit
        run: |
          cargo install cargo-audit
          cargo audit

      - name: Run dependency check
        run: |
          cargo install cargo-deny
          cargo deny check

      - name: Run security tests
        run: |
          cargo test -p terraphim_multi_agent --test security_tests

      - name: Run penetration tests
        run: |
          if [ -f "scripts/security-penetration-test.sh" ]; then
            sudo ./scripts/security-penetration-test.sh
          fi

      - name: Scan for secrets
        run: |
          detect-secrets --baseline .secrets.baseline --scan-all

      - name: Generate security report
        run: |
          python3 scripts/generate-security-report.py > security-report.html

      - name: Upload security report
        uses: actions/upload-artifact@v4
        with:
          name: security-report
          path: security-report.html
```

### 6. Implementation Timeline

#### Phase 1: Foundation (Week 1)
- [ ] Set up enhanced test matrix workflow
- [ ] Implement smart pre-commit hooks
- [ ] Create VM test environment setup script
- [ ] Add basic performance benchmarking

#### Phase 2: Integration (Week 2)
- [ ] Implement comprehensive VM test suite
- [ ] Add security scanning automation
- [ ] Create performance regression detection
- [ ] Set up test result reporting

#### Phase 3: Optimization (Week 3)
- [ ] Optimize test execution parallelization
- [ ] Add caching for faster builds
- [ ] Implement test result analytics
- [ ] Create performance dashboards

#### Phase 4: Production (Week 4)
- [ ] Full integration with existing CI
- [ ] Documentation and runbooks
- [ ] Monitoring and alerting setup
- [ ] Team training and onboarding

### 7. Success Metrics

#### Test Coverage
- Unit test coverage: >80%
- Integration test coverage: >70%
- Security test coverage: 100% for critical paths

#### Performance
- CI execution time: <30 minutes for full suite
- VM boot time: <2 seconds (95th percentile)
- Test flakiness: <1%

#### Security
- Zero critical vulnerabilities in production
- Security scan coverage: 100%
- Secret detection: 0 false negatives

This enhanced CI/testing infrastructure provides comprehensive coverage for the terraphim-ai project, with special focus on VM execution security and performance.
