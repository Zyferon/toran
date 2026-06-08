//! Axum REST API + embedded HTML/JS dashboard. Exposes the same
//! state as the socket server, plus endpoints for the web UI,
//! webhook receivers, and Prometheus metrics.

pub mod dashboard;
pub mod router;

use crate::config::Config;
use crate::metrics::Metrics;
use crate::notification::Dispatcher;
use crate::policy::loader::PolicyStore;
use crate::state::manager::StateManager;
use std::sync::Arc;

pub struct ApiState {
    pub config: Config,
    pub policies: Arc<PolicyStore>,
    pub state: Arc<dyn StateManager>,
    pub metrics: Arc<Metrics>,
    pub dispatcher: Arc<Dispatcher>,
    pub start_time: std::time::Instant,
}
