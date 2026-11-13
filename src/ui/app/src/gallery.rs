//! Debug component gallery. It keeps only the boards that still mean
//! something on the one flat system: the mobile main screens, the desktop
//! shell at 720/1280 and the shared control states, light and dark. Opened by
//! the debug hook in `mount` when `SNACKPILOT_GALLERY=1`.

use std::rc::Rc;

use gpui_kit::component::input::InputState;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::StyledExt;
use gpui_kit::prelude::*;
use gpui_kit::{div, px, AnyElement, App, Context, Entity, SharedString, Window};
use snackpilot_core::domain::{MenuCategory, MenuItem, MonthOption, OrderedMenu};
use snackpilot_model::desktop::{desktop_geometry, ControlState, HostKind};
use snackpilot_model::navigation::Tab;
use snackpilot_model::{BillingEntry, BillingLine, BillingSource, BillingTotals, EntrySource};

use crate::components::bill_card::bill_card;
use crate::components::day_navigator::mobile_day_navigator;
use crate::components::fab::{dock_height_for_lines, mobile_dock, DockState};
use crate::components::menu_card::{menu_card, DesktopMenuRowState, MenuCardState};
use crate::components::order_item::order_item;
use crate::components::segment_tabs::{segment_tabs, Segment, SegmentHandler};
use crate::components::tab_bar::TabSelect;
use crate::components::{
    button, chip, desktop_navigation, loading_overlay, record_table, spinner, text_field, toast,
    workspace,
};
use crate::navigation::ClickHandler;
use crate::screens::billing::{
    mobile_month_row, mobile_source_row, mobile_summary_surface, MonthSelect, SourceSelect,
};
use crate::screens::orders::confirm_banner;
use crate::theme::AppTheme;

/// Which half of the gallery to show. Both stacked is the default; the debug
/// section knob (see `crate::gallery_requested`) picks one so a screenshot fits
/// on one screen.
pub enum Section {
    Light,
    Dark,
    Both,
}

/// The gallery root view; holds the interactive state (one switch, one
/// text field per appearance).
pub struct GalleryRoot {
    which: Section,
    /// 2 = desktop shell, 3 = mobile boards, 4-6 = one mobile screen each.
    page: usize,
    /// Five independent desktop field states per appearance, so the desktop
    /// page can paint rest/hover/focus/disabled/selected in one frame.
    control_fields: [Vec<Entity<InputState>>; 2],
}

impl GalleryRoot {
    pub fn new(window: &mut Window, cx: &mut App, which: Section) -> Self {
        let light = AppTheme::build(false);
        let dark = AppTheme::build(true);
        let mut fields = |theme: &AppTheme| -> Vec<Entity<InputState>> {
            (0..5)
                .map(|_| text_field::text_field("Suchen", None, theme, window, cx).0)
                .collect()
        };
        let control_fields = [fields(&light), fields(&dark)];
        let page = match crate::gallery_page().as_deref() {
            Some("desktop") | Some("2") => 2,
            Some("menus") | Some("4") => 4,
            Some("orders") | Some("5") => 5,
            Some("billing") | Some("6") => 6,
            _ => 3,
        };
        Self {
            which,
            page,
            control_fields,
        }
    }
}

/// The desktop gallery page: the shell at compact and expanded widths with
/// the labelled rail, then every shared control forced through
/// rest/hover/focus/disabled/selected. Light and dark are rendered by the
/// caller; every focus ring sits in its reserved 4 px room.
fn desktop_section(
    dark: bool,
    fields: &Vec<Entity<InputState>>,
    cx: &mut Context<GalleryRoot>,
) -> AnyElement {
    let theme = AppTheme::build(dark);
    let title: SharedString = if dark {
        "Desktop · Dunkel".into()
    } else {
        "Desktop · Hell".into()
    };
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_3))
        .p(px(theme.scale.space_4))
        .bg(theme.background)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(theme.scale.space_1))
                .child(
                    div()
                        .text_size(px(theme.scale.headline))
                        .line_height(px(theme.scale.headline_line))
                        .font_semibold()
                        .text_color(theme.text_primary)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(theme.scale.meta))
                        .line_height(px(theme.scale.meta_line))
                        .text_color(theme.text_secondary)
                        .child("Rail 240 px · Surface auf BG · 40 px Zeilen"),
                ),
        )
        .child(workspace::section_label("Shell · 720 / 1280", &theme))
        .child(desktop_shell_frame(
            720.,
            desktop_shell(720., &theme, cx, "Kompakt · 720"),
        ))
        .child(desktop_shell_frame(
            1280.,
            desktop_shell(1280., &theme, cx, "Erweitert · 1280"),
        ))
        .child(desktop_controls(&theme, dark, fields, cx))
        .into_any_element()
}

