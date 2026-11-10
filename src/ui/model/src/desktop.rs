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

/// Which workspace the shell renders. `DesktopCompact` counts as narrow for
/// the settings projection (the detail is a pushed screen, not containment).
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
/// Cmd+1–4 and the other workspace commands work in the Workspace scope on
/// every tab; Cmd+R and its Retry twin reload only the tab that owns a
/// reload — Menüs, Bestellungen, Abrechnung — and are disabled on
/// Einstellungen and in the TextInput and Modal scopes, where the host must
/// not capture the key at all.
pub fn command_allowed(cmd: DesktopCommand, scope: KeyScope, tab: Tab) -> bool {
    if !matches!(scope, KeyScope::Workspace) {
        return false;
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

    /// True on the expanded desktop workspace (containment, not push).
    pub fn is_wide(&self) -> bool {
        self.layout_mode() == LayoutMode::DesktopExpanded
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

    /// Open a settings section: containment on the wide workspace, a pushed
    /// detail screen on the narrow one (design D1).
    pub fn open_settings_detail(&self, detail: SettingsDetail) {
        self.hub().update("settings_detail", move |s| {
            s.settings_detail = Some(detail);
            let wide = layout_mode(s.host_kind, s.content_width) == LayoutMode::DesktopExpanded;
            if !wide && s.navigation.top() != Screen::SettingsDetail {
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

    /// Run a keyboard command. Duplicate-safe: a command is rejected while
    /// `busy` or while any command is still claimed, so a double tap cannot
    /// start the same operation twice. Flows release their claim on every
    /// exit path; synchronous commands release before returning.
    /// Returns whether the command was accepted and dispatched.
    pub fn handle_command(&self, cmd: DesktopCommand) -> bool {
        if !command_allowed(cmd, self.key_scope(), self.active_tab()) {
            return false;
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
            DesktopCommand::OpenMenus => self.dispatch_sync(name, || {
                self.select_tab(Tab::Menus);
            }),
            DesktopCommand::OpenOrders => self.dispatch_sync(name, || {
                self.select_tab(Tab::Orders);
            }),
            DesktopCommand::OpenBilling => self.dispatch_sync(name, || {
                self.select_tab(Tab::Billing);
            }),
            DesktopCommand::OpenSettings => self.dispatch_sync(name, || {
                self.select_tab(Tab::Settings);
            }),
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
    /// narrow + open detail pushes the screen, wide pops it back into
    /// containment. The detail identity itself is preserved across resizes.
    fn reproject_settings(&self) {
        self.hub().update("settings_projection", |s| {
            let wide = layout_mode(s.host_kind, s.content_width) == LayoutMode::DesktopExpanded;
            if !wide && s.settings_detail.is_some() && s.navigation.top() != Screen::SettingsDetail
            {
                s.navigation.push(Screen::SettingsDetail);
            } else if wide && s.navigation.top() == Screen::SettingsDetail {
                s.navigation.pop();
            }
        });
    }
}
