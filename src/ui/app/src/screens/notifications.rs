//! Notifications screen — docs/requirements/04-ui-ux.md §3.8.
//!
//! Three sections: Bestell-Erinnerung (switch + twelve time chips + Android
//! hint), Standort-Benachrichtigungen (set/remove company location) and the
//! Benachrichtigungs-Log (12/24 h, status, mail export, discard). All state is
//! `AppModel` state; actions call the model methods 3.3 already provides.

use std::rc::Rc;
use std::sync::Arc;

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{Icon, Sizable};

use crate::app::AppShell;
use crate::components::time_chips::time_chips;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_model::{AppModel, State};

fn section(title: &'static str, hint: &'static str, theme: &AppTheme, body: Div) -> Div {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .mb(px(4.))
                .text_size(px(18.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(title),
        )
        .child(
            div()
                .mb(px(16.))
                .text_size(px(13.))
                .text_color(theme.text_tertiary)
                .child(hint),
        )
        .child(body)
}

fn primary(
    id: &'static str,
    label: String,
    icon: Option<IconName>,
    on_click: ClickHandler,
    theme: &AppTheme,
) -> AnyElement {
    let enabled = !label.contains("...");
    let button = div()
        .flex()
        .items_center()
        .justify_center()
        .py(px(14.))
        .px(px(24.))
        .rounded(px(12.));
    let button = theme::apply(button, &theme.button_primary()).id(SharedString::from(id));
    let button = if enabled {
        button
            .cursor_pointer()
            .on_click(move |_, window, cx| on_click(window, cx))
    } else {
        button.opacity(0.6)
    };
    let mut button = button;
    if let Some(icon) = icon {
        button = button.child(
            Icon::new(icon)
                .with_size(px(16.))
                .text_color(theme.on_primary),
        );
    }
    button
        .child(
            div()
                .ml(px(6.))
                .text_size(px(15.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.on_primary)
                .child(label),
        )
        .into_any_element()
}

/// The pushed Benachrichtigungen screen.
pub fn notifications_screen(
    state: &State,
    model: Option<&Arc<AppModel>>,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let back = {
        let entity = cx.entity().clone();
        Rc::new(move |_: &mut Window, cx: &mut App| {
            let _ = entity.update(cx, |this, cx| this.pop_detail(cx));
        })
    };

    let model = model.cloned();

    // Section 1: daily reminder.
    let toggle = {
        let shell = cx.entity().clone();
        let model = model.clone();
        move |checked: &bool, _w: &mut Window, cx: &mut App| {
            if let Some(model) = &model {
                model.set_daily_reminder_enabled(*checked);
            }
            let _ = shell.update(cx, |_, cx| cx.notify());
        }
    };
    let reminder_row = div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child("Aktiviert"),
        )
        .child(
            Switch::new("reminder-switch")
                .checked(state.daily_reminder_enabled)
                .color(theme.primary)
                .on_change(toggle),
        );

    let mut reminder_body = div().flex().flex_col().gap(px(12.)).child(reminder_row);
    if state.daily_reminder_enabled {
        let shell = cx.entity().clone();
        let pick_model = model.clone();
        let pick = Rc::new(move |hour: u8, minute: u8, _w: &mut Window, cx: &mut App| {
            if let Some(model) = &pick_model {
                model.set_daily_reminder_time(hour, minute);
            }
            let _ = shell.update(cx, |_, cx| cx.notify());
        });
        let selected = state.reminder_time.map(|t| (t.hour, t.minute));
        reminder_body = reminder_body
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text_secondary)
                    .child("Uhrzeit"),
            )
            .child(time_chips(selected, pick, theme));
        if cfg!(target_os = "android") {
            let shell = cx.entity().clone();
            let model = model.clone();
            let open = Rc::new(move |_: &mut Window, cx: &mut App| {
                if let Some(model) = &model {
                    let _ = model.open_app_settings();
                }
                let _ = shell.update(cx, |_, cx| cx.notify());
            });
            reminder_body = reminder_body.child(
                div()
                    .id("reminder-android-hint")
                    .mt(px(12.))
                    .p(px(12.))
                    .rounded(px(10.))
                    .bg(theme.surface_variant)
                    .cursor_pointer()
                    .on_click(move |_, window, cx| open(window, cx))
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(8.))
                            .child(Icon::new(IconName::Info).with_size(px(18.)).text_color(theme.text_tertiary))
                            .child(
                                div()
                            .text_size(px(12.))
                            .text_color(theme.text_secondary)
                            .child("Damit Erinnerungen zuverlässig funktionieren, muss die Hintergrundaktivität für diese App erlaubt sein. ")
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.primary)
                                    .child("App-Einstellungen öffnen"),
                            ),
                            ),
                    ),
            );
        }
    }

    // Section 2: location notifications.
    let location_body = if let Some(_location) = &state.company_location {
        let shell = cx.entity().clone();
        let model = model.clone();
        let remove = Rc::new(move |_: &mut Window, cx: &mut App| {
            if let Some(model) = &model {
                model.clear_company_location();
            }
            let _ = shell.update(cx, |_, cx| cx.notify());
        });
        let danger = theme.button_danger();
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(theme.text_secondary)
                    .child("Firmenstandort gesetzt"),
            )
            .child(
                theme::apply(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .py(px(14.))
                        .px(px(24.)),
                    &danger,
                )
                .id("location-remove")
                .cursor_pointer()
                .on_click(move |_, window, cx| remove(window, cx))
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.on_primary)
                        .child("Standort entfernen"),
                ),
            )
    } else {
        let shell = cx.entity().clone();
        let model = model.clone();
        let set = Rc::new(move |_: &mut Window, cx: &mut App| {
            if let Some(model) = &model {
                model.set_company_location_from_current_position();
            }
            let _ = shell.update(cx, |_, cx| cx.notify());
        });
        let label = if state.location_busy {
            "Standort wird ermittelt..."
        } else {
            "Aktuellen Standort als Firmenstandort setzen"
        };
        div()
            .flex()
            .flex_col()
            .child(primary("location-set", label.to_string(), None, set, theme))
    };

    // Section 3: notification log.
    let log_body = if state.log_active {
        div().child(
            div()
                .text_size(px(14.))
                .text_color(theme.text_secondary)
                .child(state.log_status.clone()),
        )
    } else if state.log_expired {
        let shell = cx.entity().clone();
        let send_model = model.clone();
        let send = Rc::new(move |_: &mut Window, cx: &mut App| {
            if let Some(model) = &send_model {
                if model.send_diagnostic_log().is_err() {
                    // The §3.8 failure alert.
                    let _ = shell.update(cx, |this, cx| {
                        this.set_dialog(
                            crate::components::dialog::Dialog::Alert {
                                title: "Fehler".into(),
                                message: Some("E-Mail-App konnte nicht geöffnet werden.".into()),
                            },
                            cx,
                        );
                    });
                }
            }
            let _ = shell.update(cx, |_, cx| cx.notify());
        });
        let shell2 = cx.entity().clone();
        let discard_model = model.clone();
        let discard = Rc::new(move |_: &mut Window, cx: &mut App| {
            if let Some(model) = &discard_model {
                model.clear_diagnostic_log();
            }
            let _ = shell2.update(cx, |_, cx| cx.notify());
        });
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(theme.text_secondary)
                    .child(state.log_status.clone()),
            )
            .child(primary(
                "log-send",
                "Log per E-Mail senden".to_string(),
                Some(IconName::Mail),
                send,
                theme,
            ))
            .child(
                div()
                    .id("log-discard")
                    .cursor_pointer()
                    .on_click(move |_, window, cx| discard(window, cx))
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(theme.text_tertiary)
                            .child("Log verwerfen"),
                    ),
            )
    } else {
        let shell = cx.entity().clone();
        let twelve_model = model.clone();
        let twelve = Rc::new(move |_: &mut Window, cx: &mut App| {
            if let Some(model) = &twelve_model {
                model.activate_diagnostic_log(12);
            }
            let _ = shell.update(cx, |_, cx| cx.notify());
        });
        let shell2 = cx.entity().clone();
        let twenty_four_model = model.clone();
        let twenty_four = Rc::new(move |_: &mut Window, cx: &mut App| {
            if let Some(model) = &twenty_four_model {
                model.activate_diagnostic_log(24);
            }
            let _ = shell2.update(cx, |_, cx| cx.notify());
        });
        div()
            .flex()
            .gap(px(12.))
            .child(primary(
                "log-12",
                "12 Stunden".to_string(),
                None,
                twelve,
                theme,
            ))
            .child(primary(
                "log-24",
                "24 Stunden".to_string(),
                None,
                twenty_four,
                theme,
            ))
    };

    let divider = div().h(px(1.)).my(px(24.)).bg(theme.divider());

    div()
        .id("notifications")
        .size_full()
        .flex()
        .flex_col()
        .p(px(20.))
        .pb(px(100.))
        .overflow_y_scroll()
        .child(crate::navigation::back_affordance(
            "Einstellungen",
            theme,
            back,
        ))
        .child(
            div()
                .text_size(px(28.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.text_primary)
                .child("Benachrichtigungen"),
        )
        .child(section(
            "Bestell-Erinnerung",
            "Tägliche Erinnerung an deine Bestellung",
            theme,
            reminder_body,
        ))
        .child(divider)
        .child(section(
            "Standort-Benachrichtigungen",
            "Erinnerung um 8:45 basierend auf deinem Standort",
            theme,
            location_body,
        ))
        .child(div().h(px(1.)).my(px(24.)).bg(theme.divider()))
        .child(section(
            "Benachrichtigungs-Log",
            "Zeichnet 24 Stunden lang Diagnose-Daten auf, um Probleme mit Benachrichtigungen zu analysieren.",
            theme,
            log_body,
        ))
        .into_any_element()
}
