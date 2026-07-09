//! Demo Ventopay backend — offline transactions swapped in on magic-credential login
//! (demo-mode.md §4). Mirrors the live `VentopayApi` surface; ZERO network. Stateless except
//! a `logged_in` flag.
use crate::datetime::Clock;
use crate::demo::data::generate_demo_transactions;
use crate::domain::{Credentials, VentopayTransaction};
use crate::error::CoreResult;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct DemoVentopayApi {
    clock: Arc<dyn Clock>,
    logged_in: AtomicBool,
}

impl DemoVentopayApi {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            logged_in: AtomicBool::new(false),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.logged_in.load(Ordering::SeqCst)
    }

    /// Ignores arguments; never fails; sets logged in (§4).
    pub async fn login(&self, _creds: Credentials) -> CoreResult<()> {
        self.logged_in.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub async fn get_transactions(
        &self,
        from_date_key: &str,
        until_date_key: &str,
    ) -> CoreResult<Vec<VentopayTransaction>> {
        Ok(generate_demo_transactions(
            self.clock.as_ref(),
            from_date_key,
            until_date_key,
        ))
    }

    pub async fn logout(&self) -> CoreResult<()> {
        self.logged_in.store(false, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::FixedClock;

    #[tokio::test]
    async fn login_logout_toggles_auth() {
        let api = DemoVentopayApi::new(Arc::new(FixedClock::new(1_752_057_600_000)));
        assert!(!api.is_authenticated());
        api.login(Credentials {
            username: "demo".into(),
            password: "demo1234!".into(),
        })
        .await
        .unwrap();
        assert!(api.is_authenticated());
        api.logout().await.unwrap();
        assert!(!api.is_authenticated());
    }

    #[tokio::test]
    async fn transactions_are_deterministic_within_a_month() {
        let api = DemoVentopayApi::new(Arc::new(FixedClock::new(1_752_057_600_000)));
        let a = api
            .get_transactions("2026-07-01", "2026-07-31")
            .await
            .unwrap();
        let b = api
            .get_transactions("2026-07-01", "2026-07-31")
            .await
            .unwrap();
        assert_eq!(a, b);
        assert!(a.iter().all(|t| t.restaurant == "Kaffeeautomat"));
    }
}
