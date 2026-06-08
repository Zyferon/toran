//! SQLite-backed state manager. Uses WAL mode for concurrent
//! readers and writers. The database is a single file on disk.

use super::manager::{ApprovalQuery, ApprovalRecord, ApprovalStatus, AuditEntry, StateManager};
use anyhow::{Context, Result};
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, Row, params};
use std::path::Path;
use std::sync::Arc;

pub struct SqliteState {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteState {
    pub fn open(path: &Path) -> Result<Arc<Self>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create db dir {}", parent.display()))?;
        }
        let conn =
            Connection::open(path).with_context(|| format!("open sqlite at {}", path.display()))?;
        // WAL mode is essential for concurrent readers + writer.
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.pragma_update(None, "synchronous", "NORMAL").ok();
        conn.pragma_update(None, "foreign_keys", "ON").ok();
        let me = Arc::new(Self {
            conn: Arc::new(Mutex::new(conn)),
        });
        me.migrate()?;
        Ok(me)
    }

    /// Open an in-memory database. Useful for tests.
    pub fn open_memory() -> Result<Arc<Self>> {
        let conn = Connection::open_in_memory().context("open in-memory sqlite")?;
        conn.pragma_update(None, "journal_mode", "MEMORY").ok();
        let me = Arc::new(Self {
            conn: Arc::new(Mutex::new(conn)),
        });
        me.migrate()?;
        Ok(me)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS approvals (
                id              TEXT PRIMARY KEY,
                function_name   TEXT NOT NULL,
                arguments_json  TEXT NOT NULL,
                context_json    TEXT NOT NULL,
                agent_id        TEXT NOT NULL,
                session_id      TEXT NOT NULL,
                policy_rule     TEXT NOT NULL,
                risk_score      INTEGER NOT NULL,
                status          TEXT NOT NULL,
                created_at      TEXT NOT NULL,
                resolved_at     TEXT,
                resolved_by     TEXT,
                comment         TEXT,
                notify_token    TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_approvals_status ON approvals(status);
            CREATE INDEX IF NOT EXISTS idx_approvals_function ON approvals(function_name);
            CREATE INDEX IF NOT EXISTS idx_approvals_agent ON approvals(agent_id);
            CREATE INDEX IF NOT EXISTS idx_approvals_created ON approvals(created_at DESC);

            CREATE TABLE IF NOT EXISTS audit_log (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type      TEXT NOT NULL,
                function_name   TEXT NOT NULL,
                arguments_json  TEXT NOT NULL,
                agent_id        TEXT NOT NULL,
                policy_rule     TEXT NOT NULL,
                decision        TEXT NOT NULL,
                timestamp       TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_audit_function ON audit_log(function_name);
            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp DESC);
            "#,
        )
        .context("migrate sqlite")?;
        Ok(())
    }

    fn row_to_record(row: &Row<'_>) -> rusqlite::Result<ApprovalRecord> {
        let status_s: String = row.get("status")?;
        let status = ApprovalStatus::from_str(&status_s).unwrap_or(ApprovalStatus::Pending);
        let created: String = row.get("created_at")?;
        let resolved: Option<String> = row.get("resolved_at")?;
        Ok(ApprovalRecord {
            id: row.get("id")?,
            function_name: row.get("function_name")?,
            arguments_json: row.get("arguments_json")?,
            context_json: row.get("context_json")?,
            agent_id: row.get("agent_id")?,
            session_id: row.get("session_id")?,
            policy_rule: row.get("policy_rule")?,
            risk_score: row.get::<_, i64>("risk_score")? as u8,
            status,
            created_at: chrono::DateTime::parse_from_rfc3339(&created)
                .map(|d| d.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            resolved_at: resolved
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&chrono::Utc)),
            resolved_by: row.get("resolved_by")?,
            comment: row.get("comment")?,
            notify_token: row.get("notify_token")?,
        })
    }
}

