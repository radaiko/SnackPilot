//! Menu card — docs/requirements/04-ui-ux.md §4.5.
//!
//! One card per menu item: state-driven surface, at most one badge, dish text,
//! allergens and price, plus the selected checkmark disc. Tappability is
//! decided by the caller from the model (`ordered || (available && !cutoff)`).

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::Styled;

use crate::components::button::ButtonKind;
use crate::components::record_table::RowFocus;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::MenuItem;
use snackpilot_model::menu_key;

/// §4.5 state overlays, later wins.
pub struct MenuCardState {
    pub selected: bool,
    pub pending_cancel: bool,
    pub ordered: bool,
    pub available: bool,
    pub cutoff: bool,
}

/// The dish a compact card presents: the required dish `subtitle`, falling
/// back to the title only when the subtitle is empty (sample M2). The category
/// stays the neutral group heading, so the card never repeats `MENÜ I`.
pub fn mobile_dish(item: &MenuItem) -> String {
    if item.subtitle.trim().is_empty() {
        item.title.clone()
    } else {
        item.subtitle.clone()
    }
}

/// The one badge label in the §4.5 precedence
/// cancellation → ordered → unavailable → cutoff. It is a status word, never
/// the pending check: no glyph, and a badge and the inline check can appear
/// together without being confused.
pub fn mobile_badge_label(state: &MenuCardState) -> Option<&'static str> {
    if state.pending_cancel {
        Some("Wird storniert")
    } else if state.ordered {
        Some("Bestellt")
    } else if !state.available {
        Some("Ausverkauft")
    } else if state.cutoff {
        Some("Geschlossen")
    } else {
        None
    }
}

/// How many lines the compact card's dish may occupy: the fourth is the last,
/// so a longer dish is clipped instead of pushing the price out of the card.
pub const MOBILE_DISH_LINES: usize = 4;

/// The height cap the dish row is painted with (four body lines).
pub fn mobile_dish_max_height(theme: &AppTheme) -> f32 {
    theme.mobile.body_line * MOBILE_DISH_LINES as f32
}

/// One slot of the compact card's name row, in paint order. The check slot is
/// inline and reserved (never floated over the text), so the dish starts after
/// it while pending and at the same edge otherwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MobileNameSlot {
    /// The reserved inline `Check` slot, in px.
    Check(f32),
    /// The dish itself.
    Dish,
}

/// The name row's slots for one state.
pub fn mobile_card_name_slots(selected: bool) -> Vec<MobileNameSlot> {
    if selected {
        vec![MobileNameSlot::Check(24.), MobileNameSlot::Dish]
    } else {
        vec![MobileNameSlot::Dish]
    }
}

/// One part of the compact card's footer, in paint order. Allergens come first
/// and wrap on the left; the unchanged price string follows, right-aligned and
/// never wrapped, so a long allergen list never eats its letters.
#[derive(Debug, Clone, PartialEq)]
pub enum MobileFooterPart {
    /// `Allergene: …` on the readable metadata ink.
    Allergens(String),
    /// The domain's own price string, right-aligned.
    Price(String),
}

/// The footer parts for one dish.
pub fn mobile_card_footer(item: &MenuItem) -> Vec<MobileFooterPart> {
    let mut parts = Vec::with_capacity(2);
    if !item.allergens.is_empty() {
        parts.push(MobileFooterPart::Allergens(format!(
            "Allergene: {}",
            item.allergens.join(", ")
        )));
    }
    parts.push(MobileFooterPart::Price(item.price.clone()));
    parts
}

/// The compact card's resting treatment, as data the builder paints: which
/// surface and boundary the card takes and whether its dish is struck through.
/// Pending cancellation is dashed and struck on readable ink — never a blanket
/// opacity, so the dish and its price stay readable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MobileCardTreatment {
    /// Not pending: the plain card surface, no boundary.
    Plain,
    /// Pending order: the accent selection tint and the resolved boundary.
    Selected,
    /// Pending cancellation: a dashed warning boundary and a struck dish.
    PendingCancel,
    /// Placed order: the success tint and its boundary.
    Ordered,
}

impl MobileCardTreatment {
    /// Whether the dish text is struck through.
    pub fn struck(self) -> bool {
        self == Self::PendingCancel
    }

