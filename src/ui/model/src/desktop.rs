//! Desktop workspace state (desktop-ui-redesign D1/D2): host-gated layout
//! mode, list previews and cursors keyed by stable identities, settings
//! projection, Esc precedence and duplicate-safe keyboard commands.
//!
//! Everything here is gpui-free: the model owns the workspace identities, the
//! shell only renders them and reports host facts (width, host kind, focus).
//! @req REQ-UI-04 @req REQ-UI-07 @req REQ-UI-10 @req REQ-UI-11

use crate::navigation::{ListKind, Screen, SettingsDetail, Tab};
use crate::session::AppModel;
use crate::state::State;

/// What the host process is. Breakpoints are only consulted on Desktop, so a
/// phone or tablet can never fall into a desktop layout by reporting a width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HostKind {
    #[default]
    Phone,
    Tablet,
    Desktop,
}

/// Which workspace the shell renders. Compact and expanded desktop both
/// contain the settings detail; only `Phone` (a phone host, or a desktop
/// narrower than 720) projects it as a pushed screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Phone,
    DesktopCompact,
    DesktopExpanded,
}

/// Pure breakpoint rule (design D1): only a Desktop host can leave `Phone`;
/// then 720 and 1024 px are the compact/expanded gates.
pub fn layout_mode(host: HostKind, width: f32) -> LayoutMode {
    match host {
        HostKind::Phone | HostKind::Tablet => LayoutMode::Phone,
        HostKind::Desktop if width < 720.0 => LayoutMode::Phone,
        HostKind::Desktop if width < 1024.0 => LayoutMode::DesktopCompact,
        HostKind::Desktop => LayoutMode::DesktopExpanded,
    }
}

/// Width of the navigation rail in compact desktop (brief "Responsive geometry").
pub const RAIL_WIDTH_COMPACT: f32 = 176.0;
/// Width of the navigation rail in expanded desktop.
pub const RAIL_WIDTH_EXPANDED: f32 = 192.0;
/// Outer gutter inside the workspace at the compact breakpoint.
pub const OUTER_COMPACT: f32 = 16.0;
/// Outer gutter inside the workspace in expanded desktop.
pub const OUTER_EXPANDED: f32 = 24.0;
/// Fixed master width of the compact two-pane workspace.
pub const MASTER_COMPACT: f32 = 248.0;
/// Divider between master and detail.
pub const DIVIDER_WIDTH: f32 = 1.0;
/// Minimum desktop touch target.
pub const TARGET_MIN: f32 = 44.0;
/// Maximum width of the centered content column.
pub const CONTENT_MAX: f32 = 960.0;
/// Maximum width of the centered phone content column.
pub const PHONE_CONTENT_MAX: f32 = 480.0;
/// Minimum contrast for text and meaningful content (WCAG AA).
pub const TEXT_MIN_CONTRAST: f32 = 4.5;
/// Minimum contrast for focus rings and controls.
pub const CONTROL_MIN_CONTRAST: f32 = 3.0;
/// Focus ring stroke width (brief `desktop.focusWidth`).
pub const FOCUS_WIDTH: f32 = 2.0;
/// Gap between the target and the focus ring (brief `desktop.focusGap`);
/// target plus 2 × gap reserves the 4 px a ring may occupy.
pub const FOCUS_GAP: f32 = 2.0;
/// Hover fill is this much `text_primary` mixed into the rest fill.
pub const ACTION_HOVER_MIX: f32 = 0.08;
/// Pressed fill is this much `text_primary` mixed into the rest fill.
pub const ACTION_PRESSED_MIX: f32 = 0.14;

/// Tile widths the shell lays out at a given host and content width, so the
/// numbers come from the brief's table and not from each screen (REQ-UI-07).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesktopGeometry {
    /// Navigation rail width; 0 on phone.
    pub rail_width: f32,
    /// Gutter between the rail edge and the workspace panel; 0 on phone.
    pub outer_gutter: f32,
    /// Width of the bordered workspace panel.
    pub workspace_width: f32,
    /// Width of the master pane.
    pub master_width: f32,
    /// Width of the divider between master and detail.
    pub divider_width: f32,
    /// Width of the detail pane; 0 on phone.
    pub detail_width: f32,
}

