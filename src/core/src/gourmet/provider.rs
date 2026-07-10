//! Swappable Gourmet backend: live scraping by default, demo data after a magic-credential
//! login (demo-mode.md §1). The three feature stores hold this instead of `GourmetApi`; the
//! facade calls `enter_demo`. Method names mirror `GourmetApi`, so store call sites are
//! unchanged, and the ban-critical live path stays untouched.
use crate::datetime::Clock;
use crate::demo::gourmet::DemoGourmetApi;
use crate::domain::{Bill, Credentials, GourmetUserInfo, MenuItem, OrderedMenu};
use crate::error::CoreResult;
use crate::gourmet::api::GourmetApi;
use std::sync::{Arc, RwLock};

/// Owned snapshot of the active backend — cloned out of the lock so awaits never hold it.
enum Handle {
    Live(Arc<GourmetApi>),
    Demo(Arc<DemoGourmetApi>),
}

pub struct GourmetProvider {
    live: Arc<GourmetApi>,
    clock: Arc<dyn Clock>,
    demo: RwLock<Option<Arc<DemoGourmetApi>>>,
}

impl GourmetProvider {
    pub fn new(live: Arc<GourmetApi>, clock: Arc<dyn Clock>) -> Self {
        Self {
            live,
            clock,
            demo: RwLock::new(None),
        }
    }

    /// Swap to a fresh demo backend (magic-credential login, §1).
    pub fn enter_demo(&self) {
        *self.demo.write().unwrap() = Some(Arc::new(DemoGourmetApi::new(self.clock.clone())));
    }

    /// Return to the live backend — a subsequent login with real (non-demo) credentials must
    /// leave demo mode, otherwise every later login keeps serving demo data (§1).
    pub fn exit_demo(&self) {
        *self.demo.write().unwrap() = None;
    }

    pub fn is_demo(&self) -> bool {
        self.demo.read().unwrap().is_some()
    }

    fn current(&self) -> Handle {
        match &*self.demo.read().unwrap() {
            Some(d) => Handle::Demo(d.clone()),
            None => Handle::Live(self.live.clone()),
        }
    }

    pub fn user_info(&self) -> Option<GourmetUserInfo> {
        match self.current() {
            Handle::Live(a) => a.user_info(),
            Handle::Demo(d) => d.user_info(),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        match self.current() {
            Handle::Live(a) => a.is_authenticated(),
            Handle::Demo(d) => d.is_authenticated(),
        }
    }

    pub async fn login(&self, creds: Credentials) -> CoreResult<GourmetUserInfo> {
        match self.current() {
            Handle::Live(a) => a.login(creds).await,
            Handle::Demo(d) => d.login(creds).await,
        }
    }

    pub async fn get_menus(&self) -> CoreResult<Vec<MenuItem>> {
        match self.current() {
            Handle::Live(a) => a.get_menus().await,
            Handle::Demo(d) => d.get_menus().await,
        }
    }

    pub async fn get_orders(&self) -> CoreResult<Vec<OrderedMenu>> {
        match self.current() {
            Handle::Live(a) => a.get_orders().await,
            Handle::Demo(d) => d.get_orders().await,
        }
    }

    pub async fn add_to_cart(&self, items: Vec<(String, String)>) -> CoreResult<()> {
        match self.current() {
            Handle::Live(a) => a.add_to_cart(items).await,
            Handle::Demo(d) => d.add_to_cart(items).await,
        }
    }

    pub async fn confirm_orders(&self) -> CoreResult<()> {
        match self.current() {
            Handle::Live(a) => a.confirm_orders().await,
            Handle::Demo(d) => d.confirm_orders().await,
        }
    }

    pub async fn cancel_orders(&self, position_ids: Vec<String>) -> CoreResult<()> {
        match self.current() {
            Handle::Live(a) => a.cancel_orders(position_ids).await,
            Handle::Demo(d) => d.cancel_orders(position_ids).await,
        }
    }

    pub async fn get_billings(&self, check_last_month_number: &str) -> CoreResult<Vec<Bill>> {
        match self.current() {
            Handle::Live(a) => a.get_billings(check_last_month_number).await,
            Handle::Demo(d) => d.get_billings(check_last_month_number).await,
        }
    }

    pub async fn logout(&self) -> CoreResult<()> {
        match self.current() {
            Handle::Live(a) => a.logout().await,
            Handle::Demo(d) => d.logout().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::FixedClock;
    use crate::http::CapturingTransport;

    fn provider() -> GourmetProvider {
        let tx = Arc::new(CapturingTransport::new());
        GourmetProvider::new(Arc::new(GourmetApi::new(tx)), Arc::new(FixedClock::new(0)))
    }

    #[test]
    fn exit_demo_returns_to_live_backend() {
        let p = provider();
        assert!(!p.is_demo(), "starts on the live backend");
        p.enter_demo();
        assert!(p.is_demo(), "demo backend after enter_demo");
        // Regression: a later real login must escape demo, else it keeps serving demo data.
        p.exit_demo();
        assert!(!p.is_demo(), "back on the live backend after exit_demo");
    }
}
