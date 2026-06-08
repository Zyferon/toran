//! State subsystem: persistent storage of approval requests, audit
//! log entries, and resolved decisions. SQLite is the default; an
//! in-memory implementation is provided for tests.

pub mod manager;
pub mod memory;
pub mod sqlite;

pub use manager::{ApprovalRecord, ApprovalStatus, AuditEntry, StateManager};
