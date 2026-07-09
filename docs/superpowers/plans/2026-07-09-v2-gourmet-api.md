# SnackPilot v2 — Gourmet API (Phase 1b-iii) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `gourmet::api` — the orchestration that wires `gourmet::client` + `gourmet::parser` into the full Gourmet operations: login (with stale-session pre-logout), `ensure_session` re-login, `get_menus` pagination, `get_orders`, `add_to_cart`, `confirm_orders`, `cancel_orders` (edit-mode loop), `get_billings`, and `logout` — the exact request sequences of `docs/requirements/01-gourmet-scraping.md` §6–§11, sequence-tested with the `CapturingTransport` and queued fixture bodies.

**Architecture:** `src/core/src/gourmet/api.rs` holds `GourmetApi`, owning a `GourmetClient` plus in-memory `user_info` and `credentials` (both `Mutex<Option<…>>`). Every operation is a documented request sequence built from `client` calls and `parser` functions; account-of-record behavior (token freshness, edit-mode state machine, re-login) lives here. Tests inject a `CapturingTransport`, queue the exact fixture bodies each step expects, run the operation, and assert both the returned data and the captured request sequence (method/url/body) per 06-testing §6.1.

**Tech Stack:** Rust 2021; `gourmet::{client, parser}`, foundation `http`/`error`/`domain`/`datetime`, `serde_json` (JSON APIs). No new dependencies.

## Global Constraints

- **Baseline:** v1.4.5 (`main` @ 6997c44); authoritative spec `docs/requirements/01-gourmet-scraping.md` §6–§11. **DO NOT MODIFY THE REQUEST SEQUENCES** — deviations ban accounts.
- **Crate:** `src/core/` on the `v2` worktree `/Users/radaiko/dev/private/SnackPilot-v2`. Test with `cd src/core && cargo test`.
- **Endpoint constants (add to `gourmet/mod.rs`, 01 §1):**
  - `GOURMET_LOGIN_URL = "https://alaclickneu.gourmet.at/start/"`
  - `GOURMET_MENUS_URL = "https://alaclickneu.gourmet.at/menus/"` (trailing slash; §8.1)
  - `GOURMET_ORDERS_URL = "https://alaclickneu.gourmet.at/bestellungen/"` (trailing slash; §9)
  - `GOURMET_ADD_TO_CART_URL = "https://alaclickneu.gourmet.at/umbraco/api/AlaCartApi/AddToMenuesCart"`
  - `GOURMET_BILLING_URL = "https://alaclickneu.gourmet.at/umbraco/api/AlaMyBillingApi/GetMyBillings"`
- **Login (§6.2):** exactly 5 multipart fields in order `Username, Password, RememberMe="false", ufprt, __ncforminfo`; tokens from `form:first-of-type`; verify `is_logged_in`; on failure raise `LoginFailed` and DO NOT clear cached creds/user info.
- **Stale-session pre-logout (§6.1):** GET `/start/`; if `is_logged_in`, extract logout tokens, POST `/start/` with `ufprt`+`__ncforminfo` (swallow all errors), re-GET `/start/`.
- **`ensure_session` (§7):** re-login only when `!is_logged_in` and creds cached; no creds → `SessionExpired`. The re-fetch after re-login is done by the caller — and only at HTML-parsing call sites (`get_menus` page 0, `get_orders`, `confirm_orders`, `cancel_orders`); `get_billings` does NOT re-fetch (§7 exception).
- **Pagination (§8.1):** loop page 0..9 (max 10); page 0 = GET `/menus/` bare, page N = `?page=N`; page 0 runs `ensure_session` + re-fetch and extracts user info if not cached (ignore failures); stop after a page with no `menues-next` link.
- **Edit-mode (§9.2):** inverted — `editMode="True"` means NOT in edit mode (posting enters it), `"False"` means IN edit mode (posting exits/confirms). Echo the extracted value.
- **AddToMenuesCart (§10.1):** requires cached user info (`Not logged in` else); group items by first-seen date; JSON key `staffgroupId` (lowercase g); `success != true` → `AddToCartFailed { message }`.
- **GetMyBillings (§10.2):** require cached user info before any request (`Not logged in`); probe GET `/start/` + `ensure_session` (NO re-fetch); POST with `checkLastMonthNumber` string; response `{Billings:[…]}` wrapper OR raw array, neither → `[]`.
- **Commit after each green task.** Conventional-commit, scope `core`.

---

## File structure (this plan)

```
src/core/src/gourmet/
├── mod.rs      # + endpoint constants + pub mod api;
└── api.rs      # GourmetApi + tests (grouped fixture-sequence tests)
```

---

### Task 1: `GourmetApi` skeleton + `login` (incl. stale-session pre-logout) + `ensure_session`

**Files:**
- Modify: `src/core/src/gourmet/mod.rs`
- Create: `src/core/src/gourmet/api.rs`
- Test: inline `#[cfg(test)]`

