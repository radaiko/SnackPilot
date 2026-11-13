//! Menu row — docs/requirements/04-ui-ux.md §4.5.
//!
//! One flat row per menu item: surface, 1 px hairline, 12 radius, 16 padding,
//! dish and price strong, allergens quiet, a status chip that always carries
//! its word, and the reserved 24 px check while the dish is pending.
//! Tappability is decided by the caller from the model
//! (`ordered || (available && !cutoff)`).

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::Styled;

use crate::components::button::ButtonKind;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::MenuItem;

/// §4.5 state overlays.
pub struct MenuCardState {
    pub selected: bool,
    pub pending_cancel: bool,
    pub ordered: bool,
    pub available: bool,
    pub cutoff: bool,
}

/// The dish a compact row presents: the required dish `subtitle`, falling
/// back to the title only when the subtitle is empty (sample M2). The category
/// stays the quiet group label, so the row never repeats `MENÜ I`.
pub fn mobile_dish(item: &MenuItem) -> String {
    if item.subtitle.trim().is_empty() {
        item.title.clone()
    } else {
        item.subtitle.clone()
    }
}

/// The row's one status word, in the design's precedence:
/// pending cancellation → ordered → pending selection → sold out → open.
/// A dish past the cutoff is simply not tappable and carries no chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuStatus {
    Open,
    Reserved,
    Ordered,
    Cancelling,
    SoldOut,
}

impl MenuStatus {
    /// The chip's visible word: the status is never conveyed by colour alone.
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Geöffnet",
            Self::Reserved => "Reserviert",
            Self::Ordered => "Bestellt",
            Self::Cancelling => "Stornierung offen",
            Self::SoldOut => "Ausgebucht",
        }
    }
}

/// The one status word for a row, or `None` past the cutoff.
pub fn mobile_menu_status(state: &MenuCardState) -> Option<MenuStatus> {
    Some(if state.pending_cancel {
        MenuStatus::Cancelling
    } else if state.ordered {
        MenuStatus::Ordered
    } else if state.selected {
        MenuStatus::Reserved
    } else if !state.available {
        MenuStatus::SoldOut
    } else if state.cutoff {
        return None;
    } else {
        MenuStatus::Open
    })
}

/// The chip's `(surface, ink, boundary)` in the flat palette: neutral for the
/// ordinary state, tinted for the semantic ones, with a readable word on it.
pub fn mobile_status_tones(status: MenuStatus, theme: &AppTheme) -> (Hsla, Hsla, Hsla) {
    match status {
        MenuStatus::Open => (theme.surface_alt, theme.text_secondary, theme.border),
        // The accent on its own tint reaches only 4.44:1 in light, so the word
        // takes the primary text role and the boundary keeps the accent.
        MenuStatus::Reserved => (theme.accent_surface, theme.text_primary, theme.accent),
        MenuStatus::Ordered => (theme.success_surface, theme.success, theme.success),
        MenuStatus::Cancelling => (theme.warning_surface, theme.warning, theme.warning),
        MenuStatus::SoldOut => (theme.error_surface, theme.error, theme.error),
    }
}

/// The trailing pending check: a 24 px Lucide glyph in a reserved slot, so a
/// long dish never slides under it and a saved status never reads as a check.
pub const MOBILE_CHECK: f32 = 24.;
/// The status chip's one height.
pub const MOBILE_CHIP_HEIGHT: f32 = 24.;

/// One slot of the compact row's name line, in paint order: the dish and, only
/// while pending, the trailing check. The price lives on the meta row, so a
/// wrapped two-line dish never crowds it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MobileRowSlot {
    Dish,
    Check(f32),
}

pub fn mobile_row_slots(selected: bool) -> Vec<MobileRowSlot> {
    if selected {
        vec![MobileRowSlot::Dish, MobileRowSlot::Check(MOBILE_CHECK)]
    } else {
        vec![MobileRowSlot::Dish]
    }
}

/// One part of the compact row's meta row, in paint order: the wrapping
/// allergen line on the left and the price at the trailing edge.
#[derive(Debug, Clone, PartialEq)]
pub enum MobileFooterPart {
    Allergens(String),
    Price(String),
}

pub fn mobile_card_footer(item: &MenuItem) -> Vec<MobileFooterPart> {
    let mut parts = Vec::new();
    if !item.allergens.is_empty() {
        parts.push(MobileFooterPart::Allergens(format!(
            "Allergene: {}",
            item.allergens.join(", ")
        )));
    }
    parts.push(MobileFooterPart::Price(item.price.clone()));
    parts
}

