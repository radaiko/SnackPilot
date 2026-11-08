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
