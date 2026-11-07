use std::sync::{Arc, Mutex};

use snackpilot_core::domain::{MenuSnapshot, MonthOption, OrdersSplit};

/// A user-visible error, mapped by [`crate::human_error`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorText {
    pub message: String,
    pub retry: bool,
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

    /// Mutate under the lock, then notify every subscriber with the new state.
    pub fn update(&self, change: &str, f: impl FnOnce(&mut State)) {
        let next = {
            let mut state = self.state.lock().unwrap();
            f(&mut state);
            state.clone()
        };
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
            subscriber(&next);
        }
        crate::runtime::log("state", &format!("update: {change}"));
    }
}
