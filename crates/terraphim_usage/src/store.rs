use rusqlite::{Connection, Result, params};
use std::path::PathBuf;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_metrics (
    agent_name TEXT PRIMARY KEY,
    budget_monthly_cents INTEGER,
    total_input_tokens INTEGER DEFAULT 0,
    total_output_tokens INTEGER DEFAULT 0,
    total_cost_sub_cents INTEGER DEFAULT 0,
    total_executions INTEGER DEFAULT 0,
    successful_executions INTEGER DEFAULT 0,
    failed_executions INTEGER DEFAULT 0,
    first_execution_at TEXT,
    last_execution_at TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS execution_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_name TEXT NOT NULL,
    input_tokens INTEGER,
    output_tokens INTEGER,
    total_tokens INTEGER,
    cost_sub_cents INTEGER,
    model TEXT,
    provider TEXT,
    success INTEGER NOT NULL,
    error_message TEXT,
    latency_ms INTEGER,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    gitea_issue INTEGER,
    FOREIGN KEY (agent_name) REFERENCES agent_metrics(agent_name)
);

CREATE TABLE IF NOT EXISTS provider_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    UNIQUE(provider_id, fetched_at)
);

CREATE TABLE IF NOT EXISTS budget_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_name TEXT NOT NULL,
    budget_cents INTEGER,
    spent_sub_cents INTEGER,
    percentage_used REAL,
    verdict TEXT NOT NULL,
    snapshot_at TEXT NOT NULL,
    FOREIGN KEY (agent_name) REFERENCES agent_metrics(agent_name)
);

CREATE INDEX IF NOT EXISTS idx_execution_agent ON execution_history(agent_name);
CREATE INDEX IF NOT EXISTS idx_execution_started ON execution_history(started_at);
CREATE INDEX IF NOT EXISTS idx_provider_usage_provider ON provider_usage(provider_id);
CREATE INDEX IF NOT EXISTS idx_budget_snapshot_agent ON budget_snapshots(agent_name);
"#;

pub struct UsageStore {
    conn: Connection,
}

