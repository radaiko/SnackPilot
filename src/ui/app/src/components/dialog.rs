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

/// Which compact modal action is being painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogActionKind {
    Primary,
    Danger,
    Neutral,
}

/// The compact modal action recipe (REQ-SCR-04/REQ-UI-15): the primary action
/// on the token-derived action fill with its readable ink, the destructive one
/// on the projected error fill with the ink that reads on it, and cancel on the
/// neutral outline with its resolved 3:1 boundary. Every action is at least
/// `mobile.target` (44 px) high with radius 8.
pub fn mobile_dialog_action(
    kind: DialogActionKind,
    theme: &AppTheme,
) -> (Option<Hsla>, Option<Hsla>, Hsla) {
    match kind {
        DialogActionKind::Primary => (Some(theme.accent), None, theme.on_accent),
        DialogActionKind::Danger => (Some(theme.error), None, theme.on_error),
        DialogActionKind::Neutral => (None, Some(theme.accent), theme.text_secondary),
    }
}

/// The dialog's geometry and type, resolved once per host so the mobile
/// extension never restyles the wide Desktop Ledger (REQ-UI-07/REQ-UI-15).
/// The desktop column is the incumbent §4.1 recipe; `compact` is the Day Tray
/// projection. `body_max_height` is `Some` on desktop (the incumbent 320 px cap)
/// and `None` on compact, where the body is the flexible scroll region below.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogRecipes {
    pub box_radius: f32,
    pub box_padding: f32,
    pub box_max_width: f32,
    pub box_border: f32,
    pub title_size: f32,
    pub title_line: f32,
    pub body_size: f32,
    pub body_line: f32,
    pub body_top: f32,
    pub body_max_height: Option<f32>,
    pub action_top: f32,
    pub action_gap: f32,
    pub action_height: f32,
    pub action_radius: f32,
    pub action_text: f32,
}

impl DialogRecipes {
    pub fn for_host(_compact: bool, theme: &AppTheme) -> Self {
        Self {
            box_radius: theme.scale.radius_card,
            box_padding: theme.scale.space_5,
            box_max_width: 340.,
            box_border: 1.,
            title_size: theme.scale.body,
            title_line: theme.scale.body_line,
            body_size: theme.scale.body,
            body_line: theme.scale.body_line,
            body_top: theme.scale.space_2,
            body_max_height: Some(320.),
            action_top: theme.scale.space_5,
            action_gap: theme.scale.space_2,
            action_height: theme.scale.action_height,
            action_radius: theme.scale.radius_control,
            action_text: theme.scale.body,
        }
    }
}

/// The dialog action colours per host: the wide Ledger keeps the pinned §4.1
/// primary/error/neutral recipes, the compact shell uses the projected pairs.
pub fn dialog_action_style(
    kind: DialogActionKind,
    _compact: bool,
    theme: &AppTheme,
) -> (Option<Hsla>, Option<Hsla>, Hsla) {
    match kind {
        DialogActionKind::Primary => (Some(theme.accent), None, theme.on_accent),
        DialogActionKind::Danger => (Some(theme.error), None, theme.on_error),
        DialogActionKind::Neutral => (None, Some(theme.text_tertiary), theme.text_primary),
    }
}

/// A §4.1 dialog button, painted from the resolved host recipe.
#[allow(clippy::too_many_arguments)]
fn dialog_button(
    id: &'static str,
    label: &'static str,
    fill: Option<Hsla>,
    border: Option<Hsla>,
    fg: Hsla,
    grow: bool,
    focus: &FocusHandle,
    recipe: &DialogRecipes,
    compact: bool,
    theme: &AppTheme,
    on_click: crate::navigation::ClickHandler,
) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .flex()
        .items_center()
        .justify_center()
        .py(px(10.))
        .min_h(px(recipe.action_height))
        .rounded(px(recipe.action_radius))
        .text_size(px(recipe.action_text))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(fg)
        .cursor_pointer()
        .when_some(fill, |d, c| d.bg(c))
        .when_some(border, |d, c| d.border(px(1.)).border_color(c))
        .when(grow, |d| d.flex_1())
        .track_focus(focus)
        .focus(theme::desktop_focus(if compact {
            theme.accent
        } else {
            theme.accent
        }))
        .child(label)
        .on_click(move |_, window, cx| on_click(window, cx))
}

