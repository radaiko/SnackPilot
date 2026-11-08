//! Settings tab — docs/requirements/04-ui-ux.md §3.4.
//!
//! Four rows with live hints from `AppModel` state, hairline dividers and the
//! Datenschutz link that opens the §4.1 alert. Benachrichtigungen is hidden on
//! platforms without notification support (desktop).

use std::rc::Rc;

use gpui::*;

use crate::app::{AppShell, Detail};
use crate::components::settings_row::settings_row;
use crate::theme::AppTheme;
use snackpilot_model::{State, ThemePreference};

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
            this.set_dialog(
                crate::components::dialog::Dialog::Alert {
                    title: "Datenschutz".into(),
                    message: Some(DATENSCHUTZ_MESSAGE.into()),
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
