//! Dialog system — docs/requirements/04-ui-ux.md §4.1.
//!
//! Custom themed modal (v1 `DialogProvider`), not the platform alert: one
//! dialog visible at a time, rendered by the shell as an absolutely
//! positioned overlay. Backdrop tap dismisses and counts as "cancelled".
//!
//! Desktop keyboard (desktop-ui-redesign REQ-UI-14): focus is trapped inside
//! the action row, Enter/Space activate the focused action, Left/Right and
//! Tab/Shift+Tab move between the actions, and Esc/backdrop resolve index 0.
//! Every path goes through one [`DialogResolution`], so a click, a Return and
//! a backdrop tap can never resolve the same dialog twice.
//! @req REQ-UI-14

use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::theme::{self, AppTheme};

/// One resolution callback for pointer, keyboard and backdrop; the index is
/// the action position in the button row (0 cancels an alert/confirm).
pub type DialogActivate = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// The one visible dialog (§4.1 call shapes).
pub enum Dialog {
    /// `alert(title, message?)` — single `OK` button.
    Alert {
        title: SharedString,
        message: Option<SharedString>,
    },
    /// `confirm(title, message)` — `[Abbrechen, Bestätigen(destructive)]`;
    /// `on_confirm` runs only when the destructive button is tapped.
    Confirm {
        title: SharedString,
        message: SharedString,
        /// Overrides the default `"Bestätigen"` (§4.1 destructive label).
        confirm_label: Option<&'static str>,
        /// Overrides the default `"Abbrechen"`.
        cancel_label: Option<&'static str>,
        on_confirm: crate::navigation::ClickHandler,
    },
}

impl Dialog {
    /// Number of action buttons (an alert has one, a confirm two).
    pub fn button_count(&self) -> usize {
        match self {
            Dialog::Alert { .. } => 1,
            Dialog::Confirm { .. } => 2,
        }
    }

    /// Label of the action at `index`, in reading order.
    pub fn label_at(&self, index: usize) -> &'static str {
        match self {
            Dialog::Alert { .. } => "OK",
            Dialog::Confirm {
                confirm_label,
                cancel_label,
                ..
            } => match index {
                0 => cancel_label.unwrap_or("Abbrechen"),
                _ => confirm_label.unwrap_or("Bestätigen"),
            },
        }
    }

    /// The initially focused action: `OK` for an alert, the visible confirm
    /// for a confirm (brief "initially focuses OK/confirm").
    pub fn initial_focus(&self) -> usize {
        self.button_count() - 1
    }
}

/// One resolution per dialog. The first activation claims it; every later
/// click/Return/backdrop sees `None`, so a double activation cannot run a
/// destructive closure twice.
#[derive(Debug, Default)]
pub struct DialogResolution {
    resolved: bool,
}

impl DialogResolution {
    pub fn reset(&mut self) {
        self.resolved = false;
    }

    /// Claim the dialog for one action. `None` when it already resolved or
    /// the index does not name an action.
    pub fn begin(&mut self, index: usize, button_count: usize) -> Option<usize> {
        if self.resolved || index >= button_count {
            return None;
        }
        self.resolved = true;
        Some(index)
    }
}

/// What one key press does inside an open dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKey {
    Ignore,
    Previous,
    Next,
    Activate(usize),
    Dismiss,
}

/// Pure dialog keyboard rule (REQ-UI-14). Auto-repeat of the confirm keys is
/// ignored, so holding the key that opened the dialog cannot confirm it;
/// Enter/Space use the focused action, Esc resolves index 0 and the arrows
/// and Tab cycle only inside the action row.
pub fn dialog_key(
    key: &str,
    shift: bool,
    is_held: bool,
    focused: usize,
    button_count: usize,
) -> DialogKey {
    let count = button_count.max(1);
    let focused = focused.min(count - 1);
    match key {
        "enter" | "space" => {
            if is_held {
                DialogKey::Ignore
            } else {
                DialogKey::Activate(focused)
            }
        }
        "escape" => DialogKey::Dismiss,
        "tab" => {
            if shift {
                DialogKey::Previous
            } else {
                DialogKey::Next
            }
        }
        "left" => DialogKey::Previous,
        "right" => DialogKey::Next,
        _ => DialogKey::Ignore,
    }
}