impl UsageStore {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                    Some(e.to_string()),
                )
            })?;
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch(SCHEMA)?;

        Ok(Self { conn })
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    // Agent metrics CRUD
    pub fn save_agent_metrics(
        &self,
        agent_name: &str,
        budget_monthly_cents: Option<u64>,
        total_input_tokens: u64,
        total_output_tokens: u64,
        total_cost_sub_cents: u64,
        total_executions: u64,
        successful_executions: u64,
        failed_executions: u64,
        first_execution_at: Option<&str>,
        last_execution_at: Option<&str>,
    ) -> Result<()> {
        let updated_at = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO agent_metrics (
                agent_name, budget_monthly_cents, total_input_tokens,
                total_output_tokens, total_cost_sub_cents, total_executions,
                successful_executions, failed_executions, first_execution_at,
                last_execution_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                agent_name,
                budget_monthly_cents,
                total_input_tokens,
                total_output_tokens,
                total_cost_sub_cents,
                total_executions,
                successful_executions,
                failed_executions,
                first_execution_at,
                last_execution_at,
                updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn load_agent_metrics(&self, agent_name: &str) -> Result<Option<AgentMetricsRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT agent_name, budget_monthly_cents, total_input_tokens,
             total_output_tokens, total_cost_sub_cents, total_executions,
             successful_executions, failed_executions, first_execution_at,
             last_execution_at, updated_at
             FROM agent_metrics WHERE agent_name = ?1",
        )?;
        let mut rows = stmt.query(params![agent_name])?;
        if let Some(row) = rows.next()? {
            Ok(Some(AgentMetricsRow {
                agent_name: row.get(0)?,
                budget_monthly_cents: row.get(1)?,
                total_input_tokens: row.get(2)?,
                total_output_tokens: row.get(3)?,
                total_cost_sub_cents: row.get(4)?,
                total_executions: row.get(5)?,
                successful_executions: row.get(6)?,
                failed_executions: row.get(7)?,
                first_execution_at: row.get(8)?,
                last_execution_at: row.get(9)?,
                updated_at: row.get(10)?,
            }))
        } else {
            Ok(None)
        }
    }

    // Execution history
    pub fn record_execution(&self, exec: &ExecutionRecord) -> Result<()> {
        self.conn.execute(
            "INSERT INTO execution_history (
                agent_name, input_tokens, output_tokens, total_tokens,
                cost_sub_cents, model, provider, success, error_message,
                latency_ms, started_at, completed_at, gitea_issue
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                exec.agent_name,
                exec.input_tokens,
                exec.output_tokens,
                exec.total_tokens,
                exec.cost_sub_cents,
                exec.model,
                exec.provider,
                exec.success as i32,
                exec.error_message,
                exec.latency_ms,
                exec.started_at,
                exec.completed_at,
                exec.gitea_issue,
            ],
        )?;
        Ok(())
    }

    pub fn get_executions_in_period(
        &self,
        agent_name: &str,
        since: &str,
        until: &str,
    ) -> Result<Vec<ExecutionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT agent_name, input_tokens, output_tokens, total_tokens,
             cost_sub_cents, model, provider, success, error_message,
             latency_ms, started_at, completed_at, gitea_issue
             FROM execution_history
             WHERE agent_name = ?1 AND started_at >= ?2 AND started_at <= ?3
             ORDER BY started_at DESC",
        )?;
        let rows = stmt.query_map(params![agent_name, since, until], |row| {
            Ok(ExecutionRecord {
                agent_name: row.get(0)?,
                input_tokens: row.get(1)?,
                output_tokens: row.get(2)?,
                total_tokens: row.get(3)?,
                cost_sub_cents: row.get(4)?,
                model: row.get(5)?,
                provider: row.get(6)?,
                success: row.get::<_, i32>(7)? != 0,
                error_message: row.get(8)?,
                latency_ms: row.get(9)?,
                started_at: row.get(10)?,
                completed_at: row.get(11)?,
                gitea_issue: row.get(12)?,
            })
        })?;
        rows.collect()
    }

    // Provider usage snapshots
    pub fn save_provider_usage(
        &self,
        provider_id: &str,
        snapshot_json: &str,
        fetched_at: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO provider_usage (provider_id, snapshot_json, fetched_at)
             VALUES (?1, ?2, ?3)",
            params![provider_id, snapshot_json, fetched_at],
        )?;
        Ok(())
    }

    pub fn get_latest_provider_usage(&self, provider_id: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT snapshot_json FROM provider_usage
             WHERE provider_id = ?1 ORDER BY fetched_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query(params![provider_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    // Budget snapshots
    pub fn record_budget_snapshot(
        &self,
        agent_name: &str,
        budget_cents: Option<u64>,
        spent_sub_cents: u64,
        percentage_used: f64,
        verdict: &str,
    ) -> Result<()> {
        let snapshot_at = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO budget_snapshots (agent_name, budget_cents, spent_sub_cents, percentage_used, verdict, snapshot_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![agent_name, budget_cents, spent_sub_cents, percentage_used, verdict, snapshot_at],
        )?;
        Ok(())
    }

    // List all agents
    pub fn list_agents(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT agent_name FROM agent_metrics ORDER BY agent_name")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect()
    }

    // Delete agent metrics
    pub fn delete_agent_metrics(&self, agent_name: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM agent_metrics WHERE agent_name = ?1",
            params![agent_name],
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct AgentMetricsRow {
    pub agent_name: String,
    pub budget_monthly_cents: Option<u64>,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_sub_cents: u64,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub first_execution_at: Option<String>,
    pub last_execution_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug)]
pub struct ExecutionRecord {
    pub agent_name: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub cost_sub_cents: Option<u64>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub latency_ms: Option<u64>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub gitea_issue: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_store_creation() {
        let store = UsageStore::in_memory().unwrap();
        assert!(store.list_agents().unwrap().is_empty());
    }

    #[test]
    fn test_save_and_load_agent_metrics() {
        let store = UsageStore::in_memory().unwrap();
        store
            .save_agent_metrics(
                "test-agent",
                Some(10000),
                1000,
                500,
                150,
                10,
                9,
                1,
                Some("2026-04-01T00:00:00Z"),
                Some("2026-04-02T00:00:00Z"),
            )
            .unwrap();

        let metrics = store.load_agent_metrics("test-agent").unwrap();
        assert!(metrics.is_some());
        let m = metrics.unwrap();
        assert_eq!(m.total_input_tokens, 1000);
        assert_eq!(m.total_output_tokens, 500);
        assert_eq!(m.total_cost_sub_cents, 150);
    }

    #[test]
    fn test_record_and_query_executions() {
        let store = UsageStore::in_memory().unwrap();
        // Create agent first to satisfy foreign key constraint
        store
            .save_agent_metrics("test-agent", Some(10000), 0, 0, 0, 0, 0, 0, None, None)
            .unwrap();

        let exec = ExecutionRecord {
            agent_name: "test-agent".to_string(),
            input_tokens: Some(1000),
            output_tokens: Some(500),
            total_tokens: Some(1500),
            cost_sub_cents: Some(15),
            model: Some("claude-3-5-sonnet".to_string()),
            provider: Some("anthropic".to_string()),
            success: true,
            error_message: None,
            latency_ms: Some(2500),
            started_at: "2026-04-02T10:00:00Z".to_string(),
            completed_at: Some("2026-04-02T10:00:02Z".to_string()),
            gitea_issue: Some(42),
        };
        store.record_execution(&exec).unwrap();

        let results = store
            .get_executions_in_period("test-agent", "2026-04-01T00:00:00Z", "2026-04-03T00:00:00Z")
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].input_tokens, Some(1000));
        assert_eq!(results[0].gitea_issue, Some(42));
    }

    #[test]
    fn test_provider_usage_roundtrip() {
        let store = UsageStore::in_memory().unwrap();
        let snapshot = r#"{"session": 42, "weekly": 60}"#;
        store
            .save_provider_usage("claude", snapshot, "2026-04-02T10:00:00Z")
            .unwrap();

        let retrieved = store.get_latest_provider_usage("claude").unwrap();
        assert_eq!(retrieved, Some(snapshot.to_string()));
    }

    #[test]
    fn test_budget_snapshot() {
        let store = UsageStore::in_memory().unwrap();
        // Create agent first to satisfy foreign key constraint
        store
            .save_agent_metrics("test-agent", Some(10000), 0, 0, 0, 0, 0, 0, None, None)
            .unwrap();
        store
            .record_budget_snapshot("test-agent", Some(10000), 8000, 80.0, "NearExhaustion")
            .unwrap();
        // Verify by checking table exists and has row
        let count: i32 = store
            .conn
            .query_row("SELECT COUNT(*) FROM budget_snapshots", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_delete_agent_metrics() {
        let store = UsageStore::in_memory().unwrap();
        store
            .save_agent_metrics("to-delete", Some(5000), 0, 0, 0, 0, 0, 0, None, None)
            .unwrap();
        assert_eq!(store.list_agents().unwrap().len(), 1);

        store.delete_agent_metrics("to-delete").unwrap();
        assert_eq!(store.list_agents().unwrap().len(), 0);
    }
}
