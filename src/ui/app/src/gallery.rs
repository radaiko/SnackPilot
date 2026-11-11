//! Component gallery (task 4.1, REQ-UI-05/REQ-UI-06): every §4 primitive in
//! German, rendered twice — once on the light `AppTheme`, once on the dark
//! one — so a screenshot proves the §7 recipes and the umlaut rendering in
//! the bundled Inter fonts. Opened by the debug hook in `mount` when
//! `SNACKPILOT_GALLERY=1` (the desktop binary is task 4.2's file).

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::InputState;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{Icon, StyledExt};
use gpui_kit::prelude::*;
use gpui_kit::{div, px, AnyElement, App, Context, Entity, SharedString, Window};
use snackpilot_core::domain::{MenuCategory, MenuItem, OrderedMenu};
use snackpilot_model::desktop::{desktop_geometry, ControlState, HostKind};
use snackpilot_model::navigation::Tab;
use snackpilot_model::{
    AccentColor, BillingEntry, BillingLine, BillingSource, BillingTotals, EntrySource,
};

use crate::app::OrderCancelState;
use crate::components::bill_card::{desktop_bill_detail, desktop_bill_header, desktop_bill_row};
use crate::components::menu_card::{desktop_menu_row, DesktopMenuRowState};
use crate::components::order_item::{
    desktop_order_detail, desktop_order_header, desktop_order_row, DesktopOrderRowState,
};
use crate::components::tab_bar::TabSelect;
use crate::components::{
    banner, button, card, chip, desktop_navigation, loading_overlay, record_table, spinner, switch,
    text_field, toast, workspace,
};
use crate::navigation::ClickHandler;
use crate::screens::billing::desktop_summary_line;
use crate::screens::menus::{desktop_menu_detail, desktop_submit_action};
use crate::theme::{self, use_flat_style, AppTheme};

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
    dark_switch_on: bool,
    /// 0 = everything stacked; 1 = only "Bestellung & Status" (the lower
    /// half). Lets a screenshot capture the lower sections on one screen.
    page: usize,
    light_field: Entity<InputState>,
    dark_field: Entity<InputState>,
    /// Five independent desktop field states per appearance, so the desktop
    /// page can paint rest/hover/focus/disabled/selected in one frame.
    control_fields: [Vec<Entity<InputState>>; 2],
}

impl GalleryRoot {
    pub fn new(window: &mut Window, cx: &mut App, which: Section) -> Self {
        let flat = use_flat_style();
        let light = AppTheme::build(false, AccentColor::Orange, flat);
        let dark = AppTheme::build(true, AccentColor::Orange, flat);
        let (light_field, _) = text_field::text_field("Suchen", None, &light, window, cx);
        let (dark_field, _) = text_field::text_field("Suchen", None, &dark, window, cx);
        let mut fields = |theme: &AppTheme| -> Vec<Entity<InputState>> {
            (0..5)
                .map(|_| text_field::text_field("Suchen", None, theme, window, cx).0)
                .collect()
        };
        let control_fields = [fields(&light), fields(&dark)];
        let page = match crate::gallery_page().as_deref() {
            Some("bottom") | Some("1") => 1,
            Some("desktop") | Some("2") => 2,
            _ => 0,
        };
        Self {
            which,
            dark_switch_on: false,
            page,
            light_field,
            dark_field,
            control_fields,
        }
    }

