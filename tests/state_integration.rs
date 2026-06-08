use toran::state::manager::{ApprovalQuery, ApprovalRecord, ApprovalStatus, StateManager};
use toran::state::memory::MemoryState;
use toran::state::sqlite::SqliteState;

fn make_rec(name: &str) -> ApprovalRecord {
    let mut args = std::collections::HashMap::new();
    args.insert("to".into(), serde_json::json!("x@x"));
    let mut ctx = std::collections::HashMap::new();
    ctx.insert("agent_id".into(), serde_json::json!("agent-1"));
    ApprovalRecord::new_pending(
        name.into(),
        &args,
        &ctx,
        "agent-1".into(),
        "sess-1".into(),
        "rule::x".into(),
        80,
        60,
    )
}

#[test]
fn memory_create_and_get() {
    let m = MemoryState::new();
    let rec = make_rec("send_email");
    m.create_approval(&rec).unwrap();
    let got = m.get_approval(&rec.id).unwrap().unwrap();
    assert_eq!(got.function_name, "send_email");
    assert_eq!(got.status, ApprovalStatus::Pending);
}

#[test]
fn memory_resolve_terminal() {
    let m = MemoryState::new();
    let rec = make_rec("send_email");
    m.create_approval(&rec).unwrap();
    let updated = m
        .resolve_approval(&rec.id, ApprovalStatus::Approved, "alice", Some("ok"))
        .unwrap();
    assert_eq!(updated.status, ApprovalStatus::Approved);
    assert_eq!(updated.resolved_by.as_deref(), Some("alice"));
    assert_eq!(updated.comment.as_deref(), Some("ok"));
    // Double-resolve should fail.
    let err = m.resolve_approval(&rec.id, ApprovalStatus::Denied, "bob", None);
    assert!(err.is_err());
}

#[test]
fn memory_list_filter() {
    let m = MemoryState::new();
    m.create_approval(&make_rec("send_email")).unwrap();
    m.create_approval(&make_rec("read_file")).unwrap();
    let q = ApprovalQuery {
        status: Some(ApprovalStatus::Pending),
        function_name: Some("send_email".into()),
        agent_id: None,
        limit: Some(10),
        offset: Some(0),
    };
    let rows = m.list_approvals(&q).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].function_name, "send_email");
}

#[test]
fn memory_audit_chain() {
    use toran::state::manager::AuditEntry;
    let m = MemoryState::new();
    for i in 0..3 {
        let e = AuditEntry {
            id: 0,
            event_type: "evaluate".into(),
            function_name: "f".into(),
            arguments_json: "{}".into(),
            agent_id: "a".into(),
            policy_rule: "r".into(),
            decision: "ALLOW".into(),
            timestamp: chrono::Utc::now(),
        };
        let id = m.append_audit(&e).unwrap();
        assert!(id >= i);
    }
    let list = m.list_audit(None, 100, 0).unwrap();
    assert_eq!(list.len(), 3);
}

#[test]
fn sqlite_open_in_memory() {
    let s = SqliteState::open_memory().unwrap();
    let rec = make_rec("send_email");
    s.create_approval(&rec).unwrap();
    let got = s.get_approval(&rec.id).unwrap().unwrap();
    assert_eq!(got.function_name, "send_email");
    s.resolve_approval(&rec.id, ApprovalStatus::Denied, "bob", None)
        .unwrap();
    let got = s.get_approval(&rec.id).unwrap().unwrap();
    assert_eq!(got.status, ApprovalStatus::Denied);
}

#[test]
fn sqlite_open_file_and_persist() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("toran.db");
    let s = SqliteState::open(&path).unwrap();
    let rec = make_rec("send_email");
    s.create_approval(&rec).unwrap();
    drop(s);
    let s2 = SqliteState::open(&path).unwrap();
    let got = s2.get_approval(&rec.id).unwrap().unwrap();
    assert_eq!(got.function_name, "send_email");
}

#[test]
fn sqlite_count_pending() {
    let s = SqliteState::open_memory().unwrap();
    assert_eq!(s.count_pending().unwrap(), 0);
    let r1 = make_rec("a");
    let r2 = make_rec("b");
    s.create_approval(&r1).unwrap();
    s.create_approval(&r2).unwrap();
    assert_eq!(s.count_pending().unwrap(), 2);
    s.resolve_approval(&r1.id, ApprovalStatus::Approved, "x", None)
        .unwrap();
    assert_eq!(s.count_pending().unwrap(), 1);
}
