//! Launch/foreground lifecycle against the real core with FakePlatform
//! (REQ-SCR-09, REQ-PLT-03, REQ-PLT-04). Demo/no credentials only, and the
//! broadcast fetch is disabled in test builds, so no test reaches a host.

use std::path::PathBuf;
use std::sync::Arc;

use snackpilot_model::navigation::{ListKind, SettingsDetail, Tab};
use snackpilot_model::{menu_key, AppModel, FakePlatform, Platform};

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

/// The debug evidence harness: the model runs its launch and foreground order
/// against isolated data and dummy credentials, never reads or writes a normal
/// credential slot, and never reaches a live broadcast.
/// @req REQ-UI-12
/// @req REQ-SCR-09
#[test]
fn evidence_harness_isolates_startup_and_normal_slots() {
    // The host platform is the real one; the harness wraps it, so any
    // credential access would be recorded here.
    let real = Arc::new(FakePlatform::new());
    let harness: Arc<dyn Platform> = AppModel::evidence_platform(real.clone());
    let model = AppModel::new(temp_dir("evidence"), harness).unwrap();
    assert!(
        !model.evidence_mode(),
        "the harness is off until the host asks"
    );

    model.set_evidence_mode(true);
    assert!(model.evidence_mode());
    model.bootstrap();

    let calls = real.calls();
    assert!(
        calls.iter().all(|call| !call.contains("credential")),
        "the harness must never touch the normal credential slots: {calls:?}"
    );
    assert!(
        real.tracks().is_empty(),
        "evidence mode must not send analytics: {:?}",
        real.tracks()
    );
    assert_eq!(real.credential("gourmet_username"), None);
    assert!(
        model.state().gourmet_authenticated,
        "the dummy demo pair opens the offline session"
    );
    assert!(
        model.state().broadcast_dismissed,
        "no live broadcast banner in an evidence run"
    );
    let (user, pass) = model.saved_credentials(false);
    assert_eq!(user.as_deref(), Some("demo"), "dummy username, not a slot");
    assert_eq!(
        pass.as_deref(),
        Some("demo1234!"),
        "dummy password, not a slot"
    );

    // A foreground return keeps the same isolation.
    model.on_foreground();
    let calls = real.calls();
    assert!(
        calls.iter().all(|call| !call.contains("credential")),
        "resume must not touch the normal slots either: {calls:?}"
    );
    assert!(
        real.tracks().is_empty(),
        "resume must not send analytics either"
    );
    assert!(model.state().broadcast_dismissed);

    model.set_evidence_mode(false);
    assert!(!model.evidence_mode(), "the next launch is a normal one");
}

/// A desktop host declares notifications and geofence unavailable, so its
/// bootstrap must not call either service. Cache-first loading, the
/// `app.launched` event and the saved-credential session still run
/// (REQ-PLT-07, REQ-DST-04, REQ-SCR-09).
/// @req REQ-PLT-07
/// @req REQ-DST-04
/// @req REQ-SCR-09
#[test]
fn bootstrap_gates_unavailable_desktop_capabilities() {
    use snackpilot_model::LocationFix;

    // `FakePlatform::default()` reports both capabilities off, like
    // `DesktopPlatform`; `new()` reports both on, like the mobile bridges.
    let platform = Arc::new(FakePlatform::default());
    let model = AppModel::new(temp_dir("capability-gate"), platform.clone()).unwrap();
    platform.set_location(Some(LocationFix {
        latitude: 48.2,
        longitude: 16.4,
    }));
    model.set_company_location_from_current_position();
    let before_bootstrap = platform.calls().len();

    model.bootstrap();

    let calls: Vec<String> = platform
        .calls()
        .into_iter()
        .skip(before_bootstrap)
        .collect();
    assert!(
        !calls
            .iter()
            .any(|call| call == "request_notification_permission"),
        "desktop bootstrap must not request an unavailable notification service: {calls:?}"
    );
    assert!(
        !calls.iter().any(|call| call == "start_geofence"),
        "desktop bootstrap must not arm an unavailable geofence service: {calls:?}"
    );
    assert!(
        calls.iter().any(|call| call == "track(app.launched)"),
        "the launch event still runs on desktop: {calls:?}"
    );
    assert!(
        !platform.tracks().is_empty(),
        "the model still tracks app.launched; the desktop platform makes it a no-op"
    );
}

/// Mobile hosts keep the documented startup order: `app.launched`, then the
/// notification permission, then the geofence restore (REQ-SCR-09).
/// @req REQ-SCR-09
#[test]
fn bootstrap_keeps_mobile_capability_order() {
    use snackpilot_model::LocationFix;

    let platform = Arc::new(FakePlatform::new());
    let model = AppModel::new(temp_dir("capability-order"), platform.clone()).unwrap();
    platform.set_location(Some(LocationFix {
        latitude: 48.2,
        longitude: 16.4,
    }));
    model.set_company_location_from_current_position();
    let before_bootstrap = platform.calls().len();

    model.bootstrap();

    let calls: Vec<String> = platform
        .calls()
        .into_iter()
        .skip(before_bootstrap)
        .collect();
    let position = |name: &str| {
        calls
            .iter()
            .position(|call| call == name)
            .unwrap_or_else(|| panic!("{name} missing from {calls:?}"))
    };
    let launched = position("track(app.launched)");
    let permission = position("request_notification_permission");
    let geofence = position("start_geofence");
    assert!(
        launched < permission && permission < geofence,
        "unchanged mobile order: launched < notification < geofence, got {calls:?}"
    );
}

