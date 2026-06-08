//! Configuration loading for Toran core.
//!
//! All settings can be overridden via environment variables. The
//! precedence is: explicit `Config::load(path)` < env vars < CLI args.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub socket_path: PathBuf,
    pub api_bind: String,
    pub policy_dir: PathBuf,
    pub database_path: PathBuf,
    pub default_action: String,
    pub max_connections: usize,
    pub max_suspended: usize,
    pub default_timeout_secs: u64,
    pub hmac_secret: String,
    pub log_level: String,
    pub slack_webhook: Option<String>,
    pub generic_webhook: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            socket_path: default_socket_path(),
            api_bind: "127.0.0.1:7878".into(),
            policy_dir: PathBuf::from("./policies"),
            database_path: PathBuf::from("./toran.db"),
            default_action: "ALLOW".into(),
            max_connections: 10_000,
            max_suspended: 10_000,
            default_timeout_secs: 300,
            hmac_secret: "change-me-in-production".into(),
            log_level: "info".into(),
            slack_webhook: None,
            generic_webhook: None,
        }
    }
}

fn default_socket_path() -> PathBuf {
    if let Ok(p) = std::env::var("TORAN_SOCKET_PATH") {
        return PathBuf::from(p);
    }
    let dir = std::env::temp_dir();
    dir.join("toran.sock")
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let mut cfg = Self::default();
        if let Ok(v) = std::env::var("TORAN_SOCKET_PATH") {
            cfg.socket_path = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("TORAN_API_BIND") {
            cfg.api_bind = v;
        }
        if let Ok(v) = std::env::var("TORAN_POLICY_DIR") {
            cfg.policy_dir = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("TORAN_DATABASE_PATH") {
            cfg.database_path = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("TORAN_DEFAULT_ACTION") {
            cfg.default_action = v;
        }
        if let Ok(v) = std::env::var("TORAN_MAX_CONNECTIONS") {
            cfg.max_connections = v.parse().context("max_connections")?;
        }
        if let Ok(v) = std::env::var("TORAN_MAX_SUSPENDED") {
            cfg.max_suspended = v.parse().context("max_suspended")?;
        }
        if let Ok(v) = std::env::var("TORAN_DEFAULT_TIMEOUT") {
            cfg.default_timeout_secs = v.parse().context("default_timeout")?;
        }
        if let Ok(v) = std::env::var("TORAN_HMAC_SECRET") {
            cfg.hmac_secret = v;
        }
        if let Ok(v) = std::env::var("TORAN_LOG_LEVEL") {
            cfg.log_level = v;
        }
        if let Ok(v) = std::env::var("TORAN_SLACK_WEBHOOK") {
            cfg.slack_webhook = Some(v);
        }
        if let Ok(v) = std::env::var("TORAN_GENERIC_WEBHOOK") {
            cfg.generic_webhook = Some(v);
        }
        Ok(cfg)
    }

    pub fn load(path: &std::path::Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read config {}", path.display()))?;
        let cfg: Config =
            toml::from_str(&raw).with_context(|| format!("parse config {}", path.display()))?;
        Ok(cfg)
    }
}