    /// Whether the card paints a dashed boundary.
    pub fn dashed(self) -> bool {
        self == Self::PendingCancel
    }
}

/// The one treatment per state, in the design's precedence
/// pending-cancel → selected → ordered → plain.
pub fn mobile_card_treatment(state: &MenuCardState) -> MobileCardTreatment {
    if state.pending_cancel {
        MobileCardTreatment::PendingCancel
    } else if state.selected {
        MobileCardTreatment::Selected
    } else if state.ordered {
        MobileCardTreatment::Ordered
    } else {
        MobileCardTreatment::Plain
    }
}

/// Whether a compact card is one selection target, exactly as the incumbent
/// model gate: an ordered item stays tappable so its cancellation can be
/// queued, an unavailable or cutoff new item cannot be added. Appearance never
/// changes this, and neither does a pending selection: a dish that became
/// unavailable or closed after it was picked is not made tappable by its tint.
pub fn mobile_menu_tappable(state: &MenuCardState) -> bool {
    state.ordered || (state.available && !state.cutoff)
}

fn badge(state: &MenuCardState, theme: &AppTheme) -> Option<(&'static str, Hsla, Hsla, Hsla)> {
    // (label, background, border, text) in the §4.5 order.
    let label = mobile_badge_label(state)?;
    let (bg, border, text) = if state.pending_cancel {
        let bg = if theme.flat {
            theme.warning_surface
        } else {
            theme.glass_warning
        };
        (bg, theme.warning, theme.warning_text)
    } else if state.ordered {
        let bg = if theme.flat {
            theme.success_surface
        } else {
            theme.glass_success
        };
        (bg, theme.success, theme.success_text)
    } else if !state.available {
        (theme.error_surface, theme.error, theme.error_text)
    } else {
        let bg = if theme.flat {
            theme.warning_surface
        } else {
            theme.glass_warning
        };
        (bg, theme.warning, theme.warning_text)
    };
    Some((label, bg, border, text))
}

