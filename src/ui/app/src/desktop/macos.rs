//! Native macOS menu bar (desktop-ui-redesign brief "Native macOS host and
//! menu bar", REQ-UI-14).
//!
//! The inventory, German labels, order and separators are the approved brief's
//! verbatim, and every content item dispatches the same guarded intent as the
//! on-screen control: the checked destination comes from the model, refresh is
//! enabled only where it can act, and a modal disables every content command
//! that could escape it. Quit/Hide/Close/Minimise keep their native semantics.
//!
//! The menus are only installed on macOS; Windows gets the standard native
//! window controls and no global menu imitation (design D5).
//! @req REQ-UI-14

use gpui::{actions, Action, App, Menu, MenuItem, OsAction, SharedString, SystemMenuType};
use snackpilot_model::desktop::{command_allowed, DesktopCommand, KeyScope};
use snackpilot_model::navigation::{ListKind, Tab};
use snackpilot_model::AppModel;

// Every command the native menu dispatches. The AppShell receives them in its
// element tree, so one handler covers menu, shortcut and on-screen paths.
actions!(
    snackpilot_desktop,
    [
        MenuAbout,
        MenuOpenMenus,
        MenuOpenOrders,
        MenuOpenBilling,
        MenuOpenSettings,
        MenuRefresh,
        MenuBack,
        MenuQuit,
        MenuHide,
        MenuHideOthers,
        MenuShowAll,
        MenuCloseWindow,
        MenuMinimize,
        MenuZoom,
        MenuBringAllFront,
        MenuShowMainWindow
    ]
);

/// Refresh can act only on a reloadable tab while no flow is busy or claimed.
pub fn refresh_enabled(model: Option<&AppModel>) -> bool {
    let Some(model) = model else {
        return false;
    };
    let state = model.state();
    command_allowed(
        DesktopCommand::Refresh,
        model.key_scope(),
        state.navigation.active_tab(),
    ) && !state.busy
        && !state.billing_loading
        && model.claimed_command().is_none()
}

/// Navigation/settings commands work at every width and even while a text
/// field has focus (they are chords, not typing keys); only a modal captures
/// them (brief "Cmd+1–4, App, no modal").
pub fn content_enabled(model: Option<&AppModel>) -> bool {
    model.is_some_and(|model| model.key_scope() != KeyScope::Modal)
}

/// Back can act when Esc has a layer to unwind (modal, batch, preview or
/// settings detail); at the root it is disabled.
pub fn back_enabled(model: Option<&AppModel>) -> bool {
    let Some(model) = model else {
        return false;
    };
    let state = model.state();
    if state.modal_open || state.order_selection_mode || state.settings_detail.is_some() {
        return true;
    }
    ListKind::for_tab(state.navigation.active_tab())
        .is_some_and(|kind| model.preview(kind).is_some())
}

fn action(name: &str, action: impl Action, enabled: bool) -> MenuItem {
    MenuItem::action(SharedString::from(name.to_string()), action).disabled(!enabled)
}

fn checked(name: &str, action: impl Action, checked: bool, enabled: bool) -> MenuItem {
    MenuItem::action(SharedString::from(name.to_string()), action)
        .checked(checked)
        .disabled(!enabled)
}

/// The approved German menu inventory, in order. `has_window` is false while
/// the main window is closed: content commands then disable, while the
/// application menu and the global navigation remain present.
pub fn menus(model: Option<&AppModel>, has_window: bool) -> Vec<Menu> {
    let active = model
        .map(|model| model.state().navigation.active_tab())
        .unwrap_or(Tab::Menus);
    let content = has_window && content_enabled(model);
    let refresh = has_window && refresh_enabled(model);
    let back = has_window && back_enabled(model);
    // About opens a themed alert through the shell, so it needs both the
    // window and the same "no modal" guard as the other content commands:
    // showing it over a modal would replace that dialog.
    let about_enabled = content;

    let app = Menu::new("SnackPilot").items(vec![
        action("Über SnackPilot", MenuAbout, about_enabled),
        MenuItem::separator(),
        action("Einstellungen …", MenuOpenSettings, content),
        MenuItem::separator(),
        MenuItem::os_submenu("Dienste", SystemMenuType::Services),
        MenuItem::separator(),
        action("SnackPilot ausblenden", MenuHide, true),
        action("Andere ausblenden", MenuHideOthers, true),
        action("Alle einblenden", MenuShowAll, true),
        MenuItem::separator(),
        action("SnackPilot beenden", MenuQuit, true),
    ]);

    let file = Menu::new("Ablage").items(vec![action(
        "Fenster schließen",
        MenuCloseWindow,
        has_window,
    )]);

    let edit = Menu::new("Bearbeiten").items(vec![
        MenuItem::os_action("Widerrufen", MenuUndo, OsAction::Undo),
        MenuItem::os_action("Wiederholen", MenuRedo, OsAction::Redo),
        MenuItem::separator(),
        MenuItem::os_action("Ausschneiden", MenuCut, OsAction::Cut),
        MenuItem::os_action("Kopieren", MenuCopy, OsAction::Copy),
        MenuItem::os_action("Einsetzen", MenuPaste, OsAction::Paste),
        MenuItem::os_action("Alles auswählen", MenuSelectAll, OsAction::SelectAll),
    ]);

    let view = Menu::new("Darstellung").items(vec![
        checked("Menüs", MenuOpenMenus, active == Tab::Menus, content),
        checked(
            "Bestellungen",
            MenuOpenOrders,
            active == Tab::Orders,
            content,
        ),
        checked(
            "Abrechnung",
            MenuOpenBilling,
            active == Tab::Billing,
            content,
        ),
        checked(
            "Einstellungen",
            MenuOpenSettings,
            active == Tab::Settings,
            content,
        ),
        MenuItem::separator(),
        action("Aktualisieren", MenuRefresh, refresh),
        action("Zurück", MenuBack, back),
    ]);

    let window = Menu::new("Fenster").items(vec![
        action("Minimieren", MenuMinimize, has_window),
        action("Zoomen", MenuZoom, has_window),
        MenuItem::separator(),
        action("Alle nach vorne bringen", MenuBringAllFront, true),
        MenuItem::separator(),
        action("SnackPilot", MenuShowMainWindow, has_window),
    ]);

    vec![app, file, edit, view, window]
}

