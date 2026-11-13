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
use crate::theme::{self, AppTheme};

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

/// The width the dock's label can wrap in: the column minus the recipe's
/// horizontal 16 px padding, and the 16 px spinner plus its 8 px gap when the
/// row is an operation phase instead of an action.
pub fn dock_available_width(state: &DockState, column_width: f32, theme: &AppTheme) -> f32 {
    let lead = if state.is_action() {
        0.
    } else {
        theme.mobile.space_4 + theme.mobile.space_2
    };
    (column_width - 2. * theme.mobile.space_4 - lead).max(1.)
}

/// The dock's height for a measured wrapped line count (REQ-UI-15): the
/// recipe's 12 px top/bottom plus one 24 px body line per line, never below the
/// 48 px action floor. The painted dock is given exactly this height, so the
/// reservation and the dock cannot disagree.
pub fn dock_height_for_lines(lines: usize, theme: &AppTheme) -> f32 {
    (theme.mobile.space_3 * 2. + lines.max(1) as f32 * theme.mobile.body_line)
        .max(theme.mobile.action_height)
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
        color: theme.mobile.action_ink,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let wrap = px(dock_available_width(state, column_width, theme));
    text_system
        .shape_text(text.into(), px(theme.mobile.body), &[run], Some(wrap), None)
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

/// The compact dock surface: the full Day Tray column, exactly `height` (the
/// measured value the shell reserved), r14 iOS / r12 Android, 16/12 padding,
/// centred wrapping body text. An action is a real target with the resolved
/// action fill/ink; a progress row is inert (no callback) and reads its phase
/// on the same fill.
pub fn mobile_dock(
    state: DockState,
    height: f32,
    on_click: Option<ClickHandler>,
    theme: &AppTheme,
) -> Stateful<Div> {
    let radius = if theme.flat { 12. } else { 14. };
    let mut dock = div()
        .id("menus-dock")
        .w_full()
        .h(px(height))
        .px(px(theme.mobile.space_4))
        .py(px(theme.mobile.space_3))
        .rounded(px(radius))
        .bg(theme.mobile.action_fill)
        .flex()
        .flex_wrap()
        .items_center()
        .justify_center()
        .gap(px(theme.mobile.space_2));
    if !state.is_action() {
        dock = dock.child(spinner(theme.mobile.action_ink, theme.mobile.space_4));
    }
    dock = dock.child(
        div()
            .min_w(px(0.))
            .text_size(px(theme.mobile.body))
            .line_height(px(theme.mobile.body_line))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.mobile.action_ink)
            .text_center()
            .child(state.text().to_string()),
    );
    if let (Some(on_click), true) = (on_click, state.is_action()) {
        dock = dock
            .tab_index(0)
            .focus(theme::desktop_focus(theme.mobile.focus))
            .cursor_pointer()
            .on_click(move |_, window, cx| on_click(window, cx));
    }
    dock
}

#[cfg(test)]
mod tests {
    use super::{
        dock_available_width, dock_bottom, dock_height_for_lines, dock_state, DockState, DOCK_GAP,
    };
    use crate::theme::AppTheme;
    use snackpilot_model::AccentColor;

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

    /// The dock's height is the measured wrapped line count in the recipe it
    /// paints (REQ-SCR-01/REQ-UI-15), so the reservation and the painted band
    /// cannot disagree: 12/12 padding around one 24 px body line per measured
    /// line, floored at 48, and the wrap width is exactly the text width the
    /// dock leaves after its padding (and the progress row's spinner).
    /// @req REQ-SCR-01 @req REQ-UI-15
    #[test]
    fn dock_height_follows_the_measured_line_count_of_its_recipe() {
        for flat in [false, true] {
            let theme = AppTheme::build(false, AccentColor::Orange, flat);
            assert_eq!(
                theme.mobile.space_3 * 2.,
                24.,
                "the dock's vertical padding"
            );
            // One measured line is the 48 px action floor; every further line
            // grows the band by exactly one body line, which is what the dock
            // paints (fixed line height, same padding).
            assert_eq!(dock_height_for_lines(1, &theme), theme.mobile.action_height);
            for lines in 2..=4 {
                let height = dock_height_for_lines(lines, &theme);
                assert_eq!(
                    height,
                    theme.mobile.space_3 * 2. + lines as f32 * theme.mobile.body_line,
                    "{lines} measured lines reserve their own height"
                );
                assert!(height > dock_height_for_lines(lines - 1, &theme));
            }

            // The width the label is shaped with is the width the dock leaves
            // for text, not the column: 16 px padding on both sides, and the
            // 16 px spinner plus its 8 px gap while an operation runs.
            let action = DockState::Action("Bestellen (1)".into());
            let progress = DockState::Progress("Bestellung wird bestätigt...".into());
            for column in [328., 358., 560.] {
                assert_eq!(
                    dock_available_width(&action, column, &theme),
                    column - theme.mobile.space_4 * 2.
                );
                assert_eq!(
                    dock_available_width(&progress, column, &theme),
                    column
                        - theme.mobile.space_4 * 2.
                        - theme.mobile.space_4
                        - theme.mobile.space_2,
                    "the phase row loses the spinner and its gap"
                );
            }
        }
        // The separation above navigation is the design's 8 px.
        assert_eq!(DOCK_GAP, 8.);
        assert_eq!(dock_bottom(90.), 98.);
        assert_eq!(dock_bottom(98.), 106.);
    }
}
