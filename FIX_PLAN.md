# Terraphim Build & Interactive Mode Fix Plan

## Executive Summary

This plan addresses the critical issues discovered during the build script and artifact review:

1. **CRITICAL**: Interactive mode (`terraphim-agent interactive`) crashes with "Device not configured (os error 6)" in non-interactive environments
2. **HIGH**: Build scripts reference non-existent packages (`terraphim_tui`)
3. **MEDIUM**: Test scripts reference incorrect package names
4. **LOW**: Build helper script expects missing binary

## Phase 1: Fix Interactive Mode Crash (Priority: CRITICAL)

### Target Behavior

The `interactive` command should:
- ✅ Work correctly in interactive terminal sessions
- ✅ Provide clear error message when run in non-interactive environments (scripts, CI/CD, piped commands)
- ✅ Suggest alternative: use `repl` mode for non-interactive use
- ✅ Not crash with cryptic "Device not configured" errors

### Implementation Plan

#### Step 1.1: Add Terminal Detection Dependency

**File**: `crates/terraphim_agent/Cargo.toml`

**Current** (lines 54-58):
```toml
# REPL dependencies - only compiled with features
rustyline = { version = "17.0", optional = true }
colored = { version = "3.0", optional = true }
comfy-table = { version = "7.0", optional = true }
indicatif = { version = "0.18", optional = true }
dirs = { version = "5.0", optional = true }
```

**Change to**:
```toml
# REPL dependencies - only compiled with features
rustyline = { version = "17.0", optional = true }
colored = { version = "3.0", optional = true }
comfy-table = { version = "7.0", optional = true }
indicatif = { version = "0.18", optional = true }
dirs = { version = "5.0", optional = true }
atty = { version = "0.2", optional = true }
```

**Add to features** (line 16-23):
```toml
default = []
repl = ["dep:rustyline", "dep:colored", "dep:comfy-table", "dep:indicatif", "dep:dirs"]
repl-full = ["repl", "repl-chat", "repl-mcp", "repl-file", "repl-custom", "repl-web", "repl-sessions"]
repl-interactive = ["repl", "dep:atty"]  # Add this feature
```

#### Step 1.2: Add Terminal Detection in Main Function

**File**: `crates/terraphim_agent/src/main.rs`

**Current** (lines 273-304):
```rust
fn main() -> Result<()> {
    // tokio runtime for subcommands; interactive mode runs sync loop and spawns async tasks if needed
    let rt = Runtime::new()?;
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Interactive) | None => {
            if cli.server {
                run_tui_server_mode(&cli.server_url, cli.transparent)
            } else {
                rt.block_on(run_tui_offline_mode(cli.transparent))
            }
        }
        // ... rest of function
    }
}
```

**Change to**:
```rust
fn main() -> Result<()> {
    // tokio runtime for subcommands; interactive mode runs sync loop and spawns async tasks if needed
    let rt = Runtime::new()?;
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Interactive) | None => {
            // Check if we're in a TTY for interactive mode
            #[cfg(feature = "repl-interactive")]
            {
                use atty::Stream;
                if !atty::is(Stream::Stdout) {
                    eprintln!("Error: Interactive mode requires a terminal.");
                    eprintln!("For non-interactive use, try:");
                    eprintln!("  1. REPL mode: terraphim-agent repl");
                    eprintln!("  2. Command mode: terraphim-agent search \"query\"");
                    eprintln!("  3. CLI tool: terraphim-cli search \"query\"");
                    std::process::exit(1);
                }
            }
            #[cfg(not(feature = "repl-interactive"))]
            {
                // Fallback: try to detect non-TTY environment
                use std::io::IsTerminal;
                if !std::io::stdout().is_terminal() {
                    eprintln!("Error: Interactive mode requires a terminal.");
                    eprintln!("For non-interactive use, try:");
                    eprintln!("  1. REPL mode: terraphim-agent repl");
                    eprintln!("  2. Command mode: terraphim-agent search \"query\"");
                    eprintln!("  3. CLI tool: terraphim-cli search \"query\"");
                    std::process::exit(1);
                }
            }

            if cli.server {
                run_tui_server_mode(&cli.server_url, cli.transparent)
            } else {
                rt.block_on(run_tui_offline_mode(cli.transparent))
            }
        }
        // ... rest of function
    }
}
```

