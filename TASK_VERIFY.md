# Right-side-of-V Verification: Waves 2 and 3

Verify the following features were implemented correctly against the spec.

## Wave 2 Verification (Task 2.1 + 2.2)

### 2.1: spawn_agent uses spawn_with_fallback
1. Read crates/terraphim_orchestrator/src/lib.rs spawn_agent function
2. Confirm it builds a SpawnRequest and calls spawn_with_fallback (not spawn_with_model_and_limits)
3. Confirm PermittedProviderFilter is passed from self.permitted_filter
4. Confirm circuit_breakers HashMap is locked and passed
5. Run: cargo test -p terraphim_orchestrator -- spawn_agent
6. Run: cargo test -p terraphim_orchestrator
7. Report any test failures

### 2.2: Skill chain resolution
1. Confirm spawn_agent resolves def.skill_chain via self.skill_resolver.resolve_skill_chain()
2. Confirm resolved descriptions are collected (even if not yet injected into prompt)
3. Run: cargo test -p terraphim_spawner -- skill

## Wave 3 Verification (Task 3.1)

### 3.1: SFIA profile integration
1. Read crates/terraphim_orchestrator/src/config.rs -- confirm SfiaSkill struct exists with code and level fields
2. Confirm AgentDefinition has sfia_skills and sfia_metaprompt fields
3. Read crates/terraphim_spawner/src/lib.rs -- confirm SpawnRequest has sfia_metaprompt field
4. Confirm orchestrator.toml has at least 3 agents with sfia_metaprompt configured
5. Confirm automation/agent-metaprompts/ has the 9 expected .md files
6. Run: cargo test -p terraphim_orchestrator -p terraphim_spawner

## Final
Run the full test suite and report results. Do NOT modify any code -- read only.
