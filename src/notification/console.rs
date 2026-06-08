//! Console adapter: always on, prints a one-line summary. Useful
//! in development and as a guaranteed last-resort delivery path.

use super::dispatcher::{Adapter, NotificationEvent};
use async_trait::async_trait;

pub struct ConsoleAdapter;

#[async_trait]
impl Adapter for ConsoleAdapter {
    fn name(&self) -> &'static str {
        "console"
    }
    async fn send(&self, event: &NotificationEvent) -> anyhow::Result<()> {
        tracing::info!(
            approval_id = %event.approval_id,
            function = %event.function_name,
            agent = %event.agent_id,
            rule = %event.policy_rule,
            risk = event.risk_score,
            "APPROVAL REQUIRED: visit {}",
            event.dashboard_url,
        );
        Ok(())
    }
}
