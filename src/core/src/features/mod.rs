//! Feature services — the in-memory state + orchestration v1 kept in Zustand stores
//! (03-features/{menus,orders,billing}.md; docs/architecture §3.4). The shells render
//! snapshots and call operations; no business logic lives in the shells.
pub mod billing;
pub mod menus;
pub mod orders;

use crate::domain::OrderProgress;

/// Fire-and-forget analytics emission (docs/architecture §4.1; 03-features/analytics.md).
/// The core emits core-originated events; the shell forwards them to TelemetryDeck.
pub trait AnalyticsSink: Send + Sync {
    fn track(&self, event: &str, props: Vec<(String, String)>);
}

/// Drives the order-submit progress banner (docs/architecture §4.1). `None` = finished.
pub trait ProgressListener: Send + Sync {
    fn on_progress(&self, phase: Option<OrderProgress>);
}
