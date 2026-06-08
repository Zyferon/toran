//! HTTP routes for the Toran API. All handlers are thin: they call
//! into the same StateManager / PolicyStore that the socket server
//! uses, so there is exactly one source of truth.

use super::ApiState;
use crate::state::manager::{ApprovalQuery, ApprovalStatus, AuditEntry};
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub fn build(state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route(
            "/api/approvals",
            get(list_approvals).post(create_approval_action),
        )
        .route("/api/approvals/:id", get(get_approval))
        .route("/api/approvals/:id/approve", post(approve_approval))
        .route("/api/approvals/:id/deny", post(deny_approval))
        .route("/api/policies", get(list_policies))
        .route("/api/policies/:name", get(get_policy))
        .route("/api/audit", get(list_audit))
        .route("/api/metrics", get(metrics))
        .route("/api/summary", get(summary))
        .route("/webhooks/toran", post(webhook_receiver))
        .route("/", get(super::dashboard::index))
        .route("/dashboard", get(super::dashboard::dashboard))
        .route(
            "/dashboard/approval/:id",
            get(super::dashboard::approval_detail),
        )
        .route("/dashboard/audit", get(super::dashboard::audit))
        .route("/dashboard/policies", get(super::dashboard::policies))
        .with_state(state)
}

async fn health(State(s): State<Arc<ApiState>>) -> impl IntoResponse {
    let pending = s.state.count_pending().unwrap_or(0);
    Json(serde_json::json!({
        "status": "ok",
        "uptime_secs": s.start_time.elapsed().as_secs(),
        "pending_approvals": pending,
        "socket": s.config.socket_path.display().to_string(),
        "default_action": s.config.default_action,
    }))
}

async fn list_approvals(
    State(s): State<Arc<ApiState>>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let status = q.get("status").and_then(|s| ApprovalStatus::from_str(s));
    let function_name = q.get("function").cloned();
    let agent_id = q.get("agent").cloned();
    let limit = q.get("limit").and_then(|l| l.parse().ok());
    let offset = q.get("offset").and_then(|l| l.parse().ok());
    let query = ApprovalQuery {
        status,
        function_name,
        agent_id,
        limit,
        offset,
    };
    let rows = s.state.list_approvals(&query).map_err(internal)?;
    Ok(Json(serde_json::json!({ "approvals": rows })))
}

async fn get_approval(
    State(s): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rec = s.state.get_approval(&id).map_err(internal)?;
    match rec {
        Some(r) => Ok(Json(serde_json::json!({ "approval": r }))),
        None => Err((StatusCode::NOT_FOUND, "approval not found".into())),
    }
}

#[derive(Deserialize)]
struct ApprovalAction {
    resolved_by: String,
    comment: Option<String>,
    token: Option<String>,
}

async fn approve_approval(
    State(s): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(body): Json<ApprovalAction>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    finalize(s, id, ApprovalStatus::Approved, body, "approve").await
}

async fn deny_approval(
    State(s): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(body): Json<ApprovalAction>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    finalize(s, id, ApprovalStatus::Denied, body, "deny").await
}

#[derive(Deserialize)]
struct WebhookPayload {
    approval_id: String,
    decision: String,
    token: String,
    resolved_by: Option<String>,
    comment: Option<String>,
}

async fn webhook_receiver(
    State(s): State<Arc<ApiState>>,
    Json(body): Json<WebhookPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // The receiver verifies the token belongs to the approval. In
    // production we would also verify an HMAC header.
    let rec = s.state.get_approval(&body.approval_id).map_err(internal)?;
    let rec = rec.ok_or((StatusCode::NOT_FOUND, "approval not found".into()))?;
    if !crate::security::ct_eq(&rec.notify_token, &body.token) {
        return Err((StatusCode::UNAUTHORIZED, "invalid token".into()));
    }
    let status = match body.decision.as_str() {
        "approve" | "approved" => ApprovalStatus::Approved,
        "deny" | "denied" => ApprovalStatus::Denied,
        _ => return Err((StatusCode::BAD_REQUEST, "bad decision".into())),
    };
    let resolved = s
        .state
        .resolve_approval(
            &body.approval_id,
            status,
            body.resolved_by.as_deref().unwrap_or("webhook"),
            body.comment.as_deref(),
        )
        .map_err(internal)?;
    let audit = AuditEntry {
        id: 0,
        event_type: "resolve".into(),
        function_name: resolved.function_name.clone(),
        arguments_json: resolved.arguments_json.clone(),
        agent_id: resolved.agent_id.clone(),
        policy_rule: resolved.policy_rule.clone(),
        decision: status.as_str().into(),
        timestamp: chrono::Utc::now(),
    };
    s.state.append_audit(&audit).map_err(internal)?;
    s.metrics.record_resolved();
    Ok(Json(
        serde_json::json!({ "ok": true, "status": status.as_str() }),
    ))
}

#[derive(Deserialize)]
struct CreateApprovalAction {
    function_name: String,
    arguments: serde_json::Value,
    context: Option<serde_json::Value>,
    agent_id: String,
    session_id: String,
    risk_score: Option<u8>,
    policy_rule: Option<String>,
}

