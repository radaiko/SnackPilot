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

/// Inter's average mixed-case advance at the body role, in em. It is chosen
/// above the bundled face's ~0.5 em so a label that might not fit one line is
/// reserved two lines instead of being covered by the dock. Every label the
/// model can produce fits one line at the accepted column widths; the
/// projection only grows as a protective bound.
const DOCK_ADVANCE_EM: f32 = 0.6;

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

/// The dock's reserved height for one label at a column width (REQ-UI-15):
/// the recipe's 12 px top/bottom plus one 24 px body line per wrapped line,
/// never below the 48 px action floor. The shell feeds this into its measured
/// obstruction reservation, so the dock owns real height instead of a blind
/// 100 px inset.
pub fn dock_height(state: &DockState, column_width: f32, theme: &AppTheme) -> f32 {
    // The progress row leads with the 16 px spinner and its 8 px gap.
    let lead = if state.is_action() {
        0.
    } else {
        theme.mobile.space_4 + theme.mobile.space_2
    };
    let available = (column_width - 2. * theme.mobile.space_4 - lead).max(1.);
    let advance = theme.mobile.body * DOCK_ADVANCE_EM;
    let chars = state.text().chars().count().max(1) as f32;
    let lines = (chars * advance / available).ceil().max(1.);
    (theme.mobile.space_3 * 2. + lines * theme.mobile.body_line).max(theme.mobile.action_height)
}

/// Where the dock's bottom edge sits above the window bottom: the platform
/// navigation's own height plus [`DOCK_GAP`]. The shell passes this offset and
/// paints the dock inside the measured band, so the last card is never covered.
pub fn dock_bottom(navigation_top: f32) -> f32 {
    navigation_top + DOCK_GAP
}

/// The compact dock surface: the full Day Tray column, min 48 high, r14 iOS /
/// r12 Android, 16/12 padding, centred wrapping body text. An action is a real
/// target with the resolved action fill/ink; a progress row is inert (no
/// callback) and reads its phase on the same fill.
pub fn mobile_dock(
    state: DockState,
    on_click: Option<ClickHandler>,
    theme: &AppTheme,
) -> Stateful<Div> {
    let radius = if theme.flat { 12. } else { 14. };
    let mut dock = div()
        .id("menus-dock")
        .w_full()
        .min_h(px(theme.mobile.action_height))
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
    use super::{dock_bottom, dock_height, dock_state, DockState, DOCK_GAP};
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

    /// The dock's height is derived from its own recipe and the column width,
    /// never a blind constant: 12/12 padding around one 24 px body line, at
    /// least the 48 px action floor, one more line when the label cannot fit.
    /// @req REQ-SCR-01 @req REQ-UI-15
    #[test]
    fn dock_height_reserves_wrapped_recipe_height() {
        for flat in [false, true] {
            let theme = AppTheme::build(false, AccentColor::Orange, flat);
            for width in [328., 358., 560.] {
                for label in [
                    "Bestellen (1)",
                    "Stornieren (2)",
                    "Änderungen bestätigen (3)",
                ] {
                    let state = DockState::Action(label.into());
                    let height = dock_height(&state, width, &theme);
                    assert_eq!(
                        height, theme.mobile.action_height,
                        "{label} fits one 24 px line at {width}"
                    );
                }
            }
            // A label beyond the one-line advance reserves its second line
            // instead of letting the dock cover the last card.
            let long = DockState::Action("Änderungen bestätigen (123456789012345678901234)".into());
            let two = dock_height(&long, 328., &theme);
            assert_eq!(two, theme.mobile.action_height + theme.mobile.body_line);
            // The progress row loses the spinner's width, and is still floored.
            let progress = DockState::Progress("Bestellung wird bestätigt...".into());
            assert!(dock_height(&progress, 328., &theme) >= theme.mobile.action_height);
        }
        // The separation above navigation is the design's 8 px.
        assert_eq!(DOCK_GAP, 8.);
        assert_eq!(dock_bottom(90.), 98.);
        assert_eq!(dock_bottom(98.), 106.);
    }
}