/// Pure layout tiles for a host kind and content width in logical pixels.
/// Below the 720 breakpoint, and on any phone host, there is no rail and the
/// content is the centered phone column. Compact desktop pins the master to
/// the list width; expanded desktop splits the workspace interior 56/44.
pub fn desktop_geometry(host: HostKind, width: f32) -> DesktopGeometry {
    let mode = layout_mode(host, width);
    if mode == LayoutMode::Phone {
        let workspace = width.min(PHONE_CONTENT_MAX);
        return DesktopGeometry {
            rail_width: 0.0,
            outer_gutter: 0.0,
            workspace_width: workspace,
            master_width: workspace,
            divider_width: 0.0,
            detail_width: 0.0,
        };
    }
    let (rail, outer) = if mode == LayoutMode::DesktopCompact {
        (RAIL_WIDTH_COMPACT, OUTER_COMPACT)
    } else {
        (RAIL_WIDTH_EXPANDED, OUTER_EXPANDED)
    };
    // The gutters absorb everything past the content cap, which is why the
    // 1280 window shows 64 px of gutter instead of a wider workspace.
    let main = (width - rail).max(0.0);
    let content = (main - 2.0 * outer).clamp(0.0, CONTENT_MAX);
    let gutter = (main - content) / 2.0;
    let interior = (content - DIVIDER_WIDTH).max(0.0);
    let (master_width, detail_width) = if mode == LayoutMode::DesktopCompact {
        (MASTER_COMPACT, interior - MASTER_COMPACT)
    } else {
        (interior * 0.56, interior * 0.44)
    };
    DesktopGeometry {
        rail_width: rail,
        outer_gutter: gutter,
        workspace_width: content,
        master_width,
        divider_width: DIVIDER_WIDTH,
        detail_width,
    }
}

/// What an Esc press consumed, in precedence order (brief §"Esc chains").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeAction {
    /// A modal was open; the app owns closing it, the model only reports.
    ClosedModal,
    /// Exited orders batch selection (checks cleared, nothing cancelled).
    EndedBatch,
    /// Closed the active tab's list preview; the cursor stays.
    ClosedPreview,
    /// Closed the open settings detail.
    ClosedSettingsDetail,
    /// Nothing to unwind; the shell may handle Esc itself.
    Nothing,
}

/// Which focus region keyboard shortcuts act on. Workspace shortcuts are
/// inert while a text field or a modal captures keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyScope {
    Workspace,
    TextInput,
    Modal,
}

/// Every keyboard-dispatchable workspace action (brief keyboard map).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopCommand {
    Refresh,
    OpenMenus,
    OpenOrders,
    OpenBilling,
    OpenSettings,
    OpenOrdersPast,
    OrderToday,
    CancelSelected,
    Retry,
    DismissAlert,
}

/// Whether `cmd` acts in `scope` on the workspace tab `tab` (REQ-UI-11).
///
/// The navigation chords are app chords, not typing keys: Cmd+1–4 and Cmd+, are
/// allowed while a text field has focus, and only a modal takes them away
/// (brief "App, no modal"). Native editing keys keep their priority because
/// the app binds no editing chord for them. Every other command stays inert
/// outside the Workspace scope, so a focused input never loses a key. Cmd+R
/// and its Retry twin reload only the tab that owns a reload — Menüs,
/// Bestellungen, Abrechnung — and are disabled on Einstellungen.
pub fn command_allowed(cmd: DesktopCommand, scope: KeyScope, tab: Tab) -> bool {
    if matches!(scope, KeyScope::Modal) {
        return false;
    }
    if matches!(scope, KeyScope::TextInput) {
        return matches!(
            cmd,
            DesktopCommand::OpenMenus
                | DesktopCommand::OpenOrders
                | DesktopCommand::OpenBilling
                | DesktopCommand::OpenSettings
        );
    }
    match cmd {
        DesktopCommand::Refresh | DesktopCommand::Retry => {
            matches!(tab, Tab::Menus | Tab::Orders | Tab::Billing)
        }
        _ => true,
    }
}

/// Stable identity of a command for the duplicate-claim guard. Refresh and
/// Retry share one identity: they run the same flow.
fn command_name(cmd: DesktopCommand) -> &'static str {
    match cmd {
        DesktopCommand::Refresh | DesktopCommand::Retry => "refresh",
        DesktopCommand::OrderToday => "order.today",
        DesktopCommand::CancelSelected => "order.cancel_selected",
        DesktopCommand::OpenMenus => "nav.menus",
        DesktopCommand::OpenOrders => "nav.orders",
        DesktopCommand::OpenBilling => "nav.billing",
        DesktopCommand::OpenSettings => "nav.settings",
        DesktopCommand::OpenOrdersPast => "orders.segment.past",
        DesktopCommand::DismissAlert => "alerts.dismiss",
    }
}

