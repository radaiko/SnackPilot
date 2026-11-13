//! Login screens — docs/requirements/04-ui-ux.md §3.5 (Kantine) and §3.6
//! (Automaten), one parametrised form.
//!
//! Save order is v1 parity: credentials are written first, then the login is
//! attempted (bad credentials still overwrite the stored ones). The input
//! entities live on the shell so typing survives re-renders.

use std::rc::Rc;
use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::input::InputState;
use snackpilot_model::desktop::ControlState;

use crate::app::AppShell;
use crate::components::button::{mobile_action_action, mobile_action_metrics, ButtonKind};
use crate::components::dialog::Dialog;
use crate::components::spinner::spinner;
use crate::components::text_field::{desktop_field_with_state, field_with_state};
use crate::theme::AppTheme;
use snackpilot_model::{AppModel, PlatformError, State};

/// Which credentials pair the form edits.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LoginKind {
    Kantine,
    Automaten,
}

/// §3.5 failure alert copy, also shown when no session model is installed
/// (iOS before 5.7); the model's own `error_text` wins when present.
const FAILURE_TITLE: &str = "Login fehlgeschlagen";
const FAILURE_FALLBACK: &str = "Anmeldung nicht möglich. Bitte Zugangsdaten prüfen.";

/// The in-place busy glyph: the button keeps its 48 px frame and swaps the
/// label for a 20 px spinner.
pub const LOGIN_SPINNER: f32 = 20.;

/// Whether a field renders masked. Both credential screens mask the password,
/// so the pinned input disables copy/cut for it (REQ-UI-11).
pub fn password_is_masked(_kind: LoginKind) -> bool {
    true
}

impl LoginKind {
    fn title(self) -> &'static str {
        match self {
            LoginKind::Kantine => "Kantine-Zugangsdaten",
            LoginKind::Automaten => "Automaten-Zugangsdaten",
        }
    }
    fn subtitle(self) -> Option<&'static str> {
        match self {
            LoginKind::Kantine => None,
            LoginKind::Automaten => Some("Für Automaten und Kassenabrechnungen"),
        }
    }
    fn username_label(self) -> &'static str {
        "Benutzername"
    }
    fn password_label(self) -> &'static str {
        "Passwort"
    }
    fn success_message(self) -> &'static str {
        match self {
            LoginKind::Kantine => "Kantine-Zugangsdaten sicher gespeichert",
            LoginKind::Automaten => "Automaten-Zugangsdaten sicher gespeichert",
        }
    }
    fn session_text(self, state: &State) -> String {
        match self {
            LoginKind::Kantine => format!(
                "Angemeldet als: {}",
                state
                    .user_info
                    .as_ref()
                    .map(|u| u.username.clone())
                    .unwrap_or_default()
            ),
            LoginKind::Automaten => "Automaten-Sitzung aktiv".to_string(),
        }
    }
    fn authenticated(self, state: &State) -> bool {
        match self {
            LoginKind::Kantine => state.gourmet_authenticated,
            LoginKind::Automaten => state.ventopay_authenticated,
        }
    }
    fn busy(self, state: &State) -> bool {
        match self {
            LoginKind::Kantine => state.busy,
            LoginKind::Automaten => state.ventopay_busy,
        }
    }
}

