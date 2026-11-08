//! Component gallery (task 4.1, REQ-UI-05/REQ-UI-06): every §4 primitive in
//! German, rendered twice — once on the light `AppTheme`, once on the dark
//! one — so a screenshot proves the §7 recipes and the umlaut rendering in
//! the bundled Inter fonts. Opened by the debug hook in `mount` when
//! `SNACKPILOT_GALLERY=1` (the desktop binary is task 4.2's file).

use gpui_kit::assets::IconName;
use gpui_kit::component::input::InputState;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{Icon, StyledExt};
use gpui_kit::prelude::*;
use gpui_kit::{div, px, App, Context, Entity, SharedString, Window};
use snackpilot_model::AccentColor;

use crate::components::{banner, button, card, chip, spinner, switch, text_field};
use crate::theme::{use_flat_style, AppTheme};

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
}

impl GalleryRoot {
    pub fn new(window: &mut Window, cx: &mut App, which: Section) -> Self {
        let flat = use_flat_style();
        let light = AppTheme::build(false, AccentColor::Orange, flat);
        let dark = AppTheme::build(true, AccentColor::Orange, flat);
        let (light_field, _) = text_field::text_field("Suchen", None, &light, window, cx);
        let (dark_field, _) = text_field::text_field("Suchen", None, &dark, window, cx);
        let page = match crate::gallery_page().as_deref() {
            Some("bottom") | Some("1") => 1,
            _ => 0,
        };
        Self {
            which,
            dark_switch_on: false,
            page,
            light_field,
            dark_field,
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
                    )),
            );
        if self.page != 1 {
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

impl Render for GalleryRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut col = div().size_full().flex().flex_col().overflow_y_scrollbar();
        match self.which {
            Section::Light => col = col.child(self.section(false, &self.light_field, cx)),
            Section::Dark => col = col.child(self.section(true, &self.dark_field, cx)),
            Section::Both => {
                col = col
                    .child(self.section(false, &self.light_field, cx))
                    .child(self.section(true, &self.dark_field, cx))
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
