//! Task 3.3: appearance preferences persist through the core `FileKv` in the
//! model's storage dir (themes §1.2), and the accent switch drives the app
//! icon (themes §6). Demo mode only, no network.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use snackpilot_core::storage::{FileKv, Kv};
use snackpilot_model::{
    AccentColor, AppModel, FakePlatform, ThemePreference, REMINDER_PICKER_DEFAULT_HOUR,
    REMINDER_PICKER_DEFAULT_MINUTE,
};

/// Storage dir removed best-effort when the test ends.
struct TestDir(PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl std::ops::Deref for TestDir {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

fn temp_dir(tag: &str) -> TestDir {
    let dir = std::env::temp_dir().join(format!(
        "snackpilot-model-prefs-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    TestDir(dir)
}

fn kv(dir: &Path) -> FileKv {
    FileKv::new(dir.to_path_buf())
}

fn model(dir: &Path, fake: &Arc<FakePlatform>) -> AppModel {
    AppModel::new(dir.to_path_buf(), fake.clone()).unwrap()
}

// @req REQ-UI-05
#[test]
fn defaults_are_system_orange_reminder_off_with_no_stored_time() {
    let dir = temp_dir("defaults");
    let fake = Arc::new(FakePlatform::new());
    let model = model(&dir, &fake);
    let s = model.state();
    assert_eq!(s.theme, ThemePreference::System);
    assert_eq!(s.accent, AccentColor::Orange);
    assert!(!s.daily_reminder_enabled);
    // v1 parity: no stored/default time until the feature is first enabled
    // (notifications-daily-reminder.md:213-217; the picker starts at 11:00,
    // AppModel.swift:63).
    assert_eq!(s.reminder_time, None);
    assert_eq!(
        (REMINDER_PICKER_DEFAULT_HOUR, REMINDER_PICKER_DEFAULT_MINUTE),
        (11, 0)
    );
    let kv = kv(&dir);
    assert_eq!(kv.get("daily_reminder_hour").unwrap(), None);
    assert_eq!(kv.get("daily_reminder_minute").unwrap(), None);
    // Loading the defaults must not count as an explicit user change
    // (AppModel.swift:655-659: setAccent is the only icon call site).
    assert!(
        fake.calls().iter().all(|c| !c.starts_with("set_app_icon")),
        "startup must not switch the icon: {:?}",
        fake.calls()
    );
}

// @req REQ-UI-05
// @req REQ-SCR-07
#[test]
fn theme_and_accent_round_trip_independently() {
    let dir = temp_dir("round_trip");
    let fake = Arc::new(FakePlatform::new());
    let m1 = model(&dir, &fake);

    m1.set_theme_preference(ThemePreference::Dark);
    m1.set_accent_color(AccentColor::Emerald);

    // Both apply immediately in the running instance, no restart needed
    // (04-ui-ux §3.7 "applying immediately", specs/ui-screens REQ-SCR-07).
    let s = m1.state();
    assert_eq!(s.theme, ThemePreference::Dark);
    assert_eq!(s.accent, AccentColor::Emerald);

    // Persisted under the v1 UserDefaults keys (AppModel.swift:54-57).
    let kv = kv(&dir);
    assert_eq!(kv.get("theme_preference").unwrap().as_deref(), Some("dark"));
    assert_eq!(kv.get("accent_color").unwrap().as_deref(), Some("emerald"));

    // A fresh model over the same dir restores both.
    let m2 = model(&dir, &fake);
    let s = m2.state();
    assert_eq!(s.theme, ThemePreference::Dark);
    assert_eq!(s.accent, AccentColor::Emerald);

    // Changing one never changes the other (AppModel.swift:52).
    m2.set_theme_preference(ThemePreference::Light);
    let s = m2.state();
    assert_eq!(s.theme, ThemePreference::Light);
    assert_eq!(s.accent, AccentColor::Emerald);
    assert_eq!(kv.get("accent_color").unwrap().as_deref(), Some("emerald"));

    m2.set_accent_color(AccentColor::Berry);
    let s = m2.state();
    assert_eq!(s.accent, AccentColor::Berry);
    assert_eq!(s.theme, ThemePreference::Light);
    assert_eq!(
        kv.get("theme_preference").unwrap().as_deref(),
        Some("light")
    );
    assert_eq!(kv.get("accent_color").unwrap().as_deref(), Some("berry"));
}

// @req REQ-PLT-05
#[test]
fn accent_change_calls_set_app_icon_once_and_only_for_explicit_changes() {
    let dir = temp_dir("icon");
    let fake = Arc::new(FakePlatform::new());
    let m1 = model(&dir, &fake);

    m1.set_accent_color(AccentColor::Ocean);
    let icon_calls: Vec<_> = fake
        .calls()
        .into_iter()
        .filter(|c| c.starts_with("set_app_icon"))
        .collect();
    assert_eq!(icon_calls, vec!["set_app_icon(ocean)"]);

    // Theme changes never touch the icon (AppModel.swift:646-651).
    m1.set_theme_preference(ThemePreference::Dark);
    assert_eq!(
        fake.calls()
            .iter()
            .filter(|c| c.starts_with("set_app_icon"))
            .count(),
        1
    );

    // Relaunch restores the accent without an icon call (no startup
    // reconciliation, AppModel.swift:656-657).
    let fake2 = Arc::new(FakePlatform::new());
    let m2 = model(&dir, &fake2);
    assert_eq!(m2.state().accent, AccentColor::Ocean);
    assert!(fake2.calls().iter().all(|c| !c.starts_with("set_app_icon")));
}