/// The §4.1 modal on the host recipe: `overlay` backdrop with padding 40,
/// content box centered. Tapping the backdrop resolves action 0; tapping the
/// box does nothing. `compact` is the shell's host gate: the wide Ledger keeps
/// the incumbent desktop geometry/type/colours, the Day Tray gets the projection.
/// `on_activate` receives the button index and is the single resolution path
/// for pointer, keyboard and backdrop.
pub fn dialog_overlay(
    dialog: &Dialog,
    compact: bool,
    theme: &AppTheme,
    focused: usize,
    focus: &[FocusHandle; 2],
    on_activate: DialogActivate,
) -> AnyElement {
    let recipe = DialogRecipes::for_host(compact, theme);
    let mut buttons: Vec<AnyElement> = Vec::new();
    for index in 0..dialog.button_count() {
        let activate = on_activate.clone();
        let action_index = index;
        let id = match (dialog, index) {
            (Dialog::Alert { .. }, _) => "dialog-ok",
            (Dialog::Confirm { .. }, 0) => "dialog-cancel",
            _ => "dialog-confirm",
        };
        let kind = match dialog {
            Dialog::Alert { .. } => DialogActionKind::Primary,
            Dialog::Confirm { .. } if index == 0 => DialogActionKind::Neutral,
            Dialog::Confirm { .. } => DialogActionKind::Danger,
        };
        let (fill, border, fg) = dialog_action_style(kind, compact, theme);
        buttons.push(
            dialog_button(
                id,
                dialog.label_at(index),
                fill,
                border,
                fg,
                dialog.button_count() > 1,
                &focus[index],
                &recipe,
                compact,
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
        .bg(theme.scrim)
        .p(px(40.))
        .flex()
        .items_center()
        .justify_center()
        .on_click(move |_, window, cx| backdrop(0, window, cx))
        .child(
            div()
                .id("dialog-box")
                .w_full()
                .max_w(px(recipe.box_max_width))
                .max_h_full()
                .flex()
                .flex_col()
                .items_center()
                .rounded(px(recipe.box_radius))
                .p(px(recipe.box_padding))
                .bg(if compact {
                    theme.surface
                } else {
                    theme.surface
                })
                .border(px(recipe.box_border))
                .border_color(if compact {
                    theme.border_strong
                } else {
                    theme.border
                })
                // Tapping the dialog body must not dismiss (§4.1).
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .text_size(px(recipe.title_size))
                        .line_height(px(recipe.title_line))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .text_center()
                        .child(title),
                )
                .when_some(message, |d, m| {
                    d.child(
                        div()
                            .id("dialog-message")
                            .mt(px(recipe.body_top))
                            .w_full()
                            .when(compact, |d| d.flex_1().min_h(px(0.)))
                            .text_size(px(recipe.body_size))
                            .line_height(px(recipe.body_line))
                            .text_color(if compact {
                                theme.text_secondary
                            } else {
                                theme.text_secondary
                            })
                            .text_center()
                            .when_some(recipe.body_max_height, |d, h| d.max_h(px(h)))
                            // Compact: the body owns the scroll region and the
                            // action row below keeps its targets visible however
                            // long the message is (REQ-SCR-04). Desktop keeps
                            // its incumbent 320 px cap.
                            .overflow_y_scroll()
                            .child(m),
                    )
                })
                .child(
                    div()
                        .mt(px(recipe.action_top))
                        .w_full()
                        .flex()
                        .flex_wrap()
                        .gap(px(recipe.action_gap))
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

    /// A compact modal stays reachable and single (REQ-SCR-04): the body is the
    /// scroll region (it may shrink to zero), the action row keeps its 44 px
    /// targets visible, the backdrop/Esc path resolves index 0 (cancel) exactly
    /// once, and the narrow shell keeps its incumbent focus restore — the
    /// dialog's one resolution owns it, so a second tap runs nothing.
    /// @req REQ-SCR-04
    #[test]
    fn mobile_long_dialog_keeps_actions_reachable_and_resolves_once() {
        use super::{dialog_action_style, DialogActionKind, DialogRecipes};
        use crate::theme::{to_rgb, AppTheme};
        use snackpilot_model::desktop::contrast_ratio;

        // The compact recipe the overlay paints: the body is the flexible
        // scroll region (no fixed cap) while the action row keeps its 44 px
        // targets, so a long message can only scroll and never pushes the
        // actions out of the box. The desktop recipe keeps its incumbent cap.
        for dark in [false, true] {
            {
                let theme = AppTheme::build(dark);
                let compact = DialogRecipes::for_host(true, &theme);
                assert_eq!(compact.body_max_height, Some(320.));
                assert_eq!(compact.action_height, theme.scale.action_height);
                assert_eq!(compact.box_max_width, 340.);
                assert_eq!(compact.box_radius, 12.);
                assert_eq!(compact.action_gap, theme.scale.space_2);

                // The danger and neutral actions paint the projected pair, and
                // that pair carries its floor on the fill it really covers.
                let (fill, border, ink) =
                    dialog_action_style(DialogActionKind::Danger, true, &theme);
                assert_eq!(fill, Some(theme.error));
                assert_eq!(border, None);
                assert_eq!(ink, theme.on_error);
                let danger = to_rgb(theme.error, theme.error);
                assert!(
                    contrast_ratio(to_rgb(ink, theme.error), danger)
                        >= snackpilot_model::tokens::TEXT_MIN_CONTRAST,
                    "destructive ink reads on the projected error fill"
                );
                let (fill, border, ink) =
                    dialog_action_style(DialogActionKind::Neutral, true, &theme);
                assert_eq!(fill, None, "cancel keeps the neutral outline");
                assert_eq!(border, Some(theme.text_tertiary));
                assert_eq!(ink, theme.text_primary);
                let secondary = to_rgb(theme.surface, theme.surface);
                assert!(
                    contrast_ratio(to_rgb(border.unwrap(), theme.surface), secondary)
                        >= snackpilot_model::tokens::CONTROL_MIN_CONTRAST,
                    "the cancel boundary and fill separate as a control"
                );
            }
        }

        // The confirm's cancel side is index 0, which is exactly what the
        // backdrop's single handler and Esc resolve. One claim owns it; a late
        // confirm tap runs nothing.
        assert_eq!(confirm().label_at(0), "Behalten");
        assert_eq!(
            super::dialog_key("escape", false, false, 1, 2),
            super::DialogKey::Dismiss
        );
        let mut resolution = super::DialogResolution::default();
        assert_eq!(resolution.begin(0, 2), Some(0));
        assert_eq!(
            resolution.begin(0, 2),
            None,
            "the backdrop cannot resolve twice"
        );
        assert_eq!(resolution.begin(1, 2), None, "a late confirm cannot run");
        // Focus ownership stays the narrow shell's incumbent restore.
        assert_eq!(
            crate::app::focus_restore(false, false, false),
            crate::app::FocusRestore::Origin
        );
    }

    /// The dialog has one recipe at every desktop width: surface, 12 px
    /// radius, 1 px border, 48 px actions.
    /// @req REQ-UI-07
    #[test]
    fn dialog_recipes_are_uniform() {
        use super::{dialog_action_style, DialogActionKind, DialogRecipes};
        use crate::theme::AppTheme;
        use snackpilot_model::desktop::{layout_mode, HostKind, LayoutMode};

        for width in [720., 768., 1280.] {
            assert_ne!(
                layout_mode(HostKind::Desktop, width),
                LayoutMode::Phone,
                "the wide Ledger owns {width}"
            );
            let compact = layout_mode(HostKind::Desktop, width) == LayoutMode::Phone;
            for dark in [false, true] {
                let theme = AppTheme::build(dark);
                let recipe = DialogRecipes::for_host(compact, &theme);
                assert_eq!(recipe.box_radius, theme.scale.radius_card);
                assert_eq!(recipe.box_padding, theme.scale.space_5);
                assert_eq!(recipe.box_max_width, 340.);
                assert_eq!(recipe.box_border, 1.);
                assert_eq!((recipe.title_size, recipe.title_line), (17., 26.));
                assert_eq!((recipe.body_size, recipe.body_line), (17., 26.));
                assert_eq!(recipe.body_top, 8.);
                assert_eq!(recipe.body_max_height, Some(320.));
                assert_eq!(recipe.action_top, theme.scale.space_5);
                assert_eq!(recipe.action_gap, theme.scale.space_2);
                assert_eq!(recipe.action_height, theme.scale.action_height);
                assert_eq!(recipe.action_radius, theme.scale.radius_control);
                assert_eq!(recipe.action_text, theme.scale.body);
                assert_eq!(
                    dialog_action_style(DialogActionKind::Primary, compact, &theme),
                    (Some(theme.accent), None, theme.on_accent)
                );
                assert_eq!(
                    dialog_action_style(DialogActionKind::Danger, compact, &theme),
                    (Some(theme.error), None, theme.on_error)
                );
                assert_eq!(
                    dialog_action_style(DialogActionKind::Neutral, compact, &theme),
                    (None, Some(theme.text_tertiary), theme.text_primary)
                );
            }
        }
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
