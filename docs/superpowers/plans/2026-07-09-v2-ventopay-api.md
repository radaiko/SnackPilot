# SnackPilot v2 — Ventopay API (Phase 1c-iii) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `ventopay::api` — the orchestration wiring `ventopay::client` + `ventopay::parser` into the full Ventopay operations: login (GET state → 11-field POST → verify), `ensure_session`, `get_transactions` (`dd.MM.yyyy` params + expiry-retry-once), and logout — per `docs/requirements/02-ventopay-scraping.md` §3–§5, sequence-tested with the `CapturingTransport`.

**Architecture:** `src/core/src/ventopay/api.rs` holds `VentopayApi` owning a `VentopayClient`, an in-memory `logged_in: Mutex<bool>`, and `credentials: Mutex<Option<Credentials>>`. `now` for the transaction parser comes from `SystemClock` (surviving transactions in tests carry real timestamps, so the value is irrelevant to assertions). Login deliberately does NOT reset the cookie jar (stale cookies ride along on re-login, 02 §4 verification note).

**Tech Stack:** Rust 2021; `ventopay::{client, parser}`, `datetime::{format_ventopay_date, Clock, SystemClock}`, foundation `domain::{Credentials, VentopayTransaction}`, `error`, constants. No new deps.

## Global Constraints

