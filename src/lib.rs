//! Toran: runtime human-approval gatekeeper for AI agents.
//!
//! Phase 1: Rust core engine. Loads YAML policies, evaluates requests,
//! suspends on REQUIRE_APPROVAL, persists in SQLite, and exposes both
//! a Unix-socket protocol (for the Python SDK) and an Axum REST API
//! (for the dashboard + webhook adapters).

#![warn(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::new_without_default)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::unnecessary_sort_by)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unused_self)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::ptr_arg)]

pub mod api;
pub mod cli;
pub mod config;
pub mod metrics;
pub mod notification;
pub mod policy;
pub mod protocol;
pub mod security;
pub mod server;
pub mod state;