/// One static item per Menüs state; the first is the selected long dish that
/// the detail renders with its allergens.
fn gallery_menu_rows() -> Vec<(MenuItem, DesktopMenuRowState)> {
    let item = |id: &str, title: &str, category: MenuCategory, subtitle: &str| MenuItem {
        id: id.to_string(),
        day: "2026-11-11".to_string(),
        title: title.to_string(),
        subtitle: subtitle.to_string(),
        allergens: Vec::new(),
        available: true,
        ordered: false,
        category,
        price: "6,40 €".to_string(),
    };
    let long = "Rindsgulasch mit Serviettenknödel, Rahmsauce und Blattsalat mit \
                Kürbiskernöl, dazu ein kleines Dessert";
    vec![
        (
            MenuItem {
                allergens: vec!["A".into(), "C".into(), "G".into()],
                ..item("g1", "MENÜ I", MenuCategory::Menu1, long)
            },
            DesktopMenuRowState {
                selected: true,
                pending_order: true,
                ..Default::default()
            },
        ),
        (
            item(
                "g2",
                "MENÜ II",
                MenuCategory::Menu2,
                "Pasta mit Tomatensugo und Parmesan",
            ),
            DesktopMenuRowState {
                pending_cancel: true,
                ordered: true,
                ..Default::default()
            },
        ),
        (
            item(
                "g3",
                "MENÜ III",
                MenuCategory::Menu3,
                "Gemüsecurry mit Basmatireis",
            ),
            DesktopMenuRowState {
                ordered: true,
                ..Default::default()
            },
        ),
        (
            item(
                "g4",
                "MENÜ I",
                MenuCategory::Menu1,
                "Schnitzel mit Kartoffelsalat",
            ),
            DesktopMenuRowState::default(),
        ),
        (
            item(
                "g5",
                "MENÜ II",
                MenuCategory::Menu2,
                "Kaiserschmarrn mit Apfelmus",
            ),
            DesktopMenuRowState {
                available: true,
                cutoff: true,
                ..Default::default()
            },
        ),
    ]
}

fn gallery_receipts() -> Vec<BillingEntry> {
    vec![
        BillingEntry {
            id: "g-1001".to_string(),
            epoch_ms: 1_794_308_400_000,
            source: EntrySource::Gourmet,
            description: String::new(),
            amount: -6.4,
            items: vec![
                BillingLine {
                    count: 2,
                    description: "Rindsgulasch mit Serviettenknödel, Rahmsauce und Blattsalat"
                        .to_string(),
                    total: 6.4,
                },
                BillingLine {
                    count: 1,
                    description: "Kürbiskernöl".to_string(),
                    total: 0.5,
                },
            ],
        },
        BillingEntry {
            id: "v-77".to_string(),
            epoch_ms: 1_794_222_000_000,
            source: EntrySource::Ventopay,
            description: "Kaffeeautomat Foyer".to_string(),
            amount: -1.2,
            items: Vec::new(),
        },
        BillingEntry {
            id: "g-1002".to_string(),
            epoch_ms: 1_794_135_600_000,
            source: EntrySource::Gourmet,
            description: String::new(),
            amount: 6.4,
            items: vec![BillingLine {
                count: 1,
                description: "Schnitzel mit Kartoffelsalat".to_string(),
                total: 6.4,
            }],
        },
    ]
}

/// A shell's own horizontal scroll viewport: the frame is exactly `width`
/// wide, so a compact shell never stretches to an expanded shell's width and
/// the gallery window can scroll each shell independently.
fn desktop_shell_frame(width: f32, shell: AnyElement) -> AnyElement {
    div()
        .w_full()
        .overflow_x_scrollbar()
        .child(div().w(px(width)).flex_shrink_0().child(shell))
        .into_any_element()
}