/// One compact §4.5 card, dish-first (task 1.3): 16 px padding, 8 px gaps and
/// 8 px after the card, the pinned card radii, the dish in the body role up to
/// four lines, a status badge on its semantic surface, a wrapping allergen line
/// above the right-aligned unchanged price, and — when pending — the accent
/// selection tint, the 1 px resolved boundary and an inline 24 px check slot.
/// Tappability comes from [`mobile_menu_tappable`]; no state dims readable text.
/// @req REQ-SCR-01 @req REQ-UI-15
pub fn menu_card(
    item: &MenuItem,
    state: MenuCardState,
    on_tap: Option<ClickHandler>,
    theme: &AppTheme,
) -> Stateful<Div> {
    let selected = state.selected;
    let card_fill = if selected {
        theme.mobile.selection_fill
    } else {
        theme.mobile_card_fill()
    };
    let text = if selected {
        theme.mobile.selection_ink
    } else {
        theme.text_primary
    };
    let meta = if selected {
        theme.mobile.selection_ink
    } else {
        theme.mobile_secondary_on(card_fill)
    };

    let mut card = theme::apply(
        div()
            .flex()
            .flex_col()
            .gap(px(theme.mobile.space_2))
            .p(px(theme.mobile.space_4))
            .mb(px(theme.mobile.space_2)),
        &theme.card_surface(),
    )
    .id(SharedString::from(format!("menu-{}|{}", item.id, item.day)));

    let treatment = mobile_card_treatment(&state);
    match treatment {
        MobileCardTreatment::Selected => {
            // Pending selection is the accent tint plus the 1 px resolved
            // boundary; it is not a saved status and not the old orange field.
            card = card
                .bg(theme.mobile.selection_fill)
                .border_1()
                .border_color(theme.mobile.selection_boundary);
        }
        MobileCardTreatment::PendingCancel => {
            // Dashed and struck through, on readable ink rather than a blanket
            // opacity reduction.
            card = card.border_dashed().border_1().border_color(theme.warning);
        }
        MobileCardTreatment::Ordered => {
            let bg = if theme.flat {
                theme.success_surface
            } else {
                theme.glass_success
            };
            card = card.bg(bg).border_1().border_color(theme.success_border);
        }
        MobileCardTreatment::Plain => {}
    }

    if let Some(on_tap) = on_tap {
        card = card
            .tab_index(0)
            .focus(crate::theme::desktop_focus(theme.mobile.focus))
            .cursor_pointer()
            .on_click(move |_, window, cx| on_tap(window, cx));
    }

    if let Some((label, bg, border, fg)) = badge(&state, theme) {
        card = card.child(
            div().flex().child(
                div()
                    .px(px(theme.mobile.space_2))
                    .py(px(theme.mobile.space_1))
                    .rounded(px(theme.mobile.radius_status))
                    .border_1()
                    .border_color(border)
                    .bg(bg)
                    .text_size(px(theme.mobile.meta))
                    .line_height(px(theme.mobile.meta_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(fg)
                    .child(label),
            ),
        );
    }

    let dish = div()
        .flex_1()
        .min_w(px(0.))
        .max_h(px(mobile_dish_max_height(theme)))
        .overflow_hidden()
        .text_size(px(theme.mobile.body))
        .line_height(px(theme.mobile.body_line))
        .text_color(text)
        .child(mobile_dish(item));
    let dish = if treatment.struck() {
        dish.line_through()
    } else {
        dish
    };
    let mut dish_row = div().flex().items_start().gap(px(theme.mobile.space_2));
    // The name row's slots, in paint order: a reserved inline check slot while
    // pending (never floated over the text), then the dish.
    let check_slot = mobile_card_name_slots(selected)
        .into_iter()
        .find_map(|slot| match slot {
            MobileNameSlot::Check(size) => Some(size),
            MobileNameSlot::Dish => None,
        });
    if let Some(slot) = check_slot {
        dish_row = dish_row.child(
            div()
                .w(px(slot))
                .h(px(slot))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(IconName::Check)
                        .with_size(px(theme.mobile.space_5))
                        .text_color(text),
                ),
        );
    }
    card = card.child(dish_row.child(dish));

    // Allergens wrap on the left and the price keeps its exact string on the
    // right; a long price drops to its own right-aligned line instead of
    // squeezing the letters away.
    let mut footer = div()
        .flex()
        .flex_wrap()
        .items_end()
        .gap(px(theme.mobile.space_2));
    for part in mobile_card_footer(item) {
        footer = match part {
            MobileFooterPart::Allergens(allergens) => footer.child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_size(px(theme.mobile.meta))
                    .line_height(px(theme.mobile.meta_line))
                    .text_color(meta)
                    .child(allergens),
            ),
            MobileFooterPart::Price(price) => footer.child(
                div()
                    .ml_auto()
                    .flex_shrink_0()
                    .whitespace_nowrap()
                    .text_size(px(theme.mobile.body))
                    .line_height(px(theme.mobile.body_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(text)
                    .child(price),
            ),
        };
    }
    card.child(footer)
}

#[cfg(test)]
mod tests {
    use super::{
        mobile_badge_label, mobile_card_footer, mobile_card_name_slots, mobile_card_treatment,
        mobile_dish, mobile_dish_max_height, mobile_menu_tappable, MenuCardState,
        MobileCardTreatment, MobileFooterPart, MobileNameSlot, MOBILE_DISH_LINES,
    };
    use snackpilot_core::domain::{MenuCategory, MenuItem};

    fn item(title: &str, subtitle: &str) -> MenuItem {
        MenuItem {
            id: "m1".into(),
            day: "2026-11-16".into(),
            title: title.into(),
            subtitle: subtitle.into(),
            allergens: vec!["A".into(), "C".into(), "G".into()],
            available: true,
            ordered: false,
            category: MenuCategory::Menu1,
            price: "6,40 €".into(),
        }
    }

    fn state() -> MenuCardState {
        MenuCardState {
            selected: false,
            pending_cancel: false,
            ordered: false,
            available: true,
            cutoff: false,
        }
    }

    /// Sample M2's defect fix (REQ-SCR-01/REQ-UI-15): the compact card's main
    /// text is the dish `subtitle`, and only an empty subtitle falls back to the
    /// title. The category heading above the group stays the sole MENÜ label,
    /// so the card never repeats `MENÜ I` over its own lunch.
    /// @req REQ-SCR-01 @req REQ-UI-15
    #[test]
    fn mobile_menu_uses_subtitle_and_empty_fallback() {
        let dish = "Rindsgulasch mit Serviettenknödel, Rahmsauce und Blattsalat mit Kürbiskernöl";
        let full = item("MENÜ I", dish);
        assert_eq!(mobile_dish(&full), dish, "the card shows the actual dish");
        assert_ne!(mobile_dish(&full), "MENÜ I", "the category is not the dish");

        for subtitle in ["", "   "] {
            let empty = item("MENÜ II", subtitle);
            assert_eq!(
                mobile_dish(&empty),
                "MENÜ II",
                "an empty subtitle falls back to the title, never a blank card"
            );
        }
        // Allergens and the price string are carried unchanged: nothing here
        // rewrites the domain text.
        assert_eq!(full.allergens, vec!["A", "C", "G"]);
        assert_eq!(full.price, "6,40 €");
    }

    /// The compact card's structure, as the builder paints it
    /// (REQ-SCR-01/REQ-UI-15): a pending selection reserves the inline 24 px
    /// check slot before the dish (never a floated check, and nothing at all
    /// when not pending), and the footer keeps the allergens first and wrapping
    /// above the unchanged right-aligned price string.
    /// @req REQ-SCR-01 @req REQ-UI-15
    #[test]
    fn mobile_menu_card_check_slot_and_footer_order() {
        // Selected: one reserved 24 px slot, then the dish.
        assert_eq!(
            mobile_card_name_slots(true),
            vec![MobileNameSlot::Check(24.), MobileNameSlot::Dish],
            "the check is an inline slot before the dish"
        );
        // Not selected: no slot, so the dish keeps the full row.
        assert_eq!(mobile_card_name_slots(false), vec![MobileNameSlot::Dish]);

        // Allergens first (left, wrapping), the price last (right, unchanged).
        let full = item("MENÜ II", "Gemüse-Curry mit Basmatireis und Naan-Brot");
        let footer = mobile_card_footer(&full);
        assert_eq!(
            footer,
            vec![
                MobileFooterPart::Allergens("Allergene: A, C, G".into()),
                MobileFooterPart::Price(full.price.clone()),
            ],
            "allergens come above the right-aligned price"
        );
        match &footer[1] {
            MobileFooterPart::Price(price) => {
                assert_eq!(price, "6,40 €", "the domain price string is unchanged")
            }
            other => panic!("the price is the trailing part, got {other:?}"),
        }
        // A dish without allergens keeps only the price line — no empty row.
        let plain = MenuItem {
            allergens: Vec::new(),
            ..full.clone()
        };
        assert_eq!(
            mobile_card_footer(&plain),
            vec![MobileFooterPart::Price("6,40 €".into())]
        );
    }

    /// The pending-cancel treatment and the four-line cap (REQ-SCR-01/
    /// REQ-UI-15): a queued cancellation is a dashed boundary with a struck
    /// dish on readable ink (never a blanket opacity), it outranks the
    /// selection tint, and the dish can never paint more than four body lines
    /// before it clips.
    /// @req REQ-SCR-01 @req REQ-UI-15
    #[test]
    fn mobile_menu_card_pending_cancel_and_line_cap() {
        use crate::theme::AppTheme;
        use snackpilot_model::AccentColor;

        let pending_cancel = MenuCardState {
            pending_cancel: true,
            selected: true,
            ordered: true,
            ..state()
        };
        let treatment = mobile_card_treatment(&pending_cancel);
        assert_eq!(treatment, MobileCardTreatment::PendingCancel);
        assert!(treatment.dashed(), "pending cancellation is dashed");
        assert!(treatment.struck(), "the dish is struck through");
        assert_eq!(
            mobile_card_treatment(&MenuCardState {
                selected: true,
                ordered: true,
                ..state()
            }),
            MobileCardTreatment::Selected,
            "the selection tint is not a cancellation"
        );
        assert_eq!(
            mobile_card_treatment(&MenuCardState {
                ordered: true,
                ..state()
            }),
            MobileCardTreatment::Ordered
        );
        assert_eq!(mobile_card_treatment(&state()), MobileCardTreatment::Plain);
        assert!(!mobile_card_treatment(&state()).struck());

        // Four body lines is the dish's whole height, so a fifth line clips
        // instead of pushing the price and badge out of the card.
        let theme = AppTheme::build(false, AccentColor::Orange, false);
        assert_eq!(MOBILE_DISH_LINES, 4);
        assert_eq!(mobile_dish_max_height(&theme), theme.mobile.body_line * 4.);
        assert!(
            mobile_dish_max_height(&theme) < theme.mobile.body_line * 5.,
            "a fifth line cannot paint"
        );
    }

    /// The badge and the pending check stay distinct (REQ-SCR-01): the badge is
    /// one status word in the §4.5 precedence cancellation → ordered →
    /// unavailable → cutoff, carries no check glyph, and eligibility is the
    /// incumbent model gate `ordered || (available && !cutoff)` — a pending
    /// selection never makes an unavailable or closed dish tappable.
    /// @req REQ-SCR-01
    #[test]
    fn mobile_menu_badge_precedence_and_eligibility() {
        // Precedence: every later flag loses to the earlier one.
        let all = MenuCardState {
            pending_cancel: true,
            ordered: true,
            available: false,
            cutoff: true,
            ..state()
        };
        assert_eq!(mobile_badge_label(&all), Some("Wird storniert"));
        assert_eq!(
            mobile_badge_label(&MenuCardState {
                ordered: true,
                available: false,
                cutoff: true,
                ..state()
            }),
            Some("Bestellt")
        );
        assert_eq!(
            mobile_badge_label(&MenuCardState {
                available: false,
                cutoff: true,
                ..state()
            }),
            Some("Ausverkauft")
        );
        assert_eq!(
            mobile_badge_label(&MenuCardState {
                cutoff: true,
                ..state()
            }),
            Some("Geschlossen")
        );
        assert_eq!(
            mobile_badge_label(&state()),
            None,
            "a plain dish has no badge"
        );

        // A badge is a status word, never a check mark: the pending check is
        // the inline Lucide slot, and the two can coexist.
        let badged = [
            MenuCardState {
                pending_cancel: true,
                ..state()
            },
            MenuCardState {
                ordered: true,
                ..state()
            },
            MenuCardState {
                available: false,
                ..state()
            },
            MenuCardState {
                cutoff: true,
                ..state()
            },
        ];
        for flags in badged {
            let label = mobile_badge_label(&flags).unwrap();
            assert!(
                !label.contains('✓') && !label.contains("Vorgemerkt"),
                "{label} must not read as the check"
            );
        }
        let selected_and_badged = MenuCardState {
            selected: true,
            ordered: true,
            ..state()
        };
        assert_eq!(mobile_badge_label(&selected_and_badged), Some("Bestellt"));

        // Eligibility is the incumbent gate: ordered items stay tappable for
        // their cancellation, available-and-open items are, an unavailable or
        // closed new item is not — and a pending selection does not change
        // that (the review's regression: `|| selected` made a picked dish that
        // became unavailable tappable again).
        assert!(mobile_menu_tappable(&state()));
        assert!(mobile_menu_tappable(&MenuCardState {
            ordered: true,
            cutoff: true,
            ..state()
        }));
        assert!(!mobile_menu_tappable(&MenuCardState {
            available: false,
            ..state()
        }));
        assert!(!mobile_menu_tappable(&MenuCardState {
            cutoff: true,
            ..state()
        }));
        for flags in [
            MenuCardState {
                selected: true,
                available: false,
                ..state()
            },
            MenuCardState {
                selected: true,
                cutoff: true,
                ..state()
            },
            MenuCardState {
                selected: true,
                available: false,
                cutoff: true,
                ..state()
            },
        ] {
            assert!(
                !mobile_menu_tappable(&flags),
                "a pending selection never re-enables an unavailable or closed dish"
            );
        }
    }
}

/// Desktop row state: the same domain facts as [`MenuCardState`] plus the
/// wide-desktop preview selection. Kept apart from the mobile card so that
/// recipe (and its opaque selected fill) stays exactly as before.
#[derive(Debug, Clone, Copy, Default)]
pub struct DesktopMenuRowState {
    pub selected: bool,
    pub pending_order: bool,
    pub pending_cancel: bool,
    pub ordered: bool,
    pub available: bool,
    pub cutoff: bool,
}

/// The brief's one desktop row/detail status: `(label, ink, fill)` in
/// precedence order. A pending order is its check plus `Vorgemerkt`, never
/// the row cursor.
pub fn desktop_menu_status(
    state: &DesktopMenuRowState,
    theme: &AppTheme,
) -> Option<(&'static str, Hsla, Hsla)> {
    if state.pending_cancel {
        return Some(("Wird storniert", theme.warning_text, theme.warning_surface));
    }
    if state.pending_order {
        return Some((
            "✓ Vorgemerkt",
            theme.desktop.accent_on_selection,
            theme.primary_surface,
        ));
    }
    if state.ordered {
        return Some(("Bestellt", theme.success_text, theme.success_surface));
    }
    if !state.available {
        return Some(("Ausverkauft", theme.error_text, theme.error_surface));
    }
    if state.cutoff {
        return Some(("Geschlossen", theme.warning_text, theme.warning_surface));
    }
    None
}

/// The desktop detail's explicit action `(label, kind)`, in the brief's
/// precedence. `None` when the incumbent model allows no toggle at all
/// (`ordered || (available && !cutoff)` is the mobile gate).
pub fn desktop_menu_action(state: &DesktopMenuRowState) -> Option<(&'static str, ButtonKind)> {
    if state.pending_cancel {
        Some(("Stornierung aufheben", ButtonKind::Secondary))
    } else if state.pending_order {
        Some(("Auswahl aufheben", ButtonKind::Secondary))
    } else if state.ordered {
        Some(("Stornierung vormerken", ButtonKind::Danger))
    } else if state.available && !state.cutoff {
        Some(("Zur Bestellung hinzufügen", ButtonKind::Primary))
    } else {
        None
    }
}

/// One wide-desktop grouped menu row: min 88 px, 12/16 padding, category/title
/// 14/20 Semibold, a two-line 14/20 excerpt and price plus one status 12/16.
/// The 3 px accent rule is always present so selecting never shifts the row;
/// picking it previews only, which the caller wires.
/// @req REQ-SCR-01 @req REQ-UI-12
pub fn desktop_menu_row(
    item: &MenuItem,
    state: DesktopMenuRowState,
    on_click: ClickHandler,
    focus: Option<&RowFocus>,
    theme: &AppTheme,
) -> AnyElement {
    let selected = state.selected;
    // Never fabricate a dish: an empty subtitle falls back to the title.
    let excerpt = if item.subtitle.trim().is_empty() {
        item.title.clone()
    } else {
        item.subtitle.clone()
    };
    let excerpt = div()
        .max_h(px(40.))
        .overflow_hidden()
        .text_size(px(14.))
        .line_height(px(20.))
        .text_color(if state.pending_cancel {
            theme.text_tertiary
        } else {
            theme.text_secondary
        })
        .child(excerpt);
    let excerpt = if state.pending_cancel {
        excerpt.line_through()
    } else {
        excerpt
    };

    let mut bottom = div()
        .mt(px(4.))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(item.price.clone()),
        );
    if let Some((label, ink, fill)) = desktop_menu_status(&state, theme) {
        bottom = bottom.child(
            div()
                .px(px(8.))
                .py(px(2.))
                .rounded(px(4.))
                .bg(fill)
                .text_size(px(12.))
                .line_height(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink)
                .whitespace_nowrap()
                .child(label),
        );
    }

    let mut row = div()
        .id(SharedString::from(format!(
            "desktop-menu-{}",
            menu_key(item)
        )))
        .min_h(px(88.))
        .flex_shrink_0()
        .flex()
        .border_b_1()
        .border_color(theme.border)
        .bg(if selected {
            theme.primary_surface
        } else {
            theme.surface
        })
        .hover(|s| s.bg(theme.surface_variant))
        .focus(crate::theme::desktop_focus(theme.desktop.focus));
    // Inside the wide list the list owns the one tab stop and this row is
    // focused by its roving handle; a standalone row keeps its own tab stop.
    row = match focus {
        Some(focus) => row
            .track_focus(&focus.handle)
            .anchor_scroll(Some(focus.anchor.clone())),
        None => row.tab_index(0),
    };
    row.cursor_pointer()
        .on_click(move |_, window, cx| on_click(window, cx))
        .child(
            div()
                .w(px(3.))
                .self_stretch()
                .flex_shrink_0()
                .bg(if selected {
                    theme.desktop.accent_boundary
                } else {
                    hsla(0., 0., 0., 0.)
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .justify_center()
                .gap(px(2.))
                .px(px(16.))
                .py(px(12.))
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(item.title.clone()),
                )
                .child(excerpt)
                .child(bottom),
        )
        .into_any_element()
}
