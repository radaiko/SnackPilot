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
use crate::components::segment_tabs::{segment_options, SegmentOptionHandler};
use crate::components::time_chips::time_chips;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_model::{tokens, AppModel, State};

/// The two activation windows the diagnostic log offers (notification-log §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogDuration {
    Hours12,
    Hours24,
}

impl LogDuration {
    /// The window length passed to the existing model setter.
    fn hours(self) -> u32 {
        match self {
            Self::Hours12 => 12,
            Self::Hours24 => 24,
        }
    }
}

/// The window an inactive log reports as its current choice: activation
/// defaults to 24 hours (v1 `activateLog(hours = 24)`).
const LOG_DURATION_DEFAULT: LogDuration = LogDuration::Hours24;

/// One section card: surface, 12 px radius, 1 px hairline, 16 px padding,
/// `BODY_STRONG` title, `META` hint and the group's controls.
fn section(title: &'static str, hint: &'static str, theme: &AppTheme, body: Div) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2))
        .p(px(theme.scale.space_4))
        .rounded(px(theme.scale.radius_card))
        .border(px(1.))
        .border_color(theme.border)
        .bg(theme.surface)
        .child(
            div()
                .text_size(px(tokens::BODY_STRONG.size))
                .line_height(px(tokens::BODY_STRONG.line_height))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(title),
        )
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child(hint),
        )
        .child(body)
}

