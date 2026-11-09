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

fn option(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    selected: bool,
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
        .rounded(px(16.))
        .bg(fill)
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
        .cursor_pointer()
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
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let back = {
        let entity = cx.entity().clone();
        Rc::new(move |_: &mut Window, cx: &mut App| {
            let _ = entity.update(cx, |this, cx| this.pop_detail(cx));
        })
    };

    let mut design = div().flex().gap(px(10.));
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
        design = design.child(option(
            match value {
                ThemePreference::System => "appearance-system",
                ThemePreference::Light => "appearance-light",
                ThemePreference::Dark => "appearance-dark",
            },
            icon,
            label,
            state.theme == value,
            handler,
            theme,
        ));
    }

    let mut swatches = div().flex().gap(px(16.));
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
        swatches = swatches.child(swatch(
            accent,
            label,
            state.accent == accent,
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

    div()
        .id("appearance")
        .size_full()
        .flex()
        .flex_col()
        .p(px(20.))
        .pb(px(100.))
        .gap(px(16.))
        .overflow_y_scroll()
        .child(crate::navigation::back_affordance(
            "Einstellungen",
            theme,
            back,
        ))
        .child(
            div()
                .text_size(px(28.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.text_primary)
                .child("Darstellung"),
        )
        .child(card("Design", design))
        .child(card("Akzentfarbe", swatches))
        .into_any_element()
}
