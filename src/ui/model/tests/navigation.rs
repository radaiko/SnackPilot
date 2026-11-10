//! Navigation model: tab switching, stack push/pop, back.
//! @req REQ-UI-03 @req REQ-UI-04

use snackpilot_model::navigation::{Direction, Navigation, Screen, Tab};

#[test]
/// @req REQ-UI-03
fn starts_on_menus_tab_with_single_entry_stack() {
    let nav = Navigation::new();
    assert_eq!(nav.active_tab(), Tab::Menus);
    assert_eq!(nav.stack(), [Screen::Menus]);
    assert!(!nav.can_go_back());
}

#[test]
/// @req REQ-UI-04
fn switching_to_another_tab_resets_the_stack_to_that_tab() {
    let mut nav = Navigation::new();
    nav.push(Screen::SettingsDetail);
    nav.select_tab(Tab::Billing);
    assert_eq!(nav.active_tab(), Tab::Billing);
    assert_eq!(nav.stack(), [Screen::Billing]);
    assert!(!nav.can_go_back());
    assert_eq!(nav.direction(), Direction::Forward);
}

#[test]
/// @req REQ-UI-04
fn re_tapping_the_active_tab_is_a_no_op() {
    let mut nav = Navigation::new();
    nav.select_tab(Tab::Orders);
    nav.push(Screen::SettingsDetail);
    let before = (nav.active_tab(), nav.stack().to_vec());
    nav.select_tab(Tab::Orders);
    assert_eq!(nav.active_tab(), before.0);
    assert_eq!(nav.stack(), before.1);
    assert_eq!(nav.direction(), Direction::Forward);
}

#[test]
/// @req REQ-UI-04
fn tab_metadata_matches_the_spec() {
    assert_eq!(
        Tab::ALL,
        [Tab::Menus, Tab::Orders, Tab::Billing, Tab::Settings]
    );
    assert_eq!(Tab::Menus.title(), "Menüs");
    assert_eq!(Tab::Orders.title(), "Bestellungen");
    assert_eq!(Tab::Billing.title(), "Abrechnung");
    assert_eq!(Tab::Settings.title(), "Einstellungen");
    assert_eq!(Tab::Menus.icon_name(), "utensils");
    assert_eq!(Tab::Orders.icon_name(), "receipt");
    assert_eq!(Tab::Billing.icon_name(), "wallet");
    assert_eq!(Tab::Settings.icon_name(), "settings");
    for tab in Tab::ALL {
        assert_eq!(tab.screen(), Screen::from(tab));
    }
}

#[test]
/// @req REQ-UI-04
fn push_extends_the_stack_and_back_pops_it() {
    let mut nav = Navigation::new();
    nav.select_tab(Tab::Settings);
    nav.push(Screen::SettingsDetail);
    assert_eq!(nav.top(), Screen::SettingsDetail);
    assert!(nav.can_go_back());
    assert_eq!(nav.direction(), Direction::Forward);

    let popped = nav.pop();
    assert_eq!(popped, Some(Screen::SettingsDetail));
    assert_eq!(nav.top(), Screen::Settings);
    assert_eq!(nav.direction(), Direction::Back);
    assert!(!nav.can_go_back());
}

#[test]
/// @req REQ-UI-04
fn pop_on_the_root_screen_does_nothing() {
    let mut nav = Navigation::new();
    let popped = nav.pop();
    assert_eq!(popped, None);
    assert_eq!(nav.stack(), [Screen::Menus]);
}

#[test]
/// @req REQ-UI-04
fn multiple_pushes_pop_in_reverse_order() {
    let mut nav = Navigation::new();
    nav.push(Screen::SettingsDetail);
    nav.push(Screen::Orders);
    assert_eq!(nav.depth(), 3);
    assert_eq!(nav.pop(), Some(Screen::Orders));
    assert_eq!(nav.pop(), Some(Screen::SettingsDetail));
    assert_eq!(nav.pop(), None);
    assert_eq!(nav.top(), Screen::Menus);
}

// --- model-owned workspace identities (REQ-UI-04) ---------------------------

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use snackpilot_model::navigation::{ListKind, SettingsDetail};
use snackpilot_model::{menu_key, AppModel, FakePlatform};

fn temp_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "snackpilot-model-nav-{tag}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn demo_model(tag: &str) -> (AppModel, Arc<FakePlatform>) {
    let fake = Arc::new(FakePlatform::new());
    let model = AppModel::new(temp_dir(tag), fake.clone()).unwrap();
    model.login("demo", "demo1234!").unwrap();
    (model, fake)
}

fn first_item(model: &AppModel) -> snackpilot_core::domain::MenuItem {
    let day = model
        .state()
        .selected_day
        .clone()
        .expect("demo day selected");
    model
        .state()
        .menu_snapshot
        .clone()
        .expect("menus loaded")
        .items
        .into_iter()
        .find(|i| i.day == day)
        .expect("demo day has items")
}

#[test]
/// @req REQ-UI-04
fn active_tab_activation_is_a_no_op() {
    let (model, _fake) = demo_model("tab-noop");

    let notifies = Arc::new(AtomicUsize::new(0));
    let counter = notifies.clone();
    model.subscribe(Box::new(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
    }));
    let baseline = notifies.load(Ordering::SeqCst);

    let active = model.state().navigation.active_tab();
    assert!(
        !model.select_tab(active),
        "activating the active tab must change nothing"
    );
    assert_eq!(
        notifies.load(Ordering::SeqCst),
        baseline,
        "a no-op tab activation must not notify subscribers"
    );

    let other = if active == Tab::Orders {
        Tab::Billing
    } else {
        Tab::Orders
    };
    assert!(model.select_tab(other), "switching tabs must change state");
    assert_eq!(notifies.load(Ordering::SeqCst), baseline + 1);
    assert_eq!(model.state().navigation.active_tab(), other);
}

#[test]
/// @req REQ-UI-04
fn switching_tabs_resets_the_departing_tab() {
    let (model, _fake) = demo_model("tab-reset");
    let item = first_item(&model);
    let key = menu_key(&item);

    model.toggle(&item);
    model.open_preview(ListKind::Menus, &key);
    model.set_cursor(ListKind::Menus, Some(key.clone()));
    model.open_settings_detail(SettingsDetail::Security);
    assert!(
        model.has_pending_changes(),
        "toggle must add a pending change"
    );

    assert!(model.select_tab(Tab::Orders));

    assert_eq!(
        model.preview(ListKind::Menus),
        None,
        "the departing tab's preview is cleared"
    );
    assert_eq!(
        model.cursor(ListKind::Menus),
        None,
        "the departing tab's cursor is cleared"
    );
    assert_eq!(
        model.settings_detail(),
        None,
        "a settings detail is cleared by a tab switch"
    );
    assert!(
        model.has_pending_changes(),
        "pending menu changes survive a tab switch"
    );
}