- **Baseline:** v1.4.5 (`main` @ 6997c44); spec `docs/requirements/02-ventopay-scraping.md` §3–§5. **DO NOT MODIFY THE SEQUENCES.**
- **Crate:** `src/core/` on the `v2` worktree `/Users/radaiko/dev/private/SnackPilot-v2`. Test with `cd src/core && cargo test`.
- **Login (§3):** GET `Login.aspx`; `extract_aspnet_state`; POST `Login.aspx` url-encoded with EXACTLY the 11 fields in order `__LASTFOCUS, __EVENTTARGET, __EVENTARGUMENT, __VIEWSTATE, __VIEWSTATEGENERATOR, __EVENTVALIDATION, DropDownList1=<UUID>, TxtUsername, TxtPassword, BtnLogin="Login", languageRadio="DE"`; verify `is_logged_in`; failure → `LoginFailed { message: "Ventopay login failed: invalid credentials or account blocked" }` leaving cached state unchanged; login does NOT reset the jar.
- **ensure_session (§4):** already logged in → ok; else re-login with cached creds; no creds → session-expired (mapped to `CoreError::SessionExpired` — v1's exact text "Ventopay session expired and no credentials saved" is swallowed by 03-features/billing, so the variant text suffices; document the divergence).
- **get_transactions (§5):** `ensure_session` first; GET `Transaktionen.aspx?fromDate=&untilDate=` (`dd.MM.yyyy` via `format_ventopay_date`); if the response is not `is_logged_in`, mark expired, `ensure_session`, retry the GET ONCE, parse the retry response WITHOUT a second check; parse via `parse_transactions`.
- **logout (§4):** GET `Ausloggen.aspx`, swallow errors, clear logged-in flag + credentials, `client.reset()`.
- Commit after each green task. Conventional-commit, scope `core`.

---

## File structure (this plan)

```
src/core/src/ventopay/
├── mod.rs      # + pub mod api;
└── api.rs      # VentopayApi + tests
```

---

### Task 1: `VentopayApi` skeleton + `login` + `ensure_session` + `logout`

**Files:**
- Modify: `src/core/src/ventopay/mod.rs`
- Create: `src/core/src/ventopay/api.rs`

**Interfaces:**
- Produces:
  - `pub struct VentopayApi { client: VentopayClient, logged_in: Mutex<bool>, credentials: Mutex<Option<Credentials>> }`
  - `pub fn new(transport: Arc<dyn Transport>) -> Self`, `pub fn is_authenticated(&self) -> bool`
  - `pub async fn login(&self, creds: Credentials) -> CoreResult<()>`
  - `async fn ensure_session(&self) -> CoreResult<()>`
  - `pub async fn logout(&self) -> CoreResult<()>`

- [ ] **Step 1: Write failing tests**

Create `src/core/src/ventopay/api.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{CapturingTransport, HttpResponse, Method, RequestBody};
    use std::sync::Arc;

    const LOGIN_PAGE: &str = include_str!("../../tests/fixtures/ventopay/login-page.html");
    const LOGIN_SUCCESS: &str = include_str!("../../tests/fixtures/ventopay/login-success.html");

    fn ok(body: &str) -> HttpResponse {
        HttpResponse { status: 200, headers: vec![], body: body.into() }
    }

    #[tokio::test]
    async fn login_posts_11_ordered_urlencoded_fields() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok(LOGIN_PAGE));    // GET Login.aspx
        t.queue_response(ok(LOGIN_SUCCESS)); // POST → authenticated
        let api = VentopayApi::new(t.clone());
        api.login(Credentials { username: "u".into(), password: "p".into() }).await.unwrap();
        assert!(api.is_authenticated());

        let post = &t.requests()[1];
        assert_eq!(post.method, Method::Post);
        assert_eq!(post.url, "https://my.ventopay.com/mocca.website/Login.aspx");
        match &post.body {
            Some(RequestBody::Form(f)) => {
                let names: Vec<&str> = f.iter().map(|(k, _)| k.as_str()).collect();
                assert_eq!(names, [
                    "__LASTFOCUS", "__EVENTTARGET", "__EVENTARGUMENT",
                    "__VIEWSTATE", "__VIEWSTATEGENERATOR", "__EVENTVALIDATION",
                    "DropDownList1", "TxtUsername", "TxtPassword", "BtnLogin", "languageRadio",
                ]);
                let val = |k: &str| f.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
                assert_eq!(val("DropDownList1"), Some("0da8d3ec-0178-47d5-9ccd-a996f04acb61"));
                assert_eq!(val("BtnLogin"), Some("Login"));
                assert_eq!(val("languageRadio"), Some("DE"));
                assert_eq!(val("__VIEWSTATE"), Some("VIEWSTATE-TOKEN-LONG-BASE64-STRING-ABC123"));
            }
            _ => panic!("expected url-encoded form"),
        }
    }

    #[tokio::test]
    async fn login_failure_raises_ventopay_message() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok(LOGIN_PAGE));
        t.queue_response(ok("<html>login form again</html>")); // no Ausloggen link
        let api = VentopayApi::new(t.clone());
        let err = api.login(Credentials { username: "u".into(), password: "x".into() }).await.unwrap_err();
        assert_eq!(err.to_string(), "Ventopay login failed: invalid credentials or account blocked");
        assert!(!api.is_authenticated());
    }

    #[tokio::test]
    async fn logout_clears_state() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok(LOGIN_PAGE));
        t.queue_response(ok(LOGIN_SUCCESS));
        let api = VentopayApi::new(t.clone());
        api.login(Credentials { username: "u".into(), password: "p".into() }).await.unwrap();
        t.queue_response(ok("bye"));
        api.logout().await.unwrap();
        assert!(!api.is_authenticated());
    }
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib ventopay::api`
Expected: FAIL — `VentopayApi` not defined.

- [ ] **Step 3: Implement skeleton + login + ensure_session + logout**

Add to `src/core/src/ventopay/mod.rs`: `pub mod api;`.

Create the top of `src/core/src/ventopay/api.rs`:

```rust
//! Ventopay operations orchestration (02-ventopay-scraping §3-§5). DO NOT MODIFY SEQUENCES.
use crate::datetime::{format_ventopay_date, Clock, SystemClock};
use crate::domain::{Credentials, VentopayTransaction};
use crate::error::{CoreError, CoreResult};
use crate::ventopay::client::VentopayClient;
use crate::ventopay::parser;
use crate::ventopay::{
    VENTOPAY_COMPANY_ID, VENTOPAY_LOGIN_URL, VENTOPAY_LOGOUT_URL, VENTOPAY_TRANSACTIONS_URL,
};
use crate::http::Transport;
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
                message: "Ventopay login failed: invalid credentials or account blocked".into(),
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

    /// §4 — best-effort logout; clears local session.
    pub async fn logout(&self) -> CoreResult<()> {
        let _ = self.client.get(VENTOPAY_LOGOUT_URL, &[]).await;
        *self.logged_in.lock().unwrap() = false;
        *self.credentials.lock().unwrap() = None;
        self.client.reset();
        Ok(())
    }
}
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib ventopay::api`
Expected: the three tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/src/ventopay
git commit -m "feat(core): ventopay::api — login (11-field), ensure_session, logout"
```

---

### Task 2: `get_transactions` (date params + expiry-retry-once)

**Files:**
- Modify: `src/core/src/ventopay/api.rs`

**Interfaces:**
- Produces: `pub async fn get_transactions(&self, from_date_key: &str, until_date_key: &str) -> CoreResult<Vec<VentopayTransaction>>` — `from_date_key`/`until_date_key` are `"YYYY-MM-DD"`, formatted to `dd.MM.yyyy` for the query (§5).

- [ ] **Step 1: Write failing tests**

Add to `tests`:

```rust
const TX_PAGE: &str = include_str!("../../tests/fixtures/ventopay/transactions-page.html");

async fn logged_in(t: &Arc<CapturingTransport>) -> VentopayApi {
    t.queue_response(ok(LOGIN_PAGE));
    t.queue_response(ok(LOGIN_SUCCESS));
    let api = VentopayApi::new(t.clone());
    api.login(Credentials { username: "u".into(), password: "p".into() }).await.unwrap();
    api
}

#[tokio::test]
async fn get_transactions_formats_dates_and_parses() {
    let t = Arc::new(CapturingTransport::new());
    let api = logged_in(&t).await;
    t.queue_response(ok(TX_PAGE));
    let txs = api.get_transactions("2026-02-01", "2026-02-28").await.unwrap();
    assert_eq!(txs.len(), 5); // Gourmet one filtered
    // the GET carried dd.MM.yyyy params
    assert_eq!(
        t.requests()[2].url,
        "https://my.ventopay.com/mocca.website/Transaktionen.aspx?fromDate=01.02.2026&untilDate=28.02.2026"
    );
}

#[tokio::test]
async fn get_transactions_retries_once_on_expiry() {
    let t = Arc::new(CapturingTransport::new());
    let api = logged_in(&t).await;
    // first transactions GET returns a not-logged-in page → triggers re-login + retry
    t.queue_response(ok("<html>session expired, no logout link</html>"));
    t.queue_response(ok(LOGIN_PAGE));    // re-login GET
    t.queue_response(ok(LOGIN_SUCCESS)); // re-login POST
    t.queue_response(ok(TX_PAGE));       // retry transactions GET
    let txs = api.get_transactions("2026-02-01", "2026-02-28").await.unwrap();
    assert_eq!(txs.len(), 5);
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib ventopay::api`
Expected: FAIL.

- [ ] **Step 3: Implement `get_transactions`**

Add inside `impl VentopayApi`:

```rust
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
        Ok(parser::parse_transactions(&html, SystemClock.now_epoch_ms()))
    }
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib ventopay::api`
Expected: all tests PASS.

- [ ] **Step 5: Full suite + fmt/clippy + commit**

Run: `cd src/core && cargo test && cargo fmt && cargo clippy --all-targets`
Expected: full suite green; clippy clean.

```bash
git add src/core
git commit -m "feat(core): ventopay::api — get_transactions (dd.MM.yyyy params, expiry-retry-once); completes Ventopay"
```

---

## Post-plan

This completes the entire scraping side of the core (`gourmet::*` + `ventopay::*`). Next: **1d feature services** (`features::{menus,orders,billing}` + `storage::cache`), **1e notify+demo**, **1f UniFFI facade + bindings + CI**.

## Self-review notes

- **Spec coverage:** §3 (login 11 fields ordered + verify), §4 (ensure_session re-login, logout swallow+reset, jar-not-reset-on-login), §5 (get_transactions date params + expiry-retry-once + parse-without-second-check).
- **Type consistency:** `VentopayApi::{new,login,get_transactions,logout,is_authenticated}` are the surface the UniFFI facade + `features::billing` consume. `format_ventopay_date` and `parse_transactions` come from prior phases.
- **SessionExpired wording divergence:** documented; billing swallows Ventopay errors so the exact v1 text ("...and no credentials saved") never surfaces to users.
