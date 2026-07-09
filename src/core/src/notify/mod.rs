//! Notification decision logic — the portable "which notification should fire" halves of
//! v1's notification subsystem (03-features/notifications-*). The core computes commands;
//! the native shells deliver them (docs/architecture §3.5).
pub mod fingerprint;