/// A flow command's claim, held while its body runs. Dropping it releases the
/// claim on every exit path, including a worker panic, so a flow that moved to
/// the host's thread cannot strand the workspace.
pub struct CommandClaim {
    hub: std::sync::Arc<crate::state::StateHub>,
    name: &'static str,
}

impl Drop for CommandClaim {
    fn drop(&mut self) {
        self.hub
            .update("command_release", |s| clear_claim(s, self.name));
    }
}

/// Release a flow's command claim from inside the flow's final state update.
pub(crate) fn clear_claim(s: &mut State, name: &str) {
    if s.claimed_command.as_deref() == Some(name) {
        s.claimed_command = None;
    }
}

/// Whether a list identity is still present after a data change.
pub fn preview_survives(id: &str, surviving_ids: &[String]) -> bool {
    surviving_ids.iter().any(|s| s == id)
}

/// The surviving row nearest to where `anchor` sat in `previous_ids`; ties go
/// to the one earlier in `surviving_ids`. `None` when no surviving row was in
/// the previous list at all. Pure cursor fallback for [`AppModel::reconcile_list`].
pub fn nearest_surviving(
    previous_ids: &[String],
    surviving_ids: &[String],
    anchor: &str,
) -> Option<String> {
    let anchor_index = previous_ids.iter().position(|id| id == anchor)?;
    surviving_ids
        .iter()
        .enumerate()
        .filter_map(|(order, id)| {
            let index = previous_ids.iter().position(|p| p == id)?;
            Some((index.abs_diff(anchor_index), order, id.clone()))
        })
        .min()
        .map(|(_, _, id)| id)
}

/// The pointer/focus state a shared desktop composition renders (brief
/// "Selectable states"). Only the desktop shell and the desktop gallery pass
/// anything but `Rest`; mobile call sites keep the incumbent defaults, so no
/// legacy value moves (REQ-UI-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlState {
    #[default]
    Rest,
    Hover,
    Focus,
    Disabled,
    Selected,
}

impl ControlState {
    /// Whether the reserved 2 px `focusInk` ring is painted.
    pub fn ring_visible(self) -> bool {
        matches!(self, ControlState::Focus)
    }
}

/// Rest/hover/selected fill of a passive desktop control or row: `surface`,
/// `surface_variant` and `primary_surface`. Focus keeps the rest fill (the
/// ring carries it) and disabled keeps the fill while the ink changes.
pub fn control_fill(state: ControlState, dark: bool) -> Rgb {
    match state {
        ControlState::Hover => token(crate::tokens::SURFACE_VARIANT, dark),
        ControlState::Selected => token(crate::tokens::PRIMARY_SURFACE, dark),
        ControlState::Rest | ControlState::Focus | ControlState::Disabled => {
            token(crate::tokens::SURFACE, dark)
        }
    }
}

/// Ink of a passive desktop control: `text_primary`, except disabled
/// (`text_tertiary`).
pub fn control_ink(state: ControlState, dark: bool) -> Rgb {
    match state {
        ControlState::Disabled => token(crate::tokens::TEXT_TERTIARY, dark),
        _ => text_ink(dark),
    }
}

/// An 8-bit RGB color parsed from a token string (REQ-UI-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// WCAG relative luminance.
    pub fn luminance(&self) -> f32 {
        let linear = |c: u8| {
            let c = c as f32 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(self.r) + 0.7152 * linear(self.g) + 0.0722 * linear(self.b)
    }
}

