//! Updates detail — design D7, REQ-UPD-14/15/16/17.
//!
//! Desktop only. Every word is the model's [`snackpilot_model::UpdateView`]
//! projection: this screen chooses what to show when and where, never what it
//! says. One column inside the detail pane padding, so nothing scrolls
//! horizontally at 360, 720 or 1280.

use std::rc::Rc;
use std::sync::Arc;

use gpui::*;
use gpui_kit::component::switch::Switch;

use crate::app::AppShell;
use crate::components::button::{desktop_action_state, ButtonKind};
use crate::theme::AppTheme;
use snackpilot_model::{
    AppModel, HandoffOutcome, UpdateAction, UpdateFailure, UpdateState, UpdateTrigger, UpdateView,
};

/// The app-crate cadence tests' scripts: a release list with no manifest, so
/// every check ends in the contained `NoManifest` failure after exactly one
/// discovery. Nothing leaves the process and no signature is needed.
#[cfg(test)]
#[derive(Clone)]
pub(crate) struct ScriptedUpdateServices(Arc<ScriptedUpdateServicesInner>);

/// Every request the model handed to the transport, in order.
#[cfg(test)]
struct ScriptedUpdateServicesInner {
    requests: std::sync::Mutex<Vec<snackpilot_core::http::Request>>,
    downloader: Arc<snackpilot_core::update::FakeDownloader>,
}

#[cfg(test)]
impl ScriptedUpdateServices {
    pub(crate) fn new() -> Self {
        Self(Arc::new(ScriptedUpdateServicesInner {
            requests: std::sync::Mutex::new(Vec::new()),
            downloader: Arc::new(snackpilot_core::update::FakeDownloader::new()),
        }))
    }

    pub(crate) fn requests(&self) -> Vec<snackpilot_core::http::Request> {
        self.0.requests.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl snackpilot_model::UpdateServices for ScriptedUpdateServices {
    fn discovery_transport(&self) -> Arc<dyn snackpilot_core::http::Transport> {
        Arc::new(self.clone())
    }

    fn downloader(&self) -> Arc<dyn snackpilot_core::update::Downloader> {
        self.0.downloader.clone()
    }
}

#[cfg(test)]
impl snackpilot_core::http::Transport for ScriptedUpdateServices {
    fn send<'a>(
        &'a self,
        req: snackpilot_core::http::Request,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = snackpilot_core::error::CoreResult<
                        snackpilot_core::http::HttpResponse,
                    >,
                > + Send
                + 'a,
        >,
    > {
        self.0.requests.lock().unwrap().push(req);
        Box::pin(async { Ok(empty_release_list()) })
    }
}

/// A valid GitHub release list without a `desktop/v…` release: the discovery
/// ends there, with no manifest request and no signature to verify.
#[cfg(test)]
fn empty_release_list() -> snackpilot_core::http::HttpResponse {
    snackpilot_core::http::HttpResponse {
        status: 200,
        headers: Vec::new(),
        body: "[]".to_string(),
    }
}

/// The Windows host the updater runs against in the app-crate tests: the offline
/// fake platform reports it, and no request leaves the process.
#[cfg(test)]
pub(crate) fn test_host() -> snackpilot_model::UpdateHostInfo {
    snackpilot_model::UpdateHostInfo {
        platform: "windows",
        architecture: "x64",
        os_version: Some("10.0.26100".into()),
        installed_version: "2.2.0".into(),
        installed_build: "41".into(),
    }
}

/// The wall clock in epoch milliseconds, as the model's cadence expects.
pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}

/// The switch row's hint (design D7): the one sentence next to the control.
pub(crate) const SWITCH_HINT: &str =
    "Beim Start und alle 24 Stunden. Es wird nur GitHub kontaktiert.";

/// One action's dispatch: the model call the button makes, exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpdateDispatch {
    ManualCheck,
    Download,
    Skip,
    Handoff(HandoffOutcome),
}

