//! Lightweight in-process metrics: evaluation counts, latencies,
//! pending totals, and a Prometheus-format exporter.

use parking_lot::Mutex;
use std::sync::Arc;

pub struct Metrics {
    evaluations: Mutex<u64>,
    total_ns: Mutex<u128>,
    pending: Mutex<u64>,
    approvals_resolved: Mutex<u64>,
    notifications_sent: Mutex<u64>,
    notifications_failed: Mutex<u64>,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            evaluations: Mutex::new(0),
            total_ns: Mutex::new(0),
            pending: Mutex::new(0),
            approvals_resolved: Mutex::new(0),
            notifications_sent: Mutex::new(0),
            notifications_failed: Mutex::new(0),
        })
    }

    pub fn record_evaluation(&self, elapsed_ns: u128) {
        *self.evaluations.lock() += 1;
        *self.total_ns.lock() += elapsed_ns;
    }
    pub fn record_pending(&self) {
        *self.pending.lock() += 1;
    }
    pub fn record_resolved(&self) {
        *self.approvals_resolved.lock() += 1;
    }
    pub fn record_notification(&self, ok: bool) {
        if ok {
            *self.notifications_sent.lock() += 1;
        } else {
            *self.notifications_failed.lock() += 1;
        }
    }

    pub fn render_prometheus(&self) -> String {
        let e = *self.evaluations.lock();
        let t = *self.total_ns.lock();
        let avg_ns = if e == 0 { 0 } else { t / e as u128 };
        let p = *self.pending.lock();
        let r = *self.approvals_resolved.lock();
        let ns_ok = *self.notifications_sent.lock();
        let ns_fail = *self.notifications_failed.lock();
        format!(
            "# HELP toran_evaluations_total Total policy evaluations\n\
             # TYPE toran_evaluations_total counter\n\
             toran_evaluations_total {e}\n\
             # HELP toran_eval_avg_ns Average evaluation latency (ns)\n\
             # TYPE toran_eval_avg_ns gauge\n\
             toran_eval_avg_ns {avg_ns}\n\
             # HELP toran_pending_approvals Approvals created\n\
             # TYPE toran_pending_approvals counter\n\
             toran_pending_approvals {p}\n\
             # HELP toran_approvals_resolved Approvals resolved\n\
             # TYPE toran_approvals_resolved counter\n\
             toran_approvals_resolved {r}\n\
             # HELP toran_notifications_sent Notifications successfully sent\n\
             # TYPE toran_notifications_sent counter\n\
             toran_notifications_sent {ns_ok}\n\
             # HELP toran_notifications_failed Notifications failed\n\
             # TYPE toran_notifications_failed counter\n\
             toran_notifications_failed {ns_fail}\n"
        )
    }
}