/// The row's resting treatment: plain surface, the accent selection tint, or
/// the warning boundary with a struck dish for a queued cancellation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MobileCardTreatment {
    Plain,
    Selected,
    PendingCancel,
}

impl MobileCardTreatment {
    pub fn struck(self) -> bool {
        self == Self::PendingCancel
    }

    pub fn dashed(self) -> bool {
        self == Self::PendingCancel
    }
}

pub fn mobile_card_treatment(state: &MenuCardState) -> MobileCardTreatment {
    if state.pending_cancel {
        MobileCardTreatment::PendingCancel
    } else if state.selected {
        MobileCardTreatment::Selected
    } else {
        MobileCardTreatment::Plain
    }
}

/// Whether a compact row is one selection target, exactly as the incumbent
/// model gate: an ordered item stays tappable so its cancellation can be
/// queued, an unavailable or cutoff new item cannot be added.
pub fn mobile_menu_tappable(state: &MenuCardState) -> bool {
    state.ordered || (state.available && !state.cutoff)
}

/// The status chip: a 24 px tag in the metadata role on its toned surface,
/// always with its word.
fn status_chip(status: MenuStatus, theme: &AppTheme) -> Div {
    let (fill, ink, border) = mobile_status_tones(status, theme);
    div()
        .h(px(MOBILE_CHIP_HEIGHT))
        .flex_shrink_0()
        .px(px(theme.scale.space_2))
        .rounded(px(theme.scale.radius_chip))
        .border(px(1.))
        .border_color(border)
        .bg(fill)
        .flex()
        .items_center()
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(ink)
                .child(status.label()),
        )
}

