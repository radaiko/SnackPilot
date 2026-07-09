//! Time and date-format helpers. Vienna-time cutoffs, all Gourmet/Ventopay wire formats,
//! and a Clock trait for deterministic tests (docs/architecture §3, 01 §12, orders.md §4.2).
pub mod formats;
pub use formats::*;

/// Injected clock so cutoff/notification logic is deterministic in tests (06-testing §9.3).
pub trait Clock: Send + Sync {
    fn now_epoch_ms(&self) -> i64;
}

pub struct SystemClock;
impl Clock for SystemClock {
    fn now_epoch_ms(&self) -> i64 {
        chrono::Utc::now().timestamp_millis()
    }
}

pub struct FixedClock {
    pub epoch_ms: i64,
}
impl Clock for FixedClock {
    fn now_epoch_ms(&self) -> i64 {
        self.epoch_ms
    }
}