/// The mobile evidence harness is offline end to end: it never reads or writes
/// a normal credential slot, never asks a system service or provider, never
/// sends analytics, and every synthetic scenario stays inside the isolated
/// store. No failure fixture sends wrong credentials anywhere real.
/// @req REQ-UI-08
/// @req REQ-SCR-09
#[test]
fn mobile_evidence_never_touches_normal_credentials_or_live_hosts() {
    use snackpilot_model::desktop::HostKind;

    // The real host is the fake; the harness wraps it, so any leak shows up
    // in its recorded calls or credentials.
    let real = Arc::new(FakePlatform::new());
    let model = AppModel::new(
        temp_dir("mobile-evidence-isolation"),
        AppModel::evidence_platform(real.clone()),
    )
    .unwrap();
    model.set_evidence_mode(true);
    model.set_host_kind(HostKind::Phone);
    model.set_content_width(390.);
    model.bootstrap();

    assert!(
        real.calls().is_empty(),
        "an isolated launch must not call the host platform: {:?}",
        real.calls()
    );
    assert!(
        real.tracks().is_empty(),
        "no analytics leaves an evidence run: {:?}",
        real.tracks()
    );
    for slot in [
        "gourmet_username",
        "gourmet_password",
        "ventopay_username",
        "ventopay_password",
    ] {
        assert_eq!(real.credential(slot), None, "normal slot {slot} untouched");
    }
    assert!(
        model.state().gourmet_authenticated,
        "the in-memory dummy pair opens the offline demo session"
    );
    assert!(
        model.state().broadcast_dismissed,
        "no live broadcast banner in an evidence run"
    );

    // Every synthetic scenario stays synthetic: no credential access, no
    // provider call, no analytics (including the permission result and the
    // pending/progress fixtures). The entry is absent from a release build.
    #[cfg(debug_assertions)]
    for scenario in [
        "populated",
        "empty",
        "loading",
        "refresh",
        "error",
        "pending",
        "progress",
        "permission-granted",
        "permission-denied",
    ] {
        model.apply_evidence_scenario(scenario);
        assert!(
            real.calls().is_empty(),
            "{scenario:?} reached the host platform: {:?}",
            real.calls()
        );
        assert!(
            real.tracks().is_empty(),
            "{scenario:?} sent analytics: {:?}",
            real.tracks()
        );
    }
    for slot in [
        "gourmet_username",
        "gourmet_password",
        "ventopay_username",
        "ventopay_password",
    ] {
        assert_eq!(
            real.credential(slot),
            None,
            "scenario runs still leave {slot} untouched"
        );
    }

    // A resume keeps the same isolation.
    model.on_foreground();
    assert!(real.calls().is_empty(), "resume must stay offline");
    assert!(real.tracks().is_empty(), "resume must not send analytics");
}

/// The evidence run keeps one root across viewport changes and a foreground
/// return: the phone layout stays single-column at 360/390/768/1280, and the
/// selected day, pending selection and preview survive without a second model
/// or a launch-order rerun.
/// @req REQ-UI-03
#[test]
fn mobile_evidence_resize_and_resume_retains_single_root() {
    use snackpilot_model::desktop::{HostKind, LayoutMode};

    let real = Arc::new(FakePlatform::new());
    let model = Arc::new(
        AppModel::new(
            temp_dir("mobile-evidence-root"),
            AppModel::evidence_platform(real.clone()),
        )
        .unwrap(),
    );
    model.set_evidence_mode(true);
    model.set_host_kind(HostKind::Phone);
    model.set_content_width(390.);
    model.bootstrap();

    let day = model
        .state()
        .selected_day
        .clone()
        .expect("demo day selected");
    let item = model
        .state()
        .menu_snapshot
        .clone()
        .expect("demo menus loaded")
        .items
        .into_iter()
        .find(|i| i.day == day)
        .expect("demo day has items");
    let key = menu_key(&item);
    model.toggle(&item);
    model.open_preview(ListKind::Menus, &key);
    assert!(model.has_pending_changes());

    let root = Arc::clone(&model);
    for width in [360., 768., 1280., 390.] {
        model.set_content_width(width);
        assert_eq!(
            model.layout_mode(),
            LayoutMode::Phone,
            "a phone host keeps the single-column Day Tray at {width}"
        );
        assert_eq!(
            model.state().selected_day.as_deref(),
            Some(day.as_str()),
            "selected day survives the resize to {width}"
        );
        assert!(
            model.has_pending_changes(),
            "pending selection survives the resize to {width}"
        );
        assert_eq!(
            model.preview(ListKind::Menus).as_deref(),
            Some(key.as_str()),
            "menus preview survives the resize to {width}"
        );
        assert!(Arc::ptr_eq(&root, &model), "one model root at {width}");
        assert!(model.evidence_mode(), "still the isolated store at {width}");
    }

    model.on_foreground();
    assert!(
        Arc::ptr_eq(&root, &model),
        "resume keeps the one model root"
    );
    assert_eq!(
        model.state().selected_day.as_deref(),
        Some(day.as_str()),
        "resume keeps the selected day"
    );
    assert!(
        model.has_pending_changes(),
        "resume keeps the pending selection"
    );
    assert_eq!(
        model.preview(ListKind::Menus).as_deref(),
        Some(key.as_str()),
        "resume keeps the preview"
    );
    assert!(
        real.calls().is_empty(),
        "resize/resume stay offline: {:?}",
        real.calls()
    );
    assert!(real.tracks().is_empty(), "no analytics on resume");
}
