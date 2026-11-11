//! Settings tab — docs/requirements/04-ui-ux.md §3.4.
//!
//! Four rows with live hints from `AppModel` state, hairline dividers and the
//! Datenschutz link that opens the §4.1 alert. Benachrichtigungen is hidden on
//! platforms without notification support (desktop).

use std::rc::Rc;

use gpui::*;

use crate::app::{AppShell, Detail};
use crate::components::settings_row::{settings_row, settings_row_desktop};
use crate::components::workspace::{detail_prompt, split_workspace};
use crate::theme::{self, AppTheme};
use snackpilot_model::navigation::SettingsDetail;
use snackpilot_model::{AppModel, State, ThemePreference};
use std::sync::Arc;

const DATENSCHUTZ_MESSAGE: &str = "Diese App erfasst anonyme Nutzungsstatistiken zur Verbesserung der Benutzererfahrung. Die Analyse erfolgt über TelemetryDeck — einen datenschutzfreundlichen, cookielosen Dienst. Es werden keine persönlichen Daten, Passwörter, Menüauswahl oder Abrechnungsdaten erfasst.";

/// The Einstellungen root screen.
pub fn settings_tab(state: &State, theme: &AppTheme, cx: &mut Context<AppShell>) -> AnyElement {
    let gourmet_hint = match &state.user_info {
        Some(user) if state.gourmet_authenticated => {
            format!("Angemeldet als {}", user.username)
        }
        _ => "Nicht angemeldet".to_string(),
    };
    let ventopay_hint = if state.ventopay_authenticated {
        "Sitzung aktiv".to_string()
    } else {
        "Nicht angemeldet".to_string()
    };
    let theme_hint = match state.theme {
        ThemePreference::System => "System",
        ThemePreference::Light => "Hell",
        ThemePreference::Dark => "Dunkel",
    }
    .to_string();

    let rows: Vec<(&'static str, &'static str, String, Detail)> = vec![
        (
            "settings-gourmet",
            "Kantine-Zugangsdaten",
            gourmet_hint,
            Detail::LoginKantine,
        ),
        (
            "settings-ventopay",
            "Automaten-Zugangsdaten",
            ventopay_hint,
            Detail::LoginAutomaten,
        ),
        (
            "settings-appearance",
            "Darstellung",
            theme_hint,
            Detail::Appearance,
        ),
        (
            "settings-notifications",
            "Benachrichtigungen",
            "Erinnerungen und Standort-Benachrichtigungen".to_string(),
            Detail::Notifications,
        ),
    ];

    let mut list = div()
        .id("settings-list")
        .flex_1()
        .flex()
        .flex_col()
        .p(px(20.))
        .pb(px(100.))
        .overflow_y_scroll();

    for (index, (id, title, hint, detail)) in rows.into_iter().enumerate() {
        // §3.4: Benachrichtigungen only where the platform supports them.
        if detail == Detail::Notifications
            && cfg!(not(any(target_os = "ios", target_os = "android")))
        {
            continue;
        }
        if index > 0 {
            list = list.child(div().h(px(1.)).my(px(24.)).bg(theme.divider()));
        }
        let entity = cx.entity().clone();
        let handler = Rc::new(move |window: &mut Window, cx: &mut App| {
            let _ = entity.update(cx, |this, cx| this.open_detail(detail, window, cx));
        });
        list = list.child(settings_row(id, title, Some(hint), handler, theme));
    }

    let datenschutz = cx.entity().clone();
    let on_privacy = Rc::new(move |_: &mut Window, cx: &mut App| {
        let _ = datenschutz.update(cx, |this, cx| {
            let message = if cfg!(not(any(target_os = "ios", target_os = "android"))) {
                crate::desktop::privacy_message()
            } else {
                DATENSCHUTZ_MESSAGE.to_string()
            };
            this.set_dialog(
                crate::components::dialog::Dialog::Alert {
                    title: "Datenschutz".into(),
                    message: Some(message.into()),
                },
                cx,
            );
        });
    });

    list.child(
        div()
            .id("datenschutz")
            .w_full()
            .flex()
            .justify_center()
            .py(px(16.))
            .cursor_pointer()
            .on_click(move |_, window, cx| on_privacy(window, cx))
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(theme.text_tertiary)
                    .child("Datenschutz"),
            ),
    )
    .into_any_element()
}

