//! Compact pending dock — task 1.3 (was the §3.1 floating action button).
//!
//! The dock replaces the absolute FAB column: it spans the Day Tray column,
//! sits in the band the shell reserves above platform navigation (separated by
//! 8 px), and carries exactly one action, the model's `fab_label()`. During an
//! operation it shows the existing phase text in the same band and offers no
//! repeat action. It never invents a current-day count or a success promise.

use gpui::*;

use crate::components::spinner::spinner;
use crate::navigation::ClickHandler;
use crate::theme::AppTheme;

/// Gap between the dock and the platform navigation (design "Menüs": separated
/// by 8 px above navigation).
pub const DOCK_GAP: f32 = 8.;

/// What the compact dock presents for the current pending set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockState {
    /// Pending changes wait: exactly `model.fab_label()`, the one action.
    Action(String),
    /// An operation runs: the existing exact phase, no repeat action.
    Progress(String),
}

impl DockState {
    /// The one text the dock paints.
    pub fn text(&self) -> &str {
        match self {
            DockState::Action(label) => label,
            DockState::Progress(phase) => phase,
        }
    }

    /// The action label while a pending set waits; `None` during an operation,
    /// which is what removes the repeat action.
    pub fn action(&self) -> Option<&str> {
        match self {
            DockState::Action(label) => Some(label),
            DockState::Progress(_) => None,
        }
    }

    /// Whether the dock offers a target at all.
    pub fn is_action(&self) -> bool {
        self.action().is_some()
    }
}

/// The dock's presence and content, under exactly the incumbent conditions
/// (task 1.3): the pending-count action while changes wait, the existing phase
/// while an operation runs, nothing otherwise. The label is never rebuilt here
/// — the caller passes `AppModel::fab_label()` verbatim.
pub fn dock_state(
    pending: usize,
    progress: Option<&str>,
    label: impl Into<String>,
) -> Option<DockState> {
    match progress {
        Some(phase) => Some(DockState::Progress(phase.to_string())),
        None if pending > 0 => Some(DockState::Action(label.into())),
        None => None,
    }
}

/// The width the dock's label can wrap in: the column minus the button's
/// horizontal 16 px padding, and the 16 px spinner plus its 8 px gap when the
/// row is an operation phase instead of an action.
pub fn dock_available_width(state: &DockState, column_width: f32, theme: &AppTheme) -> f32 {
    let lead = if state.is_action() {
        0.
    } else {
        theme.scale.space_4 + theme.scale.space_2
    };
    (column_width - 2. * theme.scale.space_4 - lead).max(1.)
}

/// The dock's height for a measured wrapped line count (REQ-UI-15): one 24 px
/// body line per line, never below the 48 px primary-action floor. The painted
/// dock is given exactly this height, so the reservation and the dock cannot
/// disagree.
pub fn dock_height_for_lines(lines: usize, theme: &AppTheme) -> f32 {
    (lines.max(1) as f32 * theme.scale.body_line).max(theme.scale.action_height)
}

/// How many lines the dock's label really wraps to at this column width: the
/// label is shaped with the body metrics in the bundled face at the width the
/// painted dock leaves for text, never estimated from a character count. This
/// is what the shell reserves, so a long phase grows the band instead of being
/// covered by it.
pub fn dock_label_lines(
    state: &DockState,
    column_width: f32,
    theme: &AppTheme,
    text_system: &WindowTextSystem,
) -> usize {
    let text = state.text();
    let mut font = gpui::font("Inter");
    font.weight = FontWeight::SEMIBOLD;
    let run = TextRun {
        len: text.len(),
        font,
        color: theme.on_accent,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let wrap = px(dock_available_width(state, column_width, theme));
    text_system
        .shape_text(text.into(), px(theme.scale.body), &[run], Some(wrap), None)
        .map(|lines| lines.len().max(1))
        .unwrap_or(1)
}

/// The dock's reserved height for one label at a column width (REQ-UI-15): the
/// measured wrapped line count at the current text metrics, in the recipe the
/// dock paints. The shell feeds this into its measured obstruction reservation,
/// so the dock owns real height instead of a blind 100 px inset.
pub fn dock_height(
    state: &DockState,
    column_width: f32,
    theme: &AppTheme,
    text_system: &WindowTextSystem,
) -> f32 {
    dock_height_for_lines(
        dock_label_lines(state, column_width, theme, text_system),
        theme,
    )
}

/// Where the dock's bottom edge sits above the window bottom: the platform
/// navigation's own height plus [`DOCK_GAP`]. The shell passes this offset and
/// paints the dock inside the measured band, so the last card is never covered.
pub fn dock_bottom(navigation_top: f32) -> f32 {
    navigation_top + DOCK_GAP
}

/// The compact pending dock: one full-width 48 px primary button carrying
/// exactly `model.fab_label()`, separated 8 px above platform navigation by
/// the shell. During an operation the same band shows the existing phase with
/// its quiet spinner and installs no repeat action.
/// @req REQ-SCR-01 @req REQ-UI-15 @req REQ-UI-16
pub fn mobile_dock(
    state: DockState,
    height: f32,
    on_click: Option<ClickHandler>,
    theme: &AppTheme,
) -> Stateful<Div> {
    let pressable = on_click.is_some() && state.is_action();
    let mut dock = div()
        .id("menus-dock")
        .w_full()
        .h(px(height))
        .px(px(theme.scale.space_4))
        .rounded(px(theme.scale.radius_control))
        .bg(theme.accent)
        .flex()
        .flex_wrap()
        .items_center()
        .justify_center()
        .gap(px(theme.scale.space_2));
    if !state.is_action() {
        dock = dock.child(spinner(theme.on_accent, theme.scale.space_4));
    }
    dock = dock.child(
        div()
            .min_w(px(0.))
            .text_size(px(theme.scale.body))
            .line_height(px(theme.scale.body_line))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.on_accent)
            .text_center()
            .child(state.text().to_string()),
    );
    if pressable {
        dock = dock
            .tab_index(0)
            .cursor_pointer()
            .hover(move |s| s.bg(theme.pressed(theme.accent)))
            .focus(crate::theme::desktop_focus(theme.on_accent));
    }
    if let (Some(on_click), true) = (on_click, state.is_action()) {
        dock = dock.on_click(move |_, window, cx| on_click(window, cx));
    }
    dock
}

