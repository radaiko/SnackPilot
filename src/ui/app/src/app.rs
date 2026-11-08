//! App shell — docs/requirements/04-ui-ux.md §2 (tab bar), §3.4 (settings),
//! §3.5 (back affordance), §4.1 (dialogs), §4.2 (loading), §4.8 (toast).
//!
//! One `Root` (mounted by `crate::mount`) hosts this view: the four-tab shell
//! over a push/pop stack from [`snackpilot_model::navigation`], with the
//! overlays painted last (gpui has no `z_index`). Safe-area insets come from
//! the host (gpui-mobile), the desktop has none.
//! @req REQ-UI-03 @req REQ-UI-04 @req REQ-UI-07

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use snackpilot_model::navigation::{Navigation, Screen, Tab};
use snackpilot_model::{format_day_short, AppModel, State};

use crate::components::day_navigator::day_navigator;
use crate::components::dialog::{dialog_overlay, Dialog};
use crate::components::loading_overlay::loading_overlay;
use crate::components::tab_bar::{reserved_height, tab_bar};
use crate::components::toast::{toast_overlay, Toast, ToastPhase, HOLD_DURATION, SLIDE_DURATION};
use crate::navigation::settings_detail_screen;
use crate::screens::menus::menus_tab;
use crate::screens::orders::orders_tab;
use crate::theme::AppTheme;

/// §3.4 Datenschutz alert message, verbatim.
const DATENSCHUTZ_MESSAGE: &str = "Diese App erfasst anonyme Nutzungsstatistiken zur Verbesserung der Benutzererfahrung. Die Analyse erfolgt über TelemetryDeck — einen datenschutzfreundlichen, cookielosen Dienst. Es werden keine persönlichen Daten, Passwörter, Menüauswahl oder Abrechnungsdaten erfasst.";

/// Safe-area insets (top, bottom, left, right) in logical px, read from the
/// host platform. Android asks the live window; iOS reads the FFI window list
/// with the v1 notch/home-indicator fallback (59/34) before the host reports
/// real insets; desktop has none.
fn safe_area() -> (f32, f32, f32, f32) {
    #[cfg(target_os = "android")]
    {
        if let Some(platform) = gpui_mobile::android::jni::platform() {
            if let Some(window) = platform.primary_window() {
                let insets = window.safe_area_insets_logical();
                return (insets.top, insets.bottom, insets.left, insets.right);
            }
        }
    }
    let (top, bottom, left, right) = gpui_mobile::safe_area_insets();
    if top == 0. && bottom == 0. && cfg!(target_os = "ios") {
        return (59., 34., 0., 0.);
    }
    (top, bottom, left, right)
}

/// The single root view: four tabs, one push/pop stack, dialogs, loading
/// overlay and the new-menu toast.
/// @req REQ-UI-04
pub struct AppShell {
    nav: Navigation,
    dialog: Option<Dialog>,
    toast: Option<Toast>,
    /// Title of the pushed `SettingsDetail` screen, set when a settings row
    /// pushes it (tasks 5.x replace the placeholder body).
    detail_title: &'static str,
    pub(crate) model: Option<Arc<AppModel>>,
    /// Transient swipe-between-days gesture state (§3.1): pointer origin and
    /// the current (damped) horizontal offset.
    pub(crate) swipe_start: Option<f32>,
    pub(crate) swipe_start_y: Option<f32>,
    pub(crate) swipe_claimed: bool,
    pub(crate) swipe_dx: f32,
    /// Orders tab: true shows the past segment (UI-only).
    pub(crate) orders_past: bool,
}

impl AppShell {
    /// Wrap the model (or nothing, e.g. the iOS host before its session is
    /// exposed) and subscribe to state changes: each notification wakes the
    /// 50 ms poll loop, which repaints the shell.
    pub fn new(model: Option<Arc<AppModel>>, cx: &mut Context<Self>) -> Self {
        if let Some(model) = model.as_ref() {
            // The subscriber must be Send + Sync, so it only flips a flag;
            // a 50 ms foreground poll repaints when it is set.
            let dirty = Arc::new(AtomicBool::new(false));
            model.subscribe(Box::new({
                let dirty = Arc::clone(&dirty);
                move |_| dirty.store(true, Ordering::Release)
            }));
            cx.spawn(
                async move |this: WeakEntity<AppShell>, cx: &mut AsyncApp| loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(50))
                        .await;
                    if dirty.swap(false, Ordering::AcqRel)
                        && this.update(cx, |_, cx| cx.notify()).is_err()
                    {
                        break;
                    }
                },
            )
            .detach();
        }
        // `mut` is only used by the debug-only evidence knobs below.
        #[cfg_attr(not(debug_assertions), allow(unused_mut))]
        let mut shell = AppShell {
            nav: Navigation::new(),
            dialog: None,
            toast: None,
            detail_title: "",
            model,
            swipe_start: None,
            swipe_start_y: None,
            swipe_claimed: false,
            swipe_dx: 0.,
            orders_past: false,
        };
        // Evidence knobs (debug builds only, same entry as 4.1's --gallery):
        // `--dialog` opens a §4.1 confirm, `--toast` shows the §4.8 toast,
        // `--tab=N` selects a tab (simulator captures cannot tap).
        #[cfg(debug_assertions)]
        {
            // `--demo` signs in with the product's magic demo credentials so
            // simulator captures show the real screens offline (task 5.1
            // evidence; the login UI itself is 5.4).
            if std::env::args().any(|a| a == "--demo") {
                if let Some(model) = &shell.model {
                    let result = model.login("demo", "demo1234!");
                    log::info!("[5.1] demo login: {result:?}");
                }
            }
            let tab = std::env::args().find_map(|a| {
                let rest = a.strip_prefix("--tab=")?;
                rest.parse::<usize>().ok()
            });
            if let Some(n) = tab.filter(|n| *n < Tab::ALL.len()) {
                shell.nav.select_tab(Tab::ALL[n]);
            }
            if std::env::args().any(|a| a == "--dialog") {
                shell.set_dialog(
                    Dialog::Confirm {
                        title: "Abmelden?".into(),
                        message: "Möchtest du dich wirklich abmelden?".into(),
                        confirm_label: None,
                        cancel_label: None,
                        on_confirm: Rc::new(|_, _| {}),
                    },
                    cx,
                );
            }
            if std::env::args().any(|a| a == "--toast") {
                shell.show_toast(cx);
            }
        }
        shell
    }

    /// Show the §4.8 new-menu toast; the phase state machine
    /// (In → Hold → Out → gone) runs on chained background timers.
    /// The menus task wires the trigger once new-menu detection lands.
    #[allow(dead_code)]
    pub fn show_toast(&mut self, cx: &mut Context<Self>) {
        self.toast = Some(Toast {
            message: "Neue Menüs verfügbar!".into(),
            phase: ToastPhase::In,
        });
        cx.notify();
        cx.spawn(async move |this: WeakEntity<AppShell>, cx: &mut AsyncApp| {
            for phase in [ToastPhase::Hold, ToastPhase::Out] {
                let duration = match phase {
                    ToastPhase::Hold => SLIDE_DURATION,
                    _ => HOLD_DURATION,
                };
                cx.background_executor().timer(duration).await;
                let _ = this.update(cx, |this, cx| {
                    if let Some(toast) = this.toast.as_mut() {
                        toast.phase = phase;
                    }
                    cx.notify();
                });
            }
            cx.background_executor().timer(SLIDE_DURATION).await;
            let _ = this.update(cx, |this, cx| {
                this.toast = None;
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn set_dialog(&mut self, dialog: Dialog, cx: &mut Context<Self>) {
        self.dialog = Some(dialog);
        cx.notify();
    }

    /// §3.4 settings rows: title 18/600, hint 13/tertiary, trailing chevron 20.
    fn settings_screen(
        &self,
        state: &State,
        theme: &AppTheme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let gourmet_hint = match &state.user_info {
            Some(user) if state.gourmet_authenticated => {
                format!("Angemeldet als {}", user.username)
            }
            _ => "Nicht angemeldet".to_string(),
        };
        let ventopay_hint = if state.ventopay_authenticated {
            "Sitzung aktiv"
        } else {
            "Nicht angemeldet"
        };
        let theme_hint = match state.theme {
            snackpilot_model::ThemePreference::System => "System",
            snackpilot_model::ThemePreference::Light => "Hell",
            snackpilot_model::ThemePreference::Dark => "Dunkel",
        };
        let rows: [(&'static str, String); 4] = [
            ("Kantine-Zugangsdaten", gourmet_hint),
            ("Automaten-Zugangsdaten", ventopay_hint.to_string()),
            ("Darstellung", theme_hint.to_string()),
            (
                "Benachrichtigungen",
                "Erinnerungen und Standort-Benachrichtigungen".to_string(),
            ),
        ];

        let mut list = div().flex_1().flex().flex_col().p(px(20.)).pb(px(100.));
        for (index, (title, hint)) in rows.iter().enumerate() {
            let title = *title;
            let hint = hint.clone();
            if index > 0 {
                // §3.4: 1 px dividers, marginVertical 24.
                list = list.child(div().h(px(1.)).my(px(24.)).bg(theme.divider()));
            }
            let entity = cx.entity().clone();
            let on_tap = Rc::new(move |_window: &mut Window, cx: &mut App| {
                entity.update(cx, |this, cx| {
                    this.detail_title = title;
                    this.nav.push(Screen::SettingsDetail);
                    cx.notify();
                });
            });
            list = list.child(
                div()
                    .id(SharedString::from(format!("settings-{title}")))
                    .flex()
                    .items_center()
                    .justify_between()
                    .py(px(8.))
                    .cursor_pointer()
                    .on_click(move |_, window, cx| on_tap(window, cx))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(18.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.text_primary)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .mt(px(2.))
                                    .text_size(px(13.))
                                    .text_color(theme.text_tertiary)
                                    .child(hint),
                            ),
                    )
                    .child(
                        Icon::new(IconName::ChevronRight)
                            .with_size(px(20.))
                            .text_color(theme.text_tertiary),
                    ),
            );
        }
        let datenschutz = cx.entity().clone();
        let on_datenschutz = Rc::new(move |_window: &mut Window, cx: &mut App| {
            datenschutz.update(cx, |this, cx| {
                this.set_dialog(
                    Dialog::Alert {
                        title: "Datenschutz".into(),
                        message: Some(DATENSCHUTZ_MESSAGE.into()),
                    },
                    cx,
                );
            });
        });
        list.child(
            div()
                .id("datenschutz")
                .w_full()
                .flex()
                .justify_center()
                .py(px(16.))
                .cursor_pointer()
                .on_click(move |_, window, cx| on_datenschutz(window, cx))
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(theme.text_tertiary)
                        .child("Datenschutz"),
                ),
        )
        .into_any_element()
    }

    /// One tab's placeholder content: page title plus a line from the model
    /// state. Tasks 5.x replace these with the real screens.
    fn placeholder(&self, state: &State, theme: &AppTheme, cx: &mut Context<Self>) -> AnyElement {
        let (title, hint): (&str, String) = match self.nav.top() {
            Screen::Menus => match &state.menu_snapshot {
                Some(snapshot) if !snapshot.available_dates.is_empty() => {
                    let day = state
                        .selected_day
                        .as_deref()
                        .or(snapshot.available_dates.first().map(String::as_str))
                        .map(format_day_short)
                        .unwrap_or_default();
                    (
                        "Menüs",
                        format!("{} Menüs am {}", snapshot.items.len(), day),
                    )
                }
                _ => ("Menüs", "Keine Menüs verfügbar".to_string()),
            },
            Screen::Orders => match &state.orders_split {
                Some(split) => (
                    "Bestellungen",
                    format!(
                        "{} bevorstehende, {} vergangene Bestellungen",
                        split.upcoming.len(),
                        split.past.len()
                    ),
                ),
                None => ("Bestellungen", "Keine Bestellungen".to_string()),
            },
            Screen::Billing => {
                if state.billing_loading {
                    ("Abrechnung", "Wird geladen…".to_string())
                } else if state.gourmet_month.is_some() || state.ventopay_month.is_some() {
                    ("Abrechnung", "Daten verfügbar".to_string())
                } else {
                    ("Abrechnung", "Keine Abrechnungsdaten".to_string())
                }
            }
            Screen::Settings => return self.settings_screen(state, theme, cx),
            Screen::SettingsDetail => unreachable!("handled before placeholders"),
        };
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(28.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child(title.to_string()),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(theme.text_secondary)
                    .child(hint),
            )
            .into_any_element()
    }

    /// Content of the active screen, including the §4.4 day navigator on the
    /// Menus tab and the §3.5 back affordance on pushed screens.
    fn screen(&self, state: &State, theme: &AppTheme, cx: &mut Context<Self>) -> AnyElement {
        if self.nav.top() == Screen::SettingsDetail {
            let entity = cx.entity().clone();
            let on_back = Rc::new(move |_window: &mut Window, cx: &mut App| {
                entity.update(cx, |this, cx| {
                    this.nav.pop();
                    cx.notify();
                });
            });
            return settings_detail_screen(self.detail_title, theme, on_back);
        }

        let mut column = div().flex_1().flex().flex_col().overflow_hidden();
        if self.nav.top() == Screen::Menus {
            if let (Some(model), Some(snapshot)) = (&self.model, &state.menu_snapshot) {
                if !snapshot.available_dates.is_empty() {
                    let (prev, next, today) =
                        (Arc::clone(model), Arc::clone(model), Arc::clone(model));
                    let (entity_p, entity_n, entity_t) = (
                        cx.entity().clone(),
                        cx.entity().clone(),
                        cx.entity().clone(),
                    );
                    let on_prev = Rc::new(move |_w: &mut Window, cx: &mut App| {
                        prev.prev_day();
                        entity_p.update(cx, |_, cx| cx.notify());
                    });
                    let on_next = Rc::new(move |_w: &mut Window, cx: &mut App| {
                        next.next_day();
                        entity_n.update(cx, |_, cx| cx.notify());
                    });
                    let on_today = Rc::new(move |_w: &mut Window, cx: &mut App| {
                        today.go_to_today();
                        entity_t.update(cx, |_, cx| cx.notify());
                    });
                    column = column.child(day_navigator(
                        &snapshot.available_dates,
                        state.selected_day.as_deref(),
                        theme,
                        on_prev,
                        on_next,
                        on_today,
                    ));
                }
            }
        }
        column
            .child(match self.nav.top() {
                Screen::Menus => menus_tab(state, self.model.as_ref(), theme, self.swipe_dx, cx),
                Screen::Orders => {
                    orders_tab(state, self.model.as_ref(), self.orders_past, theme, cx)
                }
                _ => self.placeholder(state, theme, cx),
            })
            .into_any_element()
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self
            .model
            .as_ref()
            .map(|model| model.state())
            .unwrap_or_default();
        let os_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let theme = AppTheme::resolve(state.theme, state.accent, os_dark);
        let (safe_top, safe_bottom, _, _) = safe_area();

        let entity = cx.entity().clone();
        let on_select = Rc::new(move |tab: Tab, _window: &mut Window, cx: &mut App| {
            log::info!("[4.2] tab selected: {tab:?}");
            entity.update(cx, |this, cx| {
                this.nav.select_tab(tab);
                cx.notify();
            });
        });
        let bar = tab_bar(&theme, self.nav.active_tab(), safe_bottom, on_select);

        let content = self.screen(&state, &theme, cx);
        let mut content = div()
            .flex_1()
            .flex()
            .flex_col()
            .overflow_hidden()
            .pt(px(safe_top))
            .child(content);
        let mut root = div().size_full().flex().flex_col().bg(theme.background);
        if theme.flat {
            // §2.2: in-flow bottom bar; the safe-area strip under it keeps
            // the bar's surface colour (v1 paddingBottom insets.bottom).
            root = root.child(content).child(
                div()
                    .w_full()
                    .bg(theme.surface)
                    .pb(px(safe_bottom))
                    .child(bar),
            );
        } else {
            // §2.1: floating pill, painted over the content it reserves.
            content = content.pb(px(reserved_height(false, safe_bottom)));
            root = root.child(content).child(bar);
        }

        // Overlays stack by paint order (gpui has no z-index): loading, then
        // toast, then dialog on top.
        if state.busy || state.ventopay_busy || state.billing_loading {
            root = root.child(loading_overlay(&theme));
        }
        if let Some(toast) = &self.toast {
            root = root.child(toast_overlay(toast, safe_top, &theme));
        }
        if let Some(dialog) = &self.dialog {
            let entity = cx.entity().clone();
            let on_dismiss = Rc::new(move |_window: &mut Window, cx: &mut App| {
                entity.update(cx, |this, cx| {
                    this.dialog = None;
                    cx.notify();
                });
            });
            root = root.child(dialog_overlay(dialog, &theme, on_dismiss));
        }
        root
    }
}
