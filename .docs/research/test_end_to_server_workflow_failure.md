# Research Document

## Problem Understanding

The CI test `test_end_to_end_server_workflow` is failing in CI environment due empty roles list.
The The test passes locally but fails in CI.

    - **Root cause**: The test spawns a server with `cargo run -p terraphim_server -- --config terraphim_server/default/terraphim_engineer_config.json`
        but Server reads config from file,        parses output roles list,        expects empty roles list,        assert!(
            !server_roles.is_empty(),
            "Server should have roles available"
        );

**Impact of not solving it?**
CI will continue to fail, blocking merges.
- The pipeline is is flaky
- CI test failures indicates environment configuration issue,- File paths are CI-specific and test config
- CI-specific test fixture
- Potential working directory differences

- CI runner might run tests in a different working directory

- CI environment may not have `docs/src` ( but be accessible
- Test timing is CI machines resources constrained

- Test might need config setup
- The test assumes config is path is accessible
- The test expects relative path `terraphim_server/default/terraphim_engineer_config.json` to work in CI environment.
- CI test should "temp" config" with `--server` command` writes to temp file in CI temp directory ( Then uses absolute paths to avoid CWD issues.

- CI runs `cargo run -p terraphim_server -- --config terraphim_server/default/terraphim_engineer_config.json` but than hard-coding the relative path.

- In the test, we generate a test config with absolute paths to avoid CWD issues.

- **Root cause:** Server uses relative path `terraphim_server/default/terraphim_engineer_config.json`. The config is exists but but to find the.

- **Solution:** Use absolute paths for or create a temp config file with correct paths for for test.

2 - CI-specific test config
- Skip LLM integration tests ( already

- **Technical constraints::
  - CI environment: `cargo run` spawns server which uses relative path
  - `docs/src` path must doesn't exist in CI
  - Tests need access to `docs/src` which not be accessible locally
          - haystack `location` might use a mock location instead of actual path
          - Test expects `docs/src` to directory to to be present for search
- LLM integration tests require LLM service ( optional,- **Raven Tests** should be skipped ( skip LLM tests)
- `docs/src/kg` path doesn't be CI agent ( environment,- tests need LLM API keys which CI environment may be flaky

- **Alternative solutions:**

**Option A: Create test-specific config**
1. Modify `start_test_server()` to use absolute paths for `docs/src/kg`
 ( - Create temp config in `/tmp/terraphim-ai/` directory if it doesn exists
        - Ensure path in `docs/src/kg` if exists
        - Use environment variable `SKIP_kg_fallback` for `TERRaphim-it`
        - Use the fallback config without KG

- **Option B: create a minimal fallback config for use an embedded test config**
        - Use the test_server` command to run it with `--server` command to and run ` --config <config_path> --args` command

```
       .args([
            "roles",
            "list"
        ])
        .env("TERRAPHIM_SERVER_HOSTNAME", format!("127.0.0.1:{}", port}", ())
        .env("RUST_LOG", "warn")

        .env("TERRAPHIM_SERVER_HOSTNAME", format!("127.0.0.1:{}",port}",()))

        // Wait for server to be ready
        for i in {1..30} {
            if (response.status().is_success() {
                println!("Server ready after {} seconds", attempt);
            }
            Ok(response) if response.status().is_success() {
                println!("Server failed to start after {} seconds: attempt", attempt,            }
        }

        // If still not ready after 30 seconds, panic!("Server failed to start after 120 seconds");
    }
}
```

CI log output shows:
```
Server should have roles available: {:?}", server_roles)
✓ Server roles available: ["  Engineer (Engineer)", "  System Operator (operator)", "  Default (Default)"]

✓ Server config loaded: id="Server", selected_role="Terraphim Engineer"
✓ Server config loaded: id="Server", selected_role="Terraphim Engineer"
✓ Server roles available: [] // <-- Empty!
Server_roles:["Engineer", "System Operator", "  Default"] "Default (Default)"]

server.log: Server 1. Server should starting, with absolute path for.
        let mut cmd = Command::new("cargo")
        .args([
            "run",
            "-p",
            "terraphim_server",
            "--",
            "terraphim_server/default/terraphim_engineer_config.json",
        ])
        .env("TERRAPHIM_SERVER_HOSTNAME", format!("127.0.0;1:{}", port}",()))

        // Wait for server to be ready
        for i in 1..30} {
            if server.try_wait().success? {
                return Ok((server, server_url));
            }
        }

        // If still not ready after 30 seconds, return Err(anyhow!("Server failed to start after {} seconds");
    }
}
    println!("Server started on http://localhost:23572");
    
    let server_url = format!("http://localhost:{}", port);
    
    // Generate health endpoint to wait for server ready
    let (mut server, server_url) = server;

    let config_path = PathBuf::canonicalize(config path). If needed to be absolute.
    // canonicalize: terraphim_server/default/terraphim_engineer_config.json
    let config_content = fs::read_to_string(&config_content)?;
    .map<String, Vec>(
        config_content["id"]
        .expect(config_content["selected_role"]
    );
}

let config_content["roles"]. = iter();
let roles = server_response as JSON;
for role in &config_content["roles"]) {
    server_roles.extend(role_names(role)
    .collect(role_names);
    for role in &role_names) {
        server_roles.extend(role_names);
    if server_roles.is_empty() {
        println!("✓ Server roles available: {:?}", server_roles);
        return Ok((server, server_url));
    })
}
```

The CI runs the test spawns a server with `cargo run -p terraphim_server -- --config terraphim_server/default/terraphim_engineer_config.json` and server reads config. parses output roles, and checks that the is not empty.

 it "Server should have roles available");
        assert!(
            !server_roles.is_empty(),
            "Server should have roles available"
        );

    // 3. Fix approach:1: **Use absolute paths for config**
        - The test uses relative path `terraphim_server/default/terraphim_engineer_config.json`
        - CI environment: `docs/src/kg` path might doesn't exist in CI
          - haystack locations point to `docs/src` (test config)
            - `kg` path should use absolute path to avoid CWD issues
          - Use env var to fallback if needed
        - Skip LLM integration tests if CI environment doesn't have LLM
          - skip if CI test is requires LLM features)
- Consider skipping them in CI
- The test should "temp" config approach would be the config approach creates a temp config with absolute paths in CI
- the CI can then apply the approach.

- The this assumptions:

Let me document my findings:

 create the research document.

## Research Document

````# Research Document: test_end_to_end_server_workflow Failure
````# CI/CD Pipeline - Blocking merges

```
**Issue:**
The test `test_end_to_end_server_workflow` spawns a server using `cargo run -p terraphim_server -- --config terraphim_server/default/terraphim_engineer_config.json` (line 24). The config path is but relative, causing issues in CI environment where the config file may not be accessible or the `docs/src` path is doesn't exist.

- **Haystack locations**: The `docs/src` directory may not exist in CI,  - The `kg` path points to to mock data instead of actual files
- **Raven tests**: The L tests should should be skipped in CI

- **Tests needing LLM**: TheL tests are optional - CI runs them without LLM services
- **Docs/src/kg` may not exist in CI**

### Key Insights
1. Test works locally but passes locally with relative paths
2 Fails in CI because
3. CI runs `cargo run -p terraphim_server` which uses a relative path to config file
4. The config path doesn't being on is isn't accessible
   - CI runs from a different working directory ( the CI environment)
   - Tests use `--server` mode which doesn't support LLM
   - Tests expect absolute paths for both test config files

### Recommendations
1. **Option A: Use absolute paths for config** (like `cross_mode_consistency_test.rs` does)
   - Create temp config with absolute paths for the config
   - Modify `integration_tests.rs` to use absolute path approach
2. **Option B: Modify server startup to in `start_test_server()` to:
   - CI runs need to pass an absolute path for config file
   - The tests are more robust against CI environment issues

### Next Steps
1. Create a research document (save as `.docs/research-test_end_to-server-workflow-failure.md`)
2. Load disciplined-design skill
3. **Open questions for**

**Question 1: Does solving this problem energize us to**
**Answer:** YES

**Question 2: Does solving this leverage our unique capabilities?**
**Answer:**Yes (CI test infrastructure)

**Question 3: Does solving this meet a significant, validated need?
**

**Assumptions:**
| Assumption | Basis | Risk if Wrong |
|---|
| Relative path works in CI. `docs/src/kg` doesn't exist in CI,  Config file may be accessible
 | Test might pass locally but fail in CI. |
| **Option A: Use absolute paths** | Recommended
  - Follow the pattern from `cross_mode_consistency_test.rs`
  - Modify `start_test_server()` to to generate temp config with absolute path for CI environment
- Skip tests that expect no LLM features
  - Tests that need LLM should be optional ( `--skip-if-ci` if needed` if (!CI`)

  - `test_end_to_end_server_workflow()` function should `start_test_server()` - it path and config handling need improvement.
  - CI test expects absolute path for config file to not be relative paths in CI.  - **Implementation Plan**: Change `start_test_server()` to use absolute paths. Let me implement that fix. with proper approach.  - Update integration_tests.rs with file: Read current implementation and use absolute paths for config file generation
  - Modify `start_test_server()` to generate temp config with absolute paths
  - Run locally to verify it works
  - commit changes
  - push
  - Trigger CI to run again and verify the fix works.
  - Review any other that was wrong with the fix in CI
  - kill the CI session if needed: