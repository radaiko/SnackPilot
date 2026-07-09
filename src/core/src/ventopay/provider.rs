//! Swappable Ventopay backend: live scraping by default, demo transactions after a
//! magic-credential login (demo-mode.md §1, §4). BillingStore holds this instead of
//! `VentopayApi`; the facade calls `enter_demo`. Keeps the ban-critical live path untouched.
use crate::datetime::Clock;
use crate::demo::ventopay::DemoVentopayApi;
use crate::domain::{Credentials, VentopayTransaction};
use crate::error::CoreResult;
use crate::ventopay::api::VentopayApi;
use std::sync::{Arc, RwLock};

enum Handle {
    Live(Arc<VentopayApi>),
    Demo(Arc<DemoVentopayApi>),
}

pub struct VentopayProvider {
    live: Arc<VentopayApi>,
    clock: Arc<dyn Clock>,
    demo: RwLock<Option<Arc<DemoVentopayApi>>>,
}

impl VentopayProvider {
    pub fn new(live: Arc<VentopayApi>, clock: Arc<dyn Clock>) -> Self {
        Self {
            live,
            clock,
            demo: RwLock::new(None),
        }
    }

    pub fn enter_demo(&self) {
        *self.demo.write().unwrap() = Some(Arc::new(DemoVentopayApi::new(self.clock.clone())));
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

    pub fn is_authenticated(&self) -> bool {
        match self.current() {
            Handle::Live(a) => a.is_authenticated(),
            Handle::Demo(d) => d.is_authenticated(),
        }
    }

    pub async fn login(&self, creds: Credentials) -> CoreResult<()> {
        match self.current() {
            Handle::Live(a) => a.login(creds).await,
            Handle::Demo(d) => d.login(creds).await,
        }
    }

    pub async fn get_transactions(
        &self,
        from_date_key: &str,
        until_date_key: &str,
    ) -> CoreResult<Vec<VentopayTransaction>> {
        match self.current() {
            Handle::Live(a) => a.get_transactions(from_date_key, until_date_key).await,
            Handle::Demo(d) => d.get_transactions(from_date_key, until_date_key).await,
        }
    }

    pub async fn logout(&self) -> CoreResult<()> {
        match self.current() {
            Handle::Live(a) => a.logout().await,
            Handle::Demo(d) => d.logout().await,
        }
    }
}