/// Move the dialog focus one action, wrapping inside the action row.
pub fn dialog_move(focused: usize, button_count: usize, forward: bool) -> usize {
    let count = button_count.max(1);
    if forward {
        (focused + 1) % count
    } else {
        (focused + count - 1) % count
    }
}

/// A §4.1 dialog button: `py 10`, `radius 8`, fontSize 15 weight 600.
#[allow(clippy::too_many_arguments)]
fn dialog_button(
    id: &'static str,
    label: &'static str,
    fill: Option<Hsla>,
    border: Option<Hsla>,
    fg: Hsla,
    grow: bool,
    focus: &FocusHandle,
    theme: &AppTheme,
    on_click: crate::navigation::ClickHandler,
) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .flex()
        .items_center()
        .justify_center()
        .py(px(10.))
        .min_h(px(snackpilot_model::desktop::TARGET_MIN))
        .rounded(px(8.))
        .text_size(px(15.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(fg)
        .cursor_pointer()
        .when_some(fill, |d, c| d.bg(c))
        .when_some(border, |d, c| d.border_1().border_color(c))
        .when(grow, |d| d.flex_1())
        .track_focus(focus)
        .focus(theme::desktop_focus(theme.desktop.focus))
        .child(label)
        .on_click(move |_, window, cx| on_click(window, cx))
}

/// The §4.1 modal: `overlay` backdrop with padding 40, content box centered
/// (surface, radius 14, padding 24, maxWidth 340, 0.5 `border`).
/// Tapping the backdrop resolves action 0; tapping the box does nothing.
/// `on_activate` receives the button index and is the single resolution path
/// for pointer, keyboard and backdrop.
pub fn dialog_overlay(
    dialog: &Dialog,
    theme: &AppTheme,
    focused: usize,
    focus: &[FocusHandle; 2],
    on_activate: DialogActivate,
) -> AnyElement {
    let mut buttons: Vec<AnyElement> = Vec::new();
    for index in 0..dialog.button_count() {
        let activate = on_activate.clone();
        let action_index = index;
        let id = match (dialog, index) {
            (Dialog::Alert { .. }, _) => "dialog-ok",
            (Dialog::Confirm { .. }, 0) => "dialog-cancel",
            _ => "dialog-confirm",
        };
        let (fill, border, fg) = match dialog {
            Dialog::Alert { .. } => (Some(theme.primary), None, theme.on_primary),
            Dialog::Confirm { .. } if index == 0 => {
                (None, Some(theme.border), theme.text_secondary)
            }
            Dialog::Confirm { .. } => (Some(theme.error), None, theme.on_primary),
        };
        buttons.push(
            dialog_button(
                id,
                dialog.label_at(index),
                fill,
                border,
                fg,
                dialog.button_count() > 1,
                &focus[index],
                theme,
                Rc::new(move |w, cx| activate(action_index, w, cx)),
            )
            .into_any_element(),
        );
    }

    let backdrop = on_activate;
    let title = match dialog {
        Dialog::Alert { title, .. } => title.clone(),
        Dialog::Confirm { title, .. } => title.clone(),
    };
    let message = match dialog {
        Dialog::Alert { message, .. } => message.clone(),
        Dialog::Confirm { message, .. } => Some(message.clone()),
    };
    // The AppShell owns the focused index and the key rule; the overlay only
    // paints the row so no second keyboard path exists beside it.
    let _ = focused.min(dialog.button_count() - 1);

    div()
        .id("dialog-backdrop")
        // §4.1: the modal must occlude what is underneath — a backdrop tap
        // dismisses and must not also reach the tab bar or settings rows
        // (same pattern as loading_overlay).
        .occlude()
        .absolute()
        .inset_0()
        .bg(theme.overlay)
        .p(px(40.))
        .flex()
        .items_center()
        .justify_center()
        .on_click(move |_, window, cx| backdrop(0, window, cx))
        .child(
            div()
                .id("dialog-box")
                .w_full()
                .max_w(px(340.))
                .max_h_full()
                .flex()
                .flex_col()
                .items_center()
                .rounded(px(14.))
                .p(px(24.))
                .bg(theme.surface)
                .border(px(0.5))
                .border_color(theme.border)
                // Tapping the dialog body must not dismiss (§4.1).
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .text_size(px(17.))
                        .line_height(px(22.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .text_center()
                        .child(title),
                )
                .when_some(message, |d, m| {
                    d.child(
                        div()
                            .id("dialog-message")
                            .mt(px(8.))
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(theme.text_secondary)
                            .text_center()
                            // A long message scrolls; the action row stays
                            // visible (brief "short window").
                            .max_h(px(320.))
                            .overflow_y_scroll()
                            .child(m),
                    )
                })
                .child(
                    div()
                        .mt(px(20.))
                        .w_full()
                        .flex()
                        .flex_wrap()
                        .gap(px(10.))
                        .items_center()
                        .justify_center()
                        .children(buttons),
                ),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{dialog_key, dialog_move, Dialog, DialogKey, DialogResolution};

    fn alert() -> Dialog {
        Dialog::Alert {
            title: "Fehler".into(),
            message: None,
        }
    }

    fn confirm() -> Dialog {
        Dialog::Confirm {
            title: "Bestellung stornieren".into(),
            message: "\"MENÜ I\" stornieren?".into(),
            confirm_label: Some("Stornieren"),
            cancel_label: Some("Behalten"),
            on_confirm: std::rc::Rc::new(|_, _| {}),
        }
    }

    /// Enter/Space activate the focused action, auto-repeat is ignored so a
    /// held opening key cannot confirm, Esc resolves index 0, and Tab/arrows
    /// cycle only inside the action row (REQ-UI-14).
    /// @req REQ-UI-14
    #[test]
    fn dialog_keys_activate_focus_and_never_repeat() {
        assert_eq!(
            dialog_key("enter", false, false, 1, 2),
            DialogKey::Activate(1)
        );
        assert_eq!(dialog_key("enter", false, true, 1, 2), DialogKey::Ignore);
        assert_eq!(dialog_key("space", false, true, 1, 2), DialogKey::Ignore);
        assert_eq!(
            dialog_key("space", false, false, 0, 1),
            DialogKey::Activate(0)
        );
        assert_eq!(dialog_key("escape", false, false, 1, 2), DialogKey::Dismiss);
        assert_eq!(dialog_key("tab", false, false, 1, 2), DialogKey::Next);
        assert_eq!(dialog_key("tab", true, false, 0, 2), DialogKey::Previous);
        assert_eq!(dialog_key("right", false, false, 0, 2), DialogKey::Next);
        assert_eq!(dialog_key("left", false, false, 0, 2), DialogKey::Previous);
        assert_eq!(dialog_key("a", false, false, 0, 2), DialogKey::Ignore);
        assert_eq!(dialog_move(1, 2, true), 0, "Tab wraps inside the modal");
        assert_eq!(dialog_move(0, 2, false), 1, "Shift+Tab wraps backwards");
    }

    /// A dialog resolves exactly once: a second Enter, click or backdrop tap
    /// after the first activation resolves nothing (REQ-UI-14).
    /// @req REQ-UI-14
    #[test]
    fn dialog_resolves_once() {
        let mut resolution = DialogResolution::default();
        assert_eq!(resolution.begin(1, 2), Some(1));
        assert_eq!(resolution.begin(1, 2), None);
        assert_eq!(resolution.begin(0, 2), None);
        resolution.reset();
        assert_eq!(resolution.begin(0, 2), Some(0));
        assert_eq!(resolution.begin(2, 2), None, "index outside the row");
    }

    /// The alert focuses OK; a confirm focuses its visible confirm action,
    /// and the cancellation override labels stay verbatim.
    /// @req REQ-UI-14
    #[test]
    fn dialog_focus_and_labels_follow_the_api() {
        assert_eq!(alert().button_count(), 1);
        assert_eq!(alert().initial_focus(), 0);
        assert_eq!(alert().label_at(0), "OK");
        assert_eq!(confirm().button_count(), 2);
        assert_eq!(confirm().initial_focus(), 1);
        assert_eq!(confirm().label_at(0), "Behalten");
        assert_eq!(confirm().label_at(1), "Stornieren");
    }
}