/// Parses a token color string: `#RGB`, `#RRGGBB` or `rgba(r,g,b,a)`.
/// Panics on anything else: every input is a token constant from `tokens.rs`.
pub fn parse_color(value: &str) -> Rgb {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        let out = match hex.len() {
            3 => {
                // "#RGB" expands each digit to a byte, the usual CSS rule.
                let digit = |i: usize| {
                    hex[..=i]
                        .chars()
                        .next_back()
                        .and_then(|c| c.to_digit(16))
                        .map(|d| (d * 17) as u8)
                        .unwrap_or_else(|| bad_token(value))
                };
                Rgb::new(digit(0), digit(1), digit(2))
            }
            6 => {
                let byte = |range: std::ops::Range<usize>| {
                    u8::from_str_radix(&hex[range], 16).unwrap_or_else(|_| bad_token(value))
                };
                Rgb::new(byte(0..2), byte(2..4), byte(4..6))
            }
            _ => bad_token(value),
        };
        return out;
    }
    let inner = value
        .strip_prefix("rgba(")
        .or_else(|| value.strip_prefix("rgb("))
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or_else(|| bad_token(value));
    let mut parts = inner.split(',');
    let channel = |s: Option<&str>| {
        s.and_then(|s| s.trim().parse::<f32>().ok())
            .map(|v| v.clamp(0.0, 255.0) as u8)
            .unwrap_or_else(|| bad_token(value))
    };
    Rgb::new(
        channel(parts.next()),
        channel(parts.next()),
        channel(parts.next()),
    )
}

#[cold]
fn bad_token(value: &str) -> ! {
    panic!("unparsable color token: {value:?}")
}

/// WCAG contrast ratio between two colors: 1.0 (identical) to 21.0.
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f32 {
    let (la, lb) = (a.luminance(), b.luminance());
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Linear mix of `fill` toward `toward` at `amount` (0.0..=1.0).
pub fn mix_toward(fill: Rgb, toward: Rgb, amount: f32) -> Rgb {
    let a = amount.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 * (1.0 - a) + y as f32 * a).round() as u8;
    Rgb::new(
        mix(fill.r, toward.r),
        mix(fill.g, toward.g),
        mix(fill.b, toward.b),
    )
}

fn token(token: crate::tokens::ColorToken, dark: bool) -> Rgb {
    parse_color(if dark { token.dark } else { token.light })
}

/// Primary text ink of an appearance.
pub fn text_ink(dark: bool) -> Rgb {
    token(crate::tokens::TEXT_PRIMARY, dark)
}

/// Rest fill of a filled primary action: the darker accent in light mode, the
/// base accent in dark mode (brief "Action states").
pub fn action_fill(accent: &crate::tokens::AccentColors, dark: bool) -> Rgb {
    parse_color(if dark {
        accent.primary
    } else {
        accent.primary_dark
    })
}

/// Hover fill: 8 % `text_primary` mixed into the rest fill.
pub fn action_hover_fill(fill: Rgb, ink: Rgb) -> Rgb {
    mix_toward(fill, ink, ACTION_HOVER_MIX)
}

/// Pressed fill: 14 % `text_primary` mixed into the rest fill.
pub fn action_pressed_fill(fill: Rgb, ink: Rgb) -> Rgb {
    mix_toward(fill, ink, ACTION_PRESSED_MIX)
}

/// Rest fill of a destructive action: the incumbent error token.
pub fn danger_fill(dark: bool) -> Rgb {
    token(crate::tokens::ERROR, dark)
}

/// White or black, whichever reads better on this fill, chosen per fill.
pub fn on_fill(fill: Rgb) -> Rgb {
    let white = parse_color(crate::tokens::SURFACE.light);
    let black = parse_color(crate::tokens::BACKGROUND.dark);
    if contrast_ratio(white, fill) >= contrast_ratio(black, fill) {
        white
    } else {
        black
    }
}

/// `preferred` kept only if it reaches `min` on `fill`, else the appearance's
/// primary text ink for that fill's lightness.
fn ink_at_least(fill: Rgb, preferred: Rgb, min: f32) -> Rgb {
    if contrast_ratio(preferred, fill) >= min {
        preferred
    } else {
        text_ink(fill.luminance() <= 0.5)
    }
}

/// `preferred` kept only if it reaches 4.5:1 on `fill`, else the appearance's
/// primary text ink (REQ-UI-13).
pub fn readable_ink(fill: Rgb, preferred: Rgb) -> Rgb {
    ink_at_least(fill, preferred, TEXT_MIN_CONTRAST)
}

/// `preferred` kept only if it reaches 3:1 on `fill`, else the appearance's
/// primary text ink: an accent that cannot carry a control boundary falls
/// back to the ink that can (REQ-UI-13).
pub fn boundary_ink(fill: Rgb, preferred: Rgb) -> Rgb {
    ink_at_least(fill, preferred, CONTROL_MIN_CONTRAST)
}

