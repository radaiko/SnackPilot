//! Settings tab — docs/requirements/04-ui-ux.md §3.4.
//!
//! Four rows with live hints from `AppModel` state, hairline dividers and the
//! Datenschutz link that opens the §4.1 alert. Benachrichtigungen is hidden on
//! platforms without notification support (desktop).

use std::rc::Rc;

use gpui::*;
use gpui_kit::assets::IconName;

use crate::app::{AppShell, Detail};
use crate::components::settings_row::{settings_row, settings_row_desktop};
use crate::components::workspace::{detail_prompt, split_workspace};
use crate::theme::{self, AppTheme};
use snackpilot_model::navigation::SettingsDetail;
use snackpilot_model::{AppModel, State, ThemePreference};
use std::sync::Arc;

/// The approved Aptabase paragraph, without the desktop-only sentence: mobile
/// analytics behaviour is unchanged, only the historical provider copy is
/// corrected (`docs/requirements/07-release.md`).
const DATENSCHUTZ_MESSAGE: &str = crate::desktop::PRIVACY_PARAGRAPH;

/// The Benachrichtigungen row hint: short enough to stay on one line in the
/// compact row; the full sentence wrapped and made that one card taller.
pub(crate) const NOTIFICATIONS_HINT: &str = "Erinnerungen und Standort";

/// The Datenschutz alert body for this host: the approved paragraph alone on
/// mobile, plus the truthful no-analytics sentence on desktop (REQ-SCR-04).
pub(crate) fn datenschutz_message(desktop: bool) -> String {
    if desktop {
        crate::desktop::privacy_message()
    } else {
        DATENSCHUTZ_MESSAGE.to_string()
    }
}

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

    let rows: Vec<(&'static str, IconName, &'static str, String, Detail)> = vec![
        (
            "settings-gourmet",
            IconName::Utensils,
            "Kantine-Zugangsdaten",
            gourmet_hint,
            Detail::LoginKantine,
        ),
        (
            "settings-ventopay",
            IconName::CreditCard,
            "Automaten-Zugangsdaten",
            ventopay_hint,
            Detail::LoginAutomaten,
        ),
        (
            "settings-appearance",
            IconName::Palette,
            "Darstellung",
            theme_hint,
            Detail::Appearance,
        ),
        (
            "settings-notifications",
            IconName::Bell,
            "Benachrichtigungen",
            NOTIFICATIONS_HINT.to_string(),
            Detail::Notifications,
        ),
    ];

    let mut list = div()
        .id("settings-list")
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2))
        .overflow_y_scroll();

    let section_label = |text: &'static str| {
        div()
            .pt(px(theme.scale.space_4))
            .mb(px(theme.scale.space_2))
            .text_size(px(theme.scale.meta))
            .line_height(px(theme.scale.meta_line))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.text_secondary)
            .child(text)
    };
    list = list.child(section_label("Konto"));

    for (index, (id, icon, title, hint, detail)) in rows.into_iter().enumerate() {
        // §3.4: Benachrichtigungen only where the platform supports them.
        if detail == Detail::Notifications
            && cfg!(not(any(target_os = "ios", target_os = "android")))
        {
            continue;
        }
        if index == 2 {
            list = list.child(section_label("App"));
        }
        let entity = cx.entity().clone();
        let handler = Rc::new(move |window: &mut Window, cx: &mut App| {
            let _ = entity.update(cx, |this, cx| this.open_detail(detail, window, cx));
        });
        list = list.child(settings_row(id, icon, title, Some(hint), handler, theme));
    }

    let datenschutz = cx.entity().clone();
    let on_privacy = Rc::new(move |_: &mut Window, cx: &mut App| {
        let _ = datenschutz.update(cx, |this, cx| {
            let message =
                datenschutz_message(cfg!(not(any(target_os = "ios", target_os = "android"))));
            this.set_dialog(
                crate::components::dialog::Dialog::Alert {
                    title: "Datenschutz".into(),
                    message: Some(message.into()),
                },
                cx,
            );
        });
    });

    list = list.child(settings_row(
        "datenschutz",
        IconName::Shield,
        "Datenschutz",
        Some("Was die App speichert und überträgt".to_string()),
        on_privacy,
        theme,
    ));
    list.into_any_element()
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
    login_attempted: bool,
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
    let wide = model.is_some_and(|model| model.is_expanded());
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
            NOTIFICATIONS_HINT.to_string(),
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
        .p(px(theme.scale.space_2))
        .gap(px(theme.scale.space_1))
        .tab_index(0)
        .track_focus(master_focus)
        .focus(theme::desktop_focus(theme.accent))
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
            .h(px(snackpilot_model::desktop::ROW_HEIGHT))
            .px(px(theme.scale.space_3))
            .flex()
            .items_center()
            .tab_index(0)
            .cursor_pointer()
            .hover(move |s| s.bg(theme.hover(theme.surface)))
            .focus(theme::desktop_focus(theme.accent))
            .on_click(move |_, window, cx| privacy(window, cx))
            .child(
                div()
                    .text_size(px(theme.scale.body))
                    .line_height(px(theme.scale.body_line))
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
                        login_attempted,
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
        .gap(px(theme.scale.space_4))
        .child(crate::navigation::back_affordance(
            "Einstellungen",
            theme,
            on_back,
        ))
        .child(
            div()
                .text_size(px(theme.scale.title))
                .line_height(px(theme.scale.title_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(title),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{datenschutz_message, NOTIFICATIONS_HINT};

    /// The compact Benachrichtigungen row hint stays on one line so that row
    /// keeps the same card height as every other settings row.
    /// @req REQ-UI-15 @req REQ-SCR-04
    #[test]
    fn notifications_row_hint_stays_on_one_line() {
        assert_eq!(NOTIFICATIONS_HINT, "Erinnerungen und Standort");
        assert!(
            !NOTIFICATIONS_HINT.contains("Benachrichtigungen"),
            "the long hint wrapped: {NOTIFICATIONS_HINT}"
        );
    }

    /// The mobile Datenschutz alert carries the approved Aptabase paragraph
    /// and nothing else; only desktop appends its no-analytics sentence
    /// (REQ-SCR-04, `docs/requirements/07-release.md`).
    /// @req REQ-SCR-04
    #[test]
    fn datenschutz_alert_uses_the_approved_aptabase_copy() {
        let mobile = datenschutz_message(false);
        assert!(
            mobile.contains("selbst gehostete Aptabase-Instanz"),
            "the approved provider paragraph is used on mobile: {mobile}"
        );
        assert!(
            !mobile.contains("TelemetryDeck"),
            "the historical provider is gone: {mobile}"
        );
        assert!(mobile.contains("ohne Nutzer- oder Geräte-IDs"));
        assert!(
            !mobile.contains("Auf dem Mac") && !mobile.contains("Unter Windows"),
            "the desktop-only sentence never reaches mobile: {mobile}"
        );

        let desktop = datenschutz_message(true);
        assert!(desktop.starts_with(&mobile), "same approved paragraph");
        assert!(
            desktop.contains("Auf dem Mac werden derzeit keine Nutzungsstatistiken gesendet.")
                || desktop
                    .contains("Unter Windows werden derzeit keine Nutzungsstatistiken gesendet."),
            "desktop alone states that analytics is a no-op: {desktop}"
        );
    }
}