/// One shell at `width`: the model's geometry, the rail and a split workspace.
fn desktop_shell(
    width: f32,
    theme: &AppTheme,
    cx: &mut Context<GalleryRoot>,
    caption: &'static str,
) -> AnyElement {
    let geometry = desktop_geometry(HostKind::Desktop, width);
    let active = Tab::Orders;
    let select: TabSelect = Rc::new(|_, _, _| {});
    let refresh: ClickHandler = Rc::new(|_, _| {});
    let detail_pad = if width >= 1024. { 24. } else { 16. };

    let master = div()
        .flex()
        .flex_col()
        .child(record_table::record_header(
            vec!["Datum", "Bestellung", "Status"],
            theme,
        ))
        .child(record_table::record_row(
            "gallery-order-1",
            vec![
                record_table::record_cell("10.11.2026", theme),
                record_table::record_cell_body("MENÜ I", theme),
                record_table::record_status_cell("Bestätigt", theme.success, theme),
            ],
            true,
            None,
            theme,
        ))
        .child(record_table::record_row(
            "gallery-order-2",
            vec![
                record_table::record_cell("11.11.2026", theme),
                record_table::record_cell_body("MENÜ II", theme),
                record_table::record_status_cell("Ausstehend", theme.warning, theme),
            ],
            false,
            None,
            theme,
        ));

    let detail = div()
        .p(px(theme.scale.space_4.max(detail_pad)))
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child("Dienstag, 10. November 2026"),
        )
        .child(
            div()
                .text_size(px(theme.scale.headline))
                .line_height(px(theme.scale.headline_line))
                .font_semibold()
                .text_color(theme.text_primary)
                .child("MENÜ I"),
        )
        .child(
            div()
                .text_size(px(theme.scale.body))
                .line_height(px(theme.scale.body_line))
                .text_color(theme.text_primary)
                .child("Rindsgulasch mit Serviettenknödel und Salat"),
        )
        .child(button::desktop_action(
            format!("gallery-cancel-{width}"),
            "Stornieren",
            button::ButtonKind::Danger,
            theme,
            cx,
            |_, _, _| {},
        ));

    let panel = div()
        .w(px(geometry.workspace_width))
        .h(px(360.))
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(theme.surface)
        .border_1()
        .border_color(theme.border)
        .child(workspace::workspace_header(
            active.title(),
            Some(refresh),
            theme,
        ))
        .child(workspace::split_workspace(
            master.into_any_element(),
            detail.into_any_element(),
            &geometry,
            theme,
        ));

    div()
        .w(px(width))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_1))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child(caption),
        )
        .child(
            div()
                .w(px(width))
                .flex()
                .flex_row()
                .h(px(360.))
                .child(desktop_navigation::desktop_rail(
                    active,
                    geometry.rail_width,
                    None,
                    theme,
                    select,
                ))
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .justify_center()
                        .bg(theme.surface)
                        .child(panel),
                ),
        )
        .into_any_element()
}