async fn create_approval_action(
    State(s): State<Arc<ApiState>>,
    Json(body): Json<CreateApprovalAction>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let now = chrono::Utc::now();
    let id = crate::security::random_token_hex(8);
    let token = crate::security::random_token_hex(16);
    let arguments_json = serde_json::to_string(&body.arguments).unwrap_or_else(|_| "null".into());
    let context_json = serde_json::to_string(&body.context.unwrap_or(serde_json::Value::Null))
        .unwrap_or_else(|_| "null".into());
    let rec = crate::state::manager::ApprovalRecord {
        id: id.clone(),
        function_name: body.function_name,
        arguments_json,
        context_json,
        agent_id: body.agent_id,
        session_id: body.session_id,
        policy_rule: body.policy_rule.unwrap_or_else(|| "manual".into()),
        risk_score: body.risk_score.unwrap_or(50),
        status: crate::state::manager::ApprovalStatus::Pending,
        created_at: now,
        resolved_at: None,
        resolved_by: None,
        comment: None,
        notify_token: token,
    };
    s.state.create_approval(&rec).map_err(internal)?;
    s.metrics.record_pending();
    let audit = AuditEntry {
        id: 0,
        event_type: "create".into(),
        function_name: rec.function_name.clone(),
        arguments_json: rec.arguments_json.clone(),
        agent_id: rec.agent_id.clone(),
        policy_rule: rec.policy_rule.clone(),
        decision: "PENDING".into(),
        timestamp: now,
    };
    s.state.append_audit(&audit).map_err(internal)?;
    let req = crate::policy::evaluator::Request::from_json_strings(
        &rec.function_name,
        &rec.arguments_json,
        &rec.context_json,
    );
    let event = s.dispatcher.build_event(&rec, &req);
    if let Err(e) = s.dispatcher.dispatch(&event).await {
        tracing::warn!(error = %e, "notification dispatch failed");
    }
    Ok(Json(serde_json::json!({
        "ok": true,
        "id": id,
        "notify_token": rec.notify_token,
    })))
}

async fn finalize(
    s: Arc<ApiState>,
    id: String,
    status: ApprovalStatus,
    body: ApprovalAction,
    op: &str,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rec = s.state.get_approval(&id).map_err(internal)?;
    let rec = rec.ok_or((StatusCode::NOT_FOUND, "approval not found".into()))?;
    if rec.status.is_terminal() {
        return Err((StatusCode::CONFLICT, "already resolved".into()));
    }
    if let Some(token) = &body.token {
        if !crate::security::ct_eq(&rec.notify_token, token) {
            return Err((StatusCode::UNAUTHORIZED, "invalid token".into()));
        }
    }
    let resolved = s
        .state
        .resolve_approval(&id, status, &body.resolved_by, body.comment.as_deref())
        .map_err(internal)?;
    let audit = AuditEntry {
        id: 0,
        event_type: format!("resolve:{op}"),
        function_name: resolved.function_name.clone(),
        arguments_json: resolved.arguments_json.clone(),
        agent_id: resolved.agent_id.clone(),
        policy_rule: resolved.policy_rule.clone(),
        decision: status.as_str().into(),
        timestamp: chrono::Utc::now(),
    };
    s.state.append_audit(&audit).map_err(internal)?;
    s.metrics.record_resolved();
    Ok(Json(serde_json::json!({
        "ok": true,
        "approval": resolved,
    })))
}

async fn list_policies(State(s): State<Arc<ApiState>>) -> impl IntoResponse {
    let (_default, list) = s.policies.snapshot();
    let dir = s.policies.dir();
    let mut files: Vec<(String, std::path::PathBuf)> = Vec::new();
    for entry in walkdir::WalkDir::new(dir).max_depth(1) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "yaml" | "yml") {
            continue;
        }
        let name = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        files.push((name, path.to_path_buf()));
    }
    Json(serde_json::json!({
        "policies": list.iter().map(|p| serde_json::json!({
            "name": p.name,
            "rule_count": p.rules.len(),
        })).collect::<Vec<_>>(),
        "files": files.iter().map(|(n, p)| serde_json::json!({
            "name": n, "path": p.display().to_string(),
        })).collect::<Vec<_>>(),
    }))
}

async fn get_policy(
    State(s): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let path = s.policies.dir().join(&name);
    let raw = std::fs::read_to_string(&path).map_err(internal)?;
    Ok(Json(serde_json::json!({ "name": name, "content": raw })))
}

#[derive(Deserialize)]
struct AuditQuery {
    function: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

async fn list_audit(
    State(s): State<Arc<ApiState>>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rows = s
        .state
        .list_audit(
            q.function.as_deref(),
            q.limit.unwrap_or(100),
            q.offset.unwrap_or(0),
        )
        .map_err(internal)?;
    Ok(Json(serde_json::json!({ "audit": rows })))
}

async fn metrics(State(s): State<Arc<ApiState>>) -> impl IntoResponse {
    use axum::body::Body;
    use axum::http::{Response, StatusCode, header};
    let body = s.metrics.render_prometheus();
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(body))
        .unwrap()
}

#[derive(Serialize)]
struct Summary {
    pending: u64,
    total_decisions: u64,
    avg_eval_ns: u128,
    uptime_secs: u64,
}

async fn summary(State(s): State<Arc<ApiState>>) -> Result<Json<Summary>, (StatusCode, String)> {
    let pending = s.state.count_pending().map_err(internal)?;
    let prom = s.metrics.render_prometheus();
    let avg = prom
        .lines()
        .find(|l| l.starts_with("toran_eval_avg_ns"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|n| n.parse::<u128>().ok())
        .unwrap_or(0);
    let total = prom
        .lines()
        .find(|l| l.starts_with("toran_evaluations_total"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|n| n.parse::<u64>().ok())
        .unwrap_or(0);
    Ok(Json(Summary {
        pending,
        total_decisions: total,
        avg_eval_ns: avg,
        uptime_secs: s.start_time.elapsed().as_secs(),
    }))
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
