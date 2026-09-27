# Deploy Migration Gates

Operational pattern for safe Postgres schema migrations across Odilo environments. A shared helper library (`infra/scripts/lib-migrations.sh`: `run_postgres_migration_gate`, `verify_postgres_migration_records`) is called from preview, QA, and stable deploy scripts, replacing copied migration blocks. Migrations run quiesced — app services are stopped before the migration gate and restarted after — with backup helpers validated and no startup migration side effects in compose files. The V071 preview repair established the convergence approach: precondition guards on earlier migrations (V069), table locks, repair of drifted records, final invariant assertions, plus real-Postgres convergence tests for fresh, canonical-pristine, and legacy-rejected scenarios. Fresh-init seeding lives in `infra/postgres/init/` convergence scripts, kept in step with the migration chain.

synonyms:: deploy migration gate, migration gate, migration repair, quiesced migration, migration convergence, postgres migration gate, lib-migrations

## Related Concepts
- Sidecar Deployment
- ZDP

## Sources
- `.agent/handoffs/2026-07-13-deploy-migration-gates.md` (PR #876)
- `.agent/handoffs/2026-07-21-v071-preview-migration-repair.md` (Gitea #566, PR #979)
- `docs/plans/design-migration-strategy-2026-07-21.md`