#### Step 1.3: Update Default Features

**File**: `crates/terraphim_agent/Cargo.toml`

**Current** (line 15-17):
```toml
[features]
default = []
```

**Change to**:
```toml
[features]
default = ["repl-interactive"]
```

This ensures terminal detection is always available when building the binary.

#### Step 1.4: Add Graceful Error Handling to TUI Functions

**File**: `crates/terraphim_agent/src/main.rs`

**Current** (lines 1192-1210):
```rust
fn run_tui(transparent: bool) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = ui_loop(&mut terminal, transparent);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}
```

**Change to**:
```rust
fn run_tui(transparent: bool) -> Result<()> {
    // Attempt to set up terminal for TUI
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);

    // Try to enter raw mode and alternate screen
    // These operations can fail in non-interactive environments
    match enable_raw_mode() {
        Ok(()) => {
            // Successfully entered raw mode, proceed with TUI setup
            let mut stdout = io::stdout();
            if let Err(e) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
                // Clean up raw mode before returning error
                let _ = disable_raw_mode();
                return Err(anyhow::anyhow!(
                    "Failed to initialize terminal for interactive mode: {}. \
                     Try using 'repl' mode instead: terraphim-agent repl",
                    e
                ));
            }

            let mut terminal = match Terminal::new(backend) {
                Ok(t) => t,
                Err(e) => {
                    // Clean up before returning
                    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
                    let _ = disable_raw_mode();
                    return Err(anyhow::anyhow!(
                        "Failed to create terminal: {}. \
                         Try using 'repl' mode instead: terraphim-agent repl",
                        e
                    ));
                }
            };

            let res = ui_loop(&mut terminal, transparent);

            // Always clean up terminal state
            let _ = disable_raw_mode();
            let _ = execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            );
            let _ = terminal.show_cursor();

            res
        }
        Err(e) => {
            // Failed to enter raw mode - not a TTY
            Err(anyhow::anyhow!(
                "Terminal does not support raw mode (not a TTY?). \
                 Interactive mode requires a terminal. \
                 Try using 'repl' mode instead: terraphim-agent repl. \
                 Error: {}",
                e
            ))
        }
    }
}
```

---

## Phase 2: Fix Build Scripts (Priority: HIGH)

### Step 2.1: Fix `build_multiplatform.sh`

**File**: `build_multiplatform.sh`

**Issues**:
- Lines 20-23: References `terraphim_tui` package which doesn't exist
- Lines 33-39: Copies `terraphim-tui` binary which doesn't exist
- Should build `terraphim_agent` and `terraphim-cli` instead

**Current** (lines 19-40):
```bash
# Build all three binaries
cargo build --release --target $TARGET \
    --package terraphim_server \
    --package terraphim_mcp_server \
    --package terraphim_tui 2>/dev/null

if [ $? -eq 0 ]; then
    echo "✅ Built successfully for $TARGET"

    # Create target directory
    mkdir -p releases/v1.0.2/$OS/$ARCH

    # Copy binaries with proper naming
    if [ "$OS" = "windows" ]; then
        cp target/$TARGET/release/terraphim_server.exe releases/v1.0.2/$OS/$ARCH/ 2>/dev/null || cp target/$TARGET/release/terraphim_server releases/v1.0.2/$OS/$ARCH/terraphim_server.exe 2>/dev/null
        cp target/$TARGET/release/terraphim_mcp_server.exe releases/v1.0.2/$OS/$ARCH/ 2>/dev/null || cp target/$TARGET/release/terraphim_mcp_server releases/v1.0.2/$OS/$ARCH/terraphim_mcp_server.exe 2>/dev/null
        cp target/$TARGET/release/terraphim-tui.exe releases/v1.0.2/$OS/$ARCH/ 2>/dev/null || cp target/$TARGET/release/terraphim-tui releases/v1.0.2/$OS/$ARCH/terraphim-tui.exe 2>/dev/null
    else
        cp target/$TARGET/release/terraphim_server releases/v1.0.2/$OS/$ARCH/ 2>/dev/null
        cp target/$TARGET/release/terraphim_mcp_server releases/v1.0.2/$OS/$ARCH/ 2>/dev/null
        cp target/$TARGET/release/terraphim-tui releases/v1.0.2/$OS/$ARCH/ 2>/dev/null
    fi
```