/// The login form.
pub fn login_screen(
    kind: LoginKind,
    state: &State,
    _model: Option<&Arc<AppModel>>,
    user: &Entity<InputState>,
    pass: &Entity<InputState>,
    theme: &AppTheme,
    // `Some(pad)` when the same form is contained beside the settings master
    // (desktop 720+): no back row, desktop tokens, form max 440 px. `None` is
    // the incumbent pushed mobile/narrow screen.
    containment: Option<f32>,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let back = back_handler(cx);
    let save = save_handler(kind, user.clone(), pass.clone(), cx);
    let logout = logout_handler(kind, cx);
    let desktop = containment.is_some();

    let mut form = if desktop {
        div()
            .id("login-form")
            .w_full()
            .max_w(px(440.))
            .p(px(containment.unwrap_or(16.)))
            .flex()
            .flex_col()
            .gap(px(theme.scale.space_4))
    } else {
        div()
            .id("login-form")
            .w_full()
            .flex()
            .flex_col()
            .gap(px(theme.scale.space_4))
    };

    if !desktop {
        form = form.child(crate::navigation::back_affordance(
            "Einstellungen",
            theme,
            back,
        ));
    }
    form = form
        .child(
            div()
                .text_size(px(theme.scale.title))
                .line_height(px(theme.scale.title_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(kind.title()),
        )
        .when_some(kind.subtitle(), |d, sub| {
            d.child(
                div()
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
                    .text_color(theme.text_secondary)
                    .child(sub),
            )
        });

    let submit_on_enter = |field: Div| -> Div {
        if !desktop {
            return field;
        }
        let save = save.clone();
        field.on_key_down(move |event: &KeyDownEvent, window, cx| {
            if event.keystroke.key.as_str() == "enter" && !event.is_held {
                save(window, cx);
                cx.stop_propagation();
            }
        })
    };
    let (user_field, pass_field) = if desktop {
        (
            submit_on_enter(desktop_field_with_state(user, ControlState::Rest, theme)),
            submit_on_enter(desktop_field_with_state(pass, ControlState::Rest, theme)),
        )
    } else {
        (
            field_with_state(user, theme, cx),
            field_with_state(pass, theme, cx),
        )
    };
    form = form
        .child(group(kind.username_label(), user_field, theme))
        .child(group(kind.password_label(), pass_field, theme));

    if let Some(error) = login_error(kind, state) {
        form = form.child(
            div()
                .w_full()
                .p(px(theme.scale.space_3))
                .rounded(px(theme.scale.radius_card))
                .bg(theme.error_surface)
                .border(px(1.))
                .border_color(theme.error)
                .child(
                    div()
                        .text_size(px(theme.scale.meta))
                        .line_height(px(theme.scale.meta_line))
                        .text_color(theme.error)
                        .child(error),
                ),
        );
    }

    let busy = kind.busy(state);
    let (height, radius) = mobile_action_metrics(ButtonKind::Primary);
    if busy {
        form = form.child(
            div()
                .id("login-save")
                .w_full()
                .h(px(height))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(radius))
                .bg(theme.surface_alt)
                .child(spinner(theme.text_tertiary, LOGIN_SPINNER)),
        );
    } else {
        form = form.child(
            mobile_action_action(
                "login-save",
                "Speichern",
                ButtonKind::Primary,
                theme,
                cx,
                move |_, window, cx| save(window, cx),
            )
            .w_full()
            .justify_center(),
        );
    }

    if kind.authenticated(state) {
        form = form.child(
            div()
                .mt(px(theme.scale.space_2))
                .flex()
                .flex_col()
                .gap(px(theme.scale.space_2))
                .child(
                    div()
                        .text_size(px(theme.scale.meta))
                        .line_height(px(theme.scale.meta_line))
                        .text_color(theme.text_secondary)
                        .child(kind.session_text(state)),
                )
                .child(
                    div()
                        .id("login-logout")
                        .w_full()
                        .min_h(px(theme.scale.action_height))
                        .flex()
                        .items_center()
                        .px(px(theme.scale.space_4))
                        .cursor_pointer()
                        .focus(crate::theme::desktop_focus(theme.accent))
                        .on_click(move |_, window, cx| logout(window, cx))
                        .child(
                            div()
                                .text_size(px(theme.scale.body))
                                .line_height(px(theme.scale.body_line))
                                .text_color(theme.error)
                                .child("Abmelden"),
                        ),
                ),
        );
    }

    form.into_any_element()
}

/// The kind's own error, shown as one toned banner above the actions. The
/// dialog copy stays the model's authority; this is the inline companion.
fn login_error(kind: LoginKind, state: &State) -> Option<String> {
    match kind {
        LoginKind::Kantine => state.error_text.as_ref().map(|e| e.message.clone()),
        LoginKind::Automaten => state.ventopay_error.as_ref().map(|e| e.message.clone()),
    }
}

fn group(label: &'static str, field: Div, theme: &AppTheme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child(label),
        )
        .child(field)
}

fn back_handler(cx: &mut Context<AppShell>) -> crate::navigation::ClickHandler {
    let entity = cx.entity().clone();
    Rc::new(move |_window: &mut Window, cx: &mut App| {
        let _ = entity.update(cx, |this, cx| {
            this.pop_detail(cx);
        });
    })
}

fn save_handler(
    kind: LoginKind,
    user: Entity<InputState>,
    pass: Entity<InputState>,
    cx: &mut Context<AppShell>,
) -> crate::navigation::ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        let username = user.read(cx).value().to_string();
        let password = pass.read(cx).value().to_string();
        // Never log the password itself, only its length.
        crate::app::kb_log(&format!(
            "save clicked (user={username:?} pass_len={})",
            password.len()
        ));
        let _ = shell.update(cx, |s, cx| {
            if username.trim().is_empty() || password.is_empty() {
                s.set_dialog(
                    crate::components::dialog::Dialog::Alert {
                        title: "Fehler".into(),
                        message: Some("Bitte Benutzername und Passwort eingeben".into()),
                    },
                    cx,
                );
                return;
            }
            let Some(model) = s.model.clone() else {
                // iOS before task 5.7 has no session model installed yet;
                // show the §3.5 failure alert instead of swallowing the tap.
                // The success alert lands with 5.7 when the model is wired.
                s.set_dialog(
                    crate::components::dialog::Dialog::Alert {
                        title: FAILURE_TITLE.into(),
                        message: Some(FAILURE_FALLBACK.into()),
                    },
                    cx,
                );
                return;
            };
            // One save per activation: a double click, Return or key repeat
            // cannot run the auth pipeline twice (REQ-SCR-05/06).
            let state = model.state();
            let busy = match kind {
                LoginKind::Kantine => state.busy,
                LoginKind::Automaten => state.ventopay_busy,
            };
            let Some(ticket) = s.mutation_gate.claim(busy, false) else {
                return;
            };
            // Host-gated dispatch (task 1.5): the auth pipeline blocks on the
            // model's runtime, so a desktop host runs it on the worker thread
            // and shows the same alert when it lands; a phone host keeps the
            // incumbent synchronous path.
            if crate::app::dispatch_off_thread(state.host_kind) {
                let model = Arc::clone(&model);
                let result_model = Arc::clone(&model);
                let (username, password) = (username.clone(), password.clone());
                cx.spawn(
                    async move |this: gpui::WeakEntity<AppShell>, cx: &mut gpui::AsyncApp| {
                        let worker = Arc::clone(&model);
                        let result = cx
                            .background_executor()
                            .spawn(async move { run_login(kind, &worker, &username, &password) })
                            .await;
                        drop(ticket);
                        let _ = this.update(cx, |s, cx| {
                            s.set_dialog(login_result(kind, &result_model, &result), cx);
                            cx.notify();
                        });
                    },
                )
                .detach();
                cx.notify();
                return;
            }
            let result = run_login(kind, &model, &username, &password);
            drop(ticket);
            s.set_dialog(login_result(kind, &model, &result), cx);
        });
    })
}

