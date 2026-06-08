//! CLI: clap-based subcommands. `toran start` boots the server and
//! API; the other commands are operator conveniences for inspecting
//! state and resolving approvals from a shell.

use crate::api::ApiState;
use crate::api::router;
use crate::config::Config;
use crate::metrics::Metrics;
use crate::notification::Dispatcher;
use crate::policy::evaluator::Evaluator;
use crate::policy::loader::PolicyStore;
use crate::state::manager::{ApprovalStatus, StateManager};
use crate::state::sqlite::SqliteState;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "toran",
    version,
    about = "Runtime human-approval gatekeeper for AI agents"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to a TOML config file (overrides env).
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the socket server and REST API (default mode).
    Start,
    /// Validate all YAML policy files without starting the server.
    Validate,
    /// Print a one-line status summary.
    Status,
    /// List pending or recent approvals.
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Approve a pending approval by id.
    Approve {
        id: String,
        #[arg(long, default_value = "cli")]
        by: String,
        #[arg(long)]
        token: Option<String>,
    },
    /// Deny a pending approval by id.
    Deny {
        id: String,
        #[arg(long, default_value = "cli")]
        by: String,
        #[arg(long)]
        token: Option<String>,
    },
}

pub async fn run(cli: Cli) -> Result<()> {
    let cfg = match &cli.config {
        Some(p) => Config::load(p)?,
        None => Config::from_env()?,
    };
    init_tracing(&cfg.log_level);
    match cli.command {
        Commands::Start => start(cfg).await,
        Commands::Validate => validate(cfg).await,
        Commands::Status => status(cfg).await,
        Commands::List { status, limit } => list(cfg, status, limit).await,
        Commands::Approve { id, by, token } => {
            resolve(cfg, id, by, token, ApprovalStatus::Approved).await
        }
        Commands::Deny { id, by, token } => {
            resolve(cfg, id, by, token, ApprovalStatus::Denied).await
        }
    }
}

fn init_tracing(level: &str) {
    use tracing_subscriber::{EnvFilter, fmt};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("toran={level},tower_http=info")));
    let _ = fmt().with_env_filter(filter).try_init();
}

async fn start(cfg: Config) -> Result<()> {
    tracing::info!(?cfg.socket_path, api = %cfg.api_bind, "starting toran");
    let policies = PolicyStore::load(&cfg.policy_dir)
        .with_context(|| format!("load policy dir {}", cfg.policy_dir.display()))?;
    let state = SqliteState::open(&cfg.database_path)
        .with_context(|| format!("open db {}", cfg.database_path.display()))?;
    let evaluator = Arc::new(Evaluator::new());
    let metrics = Metrics::new();
    let dispatcher = Dispatcher::new(
        cfg.slack_webhook.clone(),
        cfg.generic_webhook.clone(),
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

    let app = router::build(api_state.clone());
    let listener = tokio::net::TcpListener::bind(&cfg.api_bind)
        .await
        .with_context(|| format!("bind {}", cfg.api_bind))?;
    tracing::info!(addr = %cfg.api_bind, "toran api listening");

    let server = Arc::new(crate::server::Server::new(
        cfg.clone(),
        policies.clone(),
        state.clone(),
        evaluator.clone(),
        metrics.clone(),
        dispatcher.clone(),
    ));
    let sock_handle = tokio::spawn(async move { server.run().await });
    let api_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!(error = %e, "api server exited");
        }
    });
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("ctrl-c received, shutting down");
        }
        r = sock_handle => { tracing::warn!(?r, "socket task ended"); }
        r = api_handle => { tracing::warn!(?r, "api task ended"); }
    }
    Ok(())
}

async fn validate(cfg: Config) -> Result<()> {
    let store = PolicyStore::load(&cfg.policy_dir)?;
    let (default, list) = store.snapshot();
    println!("✓ default action: {:?}", default);
    for p in &list {
        println!("✓ policy `{}` ({} rules)", p.name, p.rules.len());
    }
    Ok(())
}

async fn status(cfg: Config) -> Result<()> {
    let state = SqliteState::open(&cfg.database_path)?;
    let pending = state.count_pending()?;
    println!("pending_approvals: {pending}");
    println!("database: {}", cfg.database_path.display());
    println!("policy_dir: {}", cfg.policy_dir.display());
    println!("socket: {}", cfg.socket_path.display());
    println!("api: {}", cfg.api_bind);
    Ok(())
}

async fn list(cfg: Config, status: Option<String>, limit: usize) -> Result<()> {
    let state = SqliteState::open(&cfg.database_path)?;
    let q = crate::state::manager::ApprovalQuery {
        status: status.as_deref().and_then(ApprovalStatus::from_str),
        function_name: None,
        agent_id: None,
        limit: Some(limit),
        offset: None,
    };
    let rows = state.list_approvals(&q)?;
    if rows.is_empty() {
        println!("(no rows)");
        return Ok(());
    }
    for r in rows {
        println!(
            "{:>8}  {:<20}  {:<32}  {:<10}  agent={}  rule={}",
            r.id.chars().take(8).collect::<String>(),
            r.status.as_str(),
            r.function_name,
            format!("risk={}", r.risk_score),
            r.agent_id,
            r.policy_rule,
        );
    }
    Ok(())
}

async fn resolve(
    cfg: Config,
    id: String,
    by: String,
    token: Option<String>,
    status: ApprovalStatus,
) -> Result<()> {
    let state = SqliteState::open(&cfg.database_path)?;
    if let Some(t) = token {
        let rec = state
            .get_approval(&id)?
            .ok_or_else(|| anyhow::anyhow!("not found"))?;
        if !crate::security::ct_eq(&rec.notify_token, &t) {
            anyhow::bail!("invalid token");
        }
    }
    let updated = state.resolve_approval(&id, status, &by, None)?;
    println!("resolved: {} -> {}", updated.id, updated.status.as_str());
    Ok(())
}
