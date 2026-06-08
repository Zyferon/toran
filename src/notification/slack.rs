//! Slack adapter: posts a Block-Kit style message with Approve/Deny
//! buttons to an incoming webhook URL. In a real Slack app you would
//! use the Web API to update the message on click; for an incoming
//! webhook we simply include the dashboard URL.

use super::dispatcher::{Adapter, NotificationEvent};
use async_trait::async_trait;
use serde_json::json;

pub struct SlackAdapter {
    url: String,
    client: reqwest_compat::Client,
}

impl SlackAdapter {
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: reqwest_compat::Client::new(),
        }
    }
}

#[async_trait]
impl Adapter for SlackAdapter {
    fn name(&self) -> &'static str {
        "slack"
    }
    async fn send(&self, event: &NotificationEvent) -> anyhow::Result<()> {
        let payload = json!({
            "text": format!(
                "Toran approval needed for `{}` (agent: {})",
                event.function_name, event.agent_id
            ),
            "blocks": [
                { "type": "header", "text": { "type": "plain_text",
                    "text": format!("Toran: approve `{}`", event.function_name) } },
                { "type": "section", "fields": [
                    { "type": "mrkdwn", "text": format!("*Agent:*\n{}", event.agent_id) },
                    { "type": "mrkdwn", "text": format!("*Risk:*\n{}", event.risk_score) },
                    { "type": "mrkdwn", "text": format!("*Rule:*\n{}", event.policy_rule) },
                    { "type": "mrkdwn", "text": format!("*Args:*\n```{}```",
                        serde_json::to_string_pretty(&event.arguments).unwrap_or_default()) },
                ]},
                { "type": "actions", "elements": [
                    { "type": "button", "text": { "type": "plain_text", "text": "Approve" },
                      "style": "primary",
                      "url": format!("{}/approve/{}", event.dashboard_url.trim_end_matches('/'), event.approval_id) },
                    { "type": "button", "text": { "type": "plain_text", "text": "Deny" },
                      "style": "danger",
                      "url": format!("{}/deny/{}", event.dashboard_url.trim_end_matches('/'), event.approval_id) },
                ]}
            ]
        });
        self.client.post(&self.url).json(&payload).send().await?;
        Ok(())
    }
}

/// Tiny reqwest shim. We avoid pulling in the heavy `reqwest` crate
/// at the workspace level for this single HTTP call. Instead we
/// implement just the methods we need on top of `tokio::net::TcpStream`
/// + a hand-rolled HTTP/1.1 client. This keeps the build self-contained.
pub mod reqwest_compat {
    use anyhow::{Result, anyhow};
    use serde::Serialize;
    use std::time::Duration;

    pub struct Client;

    impl Client {
        pub const fn new() -> Self {
            Self
        }
        pub fn post<'a>(&self, url: &'a str) -> RequestBuilder<'a> {
            RequestBuilder {
                url,
                body: None,
                headers: std::collections::HashMap::new(),
            }
        }
    }

    pub struct RequestBuilder<'a> {
        url: &'a str,
        body: Option<Vec<u8>>,
        headers: std::collections::HashMap<String, String>,
    }

    impl<'a> RequestBuilder<'a> {
        pub fn json<T: Serialize>(mut self, value: &T) -> Self {
            self.body = Some(serde_json::to_vec(value).unwrap_or_default());
            self
        }
        pub fn headers(mut self, h: std::collections::HashMap<String, String>) -> Self {
            self.headers = h;
            self
        }
        pub async fn send(self) -> Result<()> {
            let url = self.url;
            let url = url
                .strip_prefix("http://")
                .or_else(|| url.strip_prefix("https://"))
                .ok_or_else(|| anyhow!("only http(s) urls supported"))?;
            let use_tls = self.url.starts_with("https://");
            let (host_port, path) = match url.find('/') {
                Some(i) => (&url[..i], &url[i..]),
                None => (url, "/"),
            };
            let (host, port) = match host_port.rsplit_once(':') {
                Some((h, p)) => (
                    h.to_string(),
                    p.parse::<u16>().unwrap_or(if use_tls { 443 } else { 80 }),
                ),
                None => (host_port.to_string(), if use_tls { 443 } else { 80 }),
            };
            let body = self.body.unwrap_or_default();
            let mut req = format!(
                "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nConnection: close\r\n",
                path = path,
                host = host,
                len = body.len()
            );
            for (k, v) in &self.headers {
                req.push_str(&format!("{k}: {v}\r\n"));
            }
            req.push_str("\r\n");
            let mut stream = tokio::time::timeout(
                Duration::from_secs(5),
                tokio::net::TcpStream::connect((host.as_str(), port)),
            )
            .await??;
            tokio::time::timeout(Duration::from_secs(5), stream.write_all(req.as_bytes()))
                .await??;
            tokio::time::timeout(Duration::from_secs(5), stream.write_all(&body)).await??;
            let mut buf = Vec::new();
            tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut buf)).await??;
            if !buf.starts_with(b"HTTP/1.1 2") && !buf.starts_with(b"HTTP/1.0 2") {
                let snippet = String::from_utf8_lossy(&buf[..buf.len().min(200)]);
                return Err(anyhow!("http non-2xx: {snippet}"));
            }
            Ok(())
        }
    }

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
}
