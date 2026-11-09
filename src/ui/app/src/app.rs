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
use gpui_kit::component::input::InputState;
#[cfg(target_os = "ios")]
use gpui_kit::component::WindowExt;
use snackpilot_model::navigation::{Navigation, Screen, Tab};
use snackpilot_model::{format_day_short, AppModel, State};

use crate::components::broadcast_banner::broadcast_banner;
use crate::components::day_navigator::day_navigator;
use crate::components::dialog::{dialog_overlay, Dialog};
use crate::components::loading_overlay::loading_overlay;
use crate::components::tab_bar::{reserved_height, tab_bar};
use crate::components::toast::{toast_overlay, Toast, ToastPhase, HOLD_DURATION, SLIDE_DURATION};
use crate::screens::appearance::appearance_screen;
use crate::screens::billing::billing_tab;
use crate::screens::login::{login_screen, LoginKind};
use crate::screens::menus::menus_tab;
use crate::screens::notifications::notifications_screen;
use crate::screens::orders::orders_tab;
use crate::screens::settings::{placeholder_detail, settings_tab};
use crate::theme::AppTheme;

/// The pushed settings screens (one detail at a time under
/// `Screen::SettingsDetail`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Detail {
    Settings,
    LoginKantine,
    LoginAutomaten,
    Appearance,
    Notifications,
}

/// iOS keyboard driver decision: `Some(true)` show, `Some(false)` hide,
/// `None` no change (§3.6 typing proof, task 5.4).
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
fn keyboard_action(up: bool, focused: bool) -> Option<bool> {
    (focused != up).then_some(focused)
}

