//! State manager trait + DTOs.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Timeout,
    Cancelled,
}

impl ApprovalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "approved" => Some(Self::Approved),
            "denied" => Some(Self::Denied),
            "timeout" => Some(Self::Timeout),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::Pending)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub id: String,
    pub function_name: String,
    pub arguments_json: String,
    pub context_json: String,
    pub agent_id: String,
    pub session_id: String,
    pub policy_rule: String,
    pub risk_score: u8,
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
    pub comment: Option<String>,
    pub notify_token: String,
}

impl ApprovalRecord {
    pub fn new_pending(
        function_name: String,
        arguments: &HashMap<String, serde_json::Value>,
        context: &HashMap<String, serde_json::Value>,
        agent_id: String,
        session_id: String,
        policy_rule: String,
        risk_score: u8,
        timeout_secs: u64,
    ) -> Self {
        let now = Utc::now();
        let deadline = now + chrono::Duration::seconds(timeout_secs as i64);
        let _ = deadline; // reserved for future scheduling
        Self {
            id: Uuid::new_v4().to_string(),
            function_name,
            arguments_json: serde_json::to_string(arguments).unwrap_or_default(),
            context_json: serde_json::to_string(context).unwrap_or_default(),
            agent_id,
            session_id,
            policy_rule,
            risk_score,
            status: ApprovalStatus::Pending,
            created_at: now,
            resolved_at: None,
            resolved_by: None,
            comment: None,
            notify_token: security::random_token_hex(16),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub event_type: String,
    pub function_name: String,
    pub arguments_json: String,
    pub agent_id: String,
    pub policy_rule: String,
    pub decision: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApprovalQuery {
    pub status: Option<ApprovalStatus>,
    pub function_name: Option<String>,
    pub agent_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

pub trait StateManager: Send + Sync {
    fn create_approval(&self, rec: &ApprovalRecord) -> Result<()>;
    fn get_approval(&self, id: &str) -> Result<Option<ApprovalRecord>>;
    fn list_approvals(&self, q: &ApprovalQuery) -> Result<Vec<ApprovalRecord>>;
    fn resolve_approval(
        &self,
        id: &str,
        status: ApprovalStatus,
        resolved_by: &str,
        comment: Option<&str>,
    ) -> Result<ApprovalRecord>;
    fn append_audit(&self, entry: &AuditEntry) -> Result<i64>;
    fn list_audit(
        &self,
        function_name: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEntry>>;
    fn count_pending(&self) -> Result<u64>;
}

// Re-export security module from crate root for convenience.
pub use crate::security;