/// Every shared control on the desktop recipe, forced through the five states
/// so a screenshot shows rest, hover, focus, disabled and selected side by
/// side. Each ring is painted inside a reserved 4 px frame, so none clips.
fn desktop_controls(
    theme: &AppTheme,
    dark: bool,
    fields: &Vec<Entity<InputState>>,
    cx: &mut Context<GalleryRoot>,
) -> AnyElement {
    const STATES: [ControlState; 5] = [
        ControlState::Rest,
        ControlState::Hover,
        ControlState::Focus,
        ControlState::Disabled,
        ControlState::Selected,
    ];
    let caption = |text: &'static str| {
        div()
            .text_size(px(theme.scale.meta))
            .line_height(px(theme.scale.meta_line))
            .font_semibold()
            .text_color(theme.text_secondary)
            .child(text)
    };
    let state_label = |state: ControlState| {
        let text = match state {
            ControlState::Rest => "Ruhe",
            ControlState::Hover => "Hover",
            ControlState::Focus => "Fokus",
            ControlState::Disabled => "Deaktiviert",
            ControlState::Selected => "Ausgewählt",
        };
        div()
            .text_size(px(theme.scale.meta))
            .line_height(px(theme.scale.meta_line))
            .text_color(theme.text_secondary)
            .child(text)
    };
    // One row per control: five forced states, each captioned underneath.
    let state_row = |render: &dyn Fn(ControlState) -> AnyElement| -> AnyElement {
        div()
            .flex()
            .flex_wrap()
            .items_start()
            .gap(px(theme.scale.space_2))
            .children(STATES.into_iter().map(|state| {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(theme.scale.space_1))
                    .child(render(state))
                    .child(state_label(state))
                    .into_any_element()
            }))
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_3))
        .child(caption("Aktion · 48 px, Radius 10, ohne Schatten"))
        .child(state_row(&|state| {
            button::desktop_action_state(
                format!("gallery-action-{dark}-{state:?}"),
                "Bestellen",
                button::ButtonKind::Primary,
                state,
                theme,
                cx,
                |_, _, _| {},
            )
            .into_any_element()
        }))
        .child(caption(
            "Filter · 44 px, 12 px Innenabstand, Haken bei Auswahl",
        ))
        .child(state_row(&|state| {
            chip::desktop_chip(None, "Kantine", state, theme).into_any_element()
        }))
        .child(caption("Eingabe · 44 px, surface + border_input"))
        .child(state_row(&|state| {
            text_field::desktop_field_with_state(&fields[state_index(state)], state, theme)
                .w(px(160.))
                .into_any_element()
        }))
        .child(caption(
            "Zeile · 40 px, Surface, Hover 4 % Text, Auswahl Accent-Surface",
        ))
        .child(state_row(&|state| {
            div()
                .min_h(px(snackpilot_model::desktop::ROW_HEIGHT))
                .flex()
                .items_center()
                .px(px(theme.scale.space_3))
                .bg(if state == ControlState::Hover {
                    theme.hover(theme.surface)
                } else {
                    theme.control_fill(state)
                })
                .border_1()
                .border_color(if state == ControlState::Selected {
                    theme.accent
                } else {
                    theme.border
                })
                .text_color(if state == ControlState::Selected {
                    theme.accent
                } else {
                    theme.text_primary
                })
                .child(
                    div()
                        .text_size(px(theme.scale.body))
                        .line_height(px(theme.scale.body_line))
                        .font_semibold()
                        .child("Belege"),
                )
                .into_any_element()
        }))
        .child(caption(
            "Toast · 300/4000/300 ms, Surface + Hairline, 16 px Innenabstand",
        ))
        .child(
            div()
                .relative()
                .h(px(96.))
                .w_full()
                .overflow_hidden()
                .child(toast::desktop_toast(
                    &toast::Toast {
                        message: "Neue Menüs verfügbar!".into(),
                        phase: toast::ToastPhase::Hold,
                    },
                    8.,
                    theme,
                )),
        )
        .child(caption("Lade-Overlay · Oberfläche über der ganzen Shell"))
        .child(
            div()
                .relative()
                .h(px(160.))
                .w_full()
                .overflow_hidden()
                .child(loading_overlay::desktop_loading_overlay(theme)),
        )
        .into_any_element()
}

/// Index of a forced state in the `STATES` order used by the gallery rows.
fn state_index(state: ControlState) -> usize {
    match state {
        ControlState::Rest => 0,
        ControlState::Hover => 1,
        ControlState::Focus => 2,
        ControlState::Disabled => 3,
        ControlState::Selected => 4,
    }
}