/// What pressing the clicked `action` does, or `None` while the model would
/// refuse it (design D7: "actions call the model methods once and are disabled
/// while an operation runs"). Every button of a row dispatches its own action;
/// nothing depends on its position. The handoff outcome comes from the current
/// state, so a `HandedOff` re-offer repeats the handoff it already did.
pub(crate) fn action_dispatch(
    action: UpdateAction,
    view: &UpdateView,
    state: &UpdateState,
) -> Option<UpdateDispatch> {
    // A stale frame may still carry a button the current state dropped.
    if view.actions_disabled() || !view.actions.contains(&action) {
        return None;
    }
    match action {
        UpdateAction::Check => Some(UpdateDispatch::ManualCheck),
        UpdateAction::Download => Some(UpdateDispatch::Download),
        UpdateAction::Skip => Some(UpdateDispatch::Skip),
        UpdateAction::InstallAndQuit | UpdateAction::RevealInFinder => match state {
            UpdateState::Ready { handoff, .. }
            | UpdateState::HandedOff {
                outcome: handoff, ..
            } => Some(UpdateDispatch::Handoff(*handoff)),
            _ => None,
        },
    }
}

/// The handoff button's kind: the Windows action ends the app, the macOS one
/// only reveals the bundle.
pub(crate) fn handoff_kind(outcome: HandoffOutcome) -> ButtonKind {
    match outcome {
        HandoffOutcome::QuitRequested => ButtonKind::Primary,
        HandoffOutcome::Revealed => ButtonKind::Secondary,
    }
}

/// A check whose failure is the state machine's own business: no dialog, no
/// toast, nothing outside the Updates detail (REQ-UPD-13).
pub(crate) fn check_for_update(model: &AppModel, now_ms: i64) {
    model.check_for_update(UpdateTrigger::Manual, now_ms);
}

/// One model call for one pressed button (design D7). The switch has its own
/// row; every other action is disabled while an operation runs, so a double
/// tap cannot start two.
pub(crate) fn dispatch_update(model: &AppModel, dispatch: UpdateDispatch, now_ms: i64) {
    match dispatch {
        UpdateDispatch::ManualCheck => check_for_update(model, now_ms),
        UpdateDispatch::Download => model.start_update_download(),
        UpdateDispatch::Skip => model.skip_update(),
        UpdateDispatch::Handoff(_) => model.hand_off_update(),
    }
}

/// Whether the shell quits after a model change (REQ-UPD-07/08): only the
/// Windows installer handoff asks for it, and only on the change that enters
/// `HandedOff` with `QuitRequested` — not on every later paint of that state.
pub(crate) fn quit_after_handoff(previous: &UpdateState, current: &UpdateState) -> bool {
    !matches!(previous, UpdateState::HandedOff { .. })
        && matches!(
            current,
            UpdateState::HandedOff {
                outcome: HandoffOutcome::QuitRequested,
                ..
            }
        )
}

/// One hourly update tick (REQ-UPD-11): automatic, so a failure stays in the
/// Updates detail. `None` before the shell has a model.
pub(crate) fn tick_update(model: Option<&Arc<AppModel>>, now_ms: i64) {
    if let Some(model) = model {
        model.check_for_update(UpdateTrigger::Periodic, now_ms);
    }
}

/// The launch check, once after `bootstrap` (REQ-UPD-11).
pub(crate) fn launch_update(model: Option<&Arc<AppModel>>, now_ms: i64) {
    if let Some(model) = model {
        model.check_for_update(UpdateTrigger::Launch, now_ms);
    }
}

/// The Datenschutz alert body for the desktop host: the approved paragraph, the
/// no-analytics sentence and — only where the updater is live — the GitHub
/// sentence (REQ-UPD-20).
pub(crate) fn desktop_datenschutz_text(sentence: Option<&str>) -> String {
    let base = crate::desktop::privacy_message();
    match sentence {
        Some(sentence) => format!("{base}\n\n{sentence}"),
        None => base,
    }
}

