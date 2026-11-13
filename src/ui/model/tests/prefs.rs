//! Task 3.3: appearance preferences persist through the core `FileKv` in the
//! model's storage dir (themes §1.2). Demo mode only, no network.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use snackpilot_core::storage::{FileKv, Kv};
use snackpilot_model::{
    AppModel, FakePlatform, ThemePreference, REMINDER_PICKER_DEFAULT_HOUR,
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
fn defaults_are_system_reminder_off_with_no_stored_time() {
    let dir = temp_dir("defaults");
    let fake = Arc::new(FakePlatform::new());
    let model = model(&dir, &fake);
    let s = model.state();
    assert_eq!(s.theme, ThemePreference::System);
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
}

// @req REQ-UI-05
// @req REQ-SCR-07
#[test]
fn theme_round_trips_independently() {
    let dir = temp_dir("round_trip");
    let fake = Arc::new(FakePlatform::new());
    let m1 = model(&dir, &fake);

    m1.set_theme_preference(ThemePreference::Dark);

    // Applies immediately in the running instance, no restart needed
    // (04-ui-ux §3.7 "applying immediately", specs/ui-screens REQ-SCR-07).
    let s = m1.state();
    assert_eq!(s.theme, ThemePreference::Dark);

    // Persisted under the v1 UserDefaults key (AppModel.swift:54-57).
    let kv = kv(&dir);
    assert_eq!(kv.get("theme_preference").unwrap().as_deref(), Some("dark"));

    // A fresh model over the same dir restores it.
    let m2 = model(&dir, &fake);
    assert_eq!(m2.state().theme, ThemePreference::Dark);

    m2.set_theme_preference(ThemePreference::Light);
    assert_eq!(m2.state().theme, ThemePreference::Light);
    assert_eq!(
        kv.get("theme_preference").unwrap().as_deref(),
        Some("light")
    );
    assert_eq!(kv.get("accent_color").unwrap(), None);
}