/// Compact Day Tray component state board (REQ-UI-15/16): every state the
/// shared controls can show, for all five accents in the requested appearance,
/// on the host's dialect. It is a component board only — never proof that a
/// running screen or a native input works.
fn mobile_state_board(dark: bool, cx: &mut Context<GalleryRoot>) -> AnyElement {
    use gpui_kit::component::{Icon, Sizable};

    let appearance: SharedString = if dark { "Dunkel" } else { "Hell" }.into();
    let mut board = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(snackpilot_model::tokens::SPACE_4))
        .p(px(snackpilot_model::tokens::SPACE_4))
        .bg(AppTheme::build(dark).background)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(snackpilot_model::tokens::SPACE_2))
                .child(
                    div()
                        .text_size(px(20.))
                        .font_semibold()
                        .text_color(AppTheme::build(dark).text_primary)
                        .child(format!("Komponenten · {appearance}")),
                ),
        );

    {
        let theme = AppTheme::build(dark);
        let label = "fest";
        let card = theme.surface;
        let focus_room = theme.scale.focus_width + theme.scale.focus_gap;
        board = board.child(
            div()
                .flex()
                .flex_col()
                .gap(px(theme.scale.space_2))
                .p(px(theme.scale.space_3))
                .rounded(px(theme.scale.radius_chip))
                .bg(card)
                .border(px(1.))
                .border_color(theme.border)
                .child(
                    div()
                        .text_size(px(theme.scale.meta))
                        .text_color(theme.text_secondary)
                        .child(format!("Akzent: {label}")),
                )
                // Primary and danger actions, plus the disabled variant (whose
                // fill swaps and whose callback is absent).
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(theme.scale.space_2))
                        .child(button::mobile_action(
                            format!("mb-primary-{label}-{dark}"),
                            "Bestellen",
                            button::ButtonKind::Primary,
                            true,
                            &theme,
                            cx,
                        ))
                        .child(button::mobile_action(
                            format!("mb-danger-{label}-{dark}"),
                            "Stornieren",
                            button::ButtonKind::Danger,
                            true,
                            &theme,
                            cx,
                        ))
                        .child(button::mobile_action(
                            format!("mb-disabled-{label}-{dark}"),
                            "Geschlossen",
                            button::ButtonKind::Primary,
                            false,
                            &theme,
                            cx,
                        )),
                )
                // Pending selection (tint + check + boundary), unselected
                // option, selected filter and the semantic statuses.
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(theme.scale.space_2))
                        .child(chip::chip("Vorgemerkt", true, &theme))
                        .child(chip::chip("Verfügbar", false, &theme))
                        .child(chip::status_tag(
                            "Bestellt",
                            chip::StatusTone::Success,
                            &theme,
                        ))
                        .child(chip::status_tag(
                            "Geschlossen",
                            chip::StatusTone::Warning,
                            &theme,
                        ))
                        .child(chip::status_tag(
                            "Ausverkauft",
                            chip::StatusTone::Error,
                            &theme,
                        )),
                )
                // Navigation: active and inactive destinations, and the
                // independent focus ring in its reserved 2 + 2 room.
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(theme.scale.space_3))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(theme.scale.space_2))
                                .child(
                                    Icon::new(crate::components::tab_bar::tab_icon(Tab::Menus))
                                        .with_size(px(theme.scale.icon))
                                        .text_color(theme.accent),
                                )
                                .child(
                                    div()
                                        .text_size(px(theme.scale.meta))
                                        .text_color(theme.accent)
                                        .child("Menüs (aktiv)"),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(theme.scale.space_2))
                                .child(
                                    Icon::new(crate::components::tab_bar::tab_icon(Tab::Orders))
                                        .with_size(px(24.))
                                        .text_color(theme.text_secondary),
                                )
                                .child(
                                    div()
                                        .text_size(px(theme.scale.meta))
                                        .text_color(theme.text_secondary)
                                        .child("Bestellungen (inaktiv)"),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(theme.scale.space_2))
                        .child(
                            div()
                                .p(px(theme.scale.focus_gap))
                                .rounded(px(theme.scale.radius_chip + focus_room))
                                .border(px(theme.scale.focus_width))
                                .border_color(theme.accent)
                                .child(
                                    div()
                                        .min_h(px(theme.scale.target))
                                        .flex()
                                        .items_center()
                                        .px(px(theme.scale.space_3))
                                        .rounded(px(theme.scale.radius_chip))
                                        .bg(card)
                                        .border(px(1.))
                                        .border_color(theme.border)
                                        .child(
                                            div()
                                                .text_size(px(theme.scale.meta))
                                                .text_color(theme.text_primary)
                                                .child("Fokus (2 + 2 reserviert)"),
                                        ),
                                ),
                        ),
                ),
        );
    }

    // Task 1.3's Menüs board: the compact navigator, cards and dock at the
    // 360 px content width, with the Orange appearance the loop ends on.
    let menus_theme = AppTheme::build(dark);
    board = board.child(mobile_menus_case(328., &menus_theme));
    board = board.child(mobile_orders_case(328., &menus_theme, cx));
    board = board.child(mobile_billing_case(328., &menus_theme));

    board.into_any_element()
}