    /// One appearance section: every §4 primitive on the resolved theme's
    /// background, German copy from 04 §4 verbatim.
    fn section(
        &self,
        dark: bool,
        field: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = AppTheme::build(dark, AccentColor::Orange, use_flat_style());
        let entity = cx.entity().clone();
        let title: SharedString = if dark { "Dunkel" } else { "Hell" }.into();
        let subtitle: SharedString = if self.page == 1 {
            "Komponenten · Seite 2".into()
        } else {
            "Komponenten".into()
        };
        let page_entity = cx.entity().clone();

        let mut sec = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(12.))
            .p_4()
            .bg(theme.background)
            // Section header with the §4 theme label.
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(20.))
                            .font_semibold()
                            .text_color(theme.text_primary)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.text_secondary)
                            .child(subtitle),
                    ),
            )
            // Page switch (always visible): captures the lower half on one
            // screenshot without scrolling.
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.))
                    .child(button::button_action(
                        format!("page0-{dark}"),
                        "Oben",
                        button::ButtonKind::Secondary,
                        &theme,
                        cx,
                        {
                            let page_e = page_entity.clone();
                            move |_, _window, cx| {
                                page_e.update(cx, |this, cx| {
                                    this.page = 0;
                                    cx.notify();
                                });
                            }
                        },
                    ))
                    .child(button::button_action(
                        format!("page1-{dark}"),
                        "Bestellung & Status",
                        button::ButtonKind::Secondary,
                        &theme,
                        cx,
                        {
                            let page_e = page_entity.clone();
                            move |_, _window, cx| {
                                page_e.update(cx, |this, cx| {
                                    this.page = 1;
                                    cx.notify();
                                });
                            }
                        },
                    ))
                    .child(button::button_action(
                        format!("page2-{dark}"),
                        "Desktop",
                        button::ButtonKind::Secondary,
                        &theme,
                        cx,
                        {
                            let page_e = page_entity.clone();
                            move |_, _window, cx| {
                                page_e.update(cx, |this, cx| {
                                    this.page = 2;
                                    cx.notify();
                                });
                            }
                        },
                    )),
            );
        if self.page == 0 {
            sec = sec
                // Filter chips with the §2 tab icons (Lucide substitutions).
                .child(chip::chip_row(
                    [
                        chip::icon_chip(Icon::new(IconName::Utensils), "Menüs", true, &theme),
                        chip::icon_chip(
                            Icon::new(IconName::Receipt),
                            "Bestellungen",
                            false,
                            &theme,
                        ),
                        chip::icon_chip(Icon::new(IconName::Wallet), "Abrechnung", false, &theme),
                        chip::icon_chip(
                            Icon::new(IconName::Settings),
                            "Einstellungen",
                            false,
                            &theme,
                        ),
                    ]
                    .into_iter(),
                ))
                .child(card::divider(&theme))
                // Buttons (04 §4/§7).
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(8.))
                        .child(button::button(
                            format!("primary-{dark}"),
                            "Bestellen",
                            button::ButtonKind::Primary,
                            &theme,
                            cx,
                        ))
                        .child(button::button(
                            format!("secondary-{dark}"),
                            "Abbrechen",
                            button::ButtonKind::Secondary,
                            &theme,
                            cx,
                        ))
                        .child(button::button(
                            format!("danger-{dark}"),
                            "Stornieren",
                            button::ButtonKind::Danger,
                            &theme,
                            cx,
                        ))
                        .child(button::fab(format!("fab-{dark}"), "+", &theme, cx))
                        .child(button::circle_button(
                            format!("circle-{dark}"),
                            Icon::new(IconName::ChevronLeft),
                            &theme,
                            cx,
                        )),
                )
                // Text field with the §7 inputField recipe.
                .child(text_field::field_with_state(field, &theme))
                // Cards and empty states (04 §4 German strings verbatim).
                .child(card::titled_card(
                    "Menüs",
                    "Heutiger Speiseplan · Preise in €",
                    &theme,
                ))
                .child(card::card(
                    &theme,
                    [
                        card::empty_state("Keine kommenden Bestellungen", false, &theme)
                            .into_any_element(),
                        card::empty_state("Keine vergangenen Bestellungen", false, &theme)
                            .into_any_element(),
                        card::empty_state("Keine Abrechnungsdaten für diesen Monat", false, &theme)
                            .into_any_element(),
                    ],
                ))
                .child(card::empty_state("Keine Menüs verfügbar", false, &theme))
                // Loading rows (04 §4).
                .child(spinner::loading_row(
                    "Wird in den Warenkorb gelegt...",
                    &theme,
                ))
                .child(spinner::loading_row("Aktualisiere...", &theme))
                .child(spinner::loading_row("Wird storniert", &theme));
        }
        sec = sec
            // Status tags (04 §4 order/billing states).
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(8.))
                    .child(chip::status_tag(
                        "Bestätigt",
                        chip::StatusTone::Success,
                        &theme,
                    ))
                    .child(chip::status_tag(
                        "Ausstehend",
                        chip::StatusTone::Warning,
                        &theme,
                    ))
                    .child(chip::status_tag(
                        "Storniert",
                        chip::StatusTone::Error,
                        &theme,
                    )),
            )
            // Switch: the dark section's switch toggles itself.
            .child(switch::switch(
                format!("switch-{dark}"),
                if dark { "Dunkel" } else { "Hell" },
                if dark { self.dark_switch_on } else { true },
                &theme,
                move |value, _window, cx| {
                    let value = *value;
                    if dark {
                        entity.update(cx, |this, cx| {
                            this.dark_switch_on = value;
                            cx.notify();
                        });
                    }
                },
            ))
            // Banners (04 §4).
            .child(banner::refresh_banner(&theme))
            .child(banner::error_banner("Anmeldung erforderlich", &theme))
            .child(banner::banner(
                "Nicht angemeldet",
                banner::BannerTone::Neutral,
                None,
                &theme,
            ))
            .child(banner::banner(
                "Bestellung wird bestätigt",
                banner::BannerTone::Info,
                Some(Icon::new(IconName::Info)),
                &theme,
            ))
            .child(banner::surface_card(
                [
                    spinner::spinner(theme.text_secondary, 22.).into_any_element(),
                    div()
                        .text_size(px(13.))
                        .text_color(theme.text_secondary)
                        .child("Wird geladen...")
                        .into_any_element(),
                ],
                &theme,
            ));
        sec
    }
}

