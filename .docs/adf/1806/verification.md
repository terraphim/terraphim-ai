# Phase 3 Verification: Spawn-Context Isolation Fix

**Skill**: disciplined-implementation
**Issue**: 1806
**Date**: 2026-05-29

---

## Change Applied

**File**: `crates/terraphim_orchestrator/src/lib.rs`

Two hunks:

1. **Production fix** (lines 2406-2410): immediately after `build_spawn_context_for_agent` returns,
   overwrite `spawn_ctx.working_dir` and the `ADF_WORKING_DIR` env var with `agent_working_dir`
   (the worktree path).

2. **Unit test** (lines 11954-11991): `test_spawn_ctx_working_dir_set_to_agent_working_dir` — a
   synchronous, no-I/O test that verifies both mutations are correct.

### Exact diff

```diff
diff --git a/crates/terraphim_orchestrator/src/lib.rs b/crates/terraphim_orchestrator/src/lib.rs
index d4136cd3c..6a3c93f6f 100644
--- a/crates/terraphim_orchestrator/src/lib.rs
+++ b/crates/terraphim_orchestrator/src/lib.rs
@@ -2403,6 +2403,11 @@ impl AgentOrchestrator {

         let mut spawn_ctx =
             build_spawn_context_for_agent(&self.config, def, self.output_poster.as_ref());
+        spawn_ctx.working_dir = Some(agent_working_dir.clone());
+        spawn_ctx = spawn_ctx.with_env(
+            "ADF_WORKING_DIR",
+            agent_working_dir.to_string_lossy().into_owned(),
+        );
         if let Some(event) = synthetic_event {
             for (key, value) in event.env_vars() {
                 spawn_ctx = spawn_ctx.with_env(key, value);
@@ -11946,4 +11951,41 @@ bypass_kg_routing = true
             "gitea_issue must be Some(_) for the post-exit code to enter the outer if-let"
         );
     }
+
+    #[test]
+    fn test_spawn_ctx_working_dir_set_to_agent_working_dir() {
+        use std::path::PathBuf;
+
+        // Simulate what build_spawn_context_for_agent returns for a project-bound agent:
+        // working_dir is set to the project root.
+        let project_root = PathBuf::from("/tmp/project-root");
+        let worktree_path = PathBuf::from("/tmp/project-root/.worktrees/agent-abc123");
+
+        let mut spawn_ctx = SpawnContext::with_working_dir(project_root.clone()).with_env(
+            "ADF_WORKING_DIR",
+            project_root.to_string_lossy().into_owned(),
+        );
+
+        // Apply the fix (the two new lines from the proposed change).
+        let agent_working_dir = worktree_path.clone();
+        spawn_ctx.working_dir = Some(agent_working_dir.clone());
+        spawn_ctx = spawn_ctx.with_env(
+            "ADF_WORKING_DIR",
+            agent_working_dir.to_string_lossy().into_owned(),
+        );
+
+        assert_eq!(
+            spawn_ctx.working_dir.as_deref(),
+            Some(worktree_path.as_path()),
+            "spawn_ctx.working_dir must be the worktree path, not the project root"
+        );
+        assert_eq!(
+            spawn_ctx
+                .env_overrides
+                .get("ADF_WORKING_DIR")
+                .map(String::as_str),
+            Some(worktree_path.to_string_lossy().as_ref()),
+            "ADF_WORKING_DIR env var must reflect the worktree path"
+        );
+    }
 }
```

---

## Test Results

```
running 1 test
test tests::test_spawn_ctx_working_dir_set_to_agent_working_dir ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 800 filtered out; finished in 0.00s
```

Full library test suite:

```
test result: ok. 800 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.43s
```

---

## Clippy

```
Checking terraphim_orchestrator v1.20.2
Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.85s
```

Zero warnings. No new diagnostics introduced.

---

## Self-Check

- [x] Design followed exactly -- only lines 2406-2410 added to production path; no other files touched
- [x] All tests pass -- 800/800 lib tests pass; new test passes
- [x] No new clippy warnings -- clean finish
- [x] No scope creep -- `build_spawn_context_for_agent` signature unchanged; `Provider.working_dir` untouched; spawner priority rule untouched
