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

use crate::app::AppShell;
use crate::components::text_field::field_with_state;
use crate::theme::{self, AppTheme};
use snackpilot_model::{AppModel, State};

/// Which credentials pair the form edits.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LoginKind {
    Kantine,
    Automaten,
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
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let back = back_handler(cx);
    let save = save_handler(kind, user.clone(), pass.clone(), cx);
    let logout = logout_handler(kind, cx);

    let mut form = div()
        .id("login-form")
        .flex()
        .flex_col()
        .p(px(20.))
        .pb(px(100.))
        .gap(px(16.));

    form = form
        .child(crate::navigation::back_affordance(
            "Einstellungen",
            theme,
            back,
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(28.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.text_primary)
                        .child(kind.title()),
                )
                .when_some(kind.subtitle(), |d, sub| {
                    d.child(
                        div()
                            .mt(px(4.))
                            .mb(px(24.))
                            .text_size(px(14.))
                            .text_color(theme.text_tertiary)
                            .child(sub),
                    )
                }),
        );

    form = form
        .child(group(
            kind.username_label(),
            field_with_state(user, theme),
            theme,
        ))
        .child(group(
            kind.password_label(),
            field_with_state(pass, theme),
            theme,
        ));

    let label = if kind.busy(state) {
        "Speichern..."
    } else {
        "Speichern"
    };
    let primary = theme.button_primary();
    let button = div().flex().items_center().justify_center().py(px(14.));
    let button = theme::apply(button, &primary).id("login-save");
    let button = if kind.busy(state) {
        button
    } else {
        button
            .cursor_pointer()
            .on_click(move |_, window, cx| save(window, cx))
    };
    form = form.child(
        button.rounded(px(14.)).child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.on_primary)
                .child(label),
        ),
    );

    if kind.authenticated(state) {
        form = form.child(
            div()
                .mt(px(16.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .mb(px(12.))
                        .text_size(px(14.))
                        .text_color(theme.text_secondary)
                        .child(kind.session_text(state)),
                )
                .child(
                    div()
                        .id("login-logout")
                        .flex()
                        .items_center()
                        .justify_center()
                        .py(px(14.))
                        .px(px(24.))
                        .cursor_pointer()
                        .on_click(move |_, window, cx| logout(window, cx))
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(theme.on_primary)
                                .child("Abmelden"),
                        ),
                ),
        );
    }

    form.into_any_element()
}

fn group(label: &'static str, field: Div, theme: &AppTheme) -> Div {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .mb(px(6.))
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
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
            let Some(model) = &s.model else {
                return;
            };
            let result = match kind {
                LoginKind::Kantine => model.login(&username, &password),
                LoginKind::Automaten => model.ventopay_login(&username, &password, true),
            };
            let (title, message) = match result {
                Ok(()) => (
                    "Gespeichert".to_string(),
                    kind.success_message().to_string(),
                ),
                Err(_) => (
                    "Login fehlgeschlagen".to_string(),
                    model
                        .state()
                        .error_text
                        .map(|e| e.message)
                        .unwrap_or_else(|| {
                            "Anmeldung nicht möglich. Bitte Zugangsdaten prüfen.".to_string()
                        }),
                ),
            };
            s.set_dialog(
                crate::components::dialog::Dialog::Alert {
                    title: title.into(),
                    message: Some(message.into()),
                },
                cx,
            );
        });
    })
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