/// The desktop gallery page: the shell at both desktop widths with the
/// labelled rail, then every shared control forced through rest/hover/focus/
/// disabled/selected. Light and dark are rendered by the caller exactly like
/// the mobile pages, and every focus ring sits in its reserved 4 px room.
fn desktop_section(
    dark: bool,
    fields: &Vec<Entity<InputState>>,
    cx: &mut Context<GalleryRoot>,
) -> AnyElement {
    let theme = AppTheme::build(dark, AccentColor::Orange, use_flat_style());
    let title: SharedString = if dark {
        "Desktop · Dunkel".into()
    } else {
        "Desktop · Hell".into()
    };
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(12.))
        .p_4()
        .bg(theme.background)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(20.))
                        .font_semibold()
                        .text_color(theme.text_primary)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.text_secondary)
                        .child("SnackPilot Desktop Ledger · Kompakt/Erweitert"),
                ),
        )
        .child(desktop_settings_dialog_demo(&theme, cx))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .font_semibold()
                .text_color(theme.text_secondary)
                .child("Shell · Rail 176/192 px · Panel · Master/Detail"),
        )
        .child(desktop_shell_frame(
            720.,
            desktop_shell(720., &theme, cx, "Kompakt · 720"),
        ))
        .child(desktop_shell_frame(
            1280.,
            desktop_shell(1280., &theme, cx, "Erweitert · 1280"),
        ))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .font_semibold()
                .text_color(theme.text_secondary)
                .child("Menüs · Vorschau, ausdrückliche Aktion, alle Tage"),
        )
        .child(desktop_shell_frame(
            720.,
            desktop_menus_case(720., &theme, cx),
        ))
        .child(desktop_shell_frame(
            1280.,
            desktop_menus_case(1280., &theme, cx),
        ))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .font_semibold()
                .text_color(theme.text_secondary)
                .child("Bestellungen · Tabelle, Detail, Mehrfachauswahl"),
        )
        .child(desktop_shell_frame(
            720.,
            desktop_orders_case(720., &theme, cx),
        ))
        .child(desktop_shell_frame(
            1280.,
            desktop_orders_case(1280., &theme, cx),
        ))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .font_semibold()
                .text_color(theme.text_secondary)
                .child("Abrechnung · Belegtabelle, Detail, Quellen"),
        )
        .child(desktop_shell_frame(
            720.,
            desktop_billing_case(720., &theme),
        ))
        .child(desktop_shell_frame(
            1280.,
            desktop_billing_case(1280., &theme),
        ))
        .child(desktop_controls(&theme, dark, fields, cx))
        .into_any_element()
}

