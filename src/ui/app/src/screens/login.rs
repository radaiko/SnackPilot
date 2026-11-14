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
use gpui_kit::assets::IconName;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable};
use snackpilot_model::desktop::ControlState;
use snackpilot_model::tokens;

use crate::app::AppShell;
use crate::components::button::{mobile_action_action, mobile_action_metrics, ButtonKind};
use crate::components::dialog::Dialog;
use crate::components::spinner::spinner;
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

/// Inline credential-field errors: shown once a save was attempted and the
/// field is still empty (REQ-SCR-05).
const USER_REQUIRED: &str = "Benutzername erforderlich";
const PASS_REQUIRED: &str = "Passwort erforderlich";

/// Which inline errors a render shows: none before the first save attempt,
/// then only for the fields that are still empty (REQ-SCR-05).
fn missing_messages(
    attempted: bool,
    user_empty: bool,
    pass_empty: bool,
) -> (Option<&'static str>, Option<&'static str>) {
    if !attempted {
        return (None, None);
    }
    (
        user_empty.then_some(USER_REQUIRED),
        pass_empty.then_some(PASS_REQUIRED),
    )
}

/// The reveal control's copy and element id: the action it offers, never the
/// field's content.
fn reveal_label(masked: bool) -> &'static str {
    if masked {
        "Passwort anzeigen"
    } else {
        "Passwort verbergen"
    }
}

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
    // Set by the shell's `login_attempted`: the fields show their inline
    // errors only after a save that found one empty (REQ-SCR-05).
    attempted: bool,
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
    let (user_missing, pass_missing) = missing_messages(
        attempted,
        user.read(cx).value().trim().is_empty(),
        pass.read(cx).value().is_empty(),
    );
    let user_field = submit_on_enter(credential_row(
        user,
        None,
        user_missing.is_some(),
        desktop,
        theme,
        cx,
    ));
    let pass_field = submit_on_enter(credential_row(
        pass,
        Some(reveal_toggle(pass, theme, cx)),
        pass_missing.is_some(),
        desktop,
        theme,
        cx,
    ));
    form = form
        .child(group(
            kind.username_label(),
            user_field,
            user_missing,
            theme,
        ))
        .child(group(
            kind.password_label(),
            pass_field,
            pass_missing,
            theme,
        ));

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

/// The reveal control inside the password row's trailing edge: a 44×44 target
/// 8 px from the text, flipping the field's mask. It never grows the 48 px row
/// and names the action it offers.
fn reveal_toggle(pass: &Entity<InputState>, theme: &AppTheme, cx: &App) -> AnyElement {
    let masked = pass.read(cx).presentation().is_masked();
    let label = reveal_label(masked);
    let pass = pass.clone();
    div()
        .id(label)
        .tab_index(0)
        .w(px(tokens::TARGET))
        .h(px(tokens::TARGET))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .rounded(px(tokens::RADIUS_CHIP))
        .hover(move |s| s.bg(theme.hover(theme.surface)))
        .focus(crate::theme::desktop_focus(theme.accent))
        .role(Role::Button)
        .aria_label(label)
        .accessibility_id(label)
        .on_click(move |_, window, cx| {
            pass.update(cx, |state, cx| state.toggle_masked(window, cx));
        })
        .tooltip(move |window, cx| Tooltip::new(label).build(window, cx))
        .child(
            Icon::new(if masked {
                IconName::Eye
            } else {
                IconName::EyeOff
            })
            .with_size(px(theme.scale.icon))
            .text_color(theme.text_secondary),
        )
        .into_any_element()
}

/// One credential field: the shared 48 px row (surface fill, 1 px border,
/// 10 px radius), an optional trailing control inside it, and the error
/// border while its inline message is showing.
fn credential_row(
    state: &Entity<InputState>,
    trailing: Option<AnyElement>,
    error: bool,
    desktop: bool,
    theme: &AppTheme,
    cx: &App,
) -> Div {
    let (fill, border) = if desktop {
        (
            theme.control_fill(ControlState::Rest),
            if error {
                theme.error
            } else {
                theme.control_border(ControlState::Rest)
            },
        )
    } else {
        (
            theme.surface,
            if error {
                theme.error
            } else {
                theme.text_tertiary
            },
        )
    };
    let mut row = div()
        .w_full()
        .min_h(px(tokens::ACTION_HEIGHT))
        .flex()
        .items_center()
        .gap(px(tokens::SPACE_2))
        .px(px(tokens::SPACE_4))
        .rounded(px(tokens::RADIUS_CONTROL))
        .bg(fill)
        .border(px(1.))
        .border_color(border)
        .child(
            Input::new(state)
                .bg(fill)
                .rounded(px(tokens::RADIUS_CONTROL))
                .bordered(false),
        );
    if let Some(trailing) = trailing {
        row = row.child(trailing);
    }
    if desktop {
        return crate::theme::focus_frame(row, ControlState::Rest, tokens::RADIUS_CONTROL, theme);
    }
    let focus = state.read(cx).focus_handle(cx);
    div()
        .w_full()
        .p(px(tokens::FOCUS_GAP))
        .rounded(px(tokens::RADIUS_CONTROL
            + tokens::FOCUS_GAP
            + tokens::FOCUS_WIDTH))
        .border(px(tokens::FOCUS_WIDTH))
        .border_color(hsla(0., 0., 0., 0.))
        .track_focus(&focus)
        .focus(crate::theme::desktop_focus(theme.accent))
        .child(row)
}

/// One labelled field with its inline error, 4 px under the field and only
/// while that error is showing (REQ-SCR-05).
fn group(label: &'static str, field: Div, error: Option<&'static str>, theme: &AppTheme) -> Div {
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
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(tokens::SPACE_1))
                .child(field)
                .when_some(error, |d, message| {
                    d.child(
                        div()
                            .text_size(px(theme.scale.meta))
                            .line_height(px(theme.scale.meta_line))
                            .text_color(theme.error)
                            .child(message),
                    )
                }),
        )
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
                // Nothing is submitted; the form itself shows which field is
                // missing and clears that message as soon as it is filled
                // (REQ-SCR-05).
                s.login_attempted = true;
                cx.notify();
                return;
            }
            s.login_attempted = false;
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
        login_error, missing_messages, password_is_masked, reveal_label, LoginKind,
        FAILURE_FALLBACK, FAILURE_TITLE, LOGIN_SPINNER, PASS_REQUIRED, USER_REQUIRED,
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

    /// Inline errors appear only after a save attempt, and only on a field
    /// that is still empty; filling it clears its message on the next render.
    /// @req REQ-SCR-05
    #[test]
    fn missing_messages_follow_attempt_and_empty_fields() {
        assert_eq!(missing_messages(false, true, true), (None, None));
        assert_eq!(
            missing_messages(true, true, true),
            (Some(USER_REQUIRED), Some(PASS_REQUIRED))
        );
        assert_eq!(
            missing_messages(true, false, true),
            (None, Some(PASS_REQUIRED))
        );
        assert_eq!(
            missing_messages(true, true, false),
            (Some(USER_REQUIRED), None)
        );
        assert_eq!(missing_messages(true, false, false), (None, None));
    }

    /// The reveal control names the action it offers.
    /// @req REQ-UI-11
    #[test]
    fn password_reveal_copy_follows_masking() {
        assert_eq!(reveal_label(true), "Passwort anzeigen");
        assert_eq!(reveal_label(false), "Passwort verbergen");
    }
}