/// Workspace panel fill.
pub fn panel_fill(dark: bool) -> Rgb {
    token(crate::tokens::SURFACE, dark)
}

/// Navigation rail fill.
pub fn rail_fill(dark: bool) -> Rgb {
    token(crate::tokens::BACKGROUND, dark)
}

/// Focus ring ink: `text_primary`, which clears 3:1 on both the workspace
/// panel and the rail in either appearance (REQ-UI-13).
pub fn focus_ink(dark: bool) -> Rgb {
    text_ink(dark)
}

impl AppModel {
    /// Report the host kind (hosted by the platform shell at startup).
    pub fn set_host_kind(&self, host: HostKind) {
        self.hub().update("host_kind", |s| s.host_kind = host);
        self.reproject_settings();
    }

    /// Report the workspace content width in pixels (on every resize).
    pub fn set_content_width(&self, width: f32) {
        self.hub()
            .update("content_width", |s| s.content_width = width);
        self.reproject_settings();
    }

    /// Current layout mode from the last host kind and width reported.
    pub fn layout_mode(&self) -> LayoutMode {
        let s = self.state();
        layout_mode(s.host_kind, s.content_width)
    }

    /// Tile widths for the current host and width, so the shell never derives
    /// geometry itself (REQ-UI-07).
    pub fn desktop_geometry(&self) -> DesktopGeometry {
        let s = self.state();
        desktop_geometry(s.host_kind, s.content_width)
    }

    /// True on the expanded desktop workspace (detail padding 24, 56/44 split).
    pub fn is_expanded(&self) -> bool {
        self.layout_mode() == LayoutMode::DesktopExpanded
    }

    /// Whether the settings detail is contained beside the master instead of
    /// being a pushed screen. Containment starts at the 720 px compact
    /// breakpoint, so compact and expanded desktop both contain; only the
    /// phone projection (a phone host, or a desktop narrower than 720) pushes
    /// exactly one detail screen (REQ-SCR-04/REQ-UI-10).
    pub fn settings_contained(&self) -> bool {
        self.layout_mode() != LayoutMode::Phone
    }

    /// Which scope keyboard shortcuts act on, from the current focus flags.
    pub fn key_scope(&self) -> KeyScope {
        let s = self.state();
        if s.modal_open {
            KeyScope::Modal
        } else if s.text_input_focused {
            KeyScope::TextInput
        } else {
            KeyScope::Workspace
        }
    }

    /// The shell reports its modal (the model never opens one itself).
    pub fn set_modal_open(&self, open: bool) {
        self.hub().update("modal_open", |s| s.modal_open = open);
    }

    /// The shell reports text-field focus so shortcuts stay inert while typing.
    pub fn set_text_input_focused(&self, focused: bool) {
        self.hub()
            .update("text_input_focused", |s| s.text_input_focused = focused);
    }

    /// Tab activation: switching resets the departing tab's preview and cursor
    /// and the settings detail, but never pending menu changes. Re-activating
    /// the active tab is a strict no-op: no state update, no notification.
    /// Returns whether anything changed.
    pub fn select_tab(&self, tab: Tab) -> bool {
        self.hub().try_update(
            "tab",
            |s| s.navigation.active_tab() != tab,
            |s| {
                if let Some(kind) = ListKind::for_tab(s.navigation.active_tab()) {
                    s.clear_list(kind);
                }
                s.settings_detail = None;
                s.navigation.select_tab(tab);
            },
        )
    }

    /// The navigation instance the shell renders.
    pub fn navigation(&self) -> crate::navigation::Navigation {
        self.state().navigation
    }

    /// Open a row's preview by stable identity. Touches no data: opening a
    /// preview is a view action, never a mutation.
    pub fn open_preview(&self, kind: ListKind, id: &str) {
        let id = id.to_string();
        self.hub()
            .update("preview", move |s| s.set_preview(kind, Some(id)));
    }

    /// Close a list's preview; its cursor stays (Esc unwinds one layer).
    pub fn close_preview(&self, kind: ListKind) {
        self.hub()
            .update("preview_close", |s| s.set_preview(kind, None));
    }

    /// The open preview identity of a list, if any.
    pub fn preview(&self, kind: ListKind) -> Option<String> {
        self.state().preview_of(kind).map(str::to_string)
    }

