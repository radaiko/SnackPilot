//! Dialog system — docs/requirements/04-ui-ux.md §4.1.
//!
//! Custom themed modal (v1 `DialogProvider`), not the platform alert: one
//! dialog visible at a time, rendered by the shell as an absolutely
//! positioned overlay. Backdrop tap dismisses and counts as "cancelled".

use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::theme::AppTheme;

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
        on_confirm: Rc<dyn Fn(&mut Window, &mut App)>,
    },
}

/// A §4.1 dialog button: `py 10`, `radius 8`, fontSize 15 weight 600.
fn dialog_button(
    id: &'static str,
    label: &'static str,
    fill: Option<Hsla>,
    border: Option<Hsla>,
    fg: Hsla,
    grow: bool,
    on_click: Rc<dyn Fn(&mut Window, &mut App)>,
) -> Stateful<Div> {
    let div = div()
        .id(SharedString::from(id))
        .flex()
        .items_center()
        .justify_center()
        .py(px(10.))
        .rounded(px(8.))
        .text_size(px(15.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(fg)
        .cursor_pointer()
        .when_some(fill, |d, c| d.bg(c))
        .when_some(border, |d, c| d.border_1().border_color(c))
        .when(grow, |d| d.flex_1())
        .child(label);
    div.on_click(move |_, window, cx| on_click(window, cx))
}

/// The §4.1 modal: `overlay` backdrop with padding 40, content box centered
/// (surface, radius 14, padding 24, maxWidth 340, 0.5 `border`).
/// Tapping the backdrop calls `on_dismiss`; tapping the box does nothing.
pub fn dialog_overlay(
    dialog: &Dialog,
    theme: &AppTheme,
    on_dismiss: Rc<dyn Fn(&mut Window, &mut App)>,
) -> AnyElement {
    let (title, message, buttons): (SharedString, Option<SharedString>, Vec<AnyElement>) =
        match dialog {
            Dialog::Alert { title, message } => {
                let dismiss = on_dismiss.clone();
                (
                    title.clone(),
                    message.clone(),
                    vec![dialog_button(
                        "dialog-ok",
                        "OK",
                        Some(theme.primary),
                        None,
                        theme.on_primary,
                        false,
                        Rc::new(move |w, cx| dismiss(w, cx)),
                    )
                    .px(px(32.))
                    .into_any_element()],
                )
            }
            Dialog::Confirm {
                title,
                message,
                on_confirm,
            } => {
                let dismiss = on_dismiss.clone();
                let cancel = dismiss.clone();
                let confirm = on_confirm.clone();
                let confirm_dismiss = dismiss.clone();
                (
                    title.clone(),
                    Some(message.clone()),
                    vec![
                        dialog_button(
                            "dialog-cancel",
                            "Abbrechen",
                            None,
                            Some(theme.border),
                            theme.text_secondary,
                            true,
                            Rc::new(move |w, cx| cancel(w, cx)),
                        )
                        .into_any_element(),
                        dialog_button(
                            "dialog-confirm",
                            "Bestätigen",
                            Some(theme.error),
                            None,
                            theme.on_primary,
                            true,
                            Rc::new(move |w, cx| {
                                confirm(w, cx);
                                confirm_dismiss(w, cx);
                            }),
                        )
                        .into_any_element(),
                    ],
                )
            }
        };

    let backdrop = on_dismiss;
    div()
        .id("dialog-backdrop")
        .absolute()
        .inset_0()
        .bg(theme.overlay)
        .p(px(40.))
        .flex()
        .items_center()
        .justify_center()
        .on_click(move |_, window, cx| backdrop(window, cx))
        .child(
            div()
                .id("dialog-box")
                .w_full()
                .max_w(px(340.))
                .flex()
                .flex_col()
                .items_center()
                .rounded(px(14.))
                .p(px(24.))
                .bg(theme.surface)
                .border(px(0.5))
                .border_color(theme.border)
                // Tapping the dialog body must not dismiss (§4.1).
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .text_size(px(17.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .text_center()
                        .child(title),
                )
                .when_some(message, |d, m| {
                    d.child(
                        div()
                            .mt(px(8.))
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(theme.text_secondary)
                            .text_center()
                            .child(m),
                    )
                })
                .child(
                    div()
                        .mt(px(20.))
                        .w_full()
                        .flex()
                        .gap(px(10.))
                        .items_center()
                        .justify_center()
                        .children(buttons),
                ),
        )
        .into_any_element()
}