/// Run the one credential flow of `kind` (the only difference between the two
/// forms is the service binding).
fn run_login(
    kind: LoginKind,
    model: &AppModel,
    username: &str,
    password: &str,
) -> Result<(), PlatformError> {
    match kind {
        LoginKind::Kantine => model.login(username, password),
        LoginKind::Automaten => model.ventopay_login(username, password, true),
    }
}

/// The alert a credential save ends with: the success copy, or the §3.5
/// failure alert with the model's own error text when there is one.
fn login_result(kind: LoginKind, model: &AppModel, result: &Result<(), PlatformError>) -> Dialog {
    let (title, message) = match result {
        Ok(()) => (
            "Gespeichert".to_string(),
            kind.success_message().to_string(),
        ),
        Err(_) => (
            FAILURE_TITLE.to_string(),
            model
                .state()
                .error_text
                .map(|e| e.message)
                .unwrap_or_else(|| FAILURE_FALLBACK.to_string()),
        ),
    };
    Dialog::Alert {
        title: title.into(),
        message: Some(message.into()),
    }
}

fn logout_handler(kind: LoginKind, cx: &mut Context<AppShell>) -> crate::navigation::ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        let _ = shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                match kind {
                    LoginKind::Kantine => model.gourmet_logout(),
                    LoginKind::Automaten => model.ventopay_logout(),
                }
            }
            cx.notify();
        });
    })
}

#[cfg(test)]
mod tests {
    use super::{
        login_error, password_is_masked, LoginKind, FAILURE_FALLBACK, FAILURE_TITLE, LOGIN_SPINNER,
    };
    use crate::components::text_field::clipboard_copyable;
    use snackpilot_model::{ErrorText, State};

    /// The failure alert copy of §3.5, used for a rejected login and for the
    /// iOS path without a session model (before 5.7 wires it).
    /// @req REQ-SCR-05
    /// @req REQ-UI-11 @req REQ-SCR-05
    #[test]
    fn credential_fields_mask_the_password() {
        assert!(password_is_masked(LoginKind::Kantine));
        assert!(password_is_masked(LoginKind::Automaten));
        assert!(
            !clipboard_copyable(true, true),
            "a masked password can never be copied or cut, even with a selection"
        );
        assert!(
            clipboard_copyable(false, true),
            "readable text stays copyable"
        );
        assert!(
            !clipboard_copyable(false, false),
            "no selection, no command"
        );
    }

    /// The inline error banner uses the kind's own diagnosis, and the busy
    /// button keeps the 48 px frame with a 20 px spinner in place of the label
    /// (REQ-SCR-05/REQ-SCR-06).
    /// @req REQ-SCR-05 @req REQ-SCR-06
    #[test]
    fn login_inline_error_and_busy_spinner_are_kind_specific() {
        assert_eq!(LOGIN_SPINNER, 20.);
        let state = State {
            error_text: Some(ErrorText {
                message: "Kantine offline".into(),
                retry: true,
            }),
            ventopay_error: Some(ErrorText {
                message: "Automat offline".into(),
                retry: true,
            }),
            ..Default::default()
        };
        assert_eq!(
            login_error(LoginKind::Kantine, &state).as_deref(),
            Some("Kantine offline")
        );
        assert_eq!(
            login_error(LoginKind::Automaten, &state).as_deref(),
            Some("Automat offline")
        );
    }

    /// @req REQ-SCR-05
    #[test]
    fn failure_alert_copy_matches_spec() {
        assert_eq!(FAILURE_TITLE, "Login fehlgeschlagen");
        assert_eq!(
            FAILURE_FALLBACK,
            "Anmeldung nicht möglich. Bitte Zugangsdaten prüfen."
        );
    }
}