/// One wrapping paragraph of the detail: `META` secondary hint or `BODY` text.
fn paragraph(text: &str, secondary: bool, theme: &AppTheme) -> Div {
    let (size, line) = if secondary {
        (theme.scale.meta, theme.scale.meta_line)
    } else {
        (theme.scale.body, theme.scale.body_line)
    };
    div()
        .w_full()
        .text_size(px(size))
        .line_height(px(line))
        .text_color(if secondary {
            theme.text_secondary
        } else {
            theme.text_primary
        })
        .child(text.to_string())
}

/// The Updates detail. `containment` is `Some(pad)` beside the settings master
/// and `None` on the pushed phone projection, exactly like Darstellung.
pub fn updates_screen(
    model: Option<&Arc<AppModel>>,
    theme: &AppTheme,
    containment: Option<f32>,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let Some(model) = model.cloned() else {
        return crate::components::workspace::detail_prompt("Einstellungen", theme);
    };
    let view = model.update_view();
    let state = model.update_state();
    let back = {
        let entity = cx.entity().clone();
        Rc::new(move |_: &mut Window, cx: &mut App| {
            entity.update(cx, |this, cx| this.pop_detail(cx));
        })
    };

    let toggle = {
        let shell = cx.entity().clone();
        let model = model.clone();
        move |checked: &bool, _window: &mut Window, cx: &mut App| {
            model.set_auto_update_check(*checked);
            shell.update(cx, |_, cx| cx.notify());
        }
    };
    let switch_row = div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(theme.scale.space_3))
        .py(px(theme.scale.space_2))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(theme.scale.space_1))
                .child(paragraph("Automatisch nach Updates suchen", false, theme))
                .child(paragraph(SWITCH_HINT, true, theme)),
        )
        .child(
            Switch::new("update-switch")
                .checked(view.switch_on)
                .color(theme.accent)
                .on_change(toggle),
        );

    let mut page = div()
        .id("updates")
        .w_full()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2))
        .overflow_y_scroll();
    if let Some(pad) = containment {
        page = page.p(px(pad)).max_w(px(440.));
    } else {
        page = page.child(crate::navigation::back_affordance(
            "Einstellungen",
            theme,
            back,
        ));
    }
    page = page
        .child(
            div()
                .text_size(px(theme.scale.title))
                .line_height(px(theme.scale.title_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child("Updates"),
        )
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(theme.scale.space_3))
                .child(paragraph("Installierte Version", false, theme))
                .child(paragraph(&view.installed, true, theme)),
        )
        .child(div().w_full().h(px(1.)).bg(theme.border))
        .child(switch_row)
        .child(div().w_full().h(px(1.)).bg(theme.border))
        .child(paragraph(&view.state_line, false, theme));

    if let (Some(heading), Some(notes)) = (&view.notes_heading, &view.notes) {
        page = page
            .child(
                div()
                    .w_full()
                    .pt(px(theme.scale.space_2))
                    .text_size(px(theme.scale.body))
                    .line_height(px(theme.scale.body_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text_primary)
                    .child(heading.clone()),
            )
            .child(paragraph(notes, false, theme));
        if let Some(url) = &view.notes_url {
            // No host link handler exists here yet: the page shows the link as
            // its text, selectable for copying rather than a dead control.
            page = page.child(
                div()
                    .id("update-notes-link")
                    .w_full()
                    .child(paragraph("Alle Änderungen auf GitHub", false, theme))
                    .child(paragraph(url, true, theme)),
            );
        }
    }

    if let Some(progress) = view.progress {
        page = page
            .child(
                div()
                    .w_full()
                    .h(px(6.))
                    .rounded(px(theme.scale.radius_chip))
                    .bg(theme.surface_alt)
                    .child(
                        div()
                            .h_full()
                            .w(relative((progress / 100.).clamp(0., 1.)))
                            .rounded(px(theme.scale.radius_chip))
                            .bg(theme.accent),
                    ),
            )
            .child(paragraph(
                &format!("{} %", progress.round() as i64),
                true,
                theme,
            ));
    }

    if view.actions.is_empty() {
        page = page.child(
            div()
                .w_full()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child("Ein Update läuft. Die Aktionen sind so lange deaktiviert."),
        );
    }
    // While an operation runs every button stays in place, disabled: the row
    // keeps its width and the user sees what will come back after it.
    let disabled = view.actions_disabled();

    let mut buttons = div()
        .w_full()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(theme.scale.space_2));
    for action in &view.actions {
        let kind = match action {
            UpdateAction::Check => ButtonKind::Secondary,
            UpdateAction::Download => ButtonKind::Primary,
            UpdateAction::Skip => ButtonKind::Quiet,
            UpdateAction::InstallAndQuit | UpdateAction::RevealInFinder => match &state {
                UpdateState::Ready { handoff, .. }
                | UpdateState::HandedOff {
                    outcome: handoff, ..
                } => handoff_kind(*handoff),
                _ => ButtonKind::Primary,
            },
        };
        let id = match action {
            UpdateAction::Check => "update-check",
            UpdateAction::Download => "update-download",
            UpdateAction::Skip => "update-skip",
            UpdateAction::InstallAndQuit => "update-install",
            UpdateAction::RevealInFinder => "update-reveal",
        };
        let action = *action;
        let run = {
            let shell = cx.entity().clone();
            let model = model.clone();
            move |_: &ClickEvent, _window: &mut Window, cx: &mut App| {
                let view = model.update_view();
                let state = model.update_state();
                // The clicked button dispatches its own action; a stale frame
                // that no longer offers it dispatches nothing.
                let Some(dispatch) = action_dispatch(action, &view, &state) else {
                    return;
                };
                dispatch_update(&model, dispatch, now_ms());
                shell.update(cx, |_, cx| cx.notify());
            }
        };
        let state = if disabled {
            snackpilot_model::desktop::ControlState::Disabled
        } else {
            snackpilot_model::desktop::ControlState::Rest
        };
        buttons = buttons.child(desktop_action_state(
            id,
            action.label(),
            kind,
            state,
            theme,
            cx,
            run,
        ));
    }
    page = page.child(buttons);
    page.into_any_element()
}

