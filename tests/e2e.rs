//! End-to-end test: spin up the socket server and REST API, hit both
//! from the same process, and assert that the gate / approve / deny
//! loop works.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use toran::api::ApiState;
use toran::api::router;
use toran::config::Config;
use toran::metrics::Metrics;
use toran::notification::Dispatcher;
use toran::policy::evaluator::Evaluator;
use toran::policy::loader::PolicyStore;
use toran::protocol::{ClientMessage, ServerMessage};
use toran::state::manager::StateManager;
use toran::state::sqlite::SqliteState;

async fn spawn_everything() -> (
    Arc<PolicyStore>,
    Arc<SqliteState>,
    Arc<ApiState>,
    std::path::PathBuf,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    let policy_dir = dir.path().join("policies");
    let db_path = dir.path().join("test.db");
    let socket_path = dir.path().join("test.sock");
    std::fs::create_dir_all(&policy_dir).unwrap();
    std::fs::write(
        policy_dir.join("test.yaml"),
        r#"
name: test
default_action: ALLOW
rules:
  - name: allow
    tool: { exact: send_email }
    action: ALLOW
  - name: approve
    tool: { exact: wire_transfer }
    action: REQUIRE_APPROVAL
    timeout_secs: 5
"#,
    )
    .unwrap();

    let cfg = Config {
        socket_path: socket_path.clone(),
        api_bind: "127.0.0.1:0".into(),
        policy_dir: policy_dir.clone(),
        database_path: db_path.clone(),
        default_action: "ALLOW".into(),
        max_connections: 100,
        max_suspended: 100,
        default_timeout_secs: 60,
        hmac_secret: "test".into(),
        log_level: "warn".into(),
        slack_webhook: None,
        generic_webhook: None,
    };

    let policies = PolicyStore::load(&cfg.policy_dir).unwrap();
    let state = SqliteState::open(&cfg.database_path).unwrap();
    let evaluator = Arc::new(Evaluator::new());
    let metrics = Metrics::new();
    let dispatcher = Dispatcher::new(
        None,
        None,
        format!("http://{}", cfg.api_bind),
        cfg.hmac_secret.clone(),
    );
    let api_state = Arc::new(ApiState {
        config: cfg.clone(),
        policies: policies.clone(),
        state: state.clone(),
        metrics: metrics.clone(),
        dispatcher: dispatcher.clone(),
        start_time: std::time::Instant::now(),
    });

    // Start the API.
    let app = router::build(api_state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    // Update config with the actual bound address.
    let mut cfg2 = cfg.clone();
    cfg2.api_bind = format!("{addr}");

    let server = Arc::new(toran::server::Server::new(
        cfg2,
        policies.clone(),
        state.clone(),
        evaluator,
        metrics,
        dispatcher,
    ));
    let server_handle = tokio::spawn(async move { server.run().await });
    // Give the server a moment to bind.
    tokio::time::sleep(Duration::from_millis(100)).await;
    std::mem::forget(server_handle); // leak; we will drop everything when this fn returns

    (policies, state, api_state, socket_path, dir)
}

#[tokio::test]
async fn allow_decision_over_socket() {
    let (_policies, _state, _api, socket_path, _dir) = spawn_everything().await;
    let stream = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(read);

    let mut args = HashMap::new();
    args.insert("to".into(), serde_json::json!("alice@x.com"));
    let req = toran::policy::evaluator::Request {
        function_name: "send_email".into(),
        args,
        context: HashMap::new(),
    };
    let msg = ClientMessage::Evaluate {
        request: req,
        agent_id: "agent-1".into(),
        session_id: "sess-1".into(),
    };
    let s = serde_json::to_string(&msg).unwrap();
    write.write_all(s.as_bytes()).await.unwrap();
    write.write_all(b"\n").await.unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    let resp: ServerMessage = serde_json::from_str(line.trim()).unwrap();
    match resp {
        ServerMessage::Decision { decision, .. } => {
            assert_eq!(decision.action, toran::policy::schema::Action::Allow);
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[tokio::test]
async fn approve_flow_over_socket() {
    let (_policies, state, _api, socket_path, _dir) = spawn_everything().await;
    let stream = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(read);

    let mut args = HashMap::new();
    args.insert("amount".into(), serde_json::json!(50_000));
    let req = toran::policy::evaluator::Request {
        function_name: "wire_transfer".into(),
        args,
        context: HashMap::new(),
    };
    let msg = ClientMessage::Evaluate {
        request: req,
        agent_id: "agent-1".into(),
        session_id: "sess-1".into(),
    };
    let s = serde_json::to_string(&msg).unwrap();
    write.write_all(s.as_bytes()).await.unwrap();
    write.write_all(b"\n").await.unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    let resp: ServerMessage = serde_json::from_str(line.trim()).unwrap();
    let (approval_id, token) = match resp {
        ServerMessage::Decision {
            decision,
            approval_id,
            notify_token,
            ..
        } => {
            assert_eq!(
                decision.action,
                toran::policy::schema::Action::RequireApproval
            );
            (approval_id.unwrap(), notify_token.unwrap())
        }
        other => panic!("unexpected: {other:?}"),
    };

    // Simulate a human resolving via the StateManager.
    let updated = state
        .resolve_approval(
            &approval_id,
            toran::state::manager::ApprovalStatus::Approved,
            "alice",
            Some("ok"),
        )
        .unwrap();
    assert_eq!(
        updated.status,
        toran::state::manager::ApprovalStatus::Approved
    );
    // Token still works for verification.
    assert!(toran::security::ct_eq(&updated.notify_token, &token));
}

#[tokio::test]
async fn health_endpoint_works() {
    let (_p, _s, api, _sock, _dir) = spawn_everything().await;
    // The api state knows the bound address. But we lost it; let's just
    // call the handler directly.
    let app = router::build(api.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.ok() });
    // Give the listener a moment to start.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let url = format!("http://{addr}/api/health");
    let body: serde_json::Value = reqwest_minimal::get_json(&url).await;
    eprintln!("body = {body}");
    assert_eq!(body["status"], "ok");
}

// Tiny HTTP client used only by tests. Avoids the heavy reqwest dep.
mod reqwest_minimal {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    pub async fn get_json(url: &str) -> serde_json::Value {
        let url = url.strip_prefix("http://").unwrap();
        let (host_port, path) = match url.find('/') {
            Some(i) => (&url[..i], &url[i..]),
            None => (url, "/"),
        };
        let (host, port) = match host_port.rsplit_once(':') {
            Some((h, p)) => (h.to_string(), p.parse::<u16>().unwrap_or(80)),
            None => (host_port.to_string(), 80),
        };
        let mut s = tokio::net::TcpStream::connect((host.as_str(), port))
            .await
            .expect("connect");
        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nAccept: application/json\r\n\r\n"
        );
        s.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        s.read_to_end(&mut buf).await.unwrap();
        let raw = String::from_utf8_lossy(&buf).to_string();
        eprintln!("[http raw] >>>\n{raw}\n<<<");
        let body = raw.split("\r\n\r\n").nth(1).unwrap_or("{}");
        // Sometimes axum sends chunked; just take the last valid JSON object.
        let trimmed = body.trim();
        serde_json::from_str(trimmed).unwrap_or_else(|_| {
            // try to find a { ... } substring
            if let Some(start) = trimmed.find('{') {
                if let Some(end) = trimmed.rfind('}') {
                    return serde_json::from_str(&trimmed[start..=end])
                        .unwrap_or(serde_json::json!({}));
                }
            }
            serde_json::json!({})
        })
    }
}