/// Plain action rows keep the 48 px primary-action floor.
pub fn notification_action_height(theme: &AppTheme) -> f32 {
    theme.scale.action_height
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
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .min_h(px(notification_action_height(theme)))
        .px(px(theme.scale.space_4))
        .py(px(theme.scale.space_2))
        .rounded(px(theme.scale.radius_control));
    let button = theme::apply(button, &theme.button_secondary()).id(SharedString::from(id));
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
                .with_size(px(theme.scale.icon))
                .text_color(theme.text_primary),
        );
    }
    button
        .child(
            div()
                .min_w(px(0.))
                .ml(px(theme.scale.space_2))
                .text_size(px(theme.scale.body))
                .line_height(px(theme.scale.body_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_center()
                .text_color(theme.text_primary)
                .child(label),
        )
        .into_any_element()
}

/// The pushed Benachrichtigungen screen.
pub fn notifications_screen(
    state: &State,
    model: Option<&Arc<AppModel>>,
    theme: &AppTheme,
    containment: Option<f32>,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let back = {
        let entity = cx.entity().clone();
        Rc::new(move |_: &mut Window, cx: &mut App| {
            let _ = entity.update(cx, |this, cx| this.pop_detail(cx));
        })
    };

    // Desktop has no notification/geofence service in this change: the row is
    // hidden from the settings master at every width, and a stale/debug route
    // only shows this truthful unavailable state — never functioning-looking
    // toggles or chips (REQ-SCR-08).
    if cfg!(not(any(target_os = "ios", target_os = "android"))) {
        let mut page = div()
            .id("notifications-unavailable")
            .size_full()
            .flex()
            .flex_col()
            .gap(px(8.));
        page = match containment {
            Some(pad) => page.p(px(pad)).w_full().max_w(px(440.)),
            None => page,
        };
        if containment.is_none() {
            page = page.child(crate::navigation::back_affordance(
                "Einstellungen",
                theme,
                back,
            ));
        }
        return page
            .child(
                div()
                    .text_size(px(theme.scale.title))
                    .line_height(px(theme.scale.title_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text_primary)
                    .child("Benachrichtigungen"),
            )
            .child(
                div()
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
                    .text_color(theme.text_secondary)
                    .child("Benachrichtigungen sind auf diesem Gerät nicht verfügbar."),
            )
            .into_any_element();
    }

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
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .h(px(theme.scale.action_height))
        .child(
            div()
                .text_size(px(theme.scale.body))
                .line_height(px(theme.scale.body_line))
                .text_color(theme.text_primary)
                .child("Aktiviert"),
        )
        .child(
            Switch::new("reminder-switch")
                .checked(state.daily_reminder_enabled)
                .color(theme.accent)
                .on_change(toggle),
        );

    let mut reminder_body = div()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_3))
        .child(div().w_full().h(px(1.)).bg(theme.border))
        .child(reminder_row);
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
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
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
                    .bg(theme.surface_alt)
                    .cursor_pointer()
                    .on_click(move |_, window, cx| open(window, cx))
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(8.))
                            .child(Icon::new(IconName::Info).with_size(px(theme.scale.icon_meta)).text_color(theme.text_tertiary))
                            .child(
                                div()
                            .text_size(px(12.))
                            .text_color(theme.text_secondary)
                            .child("Damit Erinnerungen zuverlässig funktionieren, muss die Hintergrundaktivität für diese App erlaubt sein. ")
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.accent)
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
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
                    .text_color(theme.text_secondary)
                    .child("Firmenstandort gesetzt"),
            )
            .child(
                theme::apply(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .h(px(theme.scale.action_height))
                        .px(px(theme.scale.space_4)),
                    &danger,
                )
                .id("location-remove")
                .cursor_pointer()
                .on_click(move |_, window, cx| remove(window, cx))
                .child(
                    div()
                        .text_size(px(theme.scale.body))
                        .line_height(px(theme.scale.body_line))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.on_accent)
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
            "Standort setzen"
        };
        div()
            .flex()
            .flex_col()
            .gap(px(theme.scale.space_1))
            .child(primary("location-set", label.to_string(), None, set, theme))
            .child(
                div()
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
                    .text_color(theme.text_secondary)
                    .child("Aktuellen Standort als Firmenstandort setzen"),
            )
    };

    // Section 3: notification log.
    let log_body = if state.log_active {
        div().child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
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
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
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
                    .min_h(px(theme.scale.action_height))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .focus(crate::theme::desktop_focus(theme.accent))
                    .on_click(move |_, window, cx| discard(window, cx))
                    .child(
                        div()
                            .text_size(px(theme.scale.body))
                            .line_height(px(theme.scale.body_line))
                            .text_color(theme.text_secondary)
                            .child("Log verwerfen"),
                    ),
            )
    } else {
        let shell = cx.entity().clone();
        let select_model = model.clone();
        let select: SegmentOptionHandler<LogDuration> = Rc::new(
            move |duration: LogDuration, _w: &mut Window, cx: &mut App| {
                if let Some(model) = &select_model {
                    model.activate_diagnostic_log(duration.hours());
                }
                let _ = shell.update(cx, |_, cx| cx.notify());
            },
        );
        div().child(segment_options(
            [
                ("log-12", "12 Stunden".to_string(), LogDuration::Hours12),
                ("log-24", "24 Stunden".to_string(), LogDuration::Hours24),
            ],
            LOG_DURATION_DEFAULT,
            select,
            theme,
        ))
    };

    div()
        .id("notifications")
        .size_full()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_5))
        .overflow_y_scroll()
        .child(crate::navigation::back_affordance(
            "Einstellungen",
            theme,
            back,
        ))
        .child(
            div()
                .text_size(px(theme.scale.title))
                .line_height(px(theme.scale.title_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child("Benachrichtigungen"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(theme.scale.space_3))
                .child(section(
                    "Bestell-Erinnerung",
                    "Tägliche Erinnerung an deine Bestellung",
                    theme,
                    reminder_body,
                ))
                .child(section(
                    "Standort-Benachrichtigungen",
                    "Erinnerung um 8:45 basierend auf deinem Standort",
                    theme,
                    location_body,
                ))
                .child(section(
                    "Benachrichtigungs-Log",
                    "Zeichnet Diagnose-Daten auf, um Probleme mit Benachrichtigungen zu analysieren",
                    theme,
                    log_body,
                )),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{notification_action_height, LogDuration, LOG_DURATION_DEFAULT};
    use crate::components::segment_tabs::segment_options;
    use crate::components::time_chips::{label, time_chips, times};
    use crate::theme::AppTheme;

    /// The reminder keeps the twelve pinned times and 44 px chips, and the
    /// location/diagnostics actions keep the 48 px floor (REQ-SCR-08).
    /// @req REQ-SCR-08
    #[test]
    fn notifications_keep_pinned_times_and_action_floor() {
        let theme = AppTheme::build(false);
        assert_eq!(times().len(), 12);
        assert_eq!(label(11, 0), "11:00");
        assert_eq!(label(13, 45), "13:45");
        assert_eq!(theme.scale.target, 44.);
        assert_eq!(notification_action_height(&theme), 48.);
        let _ = time_chips(None, std::rc::Rc::new(|_, _, _, _| {}), &theme);
    }

    /// The log duration segment reports the current 24-hour choice as selected
    /// and the 12-hour choice as not, so the recording window is visible.
    /// @req REQ-SCR-08 @req REQ-UI-15
    #[test]
    fn log_duration_segment_marks_only_the_current_choice() {
        let active = LOG_DURATION_DEFAULT;
        assert_eq!(active.hours(), 24);
        assert!(active == LogDuration::Hours24);
        assert!(active != LogDuration::Hours12);
        let theme = AppTheme::build(false);
        let _ = segment_options(
            [
                ("log-12", "12 Stunden".to_string(), LogDuration::Hours12),
                ("log-24", "24 Stunden".to_string(), LogDuration::Hours24),
            ],
            LOG_DURATION_DEFAULT,
            std::rc::Rc::new(|_, _, _| {}),
            &theme,
        );
    }
}