/// The offline debug scenarios of the gallery: one state per row of the D7
/// table, plus the releases that carry a reason or notes. Evidence mode makes
/// every entry point a no-op, so none of these can reach the network.
pub(crate) const GALLERY_STATES: &[&str] = &[
    "idle",
    "checking",
    "up-to-date",
    "available",
    "skipped",
    "downloading",
    "verifying",
    "ready-windows",
    "ready-macos",
    "handed-off-windows",
    "handed-off-macos",
    "failed-check",
    "failed-download",
    "failed-verify",
    "failed-handoff",
];

/// The name of the state the model is in, for the gallery's own layout test.
#[cfg(test)]
fn gallery_state_name(state: &UpdateState) -> &'static str {
    match state {
        UpdateState::Idle => "idle",
        UpdateState::Checking => "checking",
        UpdateState::UpToDate { .. } => "up-to-date",
        UpdateState::Available { .. } => "available",
        UpdateState::Skipped { .. } => "skipped",
        UpdateState::Downloading { .. } => "downloading",
        UpdateState::Verifying { .. } => "verifying",
        UpdateState::Ready { handoff, .. } => match handoff {
            HandoffOutcome::QuitRequested => "ready-windows",
            HandoffOutcome::Revealed => "ready-macos",
        },
        UpdateState::HandedOff { outcome, .. } => match outcome {
            HandoffOutcome::QuitRequested => "handed-off-windows",
            HandoffOutcome::Revealed => "handed-off-macos",
        },
        UpdateState::Failed { stage, .. } => match stage {
            UpdateFailure::Check => "failed-check",
            UpdateFailure::Download => "failed-download",
            UpdateFailure::Verify => "failed-verify",
            UpdateFailure::Handoff => "failed-handoff",
        },
    }
}

/// The documented stand-in artifact of every offline scenario.
pub(crate) const GALLERY_ARTIFACT: &str = "/tmp/SnackPilot-2.3.0-windows-x64-setup.exe";

