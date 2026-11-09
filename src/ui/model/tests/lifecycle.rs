//! Launch/foreground lifecycle against the real core with FakePlatform
//! (REQ-SCR-09, REQ-PLT-03, REQ-PLT-04). Demo/no credentials only, and the
//! broadcast fetch is disabled in test builds, so no test reaches a host.

use std::path::PathBuf;
use std::sync::Arc;

use snackpilot_model::{AppModel, FakePlatform};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "snackpilot-model-lifecycle-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// @req REQ-PLT-04
#[test]
fn bootstrap_tracks_app_launched() {
    let platform = Arc::new(FakePlatform::new());
    let model = AppModel::new(temp_dir("launched"), platform.clone()).unwrap();
    model.bootstrap();
    assert!(platform
        .tracks()
        .iter()
        .any(|(event, _)| event == "app.launched"));
}

/// @req REQ-PLT-03
#[test]
fn geofence_restore_runs_once_across_two_launches() {
    let platform = Arc::new(FakePlatform::new());
    platform.set_location(Some(snackpilot_model::LocationFix {
        latitude: 48.2,
        longitude: 16.4,
    }));
    let model = AppModel::new(temp_dir("geofence"), platform.clone()).unwrap();
    model.set_company_location_from_current_position();
    let before = platform
        .calls()
        .iter()
        .filter(|c| *c == "start_geofence")
        .count();
    model.bootstrap();
    model.bootstrap();
    let after = platform
        .calls()
        .iter()
        .filter(|c| *c == "start_geofence")
        .count();
    assert_eq!(after - before, 1, "geofence restore must be idempotent");
}

/// @req REQ-SCR-09
#[test]
fn dismissed_broadcast_returns_on_the_next_foreground() {
    let platform = Arc::new(FakePlatform::new());
    let model = AppModel::new(temp_dir("banner"), platform).unwrap();
    model.dismiss_broadcast();
    assert!(model.state().broadcast_dismissed);
    model.on_foreground();
    assert!(!model.state().broadcast_dismissed);
}
