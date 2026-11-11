//! Appearance screen — docs/requirements/04-ui-ux.md §3.7.
//!
//! Two cards: Design (System/Hell/Dunkel) and Akzentfarbe (five light-mode
//! swatches with a checkmark ring). Both write through `AppModel`; the accent
//! setter is the only app-icon call site (themes §6).

use std::rc::Rc;
use std::sync::Arc;

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};

use crate::app::AppShell;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_model::tokens;
use snackpilot_model::{AccentColor, AppModel, ThemePreference};

/// Left/Up selects the previous option, Right/Down the next; Home/End the
/// first/last. No wrap, and only the focused group handles the key
/// (REQ-UI-11). Returns `None` when the key is not an option traversal key.
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

/// Accent equivalent of [`step_preference`].
pub fn step_accent(current: AccentColor, key: &str) -> Option<AccentColor> {
    const ALL: [AccentColor; 5] = [
        AccentColor::Orange,
        AccentColor::Emerald,
        AccentColor::Berry,
        AccentColor::Golden,
        AccentColor::Ocean,
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

fn option(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    selected: bool,
    index: isize,
    on_click: ClickHandler,
    theme: &AppTheme,
) -> AnyElement {
    let fill = if selected {
        if theme.flat {
            theme.primary_surface
        } else {
            theme.glass_primary
        }
    } else if theme.flat {
        theme.surface
    } else {
        theme.glass_surface
    };
    let fg = if selected {
        theme.primary
    } else {
        theme.text_secondary
    };
    div()
        .id(SharedString::from(id))
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.))
        .py(px(16.))
        .min_h(px(snackpilot_model::desktop::TARGET_MIN))
        .rounded(px(16.))
        .bg(fill)
        .tab_index(index)
        .focus(theme::desktop_focus(theme.desktop.focus))
        .border_b_3()
        .border_color(if selected {
            theme.primary
        } else {
            hsla(0., 0., 0., 0.)
        })
        .cursor_pointer()
        .on_click(move |_, window, cx| on_click(window, cx))
        .child(Icon::new(icon).with_size(px(22.)).text_color(fg))
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(fg)
                .child(label),
        )
        .into_any_element()
}

fn swatch(
    accent: AccentColor,
    label: &'static str,
    selected: bool,
    index: isize,
    on_click: ClickHandler,
    theme: &AppTheme,
) -> AnyElement {
    let light = tokens::accent(accent).light;
    let color = theme::color(light.primary);
    let mut circle = div()
        .w(px(40.))
        .h(px(40.))
        .rounded(px(20.))
        .bg(color)
        .border_3()
        .border_color(if selected {
            color
        } else {
            hsla(0., 0., 0., 0.)
        })
        .flex()
        .items_center()
        .justify_center();
    if selected {
        circle = circle.child(
            div()
                .text_size(px(20.))
                .text_color(hsla(0., 0., 1., 1.))
                .child("✓"),
        );
    }
    div()
        .id(SharedString::from(format!("accent-{label}")))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.))
        .min_h(px(snackpilot_model::desktop::TARGET_MIN))
        .min_w(px(56.))
        .cursor_pointer()
        .tab_index(index)
        .focus(theme::desktop_focus(theme.desktop.focus))
        .on_click(move |_, window, cx| on_click(window, cx))
        .child(circle)
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if selected {
                    theme.primary
                } else {
                    theme.text_tertiary
                })
                .child(label),
        )
        .into_any_element()
}