/// The offered update of the offline scenarios: fixed version, notes and
/// link, matching the manifest fixture's shape. Nothing is downloaded.
pub(crate) fn gallery_candidate() -> snackpilot_core::update::Candidate {
    use snackpilot_core::update::Manifest;
    let notes = "Fehler beim Anzeigen vergangener Abrechnungen behoben. Der Automaten-Kontostand \
                 aktualisiert sich schneller."
        .to_string();
    snackpilot_core::update::Candidate {
        manifest: Manifest {
            schema: 1,
            platform: "windows".into(),
            architecture: "x64".into(),
            version: "2.3.0".into(),
            build: "42".into(),
            minimum_os: "10.0.22000".into(),
            artifact: "SnackPilot-2.3.0-windows-x64-setup.exe".into(),
            size: 12_345_678,
            sha256: "0".repeat(64),
            notes,
            notes_url: "https://github.com/radaiko/SnackPilot/releases/tag/desktop/v2.3.0".into(),
            tag: "desktop/v2.3.0".into(),
        },
        artifact_url: "https://github.com/radaiko/SnackPilot/releases/download/desktop%2Fv2.3.0/SnackPilot-2.3.0-windows-x64-setup.exe"
            .into(),
    }
}

/// One row of the D7 table as an `UpdateState`, for the offline gallery.
/// @req REQ-UPD-15
pub(crate) fn gallery_state(name: &str) -> Option<UpdateState> {
    use snackpilot_model::{Retry, VerifiedUpdate};
    let verified = VerifiedUpdate {
        version: "2.3.0".into(),
        artifact: std::path::PathBuf::from(GALLERY_ARTIFACT),
        prepared: None,
    };
    let candidate = gallery_candidate();
    Some(match name {
        "idle" => UpdateState::Idle,
        "checking" => UpdateState::Checking,
        "up-to-date" => UpdateState::UpToDate {
            checked_ms: 1_770_000_000_000,
        },
        "available" => UpdateState::Available {
            candidate: candidate.clone(),
        },
        "skipped" => UpdateState::Skipped {
            version: "2.3.0".into(),
        },
        "downloading" => UpdateState::Downloading {
            candidate,
            done: 60,
            total: Some(100),
        },
        "verifying" => UpdateState::Verifying {
            candidate: candidate.clone(),
        },
        "ready-windows" => UpdateState::Ready {
            update: verified.clone(),
            handoff: HandoffOutcome::QuitRequested,
        },
        "ready-macos" => UpdateState::Ready {
            update: verified.clone(),
            handoff: HandoffOutcome::Revealed,
        },
        "handed-off-windows" => UpdateState::HandedOff {
            version: "2.3.0".into(),
            outcome: HandoffOutcome::QuitRequested,
            update: verified.clone(),
        },
        "handed-off-macos" => UpdateState::HandedOff {
            version: "2.3.0".into(),
            outcome: HandoffOutcome::Revealed,
            update: verified.clone(),
        },
        "failed-check" => UpdateState::Failed {
            stage: UpdateFailure::Check,
            reason: "Die Verbindung zu GitHub ist fehlgeschlagen.".into(),
            retry: None,
        },
        "failed-download" => UpdateState::Failed {
            stage: UpdateFailure::Download,
            reason: "Der Download wurde unterbrochen.".into(),
            retry: Some(Retry::Download(Box::new(candidate))),
        },
        "failed-verify" => UpdateState::Failed {
            stage: UpdateFailure::Verify,
            reason: String::new(),
            retry: None,
        },
        "failed-handoff" => UpdateState::Failed {
            stage: UpdateFailure::Handoff,
            reason: "Der Installer konnte nicht gestartet werden.".into(),
            retry: Some(Retry::Handoff(verified, HandoffOutcome::QuitRequested)),
        },
        _ => return None,
    })
}

