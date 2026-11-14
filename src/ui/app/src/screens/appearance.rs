//! Appearance screen: the System / Hell / Dunkel choice.

use std::rc::Rc;
use std::sync::Arc;

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};

use crate::app::AppShell;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_model::{AppModel, ThemePreference};

/// The three appearance choices, in the persisted order.
pub const APPEARANCE_OPTIONS: [(ThemePreference, &str); 3] = [
    (ThemePreference::System, "System"),
    (ThemePreference::Light, "Hell"),
    (ThemePreference::Dark, "Dunkel"),
];

/// Left/Up selects the previous option, Right/Down the next; Home/End the
/// first/last. No wrap, and only the focused group handles the key.
pub fn step_preference(current: ThemePreference, key: &str) -> Option<ThemePreference> {
    const ALL: [ThemePreference; 3] = [
        ThemePreference::System,
        ThemePreference::Light,
        ThemePreference::Dark,
    ];
    let index = ALL.iter().position(|value| *value == current)?;
    let next = match key {
        "left" | "up" => index.checked_sub(1)?,
        "right" | "down" => (index + 1 < ALL.len()).then_some(index + 1)?,
        "home" => 0,
        "end" => ALL.len() - 1,
        _ => return None,
    };
    Some(ALL[next])
}

/// Selected rows tint like every other selected surface; unselected rows stay
/// on the plain card fill.
fn row_fill(selected: bool, theme: &AppTheme) -> Hsla {
    if selected {
        theme.accent_surface
    } else {
        theme.surface
    }
}

/// One selectable appearance row: surface, hairline border, 12 radius, 16
/// padding, body label and a trailing accent check on the selected choice. The
/// whole row is the 56 px target.
/// @req REQ-SCR-07 @req REQ-UI-15
fn appearance_row(
    id: &'static str,
    label: &'static str,
    selected: bool,
    on_click: ClickHandler,
    theme: &AppTheme,
) -> AnyElement {
    let mut row = div()
        .id(SharedString::from(id))
        .w_full()
        .min_h(px(56.))
        .flex()
        .items_center()
        .p(px(theme.scale.space_4))
        .rounded(px(theme.scale.radius_card))
        .bg(row_fill(selected, theme))
        .border(px(1.))
        .border_color(if selected { theme.accent } else { theme.border })
        .cursor_pointer()
        .focus(theme::desktop_focus(theme.accent))
        .on_click(move |_, window, cx| on_click(window, cx))
        .child(
            div()
                .flex_1()
                .text_size(px(theme.scale.body))
                .line_height(px(theme.scale.body_line))
                .text_color(theme.text_primary)
                .child(label),
        );
    if !selected {
        row = row.hover(|s| s.bg(theme.pressed(theme.surface)));
    }
    if selected {
        row = row.child(
            Icon::new(IconName::Check)
                .with_size(px(theme.scale.icon))
                .text_color(theme.accent),
        );
    }
    row.into_any_element()
}

/// The pushed Darstellung screen.
pub fn appearance_screen(
    state: &snackpilot_model::State,
    model: Option<&Arc<AppModel>>,
    theme: &AppTheme,
    containment: Option<f32>,
    design_focus: &FocusHandle,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let desktop = containment.is_some();
    let back = {
        let entity = cx.entity().clone();
        Rc::new(move |_: &mut Window, cx: &mut App| {
            let _ = entity.update(cx, |this, cx| this.pop_detail(cx));
        })
    };

    let mut design = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2))
        .tab_index(0)
        .track_focus(design_focus)
        .focus(theme::desktop_focus(theme.accent));
    if let Some(model) = model.cloned() {
        let shell = cx.entity().clone();
        design = design.on_key_down(move |event: &KeyDownEvent, _window, cx| {
            if let Some(next) = step_preference(model.state().theme, event.keystroke.key.as_str()) {
                model.set_theme_preference(next);
                let _ = shell.update(cx, |_, cx| cx.notify());
                cx.stop_propagation();
            }
        });
    }
    for (value, label) in APPEARANCE_OPTIONS {
        let shell = cx.entity().clone();
        let model = model.cloned();
        let handler = Rc::new(move |_: &mut Window, cx: &mut App| {
            if let Some(model) = &model {
                model.set_theme_preference(value);
            }
            let _ = shell.update(cx, |_, cx| cx.notify());
        });
        design = design.child(appearance_row(
            match value {
                ThemePreference::System => "appearance-system",
                ThemePreference::Light => "appearance-light",
                ThemePreference::Dark => "appearance-dark",
            },
            label,
            state.theme == value,
            handler,
            theme,
        ));
    }

    let mut page = div()
        .id("appearance")
        .size_full()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_4))
        .overflow_y_scroll();
    if let Some(pad) = containment {
        page = page.p(px(pad)).w_full().max_w(px(440.));
    }
    if !desktop {
        page = page.child(crate::navigation::back_affordance(
            "Einstellungen",
            theme,
            back,
        ));
    }
    page.child(
        div()
            .text_size(px(theme.scale.title))
            .line_height(px(theme.scale.title_line))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.text_primary)
            .child("Darstellung"),
    )
    .child(
        div()
            .text_size(px(theme.scale.meta))
            .line_height(px(theme.scale.meta_line))
            .text_color(theme.text_secondary)
            .child("Die App folgt dem System oder bleibt immer hell beziehungsweise dunkel."),
    )
    .child(design)
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{row_fill, step_preference, APPEARANCE_OPTIONS};
    use crate::theme::AppTheme;
    use snackpilot_model::ThemePreference;

    /// The selected row tints like every other selected surface instead of
    /// staying on the plain card fill (REQ-SCR-07, REQ-UI-15).
    /// @req REQ-SCR-07 @req REQ-UI-15
    #[test]
    fn selected_row_tints_like_every_other_selected_surface() {
        let theme = AppTheme::build(false);
        assert_ne!(row_fill(true, &theme), row_fill(false, &theme));
        assert_eq!(row_fill(true, &theme), theme.accent_surface);
        assert_eq!(row_fill(false, &theme), theme.surface);
    }

    /// The screen offers exactly the three appearance choices, in order, and
    /// nothing else (REQ-SCR-07): System / Hell / Dunkel.
    /// @req REQ-SCR-07
    #[test]
    fn appearance_offers_only_the_three_theme_choices() {
        assert_eq!(
            APPEARANCE_OPTIONS,
            [
                (ThemePreference::System, "System"),
                (ThemePreference::Light, "Hell"),
                (ThemePreference::Dark, "Dunkel"),
            ]
        );
    }

    /// Appearance moves logically without wrapping: Left/Up previous,
    /// Right/Down next, Home/End the ends, and unrelated keys keep native
    /// editing.
    #[test]
    fn option_group_steps_without_wrapping() {
        assert_eq!(
            step_preference(ThemePreference::Light, "left"),
            Some(ThemePreference::System)
        );
        assert_eq!(
            step_preference(ThemePreference::Light, "right"),
            Some(ThemePreference::Dark)
        );
        assert_eq!(step_preference(ThemePreference::System, "left"), None);
        assert_eq!(step_preference(ThemePreference::Dark, "right"), None);
        assert_eq!(
            step_preference(ThemePreference::Dark, "home"),
            Some(ThemePreference::System)
        );
        assert_eq!(
            step_preference(ThemePreference::System, "end"),
            Some(ThemePreference::Dark)
        );
        assert_eq!(step_preference(ThemePreference::Light, "a"), None);
    }
}