/// One compact §4.5 row: surface, 1 px hairline, 12 radius, 16 padding and
/// 8 gaps. Name strong, allergens quiet, price strong at the trailing edge and
/// the 24 px check last while pending; below them the status chip. The pending
/// cancellation keeps a struck dish and a dashed warning boundary; nothing
/// dims readable text. Tappability comes from [`mobile_menu_tappable`].
/// @req REQ-SCR-01 @req REQ-UI-15 @req REQ-UI-16
pub fn menu_card(
    item: &MenuItem,
    state: MenuCardState,
    on_tap: Option<ClickHandler>,
    theme: &AppTheme,
) -> Stateful<Div> {
    let m = &theme.scale;
    let treatment = mobile_card_treatment(&state);
    let (fill, border) = match treatment {
        MobileCardTreatment::Selected => (theme.accent_surface, theme.accent),
        MobileCardTreatment::PendingCancel => (theme.surface, theme.warning),
        MobileCardTreatment::Plain => (theme.surface, theme.border),
    };
    let pressed = theme.pressed(fill);

    let mut card = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(m.space_2))
        .p(px(m.space_4))
        .mb(px(m.space_2))
        .rounded(px(theme.scale.radius_card))
        .bg(fill)
        .border(px(1.))
        .border_color(border)
        .id(SharedString::from(format!("menu-{}|{}", item.id, item.day)));
    if treatment.dashed() {
        card = card.border_dashed();
    }

    if let Some(on_tap) = on_tap {
        card = card
            .tab_index(0)
            .cursor_pointer()
            .hover(move |s| s.bg(pressed))
            .focus(theme::desktop_focus(theme.accent))
            .on_click(move |_, window, cx| on_tap(window, cx));
    }

    // Name line: the dish and, while pending, the reserved check at the right.
    let dish_text = div()
        .flex_1()
        .min_w(px(0.))
        .text_size(px(m.body))
        .line_height(px(m.body_line))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.text_primary)
        .child(mobile_dish(item));
    let dish_text = if treatment.struck() {
        dish_text.line_through()
    } else {
        dish_text
    };
    let mut name = div()
        .flex()
        .items_start()
        .gap(px(m.space_2))
        .child(dish_text);
    if state.selected {
        name = name.child(
            div()
                .w(px(MOBILE_CHECK))
                .h(px(MOBILE_CHECK))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(IconName::Check)
                        .with_size(px(MOBILE_CHECK))
                        .text_color(theme.accent),
                ),
        );
    }
    card = card.child(name);

    // Quiet meta row: the wrapping allergens on the left, the status word and
    // the price at the trailing edge.
    let mut footer = div().flex().items_start().gap(px(m.space_2));
    let mut has_allergens = false;
    let mut price = None;
    for part in mobile_card_footer(item) {
        match part {
            MobileFooterPart::Allergens(allergens) => {
                has_allergens = true;
                footer = footer.child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .text_size(px(m.meta))
                        .line_height(px(m.meta_line))
                        .text_color(theme.text_secondary)
                        .child(allergens),
                );
            }
            MobileFooterPart::Price(value) => price = Some(value),
        }
    }
    if !has_allergens {
        footer = footer.child(div().flex_1());
    }
    if let Some(status) = mobile_menu_status(&state) {
        footer = footer.child(status_chip(status, theme));
    }
    if let Some(price) = price {
        footer = footer.child(
            div()
                .flex_shrink_0()
                .whitespace_nowrap()
                .text_size(px(m.body))
                .line_height(px(m.body_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(price),
        );
    }
    card.child(footer)
}

#[cfg(test)]
mod tests {
    use super::{
        mobile_card_footer, mobile_card_treatment, mobile_dish, mobile_menu_status,
        mobile_menu_tappable, mobile_row_slots, mobile_status_tones, MenuCardState, MenuStatus,
        MobileCardTreatment, MobileFooterPart, MobileRowSlot, MOBILE_CHECK, MOBILE_CHIP_HEIGHT,
    };
    use crate::theme::{to_rgb, AppTheme};
    use snackpilot_core::domain::{MenuCategory, MenuItem};
    use snackpilot_model::desktop::contrast_ratio;
    use snackpilot_model::tokens;

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

    /// Sample M2's defect fix (REQ-SCR-01/REQ-UI-15): the compact row's main
    /// text is the dish `subtitle`, and only an empty subtitle falls back to the
    /// title. Allergens and the price string are carried unchanged.
    /// @req REQ-SCR-01 @req REQ-UI-15
    #[test]
    fn mobile_menu_uses_subtitle_and_empty_fallback() {
        let dish = "Rindsgulasch mit Serviettenknödel, Rahmsauce und Blattsalat mit Kürbiskörnern";
        let full = item("MENÜ I", dish);
        assert_eq!(mobile_dish(&full), dish, "the row shows the actual dish");
        assert_ne!(mobile_dish(&full), "MENÜ I", "the category is not the dish");

        for subtitle in ["", "   "] {
            let empty = item("MENÜ II", subtitle);
            assert_eq!(
                mobile_dish(&empty),
                "MENÜ II",
                "an empty subtitle falls back to the title, never a blank row"
            );
        }
        assert_eq!(full.allergens, vec!["A", "C", "G"]);
        assert_eq!(full.price, "6,40 €");
    }

    /// The flat row's meta row (REQ-SCR-01/REQ-UI-15): the 24 px pending check
    /// is the last name-line slot and never shares it with the price. The
    /// wrapping allergen line sits left and the price is the trailing meta-row
    /// part, so a two-line dish cannot crowd it. The shared row recipe is the
    /// one 12 radius / 16 padding / 1 px surface.
    /// @req REQ-SCR-01 @req REQ-UI-15
    #[test]
    fn mobile_menu_price_is_in_the_trailing_meta_row() {
        assert_eq!(
            mobile_row_slots(true),
            vec![MobileRowSlot::Dish, MobileRowSlot::Check(MOBILE_CHECK)],
            "the pending check is the trailing name-line slot"
        );
        assert_eq!(MOBILE_CHECK, 24.);
        assert_eq!(
            mobile_row_slots(false),
            vec![MobileRowSlot::Dish],
            "no reserved check when the dish is not pending"
        );

        let full = item("MENÜ II", "Gemüse-Curry mit Basmatireis und Naan-Brot");
        assert_eq!(
            mobile_card_footer(&full),
            vec![
                MobileFooterPart::Allergens("Allergene: A, C, G".into()),
                MobileFooterPart::Price("6,40 €".into()),
            ],
            "allergens stay left, the price is the trailing meta-row part"
        );
        let plain = MenuItem {
            allergens: Vec::new(),
            ..full.clone()
        };
        assert_eq!(
            mobile_card_footer(&plain),
            vec![MobileFooterPart::Price("6,40 €".into())],
            "the price is always present, even without allergens"
        );

        let theme = AppTheme::build(false);
        let row = theme.card_surface();
        assert_eq!(row.radius, gpui_kit::px(tokens::RADIUS_CARD));
        assert_eq!(row.border.map(|(w, _)| w), Some(gpui_kit::px(1.)));
        assert_eq!(theme.scale.space_4, 16.);
        assert_eq!(MOBILE_CHIP_HEIGHT, 24.);
    }

    /// The one status word per row (REQ-SCR-01/REQ-UI-15): pending
    /// cancellation outranks ordered, ordered outranks the pending selection,
    /// a pending selection outranks sold out, and a dish past the cutoff carries
    /// no chip. Every toned surface keeps a readable word (never colour alone).
    /// @req REQ-SCR-01 @req REQ-UI-15
    #[test]
    fn mobile_menu_status_chip_words_and_readable_tones() {
        assert_eq!(
            mobile_menu_status(&MenuCardState {
                pending_cancel: true,
                ordered: true,
                selected: true,
                available: false,
                cutoff: true,
            })
            .unwrap()
            .label(),
            "Stornierung offen"
        );
        let ordered = MenuCardState {
            ordered: true,
            ..state()
        };
        assert_eq!(mobile_menu_status(&ordered), Some(MenuStatus::Ordered));
        assert_eq!(MenuStatus::Ordered.label(), "Bestellt");
        let reserved = MenuCardState {
            selected: true,
            ..state()
        };
        assert_eq!(mobile_menu_status(&reserved), Some(MenuStatus::Reserved));
        assert_eq!(MenuStatus::Reserved.label(), "Reserviert");
        assert_eq!(
            mobile_menu_status(&MenuCardState {
                available: false,
                ..state()
            }),
            Some(MenuStatus::SoldOut)
        );
        assert_eq!(MenuStatus::SoldOut.label(), "Ausgebucht");
        assert_eq!(mobile_menu_status(&state()), Some(MenuStatus::Open));
        assert_eq!(MenuStatus::Open.label(), "Geöffnet");
        assert_eq!(
            mobile_menu_status(&MenuCardState {
                cutoff: true,
                ..state()
            }),
            None,
            "a closed dish is not a status word; it is simply not tappable"
        );

        // Each chip's word reads at 4.5:1 on its own toned surface.
        for dark in [false, true] {
            let theme = AppTheme::build(dark);
            for status in [
                MenuStatus::Open,
                MenuStatus::Reserved,
                MenuStatus::Ordered,
                MenuStatus::Cancelling,
                MenuStatus::SoldOut,
            ] {
                let (fill, ink, border) = mobile_status_tones(status, &theme);
                let on_fill = to_rgb(fill, fill);
                assert!(
                    contrast_ratio(to_rgb(ink, fill), on_fill) >= tokens::TEXT_MIN_CONTRAST,
                    "{} on its chip in {}",
                    status.label(),
                    if dark { "dark" } else { "light" }
                );
                if status == MenuStatus::Open {
                    // The neutral chip is one flat surface: its hairline is
                    // decorative, not a control boundary.
                    assert_ne!(border, fill);
                } else {
                    assert!(
                        contrast_ratio(to_rgb(border, fill), on_fill)
                            >= tokens::CONTROL_MIN_CONTRAST,
                        "{} boundary in {}",
                        status.label(),
                        if dark { "dark" } else { "light" }
                    );
                }
            }
        }
    }

    /// The pending-cancel treatment and eligibility (REQ-SCR-01): a queued
    /// cancellation is the one struck, dashed row; a pending selection is not a
    /// cancellation, and a pending selection never makes an unavailable or
    /// closed dish tappable.
    /// @req REQ-SCR-01
    #[test]
    fn mobile_menu_pending_cancel_and_eligibility() {
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
        assert_eq!(mobile_card_treatment(&state()), MobileCardTreatment::Plain);
        assert!(!mobile_card_treatment(&state()).struck());

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
        return Some(("Wird storniert", theme.warning, theme.warning_surface));
    }
    if state.pending_order {
        return Some(("✓ Vorgemerkt", theme.accent, theme.accent_surface));
    }
    if state.ordered {
        return Some(("Bestellt", theme.success, theme.success_surface));
    }
    if !state.available {
        return Some(("Ausverkauft", theme.error, theme.error_surface));
    }
    if state.cutoff {
        return Some(("Geschlossen", theme.warning, theme.warning_surface));
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