**Interfaces:**
- Consumes: `GourmetClient`, all `parser` fns, `domain::{Credentials, GourmetUserInfo}`, `error::{CoreError, CoreResult}`, constants.
- Produces:
  - `pub struct GourmetApi { client: GourmetClient, user_info: Mutex<Option<GourmetUserInfo>>, credentials: Mutex<Option<Credentials>> }`
  - `pub fn new(transport: Arc<dyn Transport>) -> Self`
  - `pub fn user_info(&self) -> Option<GourmetUserInfo>` / `pub fn is_authenticated(&self) -> bool`
  - `pub async fn login(&self, creds: Credentials) -> CoreResult<GourmetUserInfo>`
  - `async fn ensure_session(&self, html: &str) -> CoreResult<EnsureOutcome>` where `enum EnsureOutcome { Ready, Refetched }` — `Ready` = html usable as-is; `Refetched` = a re-login happened, caller must re-GET. `SessionExpired` when not logged in and no creds.

- [ ] **Step 1: Write failing login tests**

Create `src/core/src/gourmet/api.rs` with a test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{CapturingTransport, HttpResponse, Method, RequestBody};
    use std::sync::Arc;

    const LOGIN_PAGE: &str = include_str!("../../tests/fixtures/gourmet/login-page.html");
    const LOGIN_SUCCESS: &str = include_str!("../../tests/fixtures/gourmet/login-success.html");
    const LOGIN_FAILED: &str = include_str!("../../tests/fixtures/gourmet/login-failed.html");

    fn ok(body: &str) -> HttpResponse {
        HttpResponse { status: 200, headers: vec![], body: body.into() }
    }

    #[tokio::test]
    async fn login_posts_five_ordered_fields_and_caches_user_info() {
        let t = Arc::new(CapturingTransport::new());
        // Step 0: GET /start/ returns the (not-logged-in) login page → no pre-logout.
        t.queue_response(ok(LOGIN_PAGE));
        // Step 2: POST /start/ → login-success (authenticated, has user info).
        t.queue_response(ok(LOGIN_SUCCESS));
        let api = GourmetApi::new(t.clone());

        let info = api
            .login(Credentials { username: "u".into(), password: "p".into() })
            .await
            .unwrap();
        assert_eq!(info.eater_id, "EATER-TEST-456");
        assert!(api.is_authenticated());

        let reqs = t.requests();
        // req[0] GET /start/, req[1] POST /start/ multipart
        assert_eq!(reqs[1].method, Method::Post);
        assert_eq!(reqs[1].url, "https://alaclickneu.gourmet.at/start/");
        match &reqs[1].body {
            Some(RequestBody::Multipart(f)) => {
                let names: Vec<&str> = f.iter().map(|(k, _)| k.as_str()).collect();
                assert_eq!(names, ["Username", "Password", "RememberMe", "ufprt", "__ncforminfo"]);
                assert_eq!(f[2].1, "false"); // literal string
                assert_eq!(f[3].1, "CSRF-TOKEN-LOGIN-ABC123");
            }
            _ => panic!("expected multipart"),
        }
    }

    #[tokio::test]
    async fn login_failure_raises_and_keeps_no_creds_cleared() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok(LOGIN_PAGE));   // GET /start/
        t.queue_response(ok(LOGIN_FAILED)); // POST /start/ → still login page
        let api = GourmetApi::new(t.clone());
        let err = api
            .login(Credentials { username: "u".into(), password: "bad".into() })
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "Login failed: invalid credentials or account blocked");
    }

    #[tokio::test]
    async fn stale_session_triggers_pre_logout() {
        let t = Arc::new(CapturingTransport::new());
        // GET /start/ returns an authenticated page → pre-logout path.
        t.queue_response(ok(LOGIN_SUCCESS)); // step 0 GET (logged in)
        t.queue_response(ok("<html>bye</html>")); // pre-logout POST
        t.queue_response(ok(LOGIN_PAGE)); // re-GET /start/
        t.queue_response(ok(LOGIN_SUCCESS)); // login POST
        let api = GourmetApi::new(t.clone());
        api.login(Credentials { username: "u".into(), password: "p".into() })
            .await
            .unwrap();
        // 4 requests: GET, pre-logout POST, re-GET, login POST
        assert_eq!(t.requests().len(), 4);
    }
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib gourmet::api`
Expected: FAIL — `GourmetApi` not defined.

- [ ] **Step 3: Add constants and implement skeleton + `login` + `ensure_session`**

Add to `src/core/src/gourmet/mod.rs`:

```rust
pub mod api;
// ...existing client/parser mods and BASE/ORIGIN consts...

pub const GOURMET_LOGIN_URL: &str = "https://alaclickneu.gourmet.at/start/";
pub const GOURMET_MENUS_URL: &str = "https://alaclickneu.gourmet.at/menus/";
pub const GOURMET_ORDERS_URL: &str = "https://alaclickneu.gourmet.at/bestellungen/";
pub const GOURMET_ADD_TO_CART_URL: &str =
    "https://alaclickneu.gourmet.at/umbraco/api/AlaCartApi/AddToMenuesCart";
