//! Generic webhook adapter: POSTs the event to a user URL with an
//! HMAC-SHA256 signature header for verification.

use super::dispatcher::{Adapter, NotificationEvent};
use super::slack::reqwest_compat;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct WebhookAdapter {
    url: String,
    client: reqwest_compat::Client,
}

impl WebhookAdapter {
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: reqwest_compat::Client::new(),
        }
    }
}

#[async_trait]
impl Adapter for WebhookAdapter {
    fn name(&self) -> &'static str {
        "webhook"
    }
    async fn send(&self, event: &NotificationEvent) -> anyhow::Result<()> {
        let body = serde_json::to_vec(event)?;
        let sig = crate::security::hmac_sha256_hex(event.hmac_secret.as_bytes(), &body);
        let mut headers = HashMap::new();
        headers.insert("X-Toran-Signature".to_string(), sig.clone());
        headers.insert("X-Toran-Event".to_string(), "approval_required".to_string());
        let mut enriched = serde_json::to_value(event)?;
        if let Some(obj) = enriched.as_object_mut() {
            obj.insert("toran_signature".into(), serde_json::Value::String(sig));
        }
        let url = self.url.clone();
        self.client
            .post(&url)
            .headers(headers)
            .json(&enriched)
            .send()
            .await?;
        Ok(())
    }
}
