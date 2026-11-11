//! Card container (04 §4), styled by the §7 `cardSurface` recipe:
//! iOS glass surface + hairline border + soft shadow, Android flat surface
//! + 1px border.

use gpui_kit::component::StyledExt;
use gpui_kit::prelude::*;
use gpui_kit::{div, px, Div, SharedString};
use snackpilot_model::desktop::ControlState;

use crate::theme::{self, AppTheme};

/// A §4 card: rounded surface holding arbitrary child elements.
pub fn card(theme: &AppTheme, children: impl IntoIterator<Item = gpui_kit::AnyElement>) -> Div {
    let r = theme.card_surface();
    theme::apply(div().flex().flex_col().gap(px(10.)).p_3(), &r).children(children)
}

/// Desktop ledger card (brief `desktop.panelSurface`): the opaque panel at
/// rest, `surface_variant` on hover and the selection tint when selected,
/// radius 12, 1 px border and no shadow. The reserved 4 px ring room keeps a
/// focused card from clipping. The mobile `card` above keeps its glass
/// dialect and shadow exactly.
/// @req REQ-UI-07 @req REQ-UI-13
pub fn desktop_card(
    theme: &AppTheme,
    state: ControlState,
    children: impl IntoIterator<Item = gpui_kit::AnyElement>,
) -> Div {
    let hover = theme.control_fill(ControlState::Hover);
    let card = div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .p_4()
        .rounded(px(12.))
        .bg(theme.control_fill(state))
        .border(px(1.))
        .border_color(if state == ControlState::Selected {
            theme.desktop.accent_boundary
        } else {
            theme.border
        })
        .hover(move |s| s.bg(hover))
        .children(children);
    theme::focus_frame(card, state, 12., theme)
}

/// Empty/loading state row (04 §4): muted German `text`, optional leading
/// spinner for the loading variants ("Aktualisiere...", "Wird in den
/// Warenkorb gelegt...").
pub fn empty_state(text: impl Into<SharedString>, spinner: bool, theme: &AppTheme) -> Div {
    let mut row = div().flex().items_center().gap(px(8.)).p_2();
    if spinner {
        row = row.child(crate::components::spinner::spinner(
            theme.text_secondary,
            14.,
        ));
    }
    row.child(
        div()
            .text_size(px(14.))
            .text_color(theme.text_secondary)
            .child(text.into()),
    )
}

/// A card with a bold title row above muted body text.
pub fn titled_card(
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
    theme: &AppTheme,
) -> Div {
    card(
        theme,
        [
            div()
                .text_size(px(16.))
                .font_semibold()
                .text_color(theme.text_primary)
                .child(title.into())
                .into_any_element(),
            div()
                .text_size(px(14.))
                .text_color(theme.text_secondary)
                .child(body.into())
                .into_any_element(),
        ],
    )
}

/// Hairline divider between card sections (04 §7: glassHighlight / border).
pub fn divider(theme: &AppTheme) -> Div {
    div().w_full().h(px(1.)).bg(theme.divider())
}
