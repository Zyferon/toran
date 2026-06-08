//! Notification dispatcher: builds an event from a pending approval
//! record and fans it out to every configured adapter.

use super::console::ConsoleAdapter;
use super::slack::SlackAdapter;
use super::webhook::WebhookAdapter;
use crate::policy::evaluator::Request;
use crate::state::manager::ApprovalRecord;
use async_trait::async_trait;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct NotificationEvent {
    pub approval_id: String,
    pub function_name: String,
    pub arguments: serde_json::Value,
    pub context: serde_json::Value,
    pub agent_id: String,
    pub policy_rule: String,
    pub risk_score: u8,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub dashboard_url: String,
    pub api_base: String,
    pub hmac_secret: String,
}

#[async_trait]
pub trait Adapter: Send + Sync {
    fn name(&self) -> &'static str;
    async fn send(&self, event: &NotificationEvent) -> anyhow::Result<()>;
}

pub struct Dispatcher {
    adapters: Vec<Arc<dyn Adapter>>,
    dashboard_url: String,
    api_base: String,
    hmac_secret: String,
}

impl Dispatcher {
    pub fn new(
        slack_webhook: Option<String>,
        generic_webhook: Option<String>,
        api_base: String,
        hmac_secret: String,
    ) -> Arc<Self> {
        let mut adapters: Vec<Arc<dyn Adapter>> = Vec::new();
        adapters.push(Arc::new(ConsoleAdapter));
        if let Some(url) = slack_webhook {
            adapters.push(Arc::new(SlackAdapter::new(url)));
        }
        if let Some(url) = generic_webhook {
            adapters.push(Arc::new(WebhookAdapter::new(url)));
        }
        let dashboard_url = format!("{}/dashboard/approval/", api_base.trim_end_matches('/'));
        Arc::new(Self {
            adapters,
            dashboard_url,
            api_base,
            hmac_secret,
        })
    }

    pub fn build_event(&self, rec: &ApprovalRecord, _req: &Request) -> NotificationEvent {
        let args: serde_json::Value =
            serde_json::from_str(&rec.arguments_json).unwrap_or(serde_json::Value::Null);
        let ctx: serde_json::Value =
            serde_json::from_str(&rec.context_json).unwrap_or(serde_json::Value::Null);
        NotificationEvent {
            approval_id: rec.id.clone(),
            function_name: rec.function_name.clone(),
            arguments: args,
            context: ctx,
            agent_id: rec.agent_id.clone(),
            policy_rule: rec.policy_rule.clone(),
            risk_score: rec.risk_score,
            created_at: rec.created_at,
            dashboard_url: format!("{}{}", self.dashboard_url, rec.id),
            api_base: self.api_base.clone(),
            hmac_secret: self.hmac_secret.clone(),
        }
    }

    pub async fn dispatch(&self, event: &NotificationEvent) -> anyhow::Result<()> {
        for a in &self.adapters {
            match a.send(event).await {
                Ok(()) => tracing::debug!(adapter = a.name(), "notification sent"),
                Err(e) => tracing::warn!(adapter = a.name(), error = %e, "notification failed"),
            }
        }
        Ok(())
    }
}
