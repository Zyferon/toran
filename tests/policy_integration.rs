use std::collections::HashMap;
use toran::policy::evaluator::{Evaluator, Request};
use toran::policy::loader::PolicyStore;
use toran::policy::schema::Action;

fn eval_with_policies(store: &PolicyStore, function_name: &str, args: &[(&str, &str)]) -> Action {
    let ev = Evaluator::new();
    let mut a = HashMap::new();
    for (k, v) in args {
        a.insert((*k).into(), serde_json::Value::String((*v).into()));
    }
    let req = Request {
        function_name: function_name.into(),
        args: a,
        context: HashMap::new(),
    };
    let (def, ps) = store.snapshot();
    ev.evaluate(&req, &ps, def).action
}

fn eval_with_policies_numeric(
    store: &PolicyStore,
    function_name: &str,
    args: &[(&str, i64)],
) -> Action {
    let ev = Evaluator::new();
    let mut a = HashMap::new();
    for (k, v) in args {
        a.insert(
            (*k).into(),
            serde_json::Value::Number(serde_json::Number::from(*v)),
        );
    }
    let req = Request {
        function_name: function_name.into(),
        args: a,
        context: HashMap::new(),
    };
    let (def, ps) = store.snapshot();
    ev.evaluate(&req, &ps, def).action
}

#[test]
fn loads_all_example_policies() {
    let dir = std::path::Path::new("./policies");
    let store = PolicyStore::load(dir).expect("load");
    let (def, ps) = store.snapshot();
    let names: Vec<&str> = ps.iter().map(|p| p.name.as_str()).collect();
    assert!(
        names.contains(&"email-guardian"),
        "missing email-guardian, got {names:?}"
    );
    assert!(
        names.contains(&"database-guardian"),
        "missing database-guardian"
    );
    assert!(
        names.contains(&"financial-guardian"),
        "missing financial-guardian"
    );
    assert!(
        names.contains(&"allow-everything"),
        "missing allow-everything"
    );
    assert!(names.contains(&"minimal"), "missing minimal");
    assert_eq!(def, Action::Allow);
}

#[test]
fn email_internal_passes() {
    let store = PolicyStore::load(std::path::Path::new("./policies")).unwrap();
    let a = eval_with_policies(
        &store,
        "send_email",
        &[("to", "alice@company.com"), ("subject", "Lunch?")],
    );
    assert_eq!(a, Action::Allow, "internal mail should be allowed");
}

#[test]
fn email_external_requires_approval() {
    let store = PolicyStore::load(std::path::Path::new("./policies")).unwrap();
    let a = eval_with_policies(
        &store,
        "send_email",
        &[("to", "stranger@evil.xyz"), ("subject", "Hi")],
    );
    assert_eq!(
        a,
        Action::RequireApproval,
        "external mail should need approval"
    );
}

#[test]
fn email_wire_blocks_or_requires() {
    let store = PolicyStore::load(std::path::Path::new("./policies")).unwrap();
    let a = eval_with_policies(
        &store,
        "send_email",
        &[("to", "bob@x.com"), ("subject", "wire transfer please")],
    );
    assert_eq!(
        a,
        Action::RequireApproval,
        "wire transfer subject must require approval"
    );
}

#[test]
fn email_spam_is_blocked() {
    let store = PolicyStore::load(std::path::Path::new("./policies")).unwrap();
    let a = eval_with_policies(
        &store,
        "send_email",
        &[("to", "x@x.com"), ("subject", "FREE MONEY winner!!!")],
    );
    assert_eq!(a, Action::Block, "spam subject must be blocked");
}

#[test]
fn drop_table_blocked() {
    let store = PolicyStore::load(std::path::Path::new("./policies")).unwrap();
    let a = eval_with_policies(&store, "execute_sql", &[("sql", "DROP TABLE users")]);
    assert_eq!(a, Action::Block);
}

#[test]
fn delete_requires_approval() {
    let store = PolicyStore::load(std::path::Path::new("./policies")).unwrap();
    let a = eval_with_policies(
        &store,
        "execute_sql",
        &[("sql", "DELETE FROM users WHERE id=1")],
    );
    assert_eq!(a, Action::RequireApproval);
}

#[test]
fn transfer_large_requires_approval() {
    let store = PolicyStore::load(std::path::Path::new("./policies")).unwrap();
    let a = eval_with_policies_numeric(
        &store,
        "transfer_usd",
        &[("amount", 5_000), ("currency", 0)],
    );
    // currency=0 will not match; we need to use string variant
    let _ = a;
    let ev = Evaluator::new();
    let mut args = HashMap::new();
    args.insert("amount".into(), serde_json::json!(5_000));
    args.insert("currency".into(), serde_json::json!("USD"));
    let req = Request {
        function_name: "transfer_usd".into(),
        args,
        context: HashMap::new(),
    };
    let (def, ps) = store.snapshot();
    let d = ev.evaluate(&req, &ps, def);
    assert_eq!(d.action, Action::RequireApproval);
}

#[test]
fn transfer_small_allowed() {
    let store = PolicyStore::load(std::path::Path::new("./policies")).unwrap();
    let ev = Evaluator::new();
    let mut args = HashMap::new();
    args.insert("amount".into(), serde_json::json!(50));
    args.insert("currency".into(), serde_json::json!("USD"));
    let req = Request {
        function_name: "transfer_usd".into(),
        args,
        context: HashMap::new(),
    };
    let (def, ps) = store.snapshot();
    let d = ev.evaluate(&req, &ps, def);
    assert_eq!(d.action, Action::Allow);
}

#[test]
fn allow_all_everything_passes() {
    let store = PolicyStore::load(std::path::Path::new("./policies")).unwrap();
    let a = eval_with_policies(&store, "any_function_under_the_sun", &[("foo", "bar")]);
    assert_eq!(a, Action::Allow);
}