/// Offline Menüs board: the real compact day row, rows and dock over static
/// items, so the status words (Geöffnet/Reserviert/Bestellt/Stornierung
/// offen/Ausgebucht), the wrapping dish and allergens, the trailing 24 px check
/// and the 48 px dock are visible without a host or a request. It is a
/// component board, not proof of a running screen.
/// @req REQ-SCR-01 @req REQ-UI-15
fn mobile_menus_case(width: f32, theme: &AppTheme) -> AnyElement {
    let noop: ClickHandler = Rc::new(|_, _| {});
    let dates: Vec<String> = ["2026-11-16", "2026-11-17", "2026-11-18"]
        .into_iter()
        .map(str::to_string)
        .collect();
    let mut rows = gallery_menu_rows();
    // A sold-out dish shows the neutral/error chip the menu rows can carry.
    rows.push((
        MenuItem {
            id: "g6".to_string(),
            day: "2026-11-11".to_string(),
            title: "MENÜ III".to_string(),
            subtitle: "Gemüsecurry mit Basmatireis".to_string(),
            allergens: vec!["A".into()],
            available: false,
            ordered: false,
            category: MenuCategory::Menu3,
            price: "6,40 €".to_string(),
        },
        DesktopMenuRowState::default(),
    ));

    let mut list = div().w_full().flex().flex_col().pt(px(theme.scale.space_4));
    for (item, row) in rows {
        let state = MenuCardState {
            selected: row.pending_order || row.selected,
            pending_cancel: row.pending_cancel,
            ordered: row.ordered,
            available: row.available,
            cutoff: row.cutoff,
        };
        list = list.child(menu_card(&item, state, Some(noop.clone()), theme));
    }
    // The quiet states: one centred metadata line and the refresh spinner.
    list = list
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(theme.scale.space_2))
                .py(px(theme.scale.space_4))
                .child(spinner::spinner(theme.text_secondary, theme.scale.icon))
                .child(
                    div()
                        .text_size(px(theme.scale.meta))
                        .line_height(px(theme.scale.meta_line))
                        .text_color(theme.text_secondary)
                        .child("Aktualisiere..."),
                ),
        )
        .child(
            div()
                .py(px(theme.scale.space_4))
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .text_center()
                .child("Keine Menüs verfügbar"),
        );

    let column = div()
        .w(px(width))
        .flex()
        .flex_col()
        .bg(theme.background)
        .child(mobile_day_navigator(
            &dates,
            Some("2026-11-17"),
            theme,
            noop.clone(),
            noop.clone(),
            noop.clone(),
        ))
        .child(list)
        .child(div().p(px(theme.scale.space_2)).child(mobile_dock(
            DockState::Action("Änderungen bestätigen (3)".into()),
            // The board's label is one body line; the running screen measures
            // it from the shaped label instead (fab::dock_label_lines).
            dock_height_for_lines(1, theme),
            Some(noop),
            theme,
        )));

    div()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .font_semibold()
                .text_color(theme.text_secondary)
                .child(format!("Menüs · {width} px Inhalt · Zeile, Status, Dock")),
        )
        .child(column)
        .into_any_element()
}