/// Debug knob (`--log-kb`): appends keyboard-driver transitions to the app's
/// temp dir so the UI test can tell whether the driver or UIKit dropped the
/// keyboard (§3.6 typing proof, task 5.4). Read on the host via
/// `xcrun simctl get_app_container <udid> <bundle> data`/tmp/sp54-kb.log.
pub(crate) fn kb_log(line: &str) {
    #[cfg(target_os = "ios")]
    if std::env::args().any(|a| a == "--log-kb") {
        use std::io::Write;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("sp54-kb.log"))
        {
            let _ = writeln!(f, "{stamp} {line}");
        }
    }
    #[cfg(not(target_os = "ios"))]
    let _ = line;
}

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
    pub(crate) model: Option<Arc<AppModel>>,
    /// Transient swipe-between-days gesture state (§3.1): pointer origin and
    /// the current (damped) horizontal offset.
    pub(crate) swipe_start: Option<f32>,
    pub(crate) swipe_start_y: Option<f32>,
    pub(crate) swipe_claimed: bool,
    pub(crate) swipe_dx: f32,
    /// Orders tab: true shows the past segment (UI-only).
    pub(crate) orders_past: bool,
    /// Which pushed settings screen is shown.
    pub(crate) detail: Detail,
    /// Login form fields, created on push so typing survives re-renders.
    login_user: Option<Entity<InputState>>,
    login_pass: Option<Entity<InputState>>,
    /// Debug knob (`--detail=login|automaten`): opens that credential screen
    /// on the first frame so a UI test can reach it without navigation.
    pending_detail: Option<Detail>,
    /// iOS: whether the soft keyboard is currently shown (§3.6, task 5.4).
    #[cfg_attr(not(target_os = "ios"), allow(dead_code))]
    keyboard_up: bool,
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
            model,
            swipe_start: None,
            swipe_start_y: None,
            swipe_claimed: false,
            swipe_dx: 0.,
            orders_past: false,
            detail: Detail::Settings,
            login_user: None,
            login_pass: None,
            pending_detail: None,
            keyboard_up: false,
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
            // `--detail=login|automaten` opens a credential screen directly.
            shell.pending_detail = std::env::args().find_map(|a| {
                let rest = a.strip_prefix("--detail=")?;
                Some(match rest {
                    "login" => Detail::LoginKantine,
                    "automaten" => Detail::LoginAutomaten,
                    "appearance" => Detail::Appearance,
                    "notifications" => Detail::Notifications,
                    _ => return None,
                })
            });
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

    /// Push a settings detail screen; login details create fresh inputs.
    pub(crate) fn open_detail(
        &mut self,
        detail: Detail,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.detail = detail;
        if detail == Detail::Notifications {
            if let Some(model) = &self.model {
                model.refresh_diagnostic_log();
            }
        }
        if matches!(detail, Detail::LoginKantine | Detail::LoginAutomaten) {
            // §3.5: both fields pre-fill from the stored credentials.
            let (username, password) = self
                .model
                .as_ref()
                .map(|m| m.saved_credentials(detail == Detail::LoginAutomaten))
                .unwrap_or_default();
            self.login_user = Some(cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Benutzername eingeben")
                    .default_value(username.unwrap_or_default())
            }));
            self.login_pass = Some(cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Passwort eingeben")
                    // §3.6 secure entry: the field renders dots, never the text.
                    .masked(true)
                    .default_value(password.unwrap_or_default())
            }));
        }
        self.nav.push(Screen::SettingsDetail);
        cx.notify();
    }

    /// Pop back to the settings root.
    pub(crate) fn pop_detail(&mut self, cx: &mut Context<Self>) {
        self.nav.pop();
        self.detail = Detail::Settings;
        cx.notify();
    }

    pub(crate) fn set_dialog(&mut self, dialog: Dialog, cx: &mut Context<Self>) {
        self.dialog = Some(dialog);
        // iOS: an alert must not sit behind the soft keyboard (§4.1).
        #[cfg(target_os = "ios")]
        {
            gpui_mobile::hide_keyboard();
            self.keyboard_up = false;
        }
        cx.notify();
    }

    /// iOS soft-keyboard driver (§3.6, task 5.4): gpui-mobile creates the
    /// native text input view hidden, so nothing raises the keyboard on its
    /// own. gpui re-runs `render` on every repaint, so polling the focused
    /// input here reacts to a tap-to-focus (focus change ⇒ repaint) and keeps
    /// the keyboard in sync without patching the library.
    fn sync_keyboard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(target_os = "ios")]
        if self.dialog.is_none() {
            let focused = window.has_focused_input(cx);
            if let Some(show) = keyboard_action(self.keyboard_up, focused) {
                kb_log(&format!(
                    "driver {} (focused={focused} was_up={})",
                    if show { "show" } else { "hide" },
                    self.keyboard_up
                ));
                self.keyboard_up = show;
                if show {
                    gpui_mobile::show_keyboard();
                } else {
                    gpui_mobile::hide_keyboard();
                }
            }
        }
        #[cfg(not(target_os = "ios"))]
        {
            let _ = (&self, window, cx);
        }
    }
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
            Screen::Settings => return settings_tab(state, theme, cx),
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
            return match (self.detail, &self.login_user, &self.login_pass) {
                (Detail::LoginKantine, Some(user), Some(pass)) => login_screen(
                    LoginKind::Kantine,
                    state,
                    self.model.as_ref(),
                    user,
                    pass,
                    theme,
                    cx,
                ),
                (Detail::LoginAutomaten, Some(user), Some(pass)) => login_screen(
                    LoginKind::Automaten,
                    state,
                    self.model.as_ref(),
                    user,
                    pass,
                    theme,
                    cx,
                ),
                (Detail::Appearance, _, _) => {
                    appearance_screen(state, self.model.as_ref(), theme, cx)
                }
                (Detail::Notifications, _, _) => {
                    notifications_screen(state, self.model.as_ref(), theme, cx)
                }
                (detail, _, _) => placeholder_detail(detail, theme, cx),
            };
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
                Screen::Billing => billing_tab(state, self.model.as_ref(), theme, cx),
                _ => self.placeholder(state, theme, cx),
            })
            .into_any_element()
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(detail) = self.pending_detail.take() {
            self.open_detail(detail, window, cx);
        }
        self.sync_keyboard(window, cx);
        if self.dialog.is_none() {
            if let Some(alert) = self.model.as_ref().and_then(|m| m.take_location_alert()) {
                self.dialog = Some(Dialog::Alert {
                    title: alert.title.into(),
                    message: Some(alert.message.into()),
                });
            }
        }
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

        let mut banner = None;
        if let Some(message) = state
            .broadcast
            .as_ref()
            .filter(|_| !state.broadcast_dismissed)
        {
            let entity = cx.entity().clone();
            let dismiss = Rc::new(move |_w: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |this, cx| {
                    if let Some(model) = &this.model {
                        model.dismiss_broadcast();
                    }
                    cx.notify();
                });
            });
            banner = Some(broadcast_banner(message, dismiss, &theme));
        }
        let screen_content = self.screen(&state, &theme, cx);
        let mut content = div()
            .flex_1()
            .flex()
            .flex_col()
            .overflow_hidden()
            .pt(px(safe_top));
        if let Some(banner) = banner {
            content = content.child(banner);
        }
        let mut content = content.child(screen_content);
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

#[cfg(test)]
mod tests {
    use super::keyboard_action;

    /// The soft keyboard follows focus changes and stays put otherwise
    /// (§3.6 typing proof; the iOS side is proven by the XCUITest run).
    /// @req REQ-SCR-06
    #[test]
    fn keyboard_follows_focus_changes() {
        assert_eq!(keyboard_action(false, true), Some(true)); // tap focuses a field
        assert_eq!(keyboard_action(true, false), Some(false)); // focus lost
        assert_eq!(keyboard_action(true, true), None); // unchanged
        assert_eq!(keyboard_action(false, false), None); // unchanged
    }
}