/// The offline Menüs scenario (task 1.3): the real row and detail helpers over
/// static items, so preview/pending/ordered/unavailable/cutoff are visible
/// without a host, a request or a credential.
fn desktop_menus_case(width: f32, theme: &AppTheme, cx: &mut Context<GalleryRoot>) -> AnyElement {
    let geometry = desktop_geometry(HostKind::Desktop, width);
    let noop: ClickHandler = Rc::new(|_, _| {});
    let select: TabSelect = Rc::new(|_, _, _| {});
    let rows = gallery_menu_rows();

    let mut master = div().flex().flex_col().child(
        div()
            .px(px(16.))
            .pt(px(16.))
            .pb(px(8.))
            .text_size(px(14.))
            .line_height(px(20.))
            .font_semibold()
            .text_color(theme.desktop.ink)
            .child("MENÜ I"),
    );
    for (item, state) in &rows {
        master = master.child(desktop_menu_row(item, *state, noop.clone(), theme));
    }

    let (item, state) = &rows[0];
    let detail = desktop_menu_detail(item, *state, false, noop.clone(), theme, width, cx);
    let strip = workspace::action_strip(
        vec![div()
            .flex()
            .w_full()
            .justify_end()
            .child(desktop_submit_action(
                "Änderungen bestätigen (3)".to_string(),
                theme,
                noop.clone(),
            ))
            .into_any_element()],
        theme,
    );

    let panel = theme::apply(
        div()
            .w(px(geometry.workspace_width))
            .h(px(400.))
            .flex()
            .flex_col()
            .overflow_hidden(),
        &theme.workspace_panel(),
    )
    .child(workspace::workspace_header(
        Tab::Menus.title(),
        Some(noop.clone()),
        theme,
    ))
    .child(
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(workspace::split_workspace(
                master.into_any_element(),
                detail,
                &geometry,
                theme,
            ))
            .child(strip),
    );

    div()
        .w(px(width))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(theme.text_secondary)
                .child(format!("Menüs · {width} px")),
        )
        .child(
            div()
                .w(px(width))
                .flex()
                .flex_row()
                .h(px(400.))
                .child(desktop_navigation::desktop_rail(
                    Tab::Menus,
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
                        .child(panel),
                ),
        )
        .into_any_element()
}

