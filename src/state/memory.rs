//! In-memory state manager for tests and ephemeral deployments.

use super::manager::{ApprovalQuery, ApprovalRecord, ApprovalStatus, AuditEntry, StateManager};
use anyhow::Result;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

pub struct MemoryState {
    approvals: Mutex<HashMap<String, ApprovalRecord>>,
    audit: Mutex<Vec<AuditEntry>>,
    next_id: Mutex<i64>,
}

impl MemoryState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            approvals: Mutex::new(HashMap::new()),
            audit: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        })
    }
}

impl StateManager for MemoryState {
    fn create_approval(&self, rec: &ApprovalRecord) -> Result<()> {
        self.approvals.lock().insert(rec.id.clone(), rec.clone());
        Ok(())
    }
    fn get_approval(&self, id: &str) -> Result<Option<ApprovalRecord>> {
        Ok(self.approvals.lock().get(id).cloned())
    }
    fn list_approvals(&self, q: &ApprovalQuery) -> Result<Vec<ApprovalRecord>> {
        let g = self.approvals.lock();
        let mut out: Vec<ApprovalRecord> = g
            .values()
            .filter(|r| q.status.map_or(true, |s| r.status == s))
            .filter(|r| {
                q.function_name
                    .as_ref()
                    .map_or(true, |f| &r.function_name == f)
            })
            .filter(|r| q.agent_id.as_ref().map_or(true, |a| &r.agent_id == a))
            .cloned()
            .collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let offset = q.offset.unwrap_or(0);
        let limit = q.limit.unwrap_or(100);
        Ok(out.into_iter().skip(offset).take(limit).collect())
    }
    fn resolve_approval(
        &self,
        id: &str,
        status: ApprovalStatus,
        resolved_by: &str,
        comment: Option<&str>,
    ) -> Result<ApprovalRecord> {
        let mut g = self.approvals.lock();
        let rec = g
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("approval {id} not found"))?;
        if rec.status.is_terminal() {
            anyhow::bail!("approval {id} already resolved");
        }
        rec.status = status;
        rec.resolved_at = Some(chrono::Utc::now());
        rec.resolved_by = Some(resolved_by.to_string());
        rec.comment = comment.map(|s| s.to_string());
        Ok(rec.clone())
    }
    fn append_audit(&self, entry: &AuditEntry) -> Result<i64> {
        let mut g = self.audit.lock();
        let mut entry = entry.clone();
        if entry.id == 0 {
            entry.id = *self.next_id.lock();
            *self.next_id.lock() += 1;
        }
        g.push(entry.clone());
        Ok(entry.id)
    }
    fn list_audit(
        &self,
        function_name: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEntry>> {
        let g = self.audit.lock();
        let mut out: Vec<AuditEntry> = g
            .iter()
            .filter(|e| function_name.map_or(true, |f| e.function_name == f))
            .cloned()
            .collect();
        out.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(out.into_iter().skip(offset).take(limit).collect())
    }
    fn count_pending(&self) -> Result<u64> {
        Ok(self
            .approvals
            .lock()
            .values()
            .filter(|r| r.status == ApprovalStatus::Pending)
            .count() as u64)
    }
}
