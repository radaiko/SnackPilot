//! Design D2: the UI links `snackpilot-core` as plain Rust (no UniFFI runtime
//! in this process) and drives its async API from the UI's own tokio runtime.
//! Runs entirely in the core's offline demo mode.
//!
//! @req REQ-UI-08

use std::path::PathBuf;

use snackpilot_core::domain::Credentials;
use snackpilot_core::ffi::{core_version, is_demo_credentials, CoreConfig, SnackPilotCore};

fn temp_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "snackpilot-ffi-link-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test]
async fn core_links_as_plain_rust_and_serves_demo_menus() {
    assert!(!core_version().is_empty());
    assert!(is_demo_credentials("demo".into(), "demo1234!".into()));

    let core = SnackPilotCore::new(
        CoreConfig {
            storage_dir: temp_dir().to_str().unwrap().to_string(),
        },
        None,
    )
    .unwrap();

    let info = core
        .gourmet_login(Credentials {
            username: "demo".into(),
            password: "demo1234!".into(),
        })
        .await
        .unwrap();
    assert_eq!(info.username, "Demo User");
    assert!(core.gourmet_is_authenticated());

    let snapshot = core.fetch_menus(false).await.unwrap();
    assert!(
        !snapshot.available_dates.is_empty(),
        "demo mode must serve a complete offline menu snapshot"
    );
    assert_eq!(
        core.menu_snapshot().available_dates.len(),
        snapshot.available_dates.len()
    );
}
