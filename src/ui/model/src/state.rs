use std::sync::{Arc, Mutex};

use snackpilot_core::domain::{CompanyLocation, MenuSnapshot, MonthOption, OrdersSplit};

/// A user-visible error, mapped by [`crate::human_error`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorText {
    pub message: String,
    pub retry: bool,
}

/// A modal alert (Swift `LocationAlert`, AppModel.swift:716-801): one property
/// serves the location flow and the reminder-permission denial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    pub title: String,
    pub message: String,
}

impl Alert {
    /// Exact v1 strings (AppModel.swift:722-729).
    pub(crate) fn location_required() -> Self {
        Self {
            title: "Standort „Immer\" erforderlich".into(),
            message: "Für Standort-Benachrichtigungen muss der Standortzugriff auf „Immer\" \
                      gesetzt werden.\n\nBitte öffne die Einstellungen und wähle unter Standort \
                      „Immer\" aus."
                .into(),
        }
    }

    /// Exact v1 strings (AppModel.swift:684-688, 733-739).
    pub(crate) fn denied() -> Self {
        Self {
            title: "Berechtigung fehlt".into(),
            message: "Benachrichtigungen werden für diese Funktion benötigt. \
                      Bitte in den Einstellungen aktivieren."
                .into(),
        }
    }

    /// Exact v1 strings (AppModel.swift:751-754).
    pub(crate) fn saved() -> Self {
        Self {
            title: "Gespeichert".into(),
            message: "Firmenstandort gesetzt. Du wirst um 8:45 benachrichtigt, wenn du im Büro \
                      bist und nicht bestellt hast."
                .into(),
        }
    }

    /// Exact v1 strings (AppModel.swift:759-762).
    pub(crate) fn error() -> Self {
        Self {
            title: "Fehler".into(),
            message: "Standort konnte nicht ermittelt werden.".into(),
        }
    }
}

/// Light/dark/system choice, persisted under `theme_preference` (themes §1.2).
/// Raw values match the iOS `ThemePreference` (Theme.swift:11-38).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

impl std::str::FromStr for ThemePreference {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "system" => Ok(Self::System),
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            _ => Err(()),
        }
    }
}

/// Daily-reminder time of day. There is no stored time until the feature is
/// first enabled (notifications-daily-reminder.md:213-217); the settings picker
/// starts at 11:00 (AppModel.swift:63, see `REMINDER_PICKER_DEFAULT_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReminderTime {
    pub hour: u8,
    pub minute: u8,
}

/// The complete view-model state, a snapshot the UI renders from.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub busy: bool,
    pub error_text: Option<ErrorText>,
    pub demo_mode: bool,
    pub user_info: Option<snackpilot_core::domain::GourmetUserInfo>,

    pub ventopay_busy: bool,
    pub ventopay_error: Option<ErrorText>,
    pub ventopay_authenticated: bool,

    pub gourmet_authenticated: bool,
    pub menu_snapshot: Option<MenuSnapshot>,
    pub orders_split: Option<OrdersSplit>,
    pub orders_error: Option<ErrorText>,

    pub selected_offset: u8,
    pub month_options: Vec<MonthOption>,
    pub gourmet_month: Option<snackpilot_core::domain::GourmetMonthlyBilling>,
    pub ventopay_month: Option<snackpilot_core::domain::VentopayMonthlyBilling>,
    pub billing_loading: bool,
    /// Billing source filter, UI-only (mirrors BillingView's picker).
    pub billing_source: BillingSource,

    /// Selected menu day (YYYY-MM-DD); None until synced to an available day.
    pub selected_day: Option<String>,
    /// Progress label of an in-flight order submit/cancel, None when idle.
    pub order_progress: Option<String>,
    /// Id of the order whose cancel is in flight (single-flight guard).
    pub cancelling_id: Option<String>,
    /// OrdersView multi-select mode for batch cancellation.
    pub order_selection_mode: bool,
    /// Order ids selected while in [`State::order_selection_mode`].
    pub selected_order_ids: Vec<String>,

    // ---- task 3.3: appearance, reminders, location, diagnostics ----
    /// Light/dark/system choice (themes §1.2); default System.
    pub theme: ThemePreference,
    /// Daily reminder on/off (notifications-daily-reminder §9); default off.
    pub daily_reminder_enabled: bool,
    /// Daily reminder time of day; `None` until the feature is first enabled
    /// (notifications-daily-reminder.md:213-217).
    pub reminder_time: Option<ReminderTime>,
    /// In-flight company-location setup flow (AppModel.swift:717).
    pub location_busy: bool,
    /// Saved company location, the on/off switch for location notifications (§1).
    pub company_location: Option<CompanyLocation>,
    /// Last geofence state mirrored from core (§4).
    pub is_at_company: bool,
    /// The location/reminder alert to present (Swift `locationAlert`).
    pub location_alert: Option<Alert>,
    /// Diagnostic log currently recording (notification-log §2).
    pub log_active: bool,
    /// Diagnostic log has a stored, already-past expiry.
    pub log_expired: bool,
    /// Single-line status label for the diagnostics screen (04-ui-ux §3.8).
    pub log_status: String,
    /// Operator broadcast message (gist), shown as the foreground banner.
    pub broadcast: Option<String>,
    /// Banner dismissed for the current foreground (reset on the next one).
    pub broadcast_dismissed: bool,

    // ---- desktop-ui-redesign task 1.1: workspace identities ----
    /// Navigation instance: which tab is active (desktop-ui-redesign D1).
    pub navigation: crate::navigation::Navigation,
    /// Host kind the model was built for; layout breakpoints only apply on Desktop.
    pub host_kind: crate::desktop::HostKind,
    /// Last content width reported by the host, in pixels.
    pub content_width: f32,
    /// Open settings-detail section; the narrow layout projects it as a pushed screen.
    pub settings_detail: Option<crate::navigation::SettingsDetail>,
    /// Menus list preview, keyed by [`crate::menus::menu_key`].
    pub preview: Option<String>,
    /// Orders list preview, keyed by position id.
    pub order_preview: Option<String>,
    /// Receipts list preview, keyed by `g-{bill_nr}` or `v-{tx.id}`.
    pub receipt_preview: Option<String>,
    /// List cursor per [`crate::navigation::ListKind`], indexed by `index()`.
    pub cursors: [Option<String>; 3],
    /// Orders segment: false = upcoming, true = past.
    pub orders_past: bool,
    /// A text field currently captures keys; workspace shortcuts must not.
    pub text_input_focused: bool,
    /// A modal is open; Esc closes it before anything else.
    pub modal_open: bool,
    /// Identity of the command currently claimed; released by its flow.
    pub claimed_command: Option<String>,
}