**Change to**:
```bash
# Build all three binaries
cargo build --release --target $TARGET \
    --package terraphim_server \
    --package terraphim_mcp_server \
    --package terraphim_agent \
    --package terraphim-cli 2>/dev/null

if [ $? -eq 0 ]; then
    echo "✅ Built successfully for $TARGET"

    # Create target directory
    mkdir -p releases/v1.0.2/$OS/$ARCH

    # Copy binaries with proper naming
    if [ "$OS" = "windows" ]; then
        cp target/$TARGET/release/terraphim_server.exe releases/v1.0.2/$OS/$ARCH/ 2>/dev/null || cp target/$TARGET/release/terraphim_server releases/v1.0.2/$OS/$ARCH/terraphim_server.exe 2>/dev/null
        cp target/$TARGET/release/terraphim_mcp_server.exe releases/v1.0.2/$OS/$ARCH/ 2>/dev/null || cp target/$TARGET/release/terraphim_mcp_server releases/v1.0.2/$OS/$ARCH/terraphim_mcp_server.exe 2>/dev/null
        cp target/$TARGET/release/terraphim-agent.exe releases/v1.0.2/$OS/$ARCH/ 2>/dev/null || cp target/$TARGET/release/terraphim-agent releases/v1.0.2/$OS/$ARCH/terraphim-agent.exe 2>/dev/null
        cp target/$TARGET/release/terraphim-cli.exe releases/v1.0.2/$OS/$ARCH/ 2>/dev/null || cp target/$TARGET/release/terraphim-cli releases/v1.0.2/$OS/$ARCH/terraphim-cli.exe 2>/dev/null
    else
        cp target/$TARGET/release/terraphim_server releases/v1.0.2/$OS/$ARCH/ 2>/dev/null
        cp target/$TARGET/release/terraphim_mcp_server releases/v1.0.2/$OS/$ARCH/ 2>/dev/null
        cp target/$TARGET/release/terraphim-agent releases/v1.0.2/$OS/$ARCH/ 2>/dev/null
        cp target/$TARGET/release/terraphim-cli releases/v1.0.2/$OS/$ARCH/ 2>/dev/null
    fi
```

**Also update lines 63-66** (native build section):
```bash
# Current:
cargo build --release \
    --package terraphim_server \
    --package terraphim_mcp_server \
    --package terraphim_tui

# Change to:
cargo build --release \
    --package terraphim_server \
    --package terraphim_mcp_server \
    --package terraphim_agent \
    --package terraphim-cli
```

**Also update lines 72-76** (macOS ARM64 section):
```bash
# Current:
cp target/release/terraphim_server releases/v1.0.2/macos/aarch64/
cp target/release/terraphim_mcp_server releases/v1.0.2/macos/aarch64/
cp target/release/terraphim-tui releases/v1.0.2/macos/aarch64/

# Change to:
cp target/release/terraphim_server releases/v1.0.2/macos/aarch64/
cp target/release/terraphim_mcp_server releases/v1.0.2/macos/aarch64/
cp target/release/terraphim-agent releases/v1.0.2/macos/aarch64/
cp target/release/terraphim-cli releases/v1.0.2/macos/aarch64/
```

**Also update lines 78-82** (macOS x86_64 section):
```bash
# Current:
cp target/release/terraphim_server releases/v1.0.2/macos/x86_64/
cp target/release/terraphim_mcp_server releases/v1.0.2/macos/x86_64/
cp target/release/terraphim-tui releases/v1.0.2/macos/x86_64/

# Change to:
cp target/release/terraphim_server releases/v1.0.2/macos/x86_64/
cp target/release/terraphim_mcp_server releases/v1.0.2/macos/x86_64/
cp target/release/terraphim-agent releases/v1.0.2/macos/x86_64/
cp target/release/terraphim-cli releases/v1.0.2/macos/x86_64/
```

