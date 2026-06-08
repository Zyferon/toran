//! Notification subsystem: a small trait, a dispatcher that fans out
//! to all configured adapters, and three adapters: Slack, generic
//! webhook, and console (always on, prints to logs).

pub mod console;
pub mod dispatcher;
pub mod slack;
pub mod webhook;

pub use dispatcher::{Dispatcher, NotificationEvent};