pub const GOURMET_BILLING_URL: &str =
    "https://alaclickneu.gourmet.at/umbraco/api/AlaMyBillingApi/GetMyBillings";
```

Create the top of `src/core/src/gourmet/api.rs`:

```rust
//! Gourmet operations orchestration (01-gourmet-scraping §6-§11). Wires client + parser.
//! DO NOT MODIFY THE REQUEST SEQUENCES — deviations ban accounts.
use crate::domain::{Credentials, GourmetUserInfo};
use crate::error::{CoreError, CoreResult};
use crate::gourmet::client::GourmetClient;
use crate::gourmet::parser;
use crate::gourmet::{GOURMET_LOGIN_URL, GOURMET_ORDERS_URL};
use crate::http::Transport;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnsureOutcome {
    Ready,
    Refetched,
}

pub struct GourmetApi {
    client: GourmetClient,
    user_info: Mutex<Option<GourmetUserInfo>>,
    credentials: Mutex<Option<Credentials>>,
}

impl GourmetApi {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self {
            client: GourmetClient::new(transport),
            user_info: Mutex::new(None),
            credentials: Mutex::new(None),
        }
    }

    pub fn user_info(&self) -> Option<GourmetUserInfo> {
        self.user_info.lock().unwrap().clone()
    }

    pub fn is_authenticated(&self) -> bool {
        self.user_info.lock().unwrap().is_some()
    }

    /// Full login (§6): stale-session pre-logout, then the 5-field POST, verify, cache.
    pub async fn login(&self, creds: Credentials) -> CoreResult<GourmetUserInfo> {
        // Step 0 — stale-session pre-logout (§6.1).
        let start_html = self.client.get(GOURMET_LOGIN_URL, &[]).await?;
        let login_page = if parser::is_logged_in(&start_html) {
            // best-effort logout; swallow errors, then re-GET.
            if let Ok((ufprt, ncform)) = parser::extract_logout_form_tokens(&start_html) {
                let _ = self
                    .client
                    .post_form(
                        GOURMET_LOGIN_URL,
                        vec![("ufprt".into(), ufprt), ("__ncforminfo".into(), ncform)],
                    )
                    .await;
            }
            self.client.get(GOURMET_LOGIN_URL, &[]).await?
        } else {
            start_html
        };

        // Steps 1-2 — token extraction from the FIRST form + 5-field POST (§6.2).
        let (ufprt, ncform) = parser::extract_form_tokens(&login_page, "form:first-of-type")?;
        let post_html = self
            .client
            .post_form(
                GOURMET_LOGIN_URL,
                vec![
                    ("Username".into(), creds.username.clone()),
                    ("Password".into(), creds.password.clone()),
                    ("RememberMe".into(), "false".into()),
                    ("ufprt".into(), ufprt),
                    ("__ncforminfo".into(), ncform),
                ],
            )
            .await?;

        // Step 3 — verify (§6.2). Failure leaves cached creds/user info untouched (§6.2 note).
        if !parser::is_logged_in(&post_html) {
            return Err(CoreError::LoginFailed {
                message: "Login failed: invalid credentials or account blocked".into(),
            });
        }

        // Step 4 — user info from the response, else re-GET /start/ and extract there.
        let info = match parser::extract_user_info(&post_html) {
            Ok(i) => i,
            Err(_) => {
                let html = self.client.get(GOURMET_LOGIN_URL, &[]).await?;
                parser::extract_user_info(&html)?
            }
        };

        *self.user_info.lock().unwrap() = Some(info.clone());
        *self.credentials.lock().unwrap() = Some(creds);
        Ok(info)
    }

    /// §7 — if `html` is authenticated, Ready; else re-login (creds required) and signal
    /// Refetched so the caller re-GETs; no creds → SessionExpired.
    async fn ensure_session(&self, html: &str) -> CoreResult<EnsureOutcome> {
        if parser::is_logged_in(html) {
            return Ok(EnsureOutcome::Ready);
        }
        let creds = self.credentials.lock().unwrap().clone();
        match creds {
            Some(c) => {
                self.login(c).await?;
                Ok(EnsureOutcome::Refetched)
            }
            None => Err(CoreError::SessionExpired),
        }
    }
}
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib gourmet::api`
Expected: the three login tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/src/gourmet
git commit -m "feat(core): gourmet::api — login (stale-session pre-logout) + ensure_session"
```

---

### Task 2: `get_menus` (pagination) + `get_orders` + `logout`

**Files:**
- Modify: `src/core/src/gourmet/api.rs`, `src/core/src/gourmet/mod.rs` (constants already added in Task 1)