    /// Move a list's cursor (keyboard navigation; activation is a separate step).
    pub fn set_cursor(&self, kind: ListKind, id: Option<String>) {
        self.hub().update("cursor", move |s| s.set_cursor(kind, id));
    }

    /// The cursor identity of a list, if any.
    pub fn cursor(&self, kind: ListKind) -> Option<String> {
        self.state().cursor_of(kind).map(str::to_string)
    }

    /// Reconcile a list after its rows changed: keep the preview when its
    /// identity survives, otherwise clear it and move the cursor to the
    /// nearest surviving row — never auto-activate anything. Returns whether
    /// the preview was cleared.
    pub fn reconcile_list(
        &self,
        kind: ListKind,
        previous_ids: &[String],
        surviving_ids: &[String],
    ) -> bool {
        let Some(preview) = self.preview(kind) else {
            return false;
        };
        if preview_survives(&preview, surviving_ids) {
            return false;
        }
        let cursor = nearest_surviving(previous_ids, surviving_ids, &preview);
        self.hub().update("reconcile", move |s| {
            s.set_preview(kind, None);
            s.set_cursor(kind, cursor.clone());
        });
        true
    }

    /// Push a screen onto the active tab's stack. The shell reports the
    /// intent; the model owns the stack, so the shared Root keeps no second
    /// authoritative navigation.
    pub fn push_screen(&self, screen: Screen) {
        self.hub()
            .update("screen_push", move |s| s.navigation.push(screen));
    }

    /// Pop one screen from the active tab's stack (the root never pops).
    pub fn pop_screen(&self) {
        self.hub().update("screen_pop", |s| {
            s.navigation.pop();
        });
    }

    /// Open a settings section: containment from the 720 px compact desktop
    /// breakpoint on, a single pushed detail screen on the phone projection
    /// (design D1, REQ-SCR-04).
    pub fn open_settings_detail(&self, detail: SettingsDetail) {
        self.hub().update("settings_detail", move |s| {
            s.settings_detail = Some(detail);
            let contained = layout_mode(s.host_kind, s.content_width) != LayoutMode::Phone;
            if !contained && s.navigation.top() != Screen::SettingsDetail {
                s.navigation.push(Screen::SettingsDetail);
            }
        });
    }

    /// Close the open settings detail, popping the pushed screen if there is one.
    pub fn close_settings_detail(&self) {
        self.hub().update("settings_detail_close", |s| {
            s.settings_detail = None;
            if s.navigation.top() == Screen::SettingsDetail {
                s.navigation.pop();
            }
        });
    }

    /// The open settings detail, if any.
    pub fn settings_detail(&self) -> Option<SettingsDetail> {
        self.state().settings_detail
    }

    /// Esc press, in precedence order: modal, batch selection, active tab's
    /// preview, settings detail, nothing. Esc never touches pending menu
    /// changes (v1 parity: they are only dropped by clear/submit).
    pub fn escape(&self) -> EscapeAction {
        if self.state().modal_open {
            return EscapeAction::ClosedModal;
        }
        if self.state().order_selection_mode {
            self.end_order_selection();
            return EscapeAction::EndedBatch;
        }
        let active = self.state().navigation.active_tab();
        if let Some(kind) = ListKind::for_tab(active) {
            if self.state().preview_of(kind).is_some() {
                self.close_preview(kind);
                return EscapeAction::ClosedPreview;
            }
        }
        if self.state().settings_detail.is_some() {
            self.close_settings_detail();
            return EscapeAction::ClosedSettingsDetail;
        }
        EscapeAction::Nothing
    }

    /// The tab the workspace shortcuts act on.
    fn active_tab(&self) -> Tab {
        self.state().navigation.active_tab()
    }