#[cfg(test)]
mod tests {
    use super::{
        dock_available_width, dock_bottom, dock_height_for_lines, dock_state, DockState, DOCK_GAP,
    };
    use crate::theme::AppTheme;

    /// The dock presents exactly the model's pending label or the existing
    /// phase, and never both (REQ-SCR-01/REQ-UI-15): an operation removes the
    /// repeat action while keeping the phase in the same band.
    /// @req REQ-SCR-01 @req REQ-UI-16
    #[test]
    fn dock_state_matches_pending_and_progress_conditions() {
        assert_eq!(dock_state(0, None, "Bestellen (0)"), None);
        let action = dock_state(2, None, "Änderungen bestätigen (2)").unwrap();
        assert_eq!(action.text(), "Änderungen bestätigen (2)");
        assert_eq!(action.action(), Some("Änderungen bestätigen (2)"));

        let progress =
            dock_state(2, Some("Bestellung wird bestätigt..."), "Bestellen (2)").unwrap();
        assert!(!progress.is_action(), "no repeat action during progress");
        assert_eq!(progress.action(), None);
        assert_eq!(progress.text(), "Bestellung wird bestätigt...");
        // A phase without a pending set still keeps the band in place.
        let tail = dock_state(0, Some("Wird in den Warenkorb gelegt..."), "").unwrap();
        assert_eq!(tail.action(), None);
    }

    /// The dock is one full-width primary button (REQ-SCR-01/REQ-UI-15): its
    /// height is the measured wrapped line count above the 48 px floor, so the
    /// reservation and the painted band cannot disagree, and the width the
    /// label is shaped with is the width the button leaves for text (16 px
    /// padding, and the 16 px spinner plus its gap while an operation runs).
    /// @req REQ-SCR-01 @req REQ-UI-15
    #[test]
    fn dock_is_one_measured_primary_button() {
        let theme = AppTheme::build(false);
        assert_eq!(theme.scale.space_4, 16., "the button's side padding");
        assert_eq!(theme.scale.radius_control, 10., "the primary action radius");
        assert_eq!(dock_height_for_lines(1, &theme), 48.);
        for lines in 2..=4 {
            assert_eq!(
                dock_height_for_lines(lines, &theme),
                lines as f32 * theme.scale.body_line,
                "{lines} measured lines reserve their own height"
            );
        }

        let action = DockState::Action("Bestellen (1)".into());
        let progress = DockState::Progress("Bestellung wird bestätigt...".into());
        for column in [328., 358., 560.] {
            assert_eq!(
                dock_available_width(&action, column, &theme),
                column - theme.scale.space_4 * 2.
            );
            assert_eq!(
                dock_available_width(&progress, column, &theme),
                column - theme.scale.space_4 * 2. - theme.scale.space_4 - theme.scale.space_2,
                "the phase row loses the spinner and its gap"
            );
        }

        // The progress state replaces the action in the same band: no repeat
        // target and no second submit.
        assert!(!progress.is_action());
        assert_eq!(progress.action(), None);
    }

    /// @req REQ-UI-15
    #[test]
    fn dock_sits_eight_above_navigation() {
        assert_eq!(DOCK_GAP, 8.);
        assert_eq!(dock_bottom(90.), 98.);
        assert_eq!(dock_bottom(98.), 106.);
    }
}