**Interfaces:**
- Consumes: `parser::{parse_menu_items, has_next_menu_page, parse_ordered_menus, extract_user_info}`, `domain::{MenuItem, OrderedMenu}`, `GOURMET_MENUS_URL`.
- Produces:
  - `pub async fn get_menus(&self) -> CoreResult<Vec<MenuItem>>` — pages 0..=9 (max 10); page 0 GET bare + `ensure_session` (re-fetch on Refetched) + best-effort user-info extraction; page N `?page=N`; concatenate; stop after a page with no next link (§8.1).
  - `pub async fn get_orders(&self) -> CoreResult<Vec<OrderedMenu>>` — GET `/bestellungen/` + `ensure_session` (re-fetch), parse (§9.1).
  - `pub async fn logout(&self) -> CoreResult<()>` — GET `/start/`, extract logout tokens, POST 2 fields; **all errors swallowed**; always clear user_info + credentials + `client.reset()` (§11).

- [ ] **Step 1: Write failing tests**

Add to `tests`:

```rust
const MENUS_PAGE_0: &str = include_str!("../../tests/fixtures/gourmet/menus-page-0.html");
const MENUS_PAGE_1: &str = include_str!("../../tests/fixtures/gourmet/menus-page-1.html");
const ORDERS_PAGE: &str = include_str!("../../tests/fixtures/gourmet/orders-page.html");

async fn logged_in_api(t: &Arc<CapturingTransport>) -> GourmetApi {
    // login first (GET /start/ login page, POST → success)
    t.queue_response(ok(LOGIN_PAGE));
    t.queue_response(ok(LOGIN_SUCCESS));
    let api = GourmetApi::new(t.clone());
    api.login(Credentials { username: "u".into(), password: "p".into() }).await.unwrap();
    api
}

#[tokio::test]
async fn get_menus_paginates_until_no_next_link() {
    let t = Arc::new(CapturingTransport::new());
    let api = logged_in_api(&t).await;
    // page 0 has a next link, page 1 does not.
    t.queue_response(ok(MENUS_PAGE_0));
    t.queue_response(ok(MENUS_PAGE_1));
    let items = api.get_menus().await.unwrap();
    assert!(!items.is_empty());

    // the two menu GETs are the 3rd and 4th requests (after login's GET+POST).
    let reqs = t.requests();
    assert_eq!(reqs[2].url, "https://alaclickneu.gourmet.at/menus/");           // page 0 bare
    assert_eq!(reqs[3].url, "https://alaclickneu.gourmet.at/menus/?page=1");    // page 1
    assert_eq!(reqs.len(), 4); // stopped after page 1 (no next link)
}

#[tokio::test]
async fn get_orders_parses_after_session_check() {
    let t = Arc::new(CapturingTransport::new());
    let api = logged_in_api(&t).await;
    t.queue_response(ok(ORDERS_PAGE));
    let orders = api.get_orders().await.unwrap();
    assert!(!orders.is_empty());
    assert_eq!(t.requests()[2].url, "https://alaclickneu.gourmet.at/bestellungen/");
}

#[tokio::test]
async fn logout_clears_state_and_swallows_errors() {
    let t = Arc::new(CapturingTransport::new());
    let api = logged_in_api(&t).await;
    assert!(api.is_authenticated());
    t.queue_response(ok(LOGIN_SUCCESS)); // GET /start/ (has logout form)
    t.queue_response(ok("<html>bye</html>")); // logout POST
    api.logout().await.unwrap();
    assert!(!api.is_authenticated());
    assert!(api.user_info().is_none());
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib gourmet::api`
Expected: FAIL.

- [ ] **Step 3: Implement the three methods**

Add imports at the top of `api.rs`: `use crate::domain::{MenuItem, OrderedMenu}; use crate::gourmet::GOURMET_MENUS_URL;`. Add methods inside `impl GourmetApi`:

```rust
    /// §8.1 — paginate menus 0..=9, stop after a page with no next link.
    pub async fn get_menus(&self) -> CoreResult<Vec<MenuItem>> {
        const MAX_MENU_PAGES: usize = 10;
        let mut all = Vec::new();
        for page in 0..MAX_MENU_PAGES {
            let html = if page == 0 {
                // page 0: ensure session (re-fetch on re-login) + best-effort user info.
                let mut html = self.client.get(GOURMET_MENUS_URL, &[]).await?;
                if self.ensure_session(&html).await? == EnsureOutcome::Refetched {
                    html = self.client.get(GOURMET_MENUS_URL, &[]).await?;
                }
                if self.user_info.lock().unwrap().is_none() {
                    if let Ok(info) = parser::extract_user_info(&html) {
                        *self.user_info.lock().unwrap() = Some(info);
                    }
                }
                html
            } else {
                self.client
                    .get(GOURMET_MENUS_URL, &[("page", &page.to_string())])
                    .await?
            };
            all.extend(parser::parse_menu_items(&html));
            if !parser::has_next_menu_page(&html) {
                break;
            }
        }
        Ok(all)
    }

    /// §9.1 — ordered menus.
    pub async fn get_orders(&self) -> CoreResult<Vec<OrderedMenu>> {
        let html = self.get_authenticated_html(GOURMET_ORDERS_URL).await?;
        Ok(parser::parse_ordered_menus(&html))
    }

    /// GET a page, re-login+re-fetch if the session expired, return fresh authenticated HTML.
    async fn get_authenticated_html(&self, url: &str) -> CoreResult<String> {
        let html = self.client.get(url, &[]).await?;
        match self.ensure_session(&html).await? {
            EnsureOutcome::Ready => Ok(html),
            EnsureOutcome::Refetched => self.client.get(url, &[]).await,
        }
    }

    /// §11 — best-effort logout; always clears local session.
    pub async fn logout(&self) -> CoreResult<()> {
        let _ = self.logout_inner().await; // swallow every error (§11).
        *self.user_info.lock().unwrap() = None;
        *self.credentials.lock().unwrap() = None;
        self.client.reset();
        Ok(())
    }

    async fn logout_inner(&self) -> CoreResult<()> {
        let html = self.client.get(GOURMET_LOGIN_URL, &[]).await?;
        let (ufprt, ncform) = parser::extract_logout_form_tokens(&html)?;
        self.client
            .post_form(
                GOURMET_LOGIN_URL,
                vec![("ufprt".into(), ufprt), ("__ncforminfo".into(), ncform)],
            )
            .await?;
        Ok(())
    }
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib gourmet::api`
Expected: all menus/orders/logout tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/src/gourmet
git commit -m "feat(core): gourmet::api — get_menus pagination, get_orders, logout"
```

---

### Task 3: `add_to_cart` + `confirm_orders` + `cancel_orders` (edit-mode loop)

**Files:**
- Modify: `src/core/src/gourmet/api.rs`

**Interfaces:**
- Consumes: `parser::{extract_edit_mode, extract_form_tokens, extract_cancel_form_data}`, `datetime::format_menu_date`, `serde_json`, `GOURMET_ADD_TO_CART_URL`.
- Produces:
  - `pub async fn add_to_cart(&self, items: Vec<(String, String)>) -> CoreResult<()>` — `items` = `(menu_id, date_key "YYYY-MM-DD")`; require cached user info (`Not logged in`); group by first-seen date; JSON with lowercase `staffgroupId`; `success != true` → `AddToCartFailed`. (§10.1)
  - `pub async fn confirm_orders(&self) -> CoreResult<()>` — GET orders + `ensure_session`; if `editMode == "False"` POST the toggle; else no-op. (§9.3)
  - `pub async fn cancel_orders(&self, position_ids: Vec<String>) -> CoreResult<()>` — enter edit mode (verify), per position: extract cancel form, POST 5 fields, re-GET for fresh tokens; exit edit mode if still in it. (§9.4)
  - private `async fn toggle_edit_mode(&self, html: &str) -> CoreResult<String>` returning the fresh page HTML after re-GET.

- [ ] **Step 1: Write failing tests**

Add to `tests`:

```rust
const ORDERS_EDIT: &str = include_str!("../../tests/fixtures/gourmet/orders-page-edit-mode.html");

