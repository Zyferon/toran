//! Unix-socket server: the front door for the Python SDK. Uses Tokio,
//! connection limit, and a per-connection task. All policy / state /
//! notify work happens through injected traits so this module stays
//! testable.

use crate::config::Config;
use crate::metrics::Metrics;
use crate::notification::Dispatcher;
use crate::policy::evaluator::{Evaluator, Request};
use crate::policy::loader::PolicyStore;
use crate::protocol::{ClientMessage, ServerMessage};
use crate::state::manager::{ApprovalRecord, ApprovalStatus, AuditEntry, StateManager};
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{Semaphore, broadcast};

pub struct Server {
    pub config: Config,
    pub policies: Arc<PolicyStore>,
    pub state: Arc<dyn StateManager>,
    pub evaluator: Arc<Evaluator>,
    pub metrics: Arc<Metrics>,
    pub dispatcher: Arc<Dispatcher>,
    pub shutdown: broadcast::Sender<()>,
}

impl Server {
    pub fn new(
        config: Config,
        policies: Arc<PolicyStore>,
        state: Arc<dyn StateManager>,
        evaluator: Arc<Evaluator>,
        metrics: Arc<Metrics>,
        dispatcher: Arc<Dispatcher>,
    ) -> Self {
        let (tx, _rx) = broadcast::channel(8);
        Self {
            config,
            policies,
            state,
            evaluator,
            metrics,
            dispatcher,
            shutdown: tx,
        }
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        // Remove stale socket file (Unix only).
        let path = &self.config.socket_path;
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let listener = UnixListener::bind(path)?;
        self.chmod_socket(path)?;
        tracing::info!(socket = %path.display(), "toran socket server listening");

        let sem = Arc::new(Semaphore::new(self.config.max_connections));
        let mut shutdown_rx = self.shutdown.subscribe();

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    tracing::info!("socket server shutting down");
                    break;
                }
                accepted = listener.accept() => {
                    match accepted {
                        Ok((stream, _addr)) => {
                            let permit = sem.clone().acquire_owned().await;
                            let me = self.clone();
                            match permit {
                                Ok(p) => {
                                    tokio::spawn(async move {
                                        if let Err(e) = me.handle_conn(stream).await {
                                            tracing::warn!(error = %e, "connection error");
                                        }
                                        drop(p);
                                    });
                                }
                                Err(e) => {
                                    tracing::warn!(error = %e, "semaphore closed");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "accept error");
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn chmod_socket(&self, path: &std::path::Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o660);
            std::fs::set_permissions(path, perms)?;
        }
        Ok(())
    }

    async fn handle_conn(self: Arc<Self>, stream: UnixStream) -> Result<()> {
        let (read, mut write) = stream.into_split();
        let mut reader = BufReader::new(read);
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                // EOF
                return Ok(());
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parsed: Result<ClientMessage> =
                serde_json::from_str(trimmed).map_err(|e| anyhow::anyhow!("parse: {e}"));
            let msg = match parsed {
                Ok(m) => m,
                Err(e) => {
                    let resp = ServerMessage::Error {
                        message: e.to_string(),
                    };
                    let s = serde_json::to_string(&resp)?;
                    write.write_all(s.as_bytes()).await?;
                    write.write_all(b"\n").await?;
                    continue;
                }
            };
            let response = self.dispatch(msg).await;
            let s = serde_json::to_string(&response)?;
            write.write_all(s.as_bytes()).await?;
            write.write_all(b"\n").await?;
        }
    }

    async fn dispatch(&self, msg: ClientMessage) -> ServerMessage {
        match msg {
            ClientMessage::Ping => ServerMessage::Pong,
            ClientMessage::Shutdown => {
                let _ = self.shutdown.send(());
                ServerMessage::Bye
            }
            ClientMessage::Evaluate {
                request,
                agent_id,
                session_id,
            } => self.handle_evaluate(request, agent_id, session_id).await,
            ClientMessage::Wait {
                approval_id,
                timeout_secs,
                token,
            } => self.handle_wait(approval_id, timeout_secs, token).await,
        }
    }

    async fn handle_evaluate(
        &self,
        request: Request,
        agent_id: String,
        session_id: String,
    ) -> ServerMessage {
        let (default_action, policies) = self.policies.snapshot();
        let decision = self.evaluator.evaluate(&request, &policies, default_action);
        self.metrics.record_evaluation(decision.elapsed_ns as u128);

        // Write audit row first, then conditionally create an approval record.
        let audit = AuditEntry {
            id: 0,
            event_type: "evaluate".into(),
            function_name: request.function_name.clone(),
            arguments_json: serde_json::to_string(&request.args).unwrap_or_default(),
            agent_id: agent_id.clone(),
            policy_rule: decision.rule_name.clone(),
            decision: format!("{:?}", decision.action),
            timestamp: chrono::Utc::now(),
        };
        if let Err(e) = self.state.append_audit(&audit) {
            tracing::warn!(error = %e, "audit write failed");
        }

        match decision.action {
            crate::policy::schema::Action::Allow => ServerMessage::Decision {
                decision,
                approval_id: None,
                notify_token: None,
            },
            crate::policy::schema::Action::Block => ServerMessage::Decision {
                decision,
                approval_id: None,
                notify_token: None,
            },
            crate::policy::schema::Action::RequireApproval => {
                let rec = ApprovalRecord::new_pending(
                    request.function_name.clone(),
                    &request.args,
                    &request.context,
                    agent_id.clone(),
                    session_id.clone(),
                    decision.rule_name.clone(),
                    decision.risk_score,
                    decision.timeout_secs,
                );
                if let Err(e) = self.state.create_approval(&rec) {
                    tracing::error!(error = %e, "create approval failed");
                    return ServerMessage::Error {
                        message: format!("create approval: {e}"),
                    };
                }
                // Dispatch notifications. Failures are logged but do not
                // block the request from being suspended.
                let notify = self.dispatcher.build_event(&rec, &request);
                if let Err(e) = self.dispatcher.dispatch(&notify).await {
                    tracing::warn!(error = %e, "notification dispatch failed");
                }
                self.metrics.record_pending();
                ServerMessage::Decision {
                    decision,
                    approval_id: Some(rec.id.clone()),
                    notify_token: Some(rec.notify_token.clone()),
                }
            }
        }
    }

    async fn handle_wait(
        &self,
        approval_id: String,
        timeout_secs: u64,
        token: String,
    ) -> ServerMessage {
        // Verify token before doing anything.
        let rec = match self.state.get_approval(&approval_id) {
            Ok(Some(r)) => r,
            Ok(None) => {
                return ServerMessage::Error {
                    message: "unknown approval id".into(),
                };
            }
            Err(e) => {
                return ServerMessage::Error {
                    message: format!("db: {e}"),
                };
            }
        };
        if !crate::security::ct_eq(&rec.notify_token, &token) {
            return ServerMessage::Error {
                message: "invalid token".into(),
            };
        }
        if rec.status.is_terminal() {
            return match rec.status {
                ApprovalStatus::Approved => ServerMessage::Approved,
                ApprovalStatus::Denied => ServerMessage::Denied,
                ApprovalStatus::Timeout => ServerMessage::Timeout,
                ApprovalStatus::Cancelled => ServerMessage::Denied,
                ApprovalStatus::Pending => ServerMessage::Error {
                    message: "unreachable".into(),
                },
            };
        }
        // Poll. Tokio lacks a per-record condvar, so we just sleep in
        // short intervals. This is good enough for a single-process
        // server; in a multi-instance deployment the dispatcher
        // pushes via a broadcast channel.
        let timeout = Duration::from_secs(timeout_secs.min(3600));
        let start = std::time::Instant::now();
        let mut shutdown_rx = self.shutdown.subscribe();
        loop {
            if start.elapsed() > timeout {
                return ServerMessage::Timeout;
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(200)) => {}
                _ = shutdown_rx.recv() => return ServerMessage::Timeout,
            }
            match self.state.get_approval(&approval_id) {
                Ok(Some(r)) if r.status.is_terminal() => {
                    return match r.status {
                        ApprovalStatus::Approved => ServerMessage::Approved,
                        ApprovalStatus::Denied => ServerMessage::Denied,
                        ApprovalStatus::Timeout => ServerMessage::Timeout,
                        ApprovalStatus::Cancelled => ServerMessage::Denied,
                        ApprovalStatus::Pending => ServerMessage::Error {
                            message: "unreachable".into(),
                        },
                    };
                }
                Ok(_) => continue,
                Err(e) => {
                    return ServerMessage::Error {
                        message: format!("db: {e}"),
                    };
                }
            }
        }
    }
}