/// Offline Bestellungen board for task 1.4 (order scenarios only): the real
/// compact segment bar, confirm banner and cards over static orders, so the
/// confirmed/unconfirmed/cancelling/cutoff/read-only states, the longest dish
/// and the batch checks are visible without a host or a request. It is a
/// component board, not proof of a running screen.
/// @req REQ-SCR-02 @req REQ-UI-15
fn mobile_orders_case(width: f32, theme: &AppTheme, cx: &mut Context<GalleryRoot>) -> AnyElement {
    let noop: ClickHandler = Rc::new(|_, _| {});
    let select: SegmentHandler = Rc::new(|_, _, _| {});
    let order = |id: &str, title: &str, subtitle: &str, approved: bool| OrderedMenu {
        position_id: id.to_string(),
        eating_cycle_id: "cycle-2026-11-11".to_string(),
        date_epoch_ms: 1_794_308_400_000,
        title: title.to_string(),
        subtitle: subtitle.to_string(),
        approved,
    };
    let long = order(
        "b-long",
        "MENÜ I",
        "Rindsgulasch mit Serviettenknödel, Rahmsauce, Blattsalat mit Kürbiskernöl",
        false,
    );
    let confirmed = order("b-ok", "MENÜ II", "Pasta mit Tomatensugo", true);
    let cancelling = order("b-cancel", "MENÜ III", "Gemüsecurry mit Basmatireis", true);
    let cutoff = order("b-cutoff", "MENÜ I", "Rindsgulasch", false);
    let past = order("b-past", "MENÜ II", "Pasta mit Tomatensugo", true);
    let checked = order("b-checked", "MENÜ III", "Gemüsecurry mit Basmatireis", true);

    let mut list = div().w_full().flex().flex_col();
    // Confirmed, the longest dish unconfirmed, and the in-flight cancellation.
    for (row, dish, can_cancel, in_flight, past_cutoff, select, selected, tap) in [
        (&confirmed, None, true, false, false, false, false, false),
        (&long, None, true, false, false, false, false, false),
        (&cancelling, None, true, true, false, false, false, false),
        (&cutoff, None, true, false, true, false, false, false),
        (&past, None, false, false, false, false, false, false),
        (&checked, None, true, false, false, true, true, true),
    ] {
        list = list.child(order_item(
            row,
            dish,
            can_cancel,
            in_flight,
            past_cutoff,
            select,
            selected,
            noop.clone(),
            tap.then(|| noop.clone()),
            theme,
        ));
    }

    // The counted selection strip at the bottom of the list.
    let strip = div()
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(theme.scale.space_2))
        .p(px(theme.scale.space_4))
        .bg(theme.surface)
        .border_t_1()
        .border_color(theme.border)
        .child(button::mobile_action(
            "board-orders-cancel",
            "Stornieren (2)",
            button::ButtonKind::Danger,
            true,
            theme,
            cx,
        ))
        .child(button::mobile_action(
            "board-orders-done",
            "Fertig",
            button::ButtonKind::Secondary,
            true,
            theme,
            cx,
        ));

    let column = div()
        .w(px(width))
        .flex()
        .flex_col()
        .bg(theme.background)
        .child(segment_tabs(Segment::Upcoming, 12, 9, select, theme))
        .child(confirm_banner(
            "2 unbestätigte Bestellungen",
            false,
            noop.clone(),
            theme,
        ))
        .child(list)
        .child(strip);

    div()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .font_semibold()
                .text_color(theme.text_secondary)
                .child(format!(
                    "Bestellungen · {width} px Inhalt · Segmente, Banner, Zeilen, Streifen"
                )),
        )
        .child(column)
        .into_any_element()
}

/// Offline Abrechnung board for task 1.5 (billing scenarios only): the real
/// compact month choices, source options, summary surface and receipt cards
/// over static receipts, including a long Kantine description, a large credit,
/// the Automaten restaurant with and without text, and the empty text. It is a
/// component board, not proof of a running screen.
/// @req REQ-SCR-03 @req REQ-UI-15
fn mobile_billing_case(width: f32, theme: &AppTheme) -> AnyElement {
    let month: MonthSelect = Rc::new(|_, _, _| {});
    let source: SourceSelect = Rc::new(|_, _, _| {});
    let mut entries = gallery_receipts();
    // An Automaten transaction without a restaurant line: the header alone.
    entries.push(BillingEntry {
        id: "v-78".to_string(),
        epoch_ms: 1_794_135_600_000,
        source: EntrySource::Ventopay,
        description: String::new(),
        amount: -999.99,
        items: Vec::new(),
    });
    let totals = BillingTotals {
        gesamt: entries.iter().map(|entry| entry.amount).sum(),
        belege: entries.len(),
        zuschuss: 1.25,
    };
    let options: Vec<MonthOption> = ["2026-11", "2026-10", "2026-09"]
        .into_iter()
        .enumerate()
        .map(|(offset, key)| MonthOption {
            key: key.to_string(),
            label: format!("{} 2026", ["November", "Oktober", "September"][offset]),
            offset: offset as u8,
        })
        .collect();

    let mut cards = div().w_full().flex().flex_col();
    for entry in &entries {
        cards = cards.child(bill_card(entry, theme));
    }

    let column = div()
        .w(px(width))
        .flex()
        .flex_col()
        .bg(theme.background)
        .child(mobile_month_row(&options, 0, month, theme))
        .child(mobile_source_row(BillingSource::All, source, theme))
        .child(mobile_summary_surface(
            &entries,
            &totals,
            BillingSource::All,
            theme,
        ))
        .child(cards)
        .child(
            div().p(px(theme.scale.space_4)).child(
                div()
                    .text_size(px(theme.scale.body))
                    .line_height(px(theme.scale.body_line))
                    .text_color(theme.text_secondary)
                    .child("Keine Abrechnungsdaten für diesen Monat"),
            ),
        );

    div()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .font_semibold()
                .text_color(theme.text_secondary)
                .child(format!(
                    "Abrechnung · {width} px Inhalt · Monate, Quellen, Summe, Belege"
                )),
        )
        .child(column)
        .into_any_element()
}