/// The pushed Darstellung screen.
pub fn appearance_screen(
    state: &snackpilot_model::State,
    model: Option<&Arc<AppModel>>,
    theme: &AppTheme,
    containment: Option<f32>,
    design_focus: &FocusHandle,
    accent_focus: &FocusHandle,
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
        .flex()
        .flex_wrap()
        .gap(px(10.))
        .tab_index(0)
        .track_focus(design_focus)
        .focus(theme::desktop_focus(theme.desktop.focus));
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
    for (value, icon, label) in [
        (ThemePreference::System, IconName::Smartphone, "System"),
        (ThemePreference::Light, IconName::Sun, "Hell"),
        (ThemePreference::Dark, IconName::Moon, "Dunkel"),
    ] {
        let shell = cx.entity().clone();
        let model = model.cloned();
        let handler = Rc::new(move |_: &mut Window, cx: &mut App| {
            if let Some(model) = &model {
                model.set_theme_preference(value);
            }
            let _ = shell.update(cx, |_, cx| cx.notify());
        });
        let index = match value {
            ThemePreference::System => 0,
            ThemePreference::Light => 1,
            ThemePreference::Dark => 2,
        };
        design = design.child(option(
            match value {
                ThemePreference::System => "appearance-system",
                ThemePreference::Light => "appearance-light",
                ThemePreference::Dark => "appearance-dark",
            },
            icon,
            label,
            state.theme == value,
            index,
            handler,
            theme,
        ));
    }

    let mut swatches = div()
        .flex()
        .flex_wrap()
        .gap(px(16.))
        .tab_index(0)
        .track_focus(accent_focus)
        .focus(theme::desktop_focus(theme.desktop.focus));
    if let Some(model) = model.cloned() {
        let shell = cx.entity().clone();
        swatches = swatches.on_key_down(move |event: &KeyDownEvent, _window, cx| {
            if let Some(next) = step_accent(model.state().accent, event.keystroke.key.as_str()) {
                model.set_accent_color(next);
                let _ = shell.update(cx, |_, cx| cx.notify());
                cx.stop_propagation();
            }
        });
    }
    for (accent, label) in [
        (AccentColor::Orange, "Orange"),
        (AccentColor::Emerald, "Smaragd"),
        (AccentColor::Berry, "Beere"),
        (AccentColor::Golden, "Gold"),
        (AccentColor::Ocean, "Ozean"),
    ] {
        let shell = cx.entity().clone();
        let model = model.cloned();
        let handler = Rc::new(move |_: &mut Window, cx: &mut App| {
            if let Some(model) = &model {
                model.set_accent_color(accent);
            }
            let _ = shell.update(cx, |_, cx| cx.notify());
        });
        let index = match accent {
            AccentColor::Orange => 0,
            AccentColor::Emerald => 1,
            AccentColor::Berry => 2,
            AccentColor::Golden => 3,
            AccentColor::Ocean => 4,
        };
        swatches = swatches.child(swatch(
            accent,
            label,
            state.accent == accent,
            index,
            handler,
            theme,
        ));
    }

    let card = |title: &'static str, body: Div| {
        theme::apply(
            div().flex().flex_col().p(px(20.)).mb(px(16.)),
            &theme.card_surface(),
        )
        .child(
            div()
                .mb(px(14.))
                .text_size(px(18.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(title),
        )
        .child(body)
    };

    let section = |title: &'static str, body: Div| {
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text_secondary)
                    .child(title),
            )
            .child(body)
    };

    let mut page = div()
        .id("appearance")
        .size_full()
        .flex()
        .flex_col()
        .overflow_y_scroll();
    page = if desktop {
        page.p(px(containment.unwrap_or(16.)))
            .gap(px(24.))
            .w_full()
            .max_w(px(440.))
    } else {
        page.p(px(20.)).pb(px(100.)).gap(px(16.))
    };
    if !desktop {
        page = page.child(crate::navigation::back_affordance(
            "Einstellungen",
            theme,
            back,
        ));
    }
    page.child(
        div()
            .text_size(px(if desktop { 20. } else { 28. }))
            .line_height(px(if desktop { 26. } else { 34. }))
            .font_weight(if desktop {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::BOLD
            })
            .text_color(if desktop {
                theme.desktop.ink
            } else {
                theme.text_primary
            })
            .child("Darstellung"),
    )
    .child(if desktop {
        section("Design", design)
    } else {
        card("Design", design)
    })
    .child(if desktop {
        section("Akzentfarbe", swatches)
    } else {
        card("Akzentfarbe", swatches)
    })
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{step_accent, step_preference};
    use snackpilot_model::{AccentColor, ThemePreference};

    /// Appearance groups move logically without wrapping: Left/Up previous,
    /// Right/Down next, Home/End the ends, and unrelated keys keep native
    /// editing (REQ-UI-11).
    /// @req REQ-UI-11 @req REQ-SCR-04
    #[test]
    fn option_groups_step_without_wrapping() {
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

        assert_eq!(
            step_accent(AccentColor::Emerald, "right"),
            Some(AccentColor::Berry)
        );
        assert_eq!(step_accent(AccentColor::Orange, "left"), None);
        assert_eq!(
            step_accent(AccentColor::Ocean, "end"),
            Some(AccentColor::Ocean)
        );
        assert_eq!(step_accent(AccentColor::Berry, "enter"), None);
    }
}