impl State {
    pub(crate) fn preview_of(&self, kind: crate::navigation::ListKind) -> Option<&str> {
        let id = match kind {
            crate::navigation::ListKind::Menus => self.preview.as_ref(),
            crate::navigation::ListKind::Orders => self.order_preview.as_ref(),
            crate::navigation::ListKind::Receipts => self.receipt_preview.as_ref(),
        };
        id.map(String::as_str)
    }

    pub(crate) fn set_preview(&mut self, kind: crate::navigation::ListKind, id: Option<String>) {
        match kind {
            crate::navigation::ListKind::Menus => self.preview = id,
            crate::navigation::ListKind::Orders => self.order_preview = id,
            crate::navigation::ListKind::Receipts => self.receipt_preview = id,
        }
    }

    pub(crate) fn cursor_of(&self, kind: crate::navigation::ListKind) -> Option<&str> {
        self.cursors[kind.index()].as_deref()
    }

    pub(crate) fn set_cursor(&mut self, kind: crate::navigation::ListKind, id: Option<String>) {
        self.cursors[kind.index()] = id;
    }

    /// Drops both the preview and the cursor of one list.
    pub(crate) fn clear_list(&mut self, kind: crate::navigation::ListKind) {
        self.set_preview(kind, None);
        self.set_cursor(kind, None);
    }
}

/// Which billing source to show (BillingView's "Alle/Gourmet/Ventopay" picker).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BillingSource {
    #[default]
    All,
    Gourmet,
    Ventopay,
}

pub type Subscriber = Box<dyn Fn(&State) + Send + Sync>;

type SharedSubscriber = Arc<Subscriber>;

/// State store with subscribers; callbacks fire after each change.
pub struct StateHub {
    state: Mutex<State>,
    subscribers: Mutex<Vec<SharedSubscriber>>,
}

impl Default for StateHub {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHub {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State::default()),
            subscribers: Mutex::new(Vec::new()),
        }
    }

    pub fn get(&self) -> State {
        self.state.lock().unwrap().clone()
    }

    /// Registers `callback`, then immediately calls it with the current state.
    /// No lock is held while the callback runs, so a subscriber may itself
    /// subscribe from inside its callback (GPUI views mount during updates)
    /// without deadlocking.
    pub fn subscribe(&self, callback: Subscriber) {
        let callback = Arc::new(callback);
        self.subscribers.lock().unwrap().push(callback.clone());
        let state = self.state.lock().unwrap().clone();
        callback(&state);
    }

    /// Like [`update`](Self::update), but only applies `f` when `test` passes on
    /// the current state, and returns whether it did. Test and mutation happen
    /// under one lock, so two callers cannot both pass the test — this is the
    /// single-flight guard's check-and-set (orders §6.2).
    pub fn try_update(
        &self,
        change: &str,
        test: impl FnOnce(&State) -> bool,
        f: impl FnOnce(&mut State),
    ) -> bool {
        let next = {
            let mut state = self.state.lock().unwrap();
            if !test(&state) {
                return false;
            }
            f(&mut state);
            state.clone()
        };
        self.notify_subscribers(change, &next);
        true
    }

    /// Mutate under the lock, then notify every subscriber with the new state.
    pub fn update(&self, change: &str, f: impl FnOnce(&mut State)) {
        let next = {
            let mut state = self.state.lock().unwrap();
            f(&mut state);
            state.clone()
        };
        self.notify_subscribers(change, &next);
    }

    fn notify_subscribers(&self, change: &str, next: &State) {
        // Clone the list before invoking: a callback may subscribe from inside
        // its own callback without deadlocking on the subscribers mutex.
        let subscribers: Vec<_> = self
            .subscribers
            .lock()
            .unwrap()
            .iter()
            .map(Arc::clone)
            .collect();
        for subscriber in &subscribers {
            subscriber(next);
        }
        crate::runtime::log("state", &format!("update: {change}"));
    }
}