/// The gallery's offline model for one scenario name: an evidence-mode model
/// over the fake platform, reporting a Windows host so every row including
/// the installer handoff has its projection. Not gated to debug builds: the
/// gallery module itself is compiled into release builds, so gating only this
/// helper broke the release desktop build (E0432 in gallery.rs).
pub(crate) fn gallery_model(name: &str) -> Option<Arc<AppModel>> {
    use snackpilot_model::{Platform, UpdateHostInfo};

    let state = gallery_state(name)?;
    let platform = Arc::new(snackpilot_model::FakePlatform::new());
    platform.set_update_host(Some(UpdateHostInfo {
        platform: "windows",
        architecture: "x64",
        os_version: Some("10.0.26100".into()),
        installed_version: "2.2.0".into(),
        installed_build: "41".into(),
    }));
    let dir = std::env::temp_dir().join(format!(
        "snackpilot-updates-gallery-{}-{}",
        std::process::id(),
        name
    ));
    std::fs::create_dir_all(&dir).ok()?;
    let model = AppModel::new(dir, platform as Arc<dyn Platform>).ok()?;
    model.set_evidence_mode(true);
    model.set_update_state(state);
    Some(Arc::new(model))
}

#[cfg(test)]
mod tests {
    use super::{
        action_dispatch, desktop_datenschutz_text, gallery_state_name, handoff_kind,
        quit_after_handoff, UpdateDispatch, SWITCH_HINT,
    };
    use crate::components::button::ButtonKind;
    use snackpilot_model::{
        AppModel, HandoffOutcome, Platform, UpdateAction, UpdateFailure, UpdateHostInfo,
        UpdateState, VerifiedUpdate,
    };
    use std::path::PathBuf;
    use std::sync::Arc;