/// Offline proof for task 1.5 that needs no host: the 720 px settings
/// containment (master + credential form, no back row), its disabled/focused
/// action states, and the desktop confirm dialog over the whole shell.
/// @req REQ-SCR-04 @req REQ-UI-14
fn desktop_settings_dialog_demo(
    theme: &crate::theme::AppTheme,
    cx: &mut Context<GalleryRoot>,
) -> AnyElement {
    use crate::components::dialog::{dialog_overlay, Dialog, DialogActivate};
    use crate::components::settings_row::settings_row_desktop;
    use snackpilot_model::desktop::{desktop_geometry, ControlState, HostKind};

    let noop: crate::navigation::ClickHandler = Rc::new(|_, _| {});
    let geometry = desktop_geometry(HostKind::Desktop, 720.);
    let master = {
        let mut list = div().w_full().flex().flex_col().p(px(8.)).gap(px(4.));
        for (index, (id, title, hint, selected)) in [
            (
                "gallery-settings-gourmet",
                "Kantine-Zugangsdaten",
                "Angemeldet als demo",
                true,
            ),
            (
                "gallery-settings-ventopay",
                "Automaten-Zugangsdaten",
                "Nicht angemeldet",
                false,
            ),
            (
                "gallery-settings-appearance",
                "Darstellung",
                "System",
                false,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let focus = cx.focus_handle();
            list = list.child(settings_row_desktop(
                id,
                title,
                Some(hint.to_string()),
                selected,
                &focus,
                noop.clone(),
                theme,
            ));
            let _ = index;
        }
        list.child(
            div()
                .id("gallery-settings-privacy")
                .min_h(px(44.))
                .px(px(12.))
                .flex()
                .items_center()
                .tab_index(0)
                .cursor_pointer()
                .focus(crate::theme::desktop_focus(theme.desktop.focus))
                .on_click({
                    let noop = noop.clone();
                    move |_, window, cx| noop(window, cx)
                })
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(theme.text_secondary)
                        .child("Datenschutz"),
                ),
        )
        .into_any_element()
    };
    let field = |label: &'static str, focus: bool| {
        div()
            .w_full()
            .min_h(px(44.))
            .px(px(12.))
            .flex()
            .items_center()
            .rounded(px(8.))
            .bg(theme.surface)
            .border(px(1.))
            .border_color(if focus {
                theme.desktop.focus
            } else {
                theme.border
            })
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(if focus {
                        theme.desktop.ink
                    } else {
                        theme.text_secondary
                    })
                    .child(label),
            )
    };
    let form = div()
        .w_full()
        .max_w(px(440.))
        .flex()
        .flex_col()
        .p(px(16.))
        .gap(px(16.))
        .child(
            div()
                .text_size(px(20.))
                .line_height(px(26.))
                .font_semibold()
                .text_color(theme.desktop.ink)
                .child("Kantine-Zugangsdaten"),
        )
        .child(field("Benutzername", true))
        .child(field("Passwort", false))
        .child(button::desktop_action_state(
            "gallery-settings-save",
            "Speichern",
            button::ButtonKind::Primary,
            ControlState::Rest,
            theme,
            cx,
            {
                let noop = noop.clone();
                move |_, window, cx| noop(window, cx)
            },
        ))
        .child(button::desktop_action_state(
            "gallery-settings-save-busy",
            "Speichern...",
            button::ButtonKind::Primary,
            ControlState::Disabled,
            theme,
            cx,
            {
                let noop = noop.clone();
                move |_, window, cx| noop(window, cx)
            },
        ))
        .into_any_element();
    let split = div()
        .w(px(geometry.workspace_width))
        .h(px(320.))
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(workspace::workspace_header("Einstellungen", None, theme))
        .child(workspace::split_workspace(
            master,
            div().p(px(16.)).child(form).into_any_element(),
            &geometry,
            theme,
        ))
        .into_any_element();

    let f0 = cx.focus_handle();
    let f1 = cx.focus_handle();
    let dialog = Dialog::Confirm {
        title: "Bestellung stornieren".into(),
        message: "\"MENÜ I\" stornieren?".into(),
        confirm_label: Some("Stornieren"),
        cancel_label: Some("Behalten"),
        on_confirm: noop.clone(),
    };
    let activate: DialogActivate = Rc::new(|_, _, _| {});
    let overlay = dialog_overlay(&dialog, theme, 1, &[f0, f1], activate);

    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(theme.text_secondary)
                .child("Einstellungen 720 · containment, form, focus/disabled, desktop dialog"),
        )
        .child(
            div()
                .relative()
                .w(px(geometry.workspace_width))
                .flex()
                .child(split)
                .child(overlay),
        )
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