    /// Run a keyboard command. Duplicate-safe: a flow command is rejected while
    /// `busy` or while any command is still claimed, so a double tap cannot
    /// start the same operation twice. Flows release their claim on every exit
    /// path; synchronous commands release before returning. The four
    /// navigation chords run no flow at all, so they stay available while one
    /// is running (brief "App, no modal") and take no claim.
    /// Returns whether the command was accepted and dispatched.
    pub fn handle_command(&self, cmd: DesktopCommand) -> bool {
        if !command_allowed(cmd, self.key_scope(), self.active_tab()) {
            return false;
        }
        let tab = match cmd {
            DesktopCommand::OpenMenus => Some(Tab::Menus),
            DesktopCommand::OpenOrders => Some(Tab::Orders),
            DesktopCommand::OpenBilling => Some(Tab::Billing),
            DesktopCommand::OpenSettings => Some(Tab::Settings),
            _ => None,
        };
        if let Some(tab) = tab {
            self.select_tab(tab);
            return true;
        }
        let name = command_name(cmd);
        let claimed = self.hub().try_update(
            "command",
            |s| !s.busy && s.claimed_command.is_none(),
            |s| s.claimed_command = Some(name.to_string()),
        );
        if !claimed {
            return false;
        }
        match cmd {
            // Cmd+R reloads the tab it was pressed on; each flow releases
            // the shared "refresh" claim on every exit path.
            DesktopCommand::Refresh | DesktopCommand::Retry => match self.active_tab() {
                Tab::Menus => {
                    self.refresh_menus();
                    true
                }
                Tab::Orders => self.dispatch_sync(name, || self.refresh_orders()),
                Tab::Billing => self.dispatch_sync(name, || self.reload_billing()),
                // command_allowed rejects Cmd+R on Einstellungen; defensive
                // so a future gate change cannot strand the claim.
                Tab::Settings => {
                    self.release_claim(name);
                    false
                }
            },
            DesktopCommand::OrderToday => {
                self.submit_orders();
                true
            }
            DesktopCommand::CancelSelected => {
                if self.cancel_selected_orders() {
                    true
                } else {
                    self.release_claim(name);
                    false
                }
            }
            // Navigation returns above, before any claim is taken.
            DesktopCommand::OpenMenus
            | DesktopCommand::OpenOrders
            | DesktopCommand::OpenBilling
            | DesktopCommand::OpenSettings => unreachable!("navigation ran above"),
            DesktopCommand::OpenOrdersPast => {
                self.dispatch_sync(name, || self.set_orders_segment(true))
            }
            DesktopCommand::DismissAlert => self.dispatch_sync(name, || {
                self.hub().update("alert_dismissed", |s| {
                    s.broadcast_dismissed = true;
                    s.location_alert = None;
                })
            }),
        }
    }

    /// The command currently claimed, if any (for tests and the shell's busy UI).
    pub fn claimed_command(&self) -> Option<String> {
        self.state().claimed_command
    }

    /// Take a flow command's claim without running its body, so a shell that
    /// moves blocking flow bodies to another thread keeps the model's scope
    /// eligibility, busy check and duplicate rejection here. `None` for a
    /// rejected command, a navigation chord (it runs no flow) or while another
    /// command holds the claim. Drop the claim when the body finishes.
    pub fn begin_command(&self, cmd: DesktopCommand) -> Option<CommandClaim> {
        if !command_allowed(cmd, self.key_scope(), self.active_tab()) {
            return None;
        }
        if matches!(
            cmd,
            DesktopCommand::OpenMenus
                | DesktopCommand::OpenOrders
                | DesktopCommand::OpenBilling
                | DesktopCommand::OpenSettings
        ) {
            return None;
        }
        let name = command_name(cmd);
        let claimed = self.hub().try_update(
            "command",
            |s| !s.busy && s.claimed_command.is_none(),
            |s| s.claimed_command = Some(name.to_string()),
        );
        claimed.then(|| CommandClaim {
            hub: std::sync::Arc::clone(self.hub()),
            name,
        })
    }

    /// Run a synchronous command body, then release the claim. Always accepted.
    fn dispatch_sync(&self, name: &'static str, run: impl FnOnce()) -> bool {
        run();
        self.release_claim(name);
        true
    }

    fn release_claim(&self, name: &'static str) {
        self.hub()
            .update("command_release", |s| clear_claim(s, name));
    }

    /// Re-project the settings detail after a host-kind or width change:
    /// the phone projection (below 720 on desktop, or a phone host) pushes the
    /// screen, containment from 720 on pops it back. The detail identity
    /// itself is preserved across resizes.
    fn reproject_settings(&self) {
        self.hub().update("settings_projection", |s| {
            let contained = layout_mode(s.host_kind, s.content_width) != LayoutMode::Phone;
            if !contained
                && s.settings_detail.is_some()
                && s.navigation.top() != Screen::SettingsDetail
            {
                s.navigation.push(Screen::SettingsDetail);
            } else if contained && s.navigation.top() == Screen::SettingsDetail {
                s.navigation.pop();
            }
        });
    }
}