**Also update lines 124-137** (universal binary creation):
```bash
# Current:
lipo -create \
    releases/v1.0.2/macos/x86_64/terraphim_server \
    releases/v1.0.2/macos/aarch64/terraphim_server \
    -output releases/v1.0.2/macos/universal/terraphim_server
lipo -create \
    releases/v1.0.2/macos/x86_64/terraphim_mcp_server \
    releases/v1.0.2/macos/aarch64/terraphim_mcp_server \
    -output releases/v1.0.2/macos/universal/terraphim_mcp_server
lipo -create \
    releases/v1.0.2/macos/x86_64/terraphim-tui \
    releases/v1.0.2/macos/aarch64/terraphim-tui \
    -output releases/v1.0.2/macos/universal/terraphim-tui

# Change to:
lipo -create \
    releases/v1.0.2/macos/x86_64/terraphim_server \
    releases/v1.0.2/macos/aarch64/terraphim_server \
    -output releases/v1.0.2/macos/universal/terraphim_server
lipo -create \
    releases/v1.0.2/macos/x86_64/terraphim_mcp_server \
    releases/v1.0.2/macos/aarch64/terraphim_mcp_server \
    -output releases/v1.0.2/macos/universal/terraphim_mcp_server
lipo -create \
    releases/v1.0.2/macos/x86_64/terraphim-agent \
    releases/v1.0.2/macos/aarch64/terraphim-agent \
    -output releases/v1.0.2/macos/universal/terraphim-agent
lipo -create \
    releases/v1.0.2/macos/x86_64/terraphim-cli \
    releases/v1.0.2/macos/aarch64/terraphim-cli \
    -output releases/v1.0.2/macos/universal/terraphim-cli
```

### Step 2.2: Fix `test_tui_comprehensive.sh`

**File**: `test_tui_comprehensive.sh`

**Issues**:
- Line 22: References `terraphim_tui` package
- Line 80: Uses incorrect package name for running tests

**Current** (line 22):
```bash
TUI_PACKAGE="terraphim_tui"
```

**Change to**:
```bash
TUI_PACKAGE="terraphim_agent"
```

**Current** (line 80):
```bash
run_tui_offline() {
    timeout ${TEST_TIMEOUT} cargo run -p ${TUI_PACKAGE} -- "$@" 2>&1
}
```

**Change to**:
```bash
run_tui_offline() {
    timeout ${TEST_TIMEOUT} cargo run -p ${TUI_PACKAGE} --features repl-interactive -- "$@" 2>&1
}
```

**Also update lines 468-491** (unit test runner functions):
```bash
# Current:
run_unit_tests() {
    log_info "Running unit tests"

    if cargo test -p ${TUI_PACKAGE} --test offline_mode_tests; then
        log_success "Offline mode unit tests passed"
    else
        log_error "Offline mode unit tests failed"
        return 1
    fi

    if cargo test -p ${TUI_PACKAGE} --test selected_role_tests; then
        log_success "Selected role unit tests passed"
    else
        log_error "Selected role unit tests failed"
        return 1
    fi

    if cargo test -p ${TUI_PACKAGE} --test persistence_tests; then
        log_success "Persistence unit tests passed"
    else
        log_error "Persistence unit tests failed"
        return 1
    fi
}

# Change to:
run_unit_tests() {
    log_info "Running unit tests"

    if cargo test -p ${TUI_PACKAGE} --features repl-interactive --test offline_mode_tests; then
        log_success "Offline mode unit tests passed"
    else
        log_error "Offline mode unit tests failed"
        return 1
    fi

    if cargo test -p ${TUI_PACKAGE} --features repl-interactive --test selected_role_tests; then
        log_success "Selected role unit tests passed"
    else
        log_error "Selected role unit tests failed"
        return 1
    fi

    if cargo test -p ${TUI_PACKAGE} --features repl-interactive --test persistence_tests; then
        log_success "Persistence unit tests passed"
    else
        log_error "Persistence unit tests failed"
        return 1
    fi
}
```

**Update integration and server tests similarly** (lines 493-514):
```bash
# Change:
if cargo test -p ${TUI_PACKAGE} --test integration_tests; then
# To:
if cargo test -p ${TUI_PACKAGE} --features repl-interactive --test integration_tests; then

# Change:
if cargo test -p ${TUI_PACKAGE} --test server_mode_tests; then
# To:
if cargo test -p ${TUI_PACKAGE} --features repl-interactive --test server_mode_tests; then
```

### Step 2.3: Fix `build_terraphim.sh`

**File**: `scripts/build_terraphim.sh`

**Issue**: Expects `./target/release/terraphim-build-args` binary which doesn't exist

**Current** (lines 22-24):
```bash
# Invoke the build argument manager (Rust tool)
./target/release/terraphim-build-args --config "$CONFIG_FILE" \
    --output "cargo"
```