impl Render for GalleryRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut col = div().size_full().flex().flex_col().overflow_y_scrollbar();
        match self.page {
            2 => match self.which {
                Section::Light => {
                    col = col.child(desktop_section(false, &self.control_fields[0], cx))
                }
                Section::Dark => {
                    col = col.child(desktop_section(true, &self.control_fields[1], cx))
                }
                Section::Both => {
                    col = col
                        .child(desktop_section(false, &self.control_fields[0], cx))
                        .child(desktop_section(true, &self.control_fields[1], cx))
                }
            },
            4 | 5 | 6 => {
                let theme = AppTheme::build(matches!(self.which, Section::Dark));
                let width = 328.;
                col = col.child(match self.page {
                    4 => mobile_menus_case(width, &theme),
                    5 => mobile_orders_case(width, &theme, cx),
                    _ => mobile_billing_case(width, &theme),
                });
            }
            _ => match self.which {
                Section::Light => col = col.child(mobile_state_board(false, cx)),
                Section::Dark => col = col.child(mobile_state_board(true, cx)),
                Section::Both => {
                    col = col
                        .child(mobile_state_board(false, cx))
                        .child(mobile_state_board(true, cx))
                }
            },
        }
        col
    }
}

/// Open the gallery as the app's root content (debug hook in `crate::mount` and
/// `crate::mount_desktop`).
pub fn mount_gallery(cx: &mut App, which: Section) {
    let bounds = gallery_window_size().map(|(w, h)| {
        gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds::new(
            gpui_kit::point(gpui_kit::px(60.), gpui_kit::px(60.)),
            gpui_kit::size(w, h),
        ))
    });
    cx.open_window(
        gpui_kit::WindowOptions {
            window_bounds: bounds,
            ..Default::default()
        },
        |window, cx| {
            let content = cx.new(|cx| GalleryRoot::new(window, cx, which));
            cx.new(|cx| crate::Root::new(content, window, cx))
        },
    )
    .expect("open gallery window");
    cx.activate(true);
}

/// Debug-only window size for screenshot capture at specific widths:
/// `SNACKPILOT_WINDOW=390x844` places the window at a fixed origin so
/// `screencapture -R` can grab exactly it. The evidence viewport knob
/// (`--viewport=390x844`) selects the same logical size, so a Day Tray
/// baseline and its component boards share one viewport fact.
#[cfg(debug_assertions)]
fn gallery_window_size() -> Option<(gpui_kit::Pixels, gpui_kit::Pixels)> {
    if let Some(viewport) = crate::evidence::viewport() {
        return Some((
            gpui_kit::px(viewport.width as f32),
            gpui_kit::px(viewport.height as f32),
        ));
    }
    let s = std::env::var("SNACKPILOT_WINDOW").ok()?;
    let (w, h) = s.split_once('x')?;
    Some((gpui_kit::px(w.parse().ok()?), gpui_kit::px(h.parse().ok()?)))
}

#[cfg(not(debug_assertions))]
fn gallery_window_size() -> Option<(gpui_kit::Pixels, gpui_kit::Pixels)> {
    None
}
