//! Ventopay operations orchestration (02-ventopay-scraping §3-§5). DO NOT MODIFY SEQUENCES.
use crate::datetime::{ventopay_from, ventopay_until, Clock, SystemClock};
use crate::domain::{Credentials, VentopayTransaction};
use crate::error::{CoreError, CoreResult};
use crate::http::Transport;
use crate::ventopay::client::VentopayClient;
use crate::ventopay::parser;
use crate::ventopay::{
    VENTOPAY_ACCESS_TOKEN_URL, VENTOPAY_LOGIN_URL, VENTOPAY_LOGOUT_URL, VENTOPAY_RESTAURANT_ID,
    VENTOPAY_TRANSACTIONS_URL, VENTOPAY_WEB_URL,
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

    /// §3 — GET shell (cookies) → POST JSON login → GET accessToken → Bearer.
    /// Login does NOT reset the jar (§4 note).
    pub async fn login(&self, creds: Credentials) -> CoreResult<()> {
        self.client.get(VENTOPAY_WEB_URL, &[]).await?;
        let body = serde_json::json!({
            "username": creds.username,
            "password": creds.password,
            "restaurantId": VENTOPAY_RESTAURANT_ID,
            "authEntity": "Person",
        })
        .to_string();
        let resp = match self.client.post_json(VENTOPAY_LOGIN_URL, body).await {
            Ok(r) => r,
            // 401 = wrong credentials or blocked account (02 §3).
            Err(CoreError::Http {
                status: Some(401), ..
            }) => {
                return Err(CoreError::LoginFailed {
                    detail: "Ventopay login failed: invalid credentials or account blocked".into(),
                })
            }
            Err(e) => return Err(e),
        };
        parser::check_login_response(&resp)?;
        let token_json = self.client.get(VENTOPAY_ACCESS_TOKEN_URL, &[]).await?;
        let token = parser::parse_access_token(&token_json)?;
        self.client.set_bearer(Some(token));
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

    /// §5 — bons with ISO fromDate/untilDate and a single 401-retry (full re-login).
    pub async fn get_transactions(
        &self,
        from_date_key: &str,
        until_date_key: &str,
    ) -> CoreResult<Vec<VentopayTransaction>> {
        self.ensure_session().await?;
        let from = ventopay_from(from_date_key);
        let until = ventopay_until(until_date_key);
        let params = [("fromDate", from.as_str()), ("untilDate", until.as_str())];

        let body = match self.client.get(VENTOPAY_TRANSACTIONS_URL, &params).await {
            Ok(b) => b,
            // Gateway rejected the bearer → drop it, full re-login, retry once (02 §4).
            Err(CoreError::Http {
                status: Some(401), ..
            }) => {
                *self.logged_in.lock().unwrap() = false;
                self.client.set_bearer(None);
                self.ensure_session().await?;
                self.client.get(VENTOPAY_TRANSACTIONS_URL, &params).await?
            }
            Err(e) => return Err(e),
        };
        parser::parse_transactions(&body, SystemClock.now_epoch_ms())
    }

    /// §4 — best-effort logout; clears local session.
    pub async fn logout(&self) -> CoreResult<()> {
        let _ = self.client.delete(VENTOPAY_LOGOUT_URL).await;
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

    const BONS: &str = include_str!("../../tests/fixtures/ventopay/bons.json");
    const LOGIN_OK: &str = r#"{"person":{"id":"p1"},"otP_ID":null,"entityType":1}"#;
    const TOKEN_OK: &str = r#"{"accessToken":"TOK-1","expires":1}"#;

    fn ok(body: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: vec![],
            body: body.into(),
        }
    }

    fn status(code: u16) -> HttpResponse {
        HttpResponse {
            status: code,
            headers: vec![],
            body: String::new(),
        }
    }

    /// Queues the three login requests (shell GET, login POST, accessToken GET).
    fn queue_login(t: &Arc<CapturingTransport>) {
        t.queue_response(ok("shell"));
        t.queue_response(ok(LOGIN_OK));
        t.queue_response(ok(TOKEN_OK));
    }

    async fn logged_in(t: &Arc<CapturingTransport>) -> VentopayApi {
        queue_login(t);
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
    async fn login_posts_json_and_attaches_bearer() {
        let t = Arc::new(CapturingTransport::new());
        queue_login(&t);
        let api = VentopayApi::new(t.clone());
        api.login(Credentials {
            username: "u".into(),
            password: "p".into(),
        })
        .await
        .unwrap();
        assert!(api.is_authenticated());

        let reqs = t.requests();
        assert_eq!(reqs.len(), 3);
        assert_eq!(
            reqs[0].url,
            "https://xperience-trumpf.ventopay.cloud" // shell GET seeds the cookie jar
        );
        let post = &reqs[1];
        assert_eq!(post.method, Method::Post);
        assert_eq!(
            post.url,
            "https://xperience-trumpf.ventopay.cloud/auth/login"
        );
        match &post.body {
            Some(RequestBody::Json(b)) => {
                let v: serde_json::Value = serde_json::from_str(b).unwrap();
                assert_eq!(v["username"], "u");
                assert_eq!(v["password"], "p");
                assert_eq!(v["restaurantId"], "0da8d3ec-0178-47d5-9ccd-a996f04acb61");
                assert_eq!(v["authEntity"], "Person");
            }
            _ => panic!("expected json body"),
        }
        assert_eq!(
            reqs[2].url,
            "https://xperience-trumpf.ventopay.cloud/auth/accessToken"
        );

        // Bearer reaches the gateway call.
        t.queue_response(ok(BONS));
        api.get_transactions("2026-02-01", "2026-02-28")
            .await
            .unwrap();
        let bons_req = &t.requests()[3];
        assert!(bons_req
            .headers
            .iter()
            .any(|(k, v)| k == "authorization" && v == "Bearer TOK-1"));
    }

    #[tokio::test]
    async fn login_failure_401_raises_ventopay_message() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok("shell"));
        t.queue_response(status(401));
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
    async fn login_with_2fa_fails() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok("shell"));
        t.queue_response(ok(r#"{"otP_ID":"otp-1"}"#));
        let api = VentopayApi::new(t.clone());
        let err = api
            .login(Credentials {
                username: "u".into(),
                password: "p".into(),
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("two-factor"));
        assert!(!api.is_authenticated());
    }

    #[tokio::test]
    async fn logout_clears_state() {
        let t = Arc::new(CapturingTransport::new());
        let api = logged_in(&t).await;
        t.queue_response(status(204));
        api.logout().await.unwrap();
        assert!(!api.is_authenticated());
        let logout = &t.requests()[3];
        assert_eq!(logout.method, Method::Delete);
        assert_eq!(
            logout.url,
            "https://xperience-trumpf.ventopay.cloud/auth/logout"
        );
    }

    #[tokio::test]
    async fn get_transactions_formats_dates_and_parses() {
        let t = Arc::new(CapturingTransport::new());
        let api = logged_in(&t).await;
        t.queue_response(ok(BONS));
        let txs = api
            .get_transactions("2026-02-01", "2026-02-28")
            .await
            .unwrap();
        assert_eq!(txs.len(), 5);
        assert_eq!(
            t.requests()[3].url,
            "https://gw-trumpf.ventopay.cloud/Customers/v1/person/bons?fromDate=2026-02-01T00%3A00%3A00&untilDate=2026-02-28T23%3A59%3A59"
        );
    }

    #[tokio::test]
    async fn get_transactions_retries_once_on_401() {
        let t = Arc::new(CapturingTransport::new());
        let api = logged_in(&t).await;
        t.queue_response(status(401)); // gateway rejects bearer
        queue_login(&t); // full re-login: shell GET, login POST, accessToken GET
        t.queue_response(ok(BONS)); // retry
        let txs = api
            .get_transactions("2026-02-01", "2026-02-28")
            .await
            .unwrap();
        assert_eq!(txs.len(), 5);
        // 3 login + 1 failed bons + 3 re-login + 1 bons retry
        assert_eq!(t.requests().len(), 8);
    }

    #[tokio::test]
    async fn get_transactions_without_creds_is_session_expired() {
        let t = Arc::new(CapturingTransport::new());
        let api = VentopayApi::new(t);
        let err = api
            .get_transactions("2026-02-01", "2026-02-28")
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "Session expired");
    }
}
