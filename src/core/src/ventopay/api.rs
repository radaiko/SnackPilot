//! Ventopay operations orchestration (02-ventopay-scraping §3-§5). DO NOT MODIFY SEQUENCES.
use crate::datetime::{format_ventopay_date, Clock, SystemClock};
use crate::domain::{Credentials, VentopayTransaction};
use crate::error::{CoreError, CoreResult};
use crate::http::Transport;
use crate::ventopay::client::VentopayClient;
use crate::ventopay::parser;
use crate::ventopay::{
    VENTOPAY_COMPANY_ID, VENTOPAY_LOGIN_URL, VENTOPAY_LOGOUT_URL, VENTOPAY_TRANSACTIONS_URL,
};
use std::sync::{Arc, Mutex};

pub struct VentopayApi {
    client: VentopayClient,
    logged_in: Mutex<bool>,
    credentials: Mutex<Option<Credentials>>,
}

impl VentopayApi {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self {
            client: VentopayClient::new(transport),
            logged_in: Mutex::new(false),
            credentials: Mutex::new(None),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        *self.logged_in.lock().unwrap()
    }

    /// §3 — GET state → 11-field POST → verify. Login does NOT reset the jar (§4 note).
    pub async fn login(&self, creds: Credentials) -> CoreResult<()> {
        let html = self.client.get(VENTOPAY_LOGIN_URL, &[]).await?;
        let s = parser::extract_aspnet_state(&html)?;
        let resp = self
            .client
            .post_form(
                VENTOPAY_LOGIN_URL,
                vec![
                    ("__LASTFOCUS".into(), s.last_focus),
                    ("__EVENTTARGET".into(), s.event_target),
                    ("__EVENTARGUMENT".into(), s.event_argument),
                    ("__VIEWSTATE".into(), s.viewstate),
                    ("__VIEWSTATEGENERATOR".into(), s.viewstate_generator),
                    ("__EVENTVALIDATION".into(), s.event_validation),
                    ("DropDownList1".into(), VENTOPAY_COMPANY_ID.into()),
                    ("TxtUsername".into(), creds.username.clone()),
                    ("TxtPassword".into(), creds.password.clone()),
                    ("BtnLogin".into(), "Login".into()),
                    ("languageRadio".into(), "DE".into()),
                ],
            )
            .await?;
        if !parser::is_logged_in(&resp) {
            return Err(CoreError::LoginFailed {
                detail: "Ventopay login failed: invalid credentials or account blocked".into(),
            });
        }
        *self.logged_in.lock().unwrap() = true;
        *self.credentials.lock().unwrap() = Some(creds);
        Ok(())
    }

    /// §4 — re-login if not authenticated; no creds → session expired.
    /// (v1 wording "Ventopay session expired and no credentials saved" is swallowed by
    /// 03-features/billing; mapped to CoreError::SessionExpired.)
    async fn ensure_session(&self) -> CoreResult<()> {
        if *self.logged_in.lock().unwrap() {
            return Ok(());
        }
        let creds = self.credentials.lock().unwrap().clone();
        match creds {
            Some(c) => self.login(c).await,
            None => Err(CoreError::SessionExpired),
        }
    }

    /// §5 — transactions with dd.MM.yyyy params and a single expiry-retry.
    pub async fn get_transactions(
        &self,
        from_date_key: &str,
        until_date_key: &str,
    ) -> CoreResult<Vec<VentopayTransaction>> {
        self.ensure_session().await?;
        let from = format_ventopay_date(from_date_key);
        let until = format_ventopay_date(until_date_key);
        let params = [("fromDate", from.as_str()), ("untilDate", until.as_str())];

        let mut html = self.client.get(VENTOPAY_TRANSACTIONS_URL, &params).await?;
        if !parser::is_logged_in(&html) {
            *self.logged_in.lock().unwrap() = false;
            self.ensure_session().await?;
            html = self.client.get(VENTOPAY_TRANSACTIONS_URL, &params).await?;
            // §4: retry response parsed without a second logged-in check.
        }
        Ok(parser::parse_transactions(
            &html,
            SystemClock.now_epoch_ms(),
        ))
    }

    /// §4 — best-effort logout; clears local session.
    pub async fn logout(&self) -> CoreResult<()> {
        let _ = self.client.get(VENTOPAY_LOGOUT_URL, &[]).await;
        *self.logged_in.lock().unwrap() = false;
        *self.credentials.lock().unwrap() = None;
        self.client.reset();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{CapturingTransport, HttpResponse, Method, RequestBody};
    use std::sync::Arc;

    const LOGIN_PAGE: &str = include_str!("../../tests/fixtures/ventopay/login-page.html");
    const LOGIN_SUCCESS: &str = include_str!("../../tests/fixtures/ventopay/login-success.html");
    const TX_PAGE: &str = include_str!("../../tests/fixtures/ventopay/transactions-page.html");

    fn ok(body: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: vec![],
            body: body.into(),
        }
    }

    async fn logged_in(t: &Arc<CapturingTransport>) -> VentopayApi {
        t.queue_response(ok(LOGIN_PAGE));
        t.queue_response(ok(LOGIN_SUCCESS));
        let api = VentopayApi::new(t.clone());
        api.login(Credentials {
            username: "u".into(),
            password: "p".into(),
        })
        .await
        .unwrap();
        api
    }

    #[tokio::test]
    async fn login_posts_11_ordered_urlencoded_fields() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok(LOGIN_PAGE));
        t.queue_response(ok(LOGIN_SUCCESS));
        let api = VentopayApi::new(t.clone());
        api.login(Credentials {
            username: "u".into(),
            password: "p".into(),
        })
        .await
        .unwrap();
        assert!(api.is_authenticated());

        let post = &t.requests()[1];
        assert_eq!(post.method, Method::Post);
        assert_eq!(post.url, "https://my.ventopay.com/mocca.website/Login.aspx");
        match &post.body {
            Some(RequestBody::Form(f)) => {
                let names: Vec<&str> = f.iter().map(|(k, _)| k.as_str()).collect();
                assert_eq!(
                    names,
                    [
                        "__LASTFOCUS",
                        "__EVENTTARGET",
                        "__EVENTARGUMENT",
                        "__VIEWSTATE",
                        "__VIEWSTATEGENERATOR",
                        "__EVENTVALIDATION",
                        "DropDownList1",
                        "TxtUsername",
                        "TxtPassword",
                        "BtnLogin",
                        "languageRadio",
                    ]
                );
                let val = |k: &str| f.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
                assert_eq!(
                    val("DropDownList1"),
                    Some("0da8d3ec-0178-47d5-9ccd-a996f04acb61")
                );
                assert_eq!(val("BtnLogin"), Some("Login"));
                assert_eq!(val("languageRadio"), Some("DE"));
                assert_eq!(
                    val("__VIEWSTATE"),
                    Some("VIEWSTATE-TOKEN-LONG-BASE64-STRING-ABC123")
                );
            }
            _ => panic!("expected url-encoded form"),
        }
    }

    #[tokio::test]
    async fn login_failure_raises_ventopay_message() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok(LOGIN_PAGE));
        t.queue_response(ok("<html>login form again</html>"));
        let api = VentopayApi::new(t.clone());
        let err = api
            .login(Credentials {
                username: "u".into(),
                password: "x".into(),
            })
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Ventopay login failed: invalid credentials or account blocked"
        );
        assert!(!api.is_authenticated());
    }

    #[tokio::test]
    async fn logout_clears_state() {
        let t = Arc::new(CapturingTransport::new());
        let api = logged_in(&t).await;
        t.queue_response(ok("bye"));
        api.logout().await.unwrap();
        assert!(!api.is_authenticated());
    }

    #[tokio::test]
    async fn get_transactions_formats_dates_and_parses() {
        let t = Arc::new(CapturingTransport::new());
        let api = logged_in(&t).await;
        t.queue_response(ok(TX_PAGE));
        let txs = api
            .get_transactions("2026-02-01", "2026-02-28")
            .await
            .unwrap();
        assert_eq!(txs.len(), 5);
        assert_eq!(
            t.requests()[2].url,
            "https://my.ventopay.com/mocca.website/Transaktionen.aspx?fromDate=01.02.2026&untilDate=28.02.2026"
        );
    }

    #[tokio::test]
    async fn get_transactions_retries_once_on_expiry() {
        let t = Arc::new(CapturingTransport::new());
        let api = logged_in(&t).await;
        t.queue_response(ok("<html>session expired, no logout link</html>"));
        t.queue_response(ok(LOGIN_PAGE));
        t.queue_response(ok(LOGIN_SUCCESS));
        t.queue_response(ok(TX_PAGE));
        let txs = api
            .get_transactions("2026-02-01", "2026-02-28")
            .await
            .unwrap();
        assert_eq!(txs.len(), 5);
    }
}
