//! Launch/foreground lifecycle against the real core with FakePlatform
//! (REQ-SCR-09, REQ-PLT-03, REQ-PLT-04). Demo/no credentials only, and the
//! broadcast fetch is disabled in test builds, so no test reaches a host.

use std::path::PathBuf;
use std::sync::Arc;

use snackpilot_model::navigation::{ListKind, SettingsDetail, Tab};
use snackpilot_model::{menu_key, AppModel, FakePlatform};

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

/// @req REQ-UI-04
#[test]
fn foreground_preserves_workspace_identities() {
    let platform = Arc::new(FakePlatform::new());
    let model = AppModel::new(temp_dir("fg-identity"), platform).unwrap();
    model.login("demo", "demo1234!").unwrap();

    // Build up the desktop workspace state: pending cart, previews, cursor,
    // settings detail and the past segment.
    let day = model
        .state()
        .selected_day
        .clone()
        .expect("demo day selected");
    let item = model
        .state()
        .menu_snapshot
        .clone()
        .expect("menus loaded")
        .items
        .into_iter()
        .find(|i| i.day == day)
        .expect("demo day has items");
    let key = menu_key(&item);
    model.toggle(&item);
    model.open_preview(ListKind::Menus, &key);
    model.set_cursor(ListKind::Menus, Some(key.clone()));
    model.open_preview(ListKind::Receipts, "g-1");
    model.open_settings_detail(SettingsDetail::Data);
    model.set_orders_segment(true);

    model.on_foreground();

    assert_eq!(model.state().navigation.active_tab(), Tab::Menus);
    assert_eq!(
        model.preview(ListKind::Menus).as_deref(),
        Some(key.as_str()),
        "foreground must keep the menus preview"
    );
    assert_eq!(
        model.cursor(ListKind::Menus).as_deref(),
        Some(key.as_str()),
        "foreground must keep the menus cursor"
    );
    assert_eq!(
        model.preview(ListKind::Receipts).as_deref(),
        Some("g-1"),
        "foreground must keep the receipts preview"
    );
    assert_eq!(
        model.settings_detail(),
        Some(SettingsDetail::Data),
        "foreground must keep the settings detail"
    );
    assert!(
        model.state().orders_past,
        "foreground must keep the segment"
    );
    assert!(
        model.has_pending_changes(),
        "foreground must keep the pending cart"
    );
}