/// The offline Bestellungen scenario (task 1.4): the real header, row and
/// detail helpers over static orders, so the preview cursor and the batch
/// check are visibly separate without a host or a request.
fn desktop_orders_case(width: f32, theme: &AppTheme, cx: &mut Context<GalleryRoot>) -> AnyElement {
    let geometry = desktop_geometry(HostKind::Desktop, width);
    let compact = width < 1024.;
    let noop: ClickHandler = Rc::new(|_, _| {});
    let orders = gallery_orders();

    let mut master = div()
        .flex()
        .flex_col()
        .child(desktop_order_header(compact, true, theme));
    for (order, description, row) in &orders {
        master = master.child(desktop_order_row(
            order,
            description,
            *row,
            compact,
            noop.clone(),
            Some(noop.clone()),
            theme,
        ));
    }

    let (order, description, _) = &orders[0];
    let detail = desktop_order_detail(
        order,
        description,
        OrderCancelState::Ready,
        noop.clone(),
        theme,
        cx,
    );
    let strip = workspace::action_strip(
        vec![div()
            .flex()
            .w_full()
            .flex_wrap()
            .justify_end()
            .gap(px(8.))
            .child(button::desktop_action_state(
                "gallery-orders-cancel-selected",
                "Stornieren (2)",
                button::ButtonKind::Danger,
                ControlState::Rest,
                theme,
                cx,
                |_, _, _| {},
            ))
            .child(button::desktop_action_state(
                "gallery-orders-done",
                "Fertig",
                button::ButtonKind::Secondary,
                ControlState::Rest,
                theme,
                cx,
                |_, _, _| {},
            ))
            .into_any_element()],
        theme,
    );

    let panel = theme::apply(
        div()
            .w(px(geometry.workspace_width))
            .h(px(400.))
            .flex()
            .flex_col()
            .overflow_hidden(),
        &theme.workspace_panel(),
    )
    .child(workspace::workspace_header(
        Tab::Orders.title(),
        Some(noop.clone()),
        theme,
    ))
    .child(
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(workspace::split_workspace(
                master.into_any_element(),
                detail,
                &geometry,
                theme,
            ))
            .child(strip),
    );

    let select: TabSelect = Rc::new(|_, _, _| {});
    div()
        .w(px(width))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(theme.text_secondary)
                .child(format!("Bestellungen · {width} px")),
        )
        .child(
            div()
                .w(px(width))
                .flex()
                .flex_row()
                .h(px(400.))
                .child(desktop_navigation::desktop_rail(
                    Tab::Orders,
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
                        .child(panel),
                ),
        )
        .into_any_element()
}

fn gallery_orders() -> Vec<(OrderedMenu, String, DesktopOrderRowState)> {
    let order = |id: &str, title: &str, subtitle: &str, approved: bool| OrderedMenu {
        position_id: id.to_string(),
        eating_cycle_id: "cycle-2026-11-11".to_string(),
        date_epoch_ms: 1_794_308_400_000,
        title: title.to_string(),
        subtitle: subtitle.to_string(),
        approved,
    };
    vec![
        (
            order("o1", "MENÜ I", "Rindsgulasch", true),
            "Rindsgulasch mit Serviettenknödel, Rahmsauce und Blattsalat".to_string(),
            DesktopOrderRowState {
                selected: true,
                checked: true,
                cancelling: false,
            },
        ),
        (
            order("o2", "MENÜ II", "Pasta", false),
            "Pasta mit Tomatensugo und Parmesan".to_string(),
            DesktopOrderRowState {
                selected: false,
                checked: true,
                cancelling: false,
            },
        ),
        (
            order("o3", "MENÜ III", "Gemüsecurry", true),
            "Gemüsecurry mit Basmatireis".to_string(),
            DesktopOrderRowState {
                selected: false,
                checked: false,
                cancelling: true,
            },
        ),
    ]
}

/// The offline Abrechnung scenario (task 1.4): the real header, row, summary
/// and detail helpers over static receipts from both sources.
fn desktop_billing_case(width: f32, theme: &AppTheme) -> AnyElement {
    let geometry = desktop_geometry(HostKind::Desktop, width);
    let compact = width < 1024.;
    let noop: ClickHandler = Rc::new(|_, _| {});
    let entries = gallery_receipts();
    let totals = BillingTotals {
        gesamt: entries.iter().map(|entry| entry.amount).sum(),
        belege: entries.len(),
        zuschuss: 0.,
    };

    let mut master = div()
        .flex()
        .flex_col()
        .child(desktop_bill_header(compact, theme));
    for entry in &entries {
        master = master.child(desktop_bill_row(
            entry,
            entry.id == "g-1001",
            compact,
            noop.clone(),
            theme,
        ));
    }
    let detail = desktop_bill_detail(&entries[0], width < 1024., theme);
    let summary = desktop_summary_line(&entries, &totals, BillingSource::All, theme);

    let panel = theme::apply(
        div()
            .w(px(geometry.workspace_width))
            .h(px(400.))
            .flex()
            .flex_col()
            .overflow_hidden(),
        &theme.workspace_panel(),
    )
    .child(workspace::workspace_header(
        Tab::Billing.title(),
        Some(noop.clone()),
        theme,
    ))
    .child(summary)
    .child(
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(workspace::split_workspace(
                master.into_any_element(),
                detail,
                &geometry,
                theme,
            )),
    );

    let select: TabSelect = Rc::new(|_, _, _| {});
    div()
        .w(px(width))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(theme.text_secondary)
                .child(format!("Abrechnung · {width} px")),
        )
        .child(
            div()
                .w(px(width))
                .flex()
                .flex_row()
                .h(px(400.))
                .child(desktop_navigation::desktop_rail(
                    Tab::Billing,
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
                        .child(panel),
                ),
        )
        .into_any_element()
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
                record_table::record_cell("MENÜ I", theme),
                record_table::record_cell("Bestätigt", theme),
            ],
            true,
            None,
            theme,
        ))
        .child(record_table::record_row(
            "gallery-order-2",
            vec![
                record_table::record_cell("11.11.2026", theme),
                record_table::record_cell("MENÜ II", theme),
                record_table::record_cell("Ausstehend", theme),
            ],
            false,
            None,
            theme,
        ));

    let detail = div()
        .p(px(detail_pad))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(theme.text_secondary)
                .child("Dienstag, 10. November 2026"),
        )
        .child(
            div()
                .text_size(px(20.))
                .line_height(px(26.))
                .font_semibold()
                .text_color(theme.desktop.ink)
                .child("MENÜ I"),
        )
        .child(
            div()
                .text_size(px(16.))
                .line_height(px(24.))
                .text_color(theme.desktop.ink)
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

    let panel = theme::apply(
        div()
            .w(px(geometry.workspace_width))
            .h(px(360.))
            .flex()
            .flex_col()
            .overflow_hidden(),
        &theme.workspace_panel(),
    )
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
        .gap(px(4.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
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
            .text_size(px(12.))
            .line_height(px(16.))
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
            .text_size(px(12.))
            .line_height(px(16.))
            .text_color(theme.text_secondary)
            .child(text)
    };
    // One row per control: five forced states, each captioned underneath.
    let state_row = |render: &dyn Fn(ControlState) -> AnyElement| -> AnyElement {
        div()
            .flex()
            .flex_wrap()
            .items_start()
            .gap(px(8.))
            .children(STATES.into_iter().map(|state| {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(2.))
                    .child(render(state))
                    .child(state_label(state))
                    .into_any_element()
            }))
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(caption("Aktion · 44 px, Radius 8, ohne Schatten"))
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
            "Ledger-Karte · panelSurface, 1 px Rand, ohne Schatten",
        ))
        .child(state_row(&|state| {
            card::desktop_card(
                theme,
                state,
                [div()
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_semibold()
                    .text_color(theme.desktop.ink)
                    .child("Belege")
                    .into_any_element()],
            )
            .into_any_element()
        }))
        .child(caption(
            "Toast · 300/4000/300 ms, Panel-Rezept, 16 px Innenabstand",
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
        .child(caption(
            "Lade-Overlay · overlay über der ganzen Shell, Panel-Karte",
        ))
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

impl Render for GalleryRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut col = div().size_full().flex().flex_col().overflow_y_scrollbar();
        if self.page == 2 {
            match self.which {
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
            }
        } else {
            match self.which {
                Section::Light => col = col.child(self.section(false, &self.light_field, cx)),
                Section::Dark => col = col.child(self.section(true, &self.dark_field, cx)),
                Section::Both => {
                    col = col
                        .child(self.section(false, &self.light_field, cx))
                        .child(self.section(true, &self.dark_field, cx))
                }
            }
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
/// `screencapture -R` can grab exactly it.
#[cfg(debug_assertions)]
fn gallery_window_size() -> Option<(gpui_kit::Pixels, gpui_kit::Pixels)> {
    let s = std::env::var("SNACKPILOT_WINDOW").ok()?;
    let (w, h) = s.split_once('x')?;
    Some((gpui_kit::px(w.parse().ok()?), gpui_kit::px(h.parse().ok()?)))
}

#[cfg(not(debug_assertions))]
fn gallery_window_size() -> Option<(gpui_kit::Pixels, gpui_kit::Pixels)> {
    None
}
