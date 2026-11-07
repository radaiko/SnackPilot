//! Model-owned tokio runtime (design D4): the UI thread is never a tokio
//! worker, so blocking on core calls from the UI is safe.
//!
//! A current-thread runtime only runs its scheduler and its IO/timer drivers
//! while `Runtime::block_on` is executing — `Handle::block_on` alone never
//! drives them. So the runtime lives on a dedicated service thread parked in
//! `Runtime::block_on`, and the session flows run through `Handle::block_on`
//! on the caller's thread while the service thread delivers timer and IO
//! events. Without that service thread every core network call would hang.

use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::Handle;

static HANDLE: OnceLock<Handle> = OnceLock::new();

fn runtime_handle() -> &'static Handle {
    HANDLE.get_or_init(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("model runtime");
        let handle = runtime.handle().clone();
        // Park the service thread inside the scheduler forever: it then runs
        // every task, timer and IO event the flows queue through `handle()`.
        std::thread::Builder::new()
            .name("snackpilot-model-runtime".into())
            .spawn(move || runtime.block_on(std::future::pending::<()>()))
            .expect("model runtime service thread");
        handle
    })
}

/// The model runtime handle, for spawning core work.
pub fn handle() -> Handle {
    runtime_handle().clone()
}

/// Run a future on the model runtime exactly the way the session flows do.
/// Public for the driver regression test (`tests/session.rs`).
pub fn block_on_model<F: Future>(future: F) -> F::Output {
    handle().block_on(future)
}

/// Structured log line for model events.
pub fn log(event: &str, detail: &str) {
    println!("[model] {event}: {detail}");
}

/// Metrics hook for model events.
pub fn metrics(event: &str, value: u64) {
    println!("[model.metric] {event}={value}");
}
