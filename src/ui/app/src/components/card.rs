//! Card container (04 §4), styled by the §7 `cardSurface` recipe:
//! iOS glass surface + hairline border + soft shadow, Android flat surface
//! + 1px border.

use gpui_kit::component::StyledExt;
use gpui_kit::prelude::*;
use gpui_kit::{div, px, Div, SharedString};

use crate::theme::{self, AppTheme};

/// A §4 card: rounded surface holding arbitrary child elements.
pub fn card(theme: &AppTheme, children: impl IntoIterator<Item = gpui_kit::AnyElement>) -> Div {
    let r = theme.card_surface();
    theme::apply(div().flex().flex_col().gap(px(10.)).p_3(), &r).children(children)
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