/// A stable signature of everything the menus render, so the shell only
/// rebuilds the native menu bar when a checked/enabled state actually moves.
pub fn signature(model: Option<&AppModel>, has_window: bool) -> String {
    let active = model
        .map(|model| model.state().navigation.active_tab())
        .unwrap_or(Tab::Menus);
    let scope = model
        .map(|model| model.key_scope())
        .unwrap_or(KeyScope::Modal);
    format!(
        "{active:?}|{has_window}|{scope:?}|{}|{}|{}",
        refresh_enabled(model),
        back_enabled(model),
        content_enabled(model)
    )
}

/// Install the menu bar handlers that do not need a shell view. Navigation,
/// refresh, Back and About are handled by the `AppShell` root so the dialog and
/// detail state stay in one place.
pub fn install_handlers(cx: &mut App) {
    cx.on_action(|_: &MenuQuit, cx: &mut App| cx.quit());
    cx.on_action(|_: &MenuHide, cx: &mut App| cx.hide());
    cx.on_action(|_: &MenuHideOthers, cx: &mut App| cx.hide_other_apps());
    cx.on_action(|_: &MenuShowAll, cx: &mut App| cx.activate(true));
    cx.on_action(|_: &MenuBringAllFront, cx: &mut App| cx.activate(true));
    cx.on_action(|_: &MenuCloseWindow, cx: &mut App| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        }
        // Without a visible window the content commands disable; rebuild the
        // menu bar now because no shell render will follow (REQ-UI-14).
        let model = crate::shell_model();
        crate::desktop::sync_menus(cx, model.as_deref());
    });
    cx.on_action(|_: &MenuMinimize, cx: &mut App| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.minimize_window());
        }
    });
    cx.on_action(|_: &MenuZoom, cx: &mut App| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.zoom_window());
        }
    });
    cx.on_action(|_: &MenuShowMainWindow, cx: &mut App| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.activate_window());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{back_enabled, menus, refresh_enabled, signature};
    use gpui::{MenuItem, SystemMenuType};
    use snackpilot_model::desktop::HostKind;
    use snackpilot_model::navigation::{SettingsDetail, Tab};
    use snackpilot_model::{AppModel, FakePlatform};
    use std::sync::Arc;

    fn demo_model(tag: &str) -> Arc<AppModel> {
        let dir =
            std::env::temp_dir().join(format!("snackpilot-menu-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let model = AppModel::new(dir, Arc::new(FakePlatform::new())).unwrap();
        model.login("demo", "demo1234!").unwrap();
        Arc::new(model)
    }

    fn item_names(menu: &gpui::Menu) -> Vec<String> {
        menu.items
            .iter()
            .filter_map(|item| match item {
                MenuItem::Action { name, .. } => Some(name.to_string()),
                _ => None,
            })
            .collect()
    }

    /// The five native menus and their approved German inventory/order
    /// (REQ-UI-14).
    /// @req REQ-UI-14
    #[test]
    fn native_menu_inventory_matches_the_brief() {
        let bars = menus(None, true);
        let names: Vec<_> = bars.iter().map(|menu| menu.name.to_string()).collect();
        assert_eq!(
            names,
            vec![
                "SnackPilot",
                "Ablage",
                "Bearbeiten",
                "Darstellung",
                "Fenster"
            ]
        );
        assert_eq!(
            item_names(&bars[0]),
            vec![
                "Über SnackPilot",
                "Einstellungen …",
                "SnackPilot ausblenden",
                "Andere ausblenden",
                "Alle einblenden",
                "SnackPilot beenden",
            ]
        );
        assert_eq!(item_names(&bars[1]), vec!["Fenster schließen"]);
        assert_eq!(
            item_names(&bars[2]),
            vec![
                "Widerrufen",
                "Wiederholen",
                "Ausschneiden",
                "Kopieren",
                "Einsetzen",
                "Alles auswählen",
            ]
        );
        assert_eq!(
            item_names(&bars[3]),
            vec![
                "Menüs",
                "Bestellungen",
                "Abrechnung",
                "Einstellungen",
                "Aktualisieren",
                "Zurück",
            ]
        );
        assert_eq!(
            item_names(&bars[4]),
            vec![
                "Minimieren",
                "Zoomen",
                "Alle nach vorne bringen",
                "SnackPilot"
            ]
        );
        // The approved order puts Über, a separator, Einstellungen, another
        // separator and then the standard Dienste submenu: slot 4 is the
        // native OS menu, not a fake item.
        assert!(matches!(
            &bars[0].items[4],
            MenuItem::SystemMenu(menu) if menu.menu_type == SystemMenuType::Services
        ));
    }

    /// Every Bearbeiten item has the brief's key equivalent. The native menu
    /// renders that glyph from the key binding of the item's own action, so
    /// without the binding the row shows a name and an OS selector but no
    /// shortcut; the items keep their OS selectors and the app installs no
    /// handler, so text editing keeps its own bindings (REQ-UI-14).
    /// @req REQ-UI-14
    #[test]
    fn edit_items_carry_the_brief_key_equivalents() {
        use crate::desktop::{key_bindings, PlatformModifier};
        let bars = menus(None, true);
        let bindings = key_bindings(PlatformModifier::Command);
        for (name, chord) in [
            ("Widerrufen", "cmd-z"),
            ("Wiederholen", "cmd-shift-z"),
            ("Ausschneiden", "cmd-x"),
            ("Kopieren", "cmd-c"),
            ("Einsetzen", "cmd-v"),
            ("Alles auswählen", "cmd-a"),
        ] {
            let action_name = bars[2]
                .items
                .iter()
                .find_map(|item| match item {
                    MenuItem::Action {
                        name: item_name,
                        action,
                        ..
                    } if item_name.as_ref() == name => Some(action.name()),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{name} is missing from Bearbeiten"));
            let bound = bindings
                .iter()
                .find(|binding| binding.action().name() == action_name)
                .map(|binding| {
                    binding
                        .keystrokes()
                        .iter()
                        .map(|stroke| stroke.unparse())
                        .collect::<Vec<_>>()
                        .join(" ")
                });
            assert_eq!(
                bound.as_deref(),
                Some(chord),
                "{name} must show its brief key equivalent"
            );
        }
    }

    /// The checked destination follows the model's active tab, and the menu
    /// survives with no visible window while content commands disable
    /// (REQ-UI-14).
    /// @req REQ-UI-14
    #[test]
    fn checked_state_and_enablement_follow_the_model() {
        let model = demo_model("checked");
        model.set_host_kind(HostKind::Desktop);
        model.set_content_width(1280.);
        model.select_tab(Tab::Orders);
        let view = menus(Some(&model), true);
        let checked: Vec<&str> = view[3]
            .items
            .iter()
            .filter_map(|item| match item {
                MenuItem::Action {
                    name,
                    checked: true,
                    ..
                } => Some(name.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(checked, vec!["Bestellungen"]);
        assert!(refresh_enabled(Some(&model)), "orders owns a refresh");
        assert!(super::content_enabled(Some(&model)));

        // No visible window: the menu bar remains, content actions disable.
        let hidden = menus(Some(&model), false);
        for index in [1usize, 3] {
            assert!(
                hidden[index]
                    .items
                    .iter()
                    .all(|item| item.is_disabled() || matches!(item, MenuItem::Separator)),
                "menu {index} content actions disable without a window"
            );
        }
        // A modal blocks every escaping content command (but not quit/hide).
        model.set_modal_open(true);
        let modal = menus(Some(&model), true);
        assert!(
            modal[0].items[0].is_disabled(),
            "About cannot replace an open modal"
        );
        assert!(!refresh_enabled(Some(&model)));
        assert!(!super::content_enabled(Some(&model)));
        assert!(
            !modal[0].items[10].is_disabled(),
            "SnackPilot beenden stays native under a modal"
        );
        assert!(back_enabled(Some(&model)), "Esc routes to the modal cancel");
        model.set_modal_open(false);
        assert!(signature(Some(&model), true).contains("Orders"));
    }

    /// Back enables exactly when Esc has a layer to close (REQ-UI-14).
    /// @req REQ-UI-14
    #[test]
    fn back_enables_only_with_a_layer_to_close() {
        let model = demo_model("back");
        assert!(!back_enabled(Some(&model)));
        model.select_tab(Tab::Settings);
        model.open_settings_detail(SettingsDetail::Data);
        assert!(back_enabled(Some(&model)));
        model.close_settings_detail();
        assert!(!back_enabled(Some(&model)));
    }
}

// A tiny action used only by the OsAction rows above; gpui needs a distinct
// action type per menu item even when the OS selector performs the edit.
actions!(
    snackpilot_desktop,
    [
        MenuUndo,
        MenuRedo,
        MenuCut,
        MenuCopy,
        MenuPaste,
        MenuSelectAll
    ]
);