**Change to**:
```bash
# Check if terraphim-build-args exists
if [ -f "./target/release/terraphim-build-args" ]; then
    # Invoke the build argument manager (Rust tool)
    ./target/release/terraphim-build-args --config "$CONFIG_FILE" \
        --output "cargo"
else
    echo "Warning: terraphim-build-args not found, using default cargo build"
    cargo build --release
fi
```

**Also check for build_config.toml existence** (lines 14-20):
```bash
# Current:
if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "Error: Build configuration file '$CONFIG_FILE' not found!"
    exit 1
fi

# Change to:
if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "Warning: Build configuration file '$CONFIG_FILE' not found, using default build"
    cargo build --release
    echo "Build completed successfully!"
    exit 0
fi
```

---

## Phase 3: Testing Strategy

### Step 3.1: Build Verification

```bash
# Clean build
cargo clean
cargo build --release -p terraphim_agent
cargo build --release -p terraphim-cli

# Verify binaries exist
ls -lh target/release/terraphim-agent
ls -lh target/release/terraphim-cli
```

### Step 3.2: Interactive Mode Tests

```bash
# Test 1: Should show error in non-interactive environment
echo "Test 1: Non-interactive environment"
timeout 3s ./target/release/terraphim-agent interactive
echo "Exit code: $?"

# Test 2: REPL mode should work
echo "Test 2: REPL mode"
echo "quit" | timeout 3s ./target/release/terraphim-agent repl

# Test 3: Command mode should work
echo "Test 3: Command mode"
./target/release/terraphim-agent config show

# Test 4: CLI mode should work
echo "Test 4: CLI mode"
./target/release/terraphim-cli config
```

### Step 3.3: Tmux Validation

```bash
# Start tmux session for testing
tmux new-session -d -s terraphim_test 'echo "Testing interactive mode..." && ./target/release/terraphim-agent interactive'

# Wait for potential crash
sleep 2

# Check session status
tmux list-sessions -F '#{session_name}: #{session_attached} clients'

# Capture output
tmux capture-pane -t terraphim_test -p

# Clean up
tmux kill-session -t terraphim_test
```

### Step 3.4: Build Script Verification

```bash
# Test build script
bash scripts/build_terraphim.sh

# Verify correct binaries were built
ls -lh target/release/ | grep -E "(terraphim|cli)"
```

---

## Risk Review

### Potential Risks

1. **Risk**: Adding `atty` dependency might cause compatibility issues on some platforms
   - **Mitigation**: `atty` is widely used and well-maintained. It provides fallback for non-TTY environments
   - **Fallback**: We also include std::io::IsTerminal as a fallback

2. **Risk**: Changing feature defaults might break existing builds
   - **Mitigation**: We only add optional features, existing builds should continue to work
   - **Verification**: Test both with and without new features

3. **Risk**: Build script changes might break CI/CD pipelines
   - **Mitigation**: Changes are additive and preserve existing functionality
   - **Verification**: Test scripts locally before deployment

4. **Risk**: Error message changes might break scripts expecting specific output
   - **Mitigation**: New error messages are more informative and still return non-zero exit codes
   - **Verification**: Test that exit codes are correct

---

## Implementation Order

1. **Phase 1**: Fix interactive mode crash (critical - blocks users)
2. **Phase 2**: Fix build scripts (high - enables release builds)
3. **Phase 3**: Testing and validation

---

## Files to Modify

| File | Changes | Priority |
|------|---------|----------|
| `crates/terraphim_agent/Cargo.toml` | Add atty dependency, update features | CRITICAL |
| `crates/terraphim_agent/src/main.rs` | Add terminal detection, error handling | CRITICAL |
| `build_multiplatform.sh` | Fix package names, binary names | HIGH |
| `test_tui_comprehensive.sh` | Fix package names, features | MEDIUM |
| `scripts/build_terraphim.sh` | Add fallback handling | LOW |

---

## Completion Criteria

- ✅ `terraphim-agent interactive` shows helpful error in non-interactive environments
- ✅ `terraphim-agent interactive` works in real terminal sessions
- ✅ `terraphim-agent repl` continues to work
- ✅ All build scripts produce correct binaries
- ✅ All tests pass
- ✅ No regressions in existing functionality