/// Desktop Einstellungen (REQ-SCR-04): the same rows and forms beside a
/// master list from 720 on, without push/pop or a back row. The model owns the
/// selected detail identity; this function only projects it. Unsupported
/// notifications are hidden at every desktop width.
/// @req REQ-SCR-04 @req REQ-SCR-08 @req REQ-UI-10
#[allow(clippy::too_many_arguments)]
pub fn desktop_settings_tab(
    state: &State,
    model: Option<&Arc<AppModel>>,
    login_user: &Option<gpui::Entity<gpui_kit::component::input::InputState>>,
    login_pass: &Option<gpui::Entity<gpui_kit::component::input::InputState>>,
    rows_focus: &[gpui::FocusHandle; 4],
    master_focus: &gpui::FocusHandle,
    appearance_design: &gpui::FocusHandle,
    appearance_accent: &gpui::FocusHandle,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let selected = model.and_then(|model| model.settings_detail());
    let geometry = model
        .map(|model| model.desktop_geometry())
        .unwrap_or_else(|| {
            snackpilot_model::desktop::desktop_geometry(
                snackpilot_model::desktop::HostKind::Desktop,
                720.,
            )
        });
    let wide = model.is_some_and(|model| model.is_wide());
    let pad = if wide { 24. } else { 16. };

    let gourmet_hint = match &state.user_info {
        Some(user) if state.gourmet_authenticated => {
            format!("Angemeldet als {}", user.username)
        }
        _ => "Nicht angemeldet".to_string(),
    };
    let ventopay_hint = if state.ventopay_authenticated {
        "Sitzung aktiv".to_string()
    } else {
        "Nicht angemeldet".to_string()
    };
    let theme_hint = match state.theme {
        ThemePreference::System => "System",
        ThemePreference::Light => "Hell",
        ThemePreference::Dark => "Dunkel",
    }
    .to_string();

    let rows: Vec<(&'static str, &'static str, String, Detail, SettingsDetail)> = vec![
        (
            "settings-gourmet",
            "Kantine-Zugangsdaten",
            gourmet_hint,
            Detail::LoginKantine,
            SettingsDetail::Profile,
        ),
        (
            "settings-ventopay",
            "Automaten-Zugangsdaten",
            ventopay_hint,
            Detail::LoginAutomaten,
            SettingsDetail::Security,
        ),
        (
            "settings-appearance",
            "Darstellung",
            theme_hint,
            Detail::Appearance,
            SettingsDetail::Data,
        ),
        (
            "settings-notifications",
            "Benachrichtigungen",
            "Erinnerungen und Standort-Benachrichtigungen".to_string(),
            Detail::Notifications,
            SettingsDetail::Notifications,
        ),
    ];

    let visible: Vec<_> = rows
        .into_iter()
        .filter(|(_, _, _, _, kind)| {
            // Desktop has no notification service in this change: hide the
            // destination rather than offering no-op controls
            // (REQ-SCR-08).
            *kind != SettingsDetail::Notifications
        })
        .collect();

    let handles = rows_focus.clone();
    let mut list = div()
        .id("settings-master")
        .w_full()
        .flex()
        .flex_col()
        .p(px(8.))
        .gap(px(4.))
        .tab_index(0)
        .track_focus(master_focus)
        .focus(theme::desktop_focus(theme.desktop.focus))
        .on_key_down({
            let handles = handles.clone();
            move |event: &KeyDownEvent, window, cx| {
                let count = 3;
                let Some(current) = handles
                    .iter()
                    .take(count)
                    .position(|handle| handle.is_focused(window))
                else {
                    return;
                };
                let next = match event.keystroke.key.as_str() {
                    "down" => (current + 1 < count).then_some(current + 1),
                    "up" => current.checked_sub(1),
                    "home" => Some(0),
                    "end" => Some(count - 1),
                    _ => None,
                };
                if let Some(next) = next {
                    window.focus(&handles[next], cx);
                    cx.stop_propagation();
                }
            }
        });

    for (index, (id, title, hint, detail, kind)) in visible.into_iter().enumerate() {
        let entity = cx.entity().clone();
        let handle = rows_focus[index].clone();
        let row_handle = handle.clone();
        let handler = Rc::new(move |window: &mut Window, cx: &mut App| {
            row_handle.focus(window, cx);
            let _ = entity.update(cx, |this, cx| {
                this.open_detail(detail, window, cx);
            });
        });
        list = list.child(settings_row_desktop(
            id,
            title,
            Some(hint),
            selected == Some(kind),
            &handle,
            handler,
            theme,
        ));
    }

    let privacy_entity = cx.entity().clone();
    let privacy = Rc::new(move |_: &mut Window, cx: &mut App| {
        let _ = privacy_entity.update(cx, |this, cx| {
            this.set_dialog(
                crate::components::dialog::Dialog::Alert {
                    title: "Datenschutz".into(),
                    message: Some(crate::desktop::privacy_message().into()),
                },
                cx,
            );
        });
    });
    list = list.child(
        div()
            .id("datenschutz")
            .min_h(px(44.))
            .px(px(12.))
            .flex()
            .items_center()
            .tab_index(0)
            .cursor_pointer()
            .focus(theme::desktop_focus(theme.desktop.focus))
            .on_click(move |_, window, cx| privacy(window, cx))
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .text_color(theme.text_secondary)
                    .child("Datenschutz"),
            ),
    );

    let empty = detail_prompt("Einstellung auswählen", theme);
    let detail = match selected {
        Some(SettingsDetail::Profile) => Some(Detail::LoginKantine),
        Some(SettingsDetail::Security) => Some(Detail::LoginAutomaten),
        Some(SettingsDetail::Data) => Some(Detail::Appearance),
        Some(SettingsDetail::Notifications) => Some(Detail::Notifications),
        None | Some(SettingsDetail::Advanced) => None,
    };
    let pane = match detail {
        Some(Detail::LoginKantine) | Some(Detail::LoginAutomaten) => {
            // The AppShell owns the input entities; they are created once on
            // entry and survive the containment reprojection (REQ-UI-10).
            match (login_user.as_ref(), login_pass.as_ref()) {
                (Some(user), Some(pass)) => {
                    let kind = if detail == Some(Detail::LoginAutomaten) {
                        crate::screens::login::LoginKind::Automaten
                    } else {
                        crate::screens::login::LoginKind::Kantine
                    };
                    crate::screens::login::login_screen(
                        kind,
                        state,
                        model,
                        user,
                        pass,
                        theme,
                        Some(pad),
                        cx,
                    )
                }
                _ => detail_prompt("Einstellung auswählen", theme),
            }
        }
        Some(Detail::Appearance) => crate::screens::appearance::appearance_screen(
            state,
            model,
            theme,
            Some(pad),
            appearance_design,
            appearance_accent,
            cx,
        ),
        Some(Detail::Notifications) => {
            crate::screens::notifications::notifications_screen(state, model, theme, Some(pad), cx)
        }
        _ => empty,
    };

    split_workspace(list.into_any_element(), pane, &geometry, theme)
}

/// The pushed screens that 5.4 delivers (the rest arrive with 5.5/5.6).
pub(crate) fn placeholder_detail(
    detail: Detail,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let title = match detail {
        Detail::Appearance => "Darstellung",
        Detail::Notifications => "Benachrichtigungen",
        _ => "Einstellungen",
    };
    let entity = cx.entity().clone();
    let on_back = Rc::new(move |_: &mut Window, cx: &mut App| {
        let _ = entity.update(cx, |this, cx| this.pop_detail(cx));
    });
    div()
        .size_full()
        .flex()
        .flex_col()
        .p(px(20.))
        .pb(px(100.))
        .gap(px(24.))
        .child(crate::navigation::back_affordance(
            "Einstellungen",
            theme,
            on_back,
        ))
        .child(
            div()
                .text_size(px(28.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.text_primary)
                .child(title),
        )
        .into_any_element()
}