#[tokio::test]
async fn add_to_cart_groups_by_date_and_uses_lowercase_staffgroup_key() {
    let t = Arc::new(CapturingTransport::new());
    let api = logged_in_api(&t).await;
    t.queue_response(ok(r#"{"success":true}"#));
    api.add_to_cart(vec![
        ("menu-001".into(), "2026-02-10".into()),
        ("menu-004".into(), "2026-02-10".into()),
        ("menu-001".into(), "2026-02-11".into()),
    ])
    .await
    .unwrap();
    let post = &t.requests()[2];
    assert_eq!(post.url, "https://alaclickneu.gourmet.at/umbraco/api/AlaCartApi/AddToMenuesCart");
    match &post.body {
        Some(RequestBody::Json(s)) => {
            let v: serde_json::Value = serde_json::from_str(s).unwrap();
            assert!(v.get("staffgroupId").is_some()); // lowercase g
            let dates = v["dates"].as_array().unwrap();
            assert_eq!(dates.len(), 2); // grouped
            assert_eq!(dates[0]["date"], "02-10-2026"); // MM-dd-yyyy
            assert_eq!(dates[0]["menuIds"].as_array().unwrap().len(), 2);
        }
        _ => panic!("expected json"),
    }
}

#[tokio::test]
async fn add_to_cart_requires_user_info() {
    let t = Arc::new(CapturingTransport::new());
    let api = GourmetApi::new(t.clone()); // never logged in
    let err = api.add_to_cart(vec![("m".into(), "2026-02-10".into())]).await.unwrap_err();
    assert_eq!(err.to_string(), "Not logged in");
}

#[tokio::test]
async fn add_to_cart_failure_maps_message() {
    let t = Arc::new(CapturingTransport::new());
    let api = logged_in_api(&t).await;
    t.queue_response(ok(r#"{"success":false,"message":"boom"}"#));
    let err = api.add_to_cart(vec![("m".into(), "2026-02-10".into())]).await.unwrap_err();
    assert_eq!(err.to_string(), "Add to cart failed: boom");
}

#[tokio::test]
async fn cancel_orders_enters_edit_mode_then_posts_cancel() {
    let t = Arc::new(CapturingTransport::new());
    let api = logged_in_api(&t).await;
    // GET orders (confirmed view, editMode="True") → enter edit → re-GET (editMode="False")
    t.queue_response(ok(ORDERS_PAGE));       // initial GET
    t.queue_response(ok("<html>ok</html>")); // toggle POST (enter)
    t.queue_response(ok(ORDERS_EDIT));       // re-GET (now in edit mode)
    // per-position: cancel POST, then re-GET for fresh tokens
    let first = parser::parse_ordered_menus(ORDERS_EDIT)[0].position_id.clone();
    t.queue_response(ok("<html>cancelled</html>")); // cancel POST
    t.queue_response(ok(ORDERS_EDIT));              // re-GET fresh tokens
    // exit edit mode
    t.queue_response(ok("<html>exited</html>"));    // toggle POST (exit)
    api.cancel_orders(vec![first]).await.unwrap();
    // at minimum a cancel POST hit /bestellungen/ with 5 fields
    let posts: Vec<_> = t.requests().into_iter()
        .filter(|r| r.method == Method::Post && r.url == "https://alaclickneu.gourmet.at/bestellungen/")
        .collect();
    assert!(posts.iter().any(|p| matches!(&p.body,
        Some(RequestBody::Multipart(f)) if f.iter().any(|(k, _)| k == "cp_PositionId"))));
}
```

> The `cancel_orders` test's exact request count depends on the fixtures' editMode values; assert the essential invariant (a cancel POST with `cp_PositionId` reached `/bestellungen/`) rather than a brittle exact sequence. Inspect `orders-page.html` / `orders-page-edit-mode.html` for their `editMode` values and adjust the queued responses so the state machine reaches the cancel step; never edit the fixtures.

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib gourmet::api`
Expected: FAIL.

- [ ] **Step 3: Implement the three operations**

Add imports: `use crate::datetime::format_menu_date; use crate::gourmet::GOURMET_ADD_TO_CART_URL;`. Add methods:

```rust
    /// §10.1 — group by first-seen date, JSON with lowercase `staffgroupId`.
    pub async fn add_to_cart(&self, items: Vec<(String, String)>) -> CoreResult<()> {
        let info = self
            .user_info
            .lock()
            .unwrap()
            .clone()
            .ok_or(CoreError::NotLoggedIn)?;
        // group menu ids by first-seen date key, preserving order.
        let mut order: Vec<String> = Vec::new();
        let mut groups: std::collections::HashMap<String, Vec<String>> = Default::default();
        for (menu_id, date_key) in items {
            groups.entry(date_key.clone()).or_default().push(menu_id);
            if !order.contains(&date_key) {
                order.push(date_key);
            }
        }
        let dates: Vec<serde_json::Value> = order
            .iter()
            .map(|k| {
                serde_json::json!({
                    "date": format_menu_date(k),
                    "menuIds": groups[k],
                })
            })
            .collect();
        let body = serde_json::json!({
            "eaterId": info.eater_id,
            "shopModelId": info.shop_model_id,
            "staffgroupId": info.staff_group_id,   // lowercase g (01 §10.1)
            "dates": dates,
        })
        .to_string();

        let resp = self.client.post_json(GOURMET_ADD_TO_CART_URL, body).await?;
        let parsed: serde_json::Value =
            serde_json::from_str(&resp).map_err(|e| CoreError::Parse { message: e.to_string() })?;
        if parsed.get("success").and_then(|v| v.as_bool()) != Some(true) {
            let message = parsed
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_string();
            return Err(CoreError::AddToCartFailed { message });
        }
        Ok(())
    }

    /// §9.3 — confirm = exit edit mode when the page is in edit mode.
    pub async fn confirm_orders(&self) -> CoreResult<()> {
        let html = self.get_authenticated_html(GOURMET_ORDERS_URL).await?;
        let edit_mode = parser::extract_edit_mode(&html).unwrap_or_else(|| "True".to_string());
        if edit_mode == "False" {
            // in edit mode → posting the toggle exits/confirms.
            self.post_toggle(&html).await?;
        }
        Ok(())
    }

    /// §9.4 — enter edit mode, cancel each position (fresh tokens per step), exit.
    pub async fn cancel_orders(&self, position_ids: Vec<String>) -> CoreResult<()> {
        let mut html = self.get_authenticated_html(GOURMET_ORDERS_URL).await?;
        // enter edit mode if not already in it (editMode != "False").
        if parser::extract_edit_mode(&html).as_deref() != Some("False") {
            self.post_toggle(&html).await?;
            html = self.client.get(GOURMET_ORDERS_URL, &[]).await?;
            if parser::extract_edit_mode(&html).as_deref() != Some("False") {
                return Err(CoreError::EditModeFailed);
            }
        }
        for position_id in &position_ids {
            let form = parser::extract_cancel_form_data(&html, position_id)?;
            self.client
                .post_form(
                    GOURMET_ORDERS_URL,
                    vec![
                        ("cp_PositionId".into(), form.position_id),
                        (format!("cp_EatingCycleId_{position_id}"), form.eating_cycle_id),
                        (format!("cp_Date_{position_id}"), form.date),
                        ("ufprt".into(), form.ufprt),
                        ("__ncforminfo".into(), form.ncforminfo),
                    ],
                )
                .await?;
            // re-GET for fresh tokens before the next cancel (§9.4d).
            html = self.client.get(GOURMET_ORDERS_URL, &[]).await?;
        }
        // exit edit mode if still in it.
        if parser::extract_edit_mode(&html).as_deref() == Some("False") {
            self.post_toggle(&html).await?;
        }
        Ok(())
    }

    /// POST the edit-mode toggle, echoing the extracted editMode value (§9.2).
    async fn post_toggle(&self, html: &str) -> CoreResult<String> {
        let edit_mode = parser::extract_edit_mode(html).unwrap_or_else(|| "True".to_string());
        let (ufprt, ncform) = parser::extract_form_tokens(html, "form.form-toggleEditMode")
            .map_err(|_| CoreError::Parse {
                message: "Could not extract edit mode form data".into(),
            })?;
        self.client
            .post_form(
                GOURMET_ORDERS_URL,
                vec![
                    ("editMode".into(), edit_mode),
                    ("ufprt".into(), ufprt),
                    ("__ncforminfo".into(), ncform),
                ],
            )
            .await
    }
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib gourmet::api`
Expected: all cart/confirm/cancel tests PASS. (If the `cancel_orders` sequence assertion is brittle against the fixtures' editMode values, adjust the queued responses per the Step 1 note.)

- [ ] **Step 5: Commit**

```bash
git add src/core/src/gourmet
git commit -m "feat(core): gourmet::api — add_to_cart, confirm_orders, cancel_orders edit-mode loop"
```

---

### Task 4: `get_billings` + full suite gate

**Files:**
- Modify: `src/core/src/gourmet/api.rs`

**Interfaces:**
- Consumes: `domain::{Bill, BillingItem}`, `datetime::parse_bill_date`, `GOURMET_BILLING_URL`.
- Produces:
  - `pub async fn get_billings(&self, check_last_month_number: &str) -> CoreResult<Vec<Bill>>` — require cached user info before any request (`Not logged in`); probe GET `/start/` + `ensure_session` (NO re-fetch); POST JSON `{eaterId, shopModelId, checkLastMonthNumber}`; parse `{Billings:[…]}` OR raw array, map fields (§10.2).

- [ ] **Step 1: Write failing tests**

Add to `tests`:

```rust
#[tokio::test]
async fn get_billings_requires_user_info_before_any_request() {
    let t = Arc::new(CapturingTransport::new());
    let api = GourmetApi::new(t.clone());
    let err = api.get_billings("0").await.unwrap_err();
    assert_eq!(err.to_string(), "Not logged in");
    assert_eq!(t.requests().len(), 0); // no probe GET emitted
}

#[tokio::test]
async fn get_billings_probes_then_posts_and_maps_wrapper_or_array() {
    let t = Arc::new(CapturingTransport::new());
    let api = logged_in_api(&t).await;
    t.queue_response(ok(LOGIN_SUCCESS)); // probe GET /start/ (already authenticated)
    t.queue_response(ok(
        r#"{"Billings":[{"BillNr":10001,"BillDate":"2026-02-10T12:00:00","Location":"Wien","Billing":4.5,"BillingItemInfo":[{"Id":"i1","ArticleId":"a1","Count":1,"Description":"Schnitzel","Total":5.5,"Subsidy":2.5,"DiscountValue":0.0,"IsCustomMenu":false}]}]}"#,
    ));
    let bills = api.get_billings("0").await.unwrap();
    assert_eq!(bills.len(), 1);
    assert_eq!(bills[0].bill_nr, 10001);
    assert_eq!(bills[0].location, "Wien");
    assert_eq!(bills[0].items[0].description, "Schnitzel");

    // the POST body carried the string checkLastMonthNumber
    let post = t.requests().into_iter().find(|r| r.url == crate::gourmet::GOURMET_BILLING_URL).unwrap();
    match post.body {
        Some(RequestBody::Json(s)) => {
            let v: serde_json::Value = serde_json::from_str(&s).unwrap();
            assert_eq!(v["checkLastMonthNumber"], "0");
        }
        _ => panic!("expected json"),
    }
}

#[tokio::test]
async fn get_billings_accepts_raw_array_and_empty() {
    let t = Arc::new(CapturingTransport::new());
    let api = logged_in_api(&t).await;
    t.queue_response(ok(LOGIN_SUCCESS)); // probe
    t.queue_response(ok("[]"));          // raw empty array
    assert_eq!(api.get_billings("0").await.unwrap().len(), 0);
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib gourmet::api`
Expected: FAIL.

- [ ] **Step 3: Implement `get_billings`**

Add imports: `use crate::domain::{Bill, BillingItem}; use crate::datetime::parse_bill_date; use crate::gourmet::GOURMET_BILLING_URL;`. Add:

```rust
    /// §10.2 — billing. Requires cached user info before ANY request; probe + no-refetch.
    pub async fn get_billings(&self, check_last_month_number: &str) -> CoreResult<Vec<Bill>> {
        let info = self
            .user_info
            .lock()
            .unwrap()
            .clone()
            .ok_or(CoreError::NotLoggedIn)?;
        // probe session (GET /start/ + ensure_session), but DO NOT re-fetch (§7 exception).
        let html = self.client.get(GOURMET_LOGIN_URL, &[]).await?;
        let _ = self.ensure_session(&html).await?;

        let body = serde_json::json!({
            "eaterId": info.eater_id,
            "shopModelId": info.shop_model_id,
            "checkLastMonthNumber": check_last_month_number, // string (01 §10.2)
        })
        .to_string();
        let resp = self.client.post_json(GOURMET_BILLING_URL, body).await?;
        let parsed: serde_json::Value =
            serde_json::from_str(&resp).map_err(|e| CoreError::Parse { message: e.to_string() })?;

        // wrapper {Billings:[…]} OR raw array; neither → [].
        let arr = parsed
            .get("Billings")
            .and_then(|v| v.as_array())
            .or_else(|| parsed.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(arr.iter().map(map_bill).collect())
    }
```

And a free function at the bottom of `api.rs`:

```rust
fn map_bill(v: &serde_json::Value) -> Bill {
    let f = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
    let items = v
        .get("BillingItemInfo")
        .and_then(|x| x.as_array())
        .map(|arr| arr.iter().map(map_item).collect())
        .unwrap_or_default();
    Bill {
        bill_nr: v.get("BillNr").and_then(|x| x.as_i64()).unwrap_or(0),
        bill_date_epoch_ms: v
            .get("BillDate")
            .and_then(|x| x.as_str())
            .and_then(parse_bill_date)
            .unwrap_or(0),
        location: str_field(v, "Location"),
        items,
        billing: f("Billing"),
    }
}

fn map_item(v: &serde_json::Value) -> BillingItem {
    let f = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
    BillingItem {
        id: str_field(v, "Id"),
        article_id: str_field(v, "ArticleId"),
        count: v.get("Count").and_then(|x| x.as_i64()).unwrap_or(0),
        description: str_field(v, "Description"),
        total: f("Total"),
        subsidy: f("Subsidy"),
        discount_value: f("DiscountValue"),
        is_custom_menu: v.get("IsCustomMenu").and_then(|x| x.as_bool()).unwrap_or(false),
    }
}

fn str_field(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib gourmet::api`
Expected: all billing tests PASS.

- [ ] **Step 5: Full suite + fmt/clippy + commit**

Run: `cd src/core && cargo test && cargo fmt && cargo clippy --all-targets`
Expected: full suite green; clippy clean.

```bash
git add src/core/src/gourmet
git commit -m "feat(core): gourmet::api — get_billings (wrapper/array), completes Gourmet scraping"
```

---

## Post-plan

This completes the Gourmet side (`gourmet::{parser,client,api}`). Next Rust core sub-plans: **1c Ventopay** (`ventopay::{client,parser,api}` — ASP.NET viewstate flow using the foundation `CookieJar`), then **1d feature services**, **1e notify+demo**, **1f UniFFI facade + bindings + CI**.

## Self-review notes

- **Spec coverage:** §6.1 (pre-logout), §6.2 (login 5 fields + verify + user-info fallback + failure-keeps-creds), §7 (ensure_session + caller re-fetch), §8.1 (pagination), §9.1 (orders), §9.2/§9.3 (edit-mode + confirm), §9.4 (cancel loop), §10.1 (add_to_cart grouping + lowercase staffgroupId + failure), §10.2 (billing require-user-info-first + probe-no-refetch + wrapper/array), §11 (logout swallow + clear). Endpoint constants (§1) added.
- **Type consistency:** `GourmetApi::{new,login,ensure_session,get_menus,get_orders,add_to_cart,confirm_orders,cancel_orders,get_billings,logout,user_info,is_authenticated}` are the surface the UniFFI facade (`ffi`) and feature services (`features::*`) will consume. `EnsureOutcome` is private; `map_bill`/`map_item`/`str_field` are private helpers. All domain types come from the foundation.
- **Fixture-value caveats:** the `cancel_orders` sequence test depends on the `editMode` values in `orders-page.html` / `orders-page-edit-mode.html`; the Step 1 note directs adjusting queued responses (not fixtures) so the state machine reaches the cancel POST.
