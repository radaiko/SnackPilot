//! The one "this is running" surface: an action that has gone grey because it
//! is already doing its work.
//!
//! A disabled button says "you cannot press this again"; it does not say "the
//! order is on its way". This one keeps the phase text inside the same button
//! and lets a wide accent band sweep the full height beneath it, so the tap
//! visibly turned into work instead of into a dead control.

use std::time::Duration;

use gpui::*;

use crate::theme::AppTheme;

/// One sweep of the band.
const SWEEP: Duration = Duration::from_millis(1300);
/// Share of the width the sweeping band occupies.
const SEGMENT: f32 = 0.42;

/// A band the full height of its box, sweeping left to right forever. It starts
/// and ends fully outside the button, so repeating the pass never cuts back to
/// the left edge — the band arrives and leaves instead of jumping.
fn sweep_band(colour: Hsla, id: &'static str) -> AnyElement {
    div()
        .absolute()
        .top(px(0.))
        .bottom(px(0.))
        .left(px(0.))
        .w(relative(SEGMENT))
        .bg(colour)
        .with_animation(id, Animation::new(SWEEP).repeat(), |el, t| {
            el.left(relative(-SEGMENT + t * (1. + SEGMENT)))
                .opacity((t / 0.1).min((1. - t) / 0.1).clamp(0., 1.))
        })
        .into_any_element()
}

/// A greyed action carrying its own phase: neutral fill, hairline border,
/// secondary ink and the sweeping band behind the text. `height` is the box the
/// caller reserved, so a wrapped phase grows it instead of being clipped.
pub fn working_button(label: impl Into<SharedString>, height: f32, theme: &AppTheme) -> Div {
    div()
        .relative()
        .w_full()
        .h(px(height))
        .px(px(theme.scale.space_4))
        .rounded(px(theme.scale.radius_control))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface_alt)
        .overflow_hidden()
        .flex()
        .flex_wrap()
        .items_center()
        .justify_center()
        .child(sweep_band(theme.accent_surface, "working-band"))
        .child(
            div()
                .min_w(px(0.))
                .text_size(px(theme.scale.body))
                .line_height(px(theme.scale.body_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .text_center()
                .child(label.into()),
        )
}