impl StateManager for SqliteState {
    fn create_approval(&self, rec: &ApprovalRecord) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO approvals
             (id, function_name, arguments_json, context_json, agent_id, session_id,
              policy_rule, risk_score, status, created_at, resolved_at, resolved_by,
              comment, notify_token)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                rec.id,
                rec.function_name,
                rec.arguments_json,
                rec.context_json,
                rec.agent_id,
                rec.session_id,
                rec.policy_rule,
                rec.risk_score as i64,
                rec.status.as_str(),
                rec.created_at.to_rfc3339(),
                rec.resolved_at.map(|t| t.to_rfc3339()),
                rec.resolved_by,
                rec.comment,
                rec.notify_token,
            ],
        )?;
        Ok(())
    }

    fn get_approval(&self, id: &str) -> Result<Option<ApprovalRecord>> {
        let conn = self.conn.lock();
        let row = conn
            .query_row(
                "SELECT * FROM approvals WHERE id = ?1",
                params![id],
                Self::row_to_record,
            )
            .optional()?;
        Ok(row)
    }

    fn list_approvals(&self, q: &ApprovalQuery) -> Result<Vec<ApprovalRecord>> {
        let conn = self.conn.lock();
        let mut sql = String::from("SELECT * FROM approvals WHERE 1=1");
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(s) = q.status {
            sql.push_str(" AND status = ?");
            binds.push(Box::new(s.as_str().to_string()));
        }
        if let Some(f) = &q.function_name {
            sql.push_str(" AND function_name = ?");
            binds.push(Box::new(f.clone()));
        }
        if let Some(a) = &q.agent_id {
            sql.push_str(" AND agent_id = ?");
            binds.push(Box::new(a.clone()));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");
        let limit = q.limit.unwrap_or(100) as i64;
        let offset = q.offset.unwrap_or(0) as i64;
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            binds.iter().map(|b| &**b as &dyn rusqlite::ToSql).collect();
        let mut stmt = conn.prepare(&sql)?;
        let mut all_params = params_refs;
        all_params.push(&limit);
        all_params.push(&offset);
        let rows = stmt
            .query_map(rusqlite::params_from_iter(all_params), Self::row_to_record)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn resolve_approval(
        &self,
        id: &str,
        status: ApprovalStatus,
        resolved_by: &str,
        comment: Option<&str>,
    ) -> Result<ApprovalRecord> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock();
        let updated = conn.execute(
            "UPDATE approvals
             SET status = ?1, resolved_at = ?2, resolved_by = ?3, comment = ?4
             WHERE id = ?5 AND status = 'pending'",
            params![status.as_str(), now, resolved_by, comment, id],
        )?;
        if updated == 0 {
            anyhow::bail!("approval {id} not found or already resolved");
        }
        drop(conn);
        let rec = self
            .get_approval(id)?
            .ok_or_else(|| anyhow::anyhow!("approval vanished"))?;
        Ok(rec)
    }

    fn append_audit(&self, entry: &AuditEntry) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO audit_log
             (event_type, function_name, arguments_json, agent_id, policy_rule, decision, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.event_type,
                entry.function_name,
                entry.arguments_json,
                entry.agent_id,
                entry.policy_rule,
                entry.decision,
                entry.timestamp.to_rfc3339(),
            ],
        )?;
        let id = conn.last_insert_rowid();
        Ok(id)
    }

    fn list_audit(
        &self,
        function_name: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEntry>> {
        let conn = self.conn.lock();
        let (sql, args): (String, Vec<Box<dyn rusqlite::ToSql>>) = match function_name {
            Some(f) => (
                "SELECT * FROM audit_log WHERE function_name = ?1 ORDER BY timestamp DESC LIMIT ?2 OFFSET ?3".into(),
                vec![Box::new(f.to_string()), Box::new(limit as i64), Box::new(offset as i64)],
            ),
            None => (
                "SELECT * FROM audit_log ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2".into(),
                vec![Box::new(limit as i64), Box::new(offset as i64)],
            ),
        };
        let args_refs: Vec<&dyn rusqlite::ToSql> =
            args.iter().map(|b| &**b as &dyn rusqlite::ToSql).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(args_refs), |row| {
                Ok(AuditEntry {
                    id: row.get("id")?,
                    event_type: row.get("event_type")?,
                    function_name: row.get("function_name")?,
                    arguments_json: row.get("arguments_json")?,
                    agent_id: row.get("agent_id")?,
                    policy_rule: row.get("policy_rule")?,
                    decision: row.get("decision")?,
                    timestamp: {
                        let s: String = row.get("timestamp")?;
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map(|d| d.with_timezone(&chrono::Utc))
                            .unwrap_or_else(|_| chrono::Utc::now())
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn count_pending(&self) -> Result<u64> {
        let conn = self.conn.lock();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM approvals WHERE status = 'pending'",
            [],
            |r| r.get(0),
        )?;
        Ok(n as u64)
    }
}