    fn model(tag: &str, windows: bool) -> Arc<AppModel> {
        let platform = Arc::new(snackpilot_model::FakePlatform::new());
        if windows {
            platform.set_update_host(Some(UpdateHostInfo {
                platform: "windows",
                architecture: "x64",
                os_version: Some("10.0.26100".into()),
                installed_version: "2.2.0".into(),
                installed_build: "41".into(),
            }));
        }
        let dir = std::env::temp_dir().join(format!(
            "snackpilot-updates-screen-{tag}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Arc::new(AppModel::new(dir, platform as Arc<dyn Platform>).unwrap())
    }

    fn verified() -> VerifiedUpdate {
        VerifiedUpdate {
            version: "2.3.0".into(),
            artifact: PathBuf::from("/tmp/SnackPilot-2.3.0-windows-x64-setup.exe"),
            prepared: None,
        }
    }

    /// The body and the action row are the model's projection, so a rendered
    /// detail cannot show a line or a button the state machine does not (D7).
    /// @req REQ-UPD-15
    #[test]
    fn updates_detail_renders_the_model_view() {
        let model = model("view", true);
        model.set_update_state(UpdateState::Ready {
            update: verified(),
            handoff: HandoffOutcome::QuitRequested,
        });
        let view = model.update_view();
        assert_eq!(view.installed, "Version 2.2.0 (41)");
        assert_eq!(
            SWITCH_HINT,
            "Beim Start und alle 24 Stunden. Es wird nur GitHub kontaktiert."
        );
        let dispatch = action_dispatch(UpdateAction::InstallAndQuit, &view, &model.update_state());
        assert_eq!(
            dispatch,
            Some(UpdateDispatch::Handoff(HandoffOutcome::QuitRequested))
        );
        assert_eq!(
            handoff_kind(HandoffOutcome::QuitRequested),
            ButtonKind::Primary
        );
        assert_eq!(
            handoff_kind(HandoffOutcome::Revealed),
            ButtonKind::Secondary
        );
    }

    /// A download in flight shows its progress and offers no action, so
    /// nothing can be pressed twice (REQ-UPD-16).
    /// @req REQ-UPD-16
    #[test]
    fn buttons_disable_while_running() {
        let model = model("running", true);
        model.set_update_state(UpdateState::Verifying {
            candidate: snackpilot_core::update::Candidate {
                manifest: snackpilot_core::update::Manifest {
                    schema: 1,
                    platform: "windows".into(),
                    architecture: "x64".into(),
                    version: "2.3.0".into(),
                    build: "42".into(),
                    minimum_os: "10.0.22000".into(),
                    artifact: "a.exe".into(),
                    size: 1,
                    sha256: "0".repeat(64),
                    notes: String::new(),
                    notes_url: String::new(),
                    tag: "desktop/v2.3.0".into(),
                },
                artifact_url: String::new(),
            },
        });
        let view = model.update_view();
        assert!(view.actions.is_empty());
        assert!(view.actions_disabled());
        assert_eq!(
            action_dispatch(UpdateAction::Check, &view, &model.update_state()),
            None
        );
    }

    /// The gallery's offline scenarios name every state the detail has a row
    /// for, without a host or a network.
    /// @req REQ-UPD-15
    #[test]
    fn gallery_lists_every_detail_state() {
        assert_eq!(super::GALLERY_STATES.len(), 15);
        for name in super::GALLERY_STATES {
            let Some(state) = super::gallery_state(name) else {
                panic!("{name} has a scenario");
            };
            assert_eq!(super::gallery_state_name(&state), *name);
            let Some(model) = super::gallery_model(name) else {
                panic!("{name} builds an offline model");
            };
            assert!(
                !model.update_view().state_line.is_empty(),
                "{name} has a line"
            );
            assert!(!model.update_supported(), "{name} stays offline");
        }
    }

    /// Only the Windows installer handoff quits, and only on the change that
    /// enters it; the macOS reveal keeps the app running (REQ-UPD-07, D7).
    /// @req REQ-UPD-07
    #[test]
    fn only_the_installer_handoff_quits() {
        let quitting = UpdateState::HandedOff {
            version: "2.3.0".into(),
            outcome: HandoffOutcome::QuitRequested,
            update: verified(),
        };
        let revealed = UpdateState::HandedOff {
            version: "2.3.0".into(),
            outcome: HandoffOutcome::Revealed,
            update: verified(),
        };
        assert!(quit_after_handoff(&UpdateState::Idle, &quitting));
        assert!(quit_after_handoff(
            &UpdateState::Ready {
                update: verified(),
                handoff: HandoffOutcome::QuitRequested,
            },
            &quitting
        ));
        assert!(
            !quit_after_handoff(&quitting, &quitting),
            "once, not per paint"
        );
        assert!(!quit_after_handoff(&UpdateState::Idle, &revealed));
        assert!(!quit_after_handoff(
            &UpdateState::Idle,
            &UpdateState::Checking
        ));
    }

    /// The desktop alert names the GitHub check only where the updater is
    /// live; the mobile body is the approved paragraph, unchanged (REQ-UPD-20).
    /// @req REQ-UPD-20
    #[test]
    fn datenschutz_sentence_only_where_supported() {
        let desktop = desktop_datenschutz_text(Some(crate::desktop::UPDATE_PRIVACY_SENTENCE));
        assert!(desktop.contains(crate::desktop::UPDATE_PRIVACY_SENTENCE));
        assert!(desktop.contains("selbst gehostete Aptabase-Instanz"));
        let unsupported = desktop_datenschutz_text(None);
        assert_eq!(unsupported, crate::desktop::privacy_message());
        assert!(!unsupported.contains("GitHub"));
    }

    /// Each failure names its own row of the D7 table.
    /// @req REQ-UPD-15
    #[test]
    fn failure_rows_are_distinguished() {
        for (stage, name) in [
            (UpdateFailure::Check, "failed-check"),
            (UpdateFailure::Download, "failed-download"),
            (UpdateFailure::Verify, "failed-verify"),
            (UpdateFailure::Handoff, "failed-handoff"),
        ] {
            assert_eq!(
                gallery_state_name(&UpdateState::Failed {
                    stage,
                    reason: String::new(),
                    retry: None,
                }),
                name
            );
        }
    }
}
