//! Gourmet operations orchestration (01-gourmet-scraping §6-§11). Wires client + parser.
//! DO NOT MODIFY THE REQUEST SEQUENCES — deviations ban accounts.
use crate::datetime::{format_menu_date, parse_bill_date};
use crate::domain::{Bill, BillingItem, Credentials, GourmetUserInfo, MenuItem, OrderedMenu};
use crate::error::{CoreError, CoreResult};
use crate::gourmet::client::GourmetClient;
use crate::gourmet::parser;
use crate::gourmet::{
    GOURMET_ADD_TO_CART_URL, GOURMET_BILLING_URL, GOURMET_CANCEL_POSITION_URL,
    GOURMET_LOGIN_SUBMIT_URL, GOURMET_LOGIN_URL, GOURMET_LOGOUT_URL, GOURMET_MENUS_URL,
    GOURMET_ORDERS_URL, GOURMET_TOGGLE_EDIT_MODE_URL,
};
use crate::http::Transport;
use std::collections::HashMap;
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

    /// Full login: stale-session pre-logout, then the tokenless urlencoded credential POST,
    /// verify, cache. The site was rebuilt (AngularJS) 2026-07-23 — login now posts
    /// `Email`/`Password`/`RememberMe` as `application/x-www-form-urlencoded` to
    /// `/Controller/AlaLogin/Submit` with NO `ufprt`/`__ncforminfo` tokens. Login-state
    /// detection (§4) and user-info extraction (§5) are unchanged.
    pub async fn login(&self, creds: Credentials) -> CoreResult<GourmetUserInfo> {
        // Step 0 — stale-session pre-logout. Native cookie stores persist across restarts, so
        // GET /start/ may already be authenticated; log out first (empty tokenless POST) so the
        // credential POST starts from a clean session.
        let start_html = self.client.get(GOURMET_LOGIN_URL, &[]).await?;
        if parser::is_logged_in(&start_html) {
            let _ = self
                .client
                .post_urlencoded(GOURMET_LOGOUT_URL, vec![])
                .await;
            let _ = self.client.get(GOURMET_LOGIN_URL, &[]).await?;
        }

        // Steps 1-2 — the credential POST (no token extraction; the new form carries none).
        let post_html = self
            .client
            .post_urlencoded(
                GOURMET_LOGIN_SUBMIT_URL,
                vec![
                    ("Email".into(), creds.username.clone()),
                    ("Password".into(), creds.password.clone()),
                    ("RememberMe".into(), "false".into()),
                ],
            )
            .await?;

        // Step 3 — verify. Failure leaves cached creds/user info untouched (§6.2 note).
        if !parser::is_logged_in(&post_html) {
            return Err(CoreError::LoginFailed {
                detail: "Login failed: invalid credentials or account blocked".into(),
            });
        }

        // Step 4 — user info from response, else re-GET /start/ and extract there.
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

    /// §7 — Ready if authenticated; else re-login (creds required) → Refetched;
    /// no creds → SessionExpired.
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

    /// §8.1 — paginate menus 0..=9, stop after a page with no next link. De-dupes by (id, day):
    /// the rebuilt site's "next" link is always present, so a single stray extra page (or any
    /// server-side overlap) must never surface the same menu twice.
    pub async fn get_menus(&self) -> CoreResult<Vec<MenuItem>> {
        const MAX_MENU_PAGES: usize = 10;
        let mut all = Vec::new();
        let mut seen: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        for page in 0..MAX_MENU_PAGES {
            let html = if page == 0 {
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
            for item in parser::parse_menu_items(&html) {
                if seen.insert((item.id.clone(), item.day.clone())) {
                    all.push(item);
                }
            }
            if !parser::has_next_menu_page(&html) {
                break;
            }
        }
        Ok(all)
    }

    /// §9.1 — ordered menus. `now_epoch_ms` is the fallback for an order missing its date (G-2).
    pub async fn get_orders(&self, now_epoch_ms: i64) -> CoreResult<Vec<OrderedMenu>> {
        let html = self.get_authenticated_html(GOURMET_ORDERS_URL).await?;
        Ok(parser::parse_ordered_menus(&html, now_epoch_ms))
    }

    /// GET a page, re-login+re-fetch if the session expired, return fresh authenticated HTML.
    async fn get_authenticated_html(&self, url: &str) -> CoreResult<String> {
        let html = self.client.get(url, &[]).await?;
        match self.ensure_session(&html).await? {
            EnsureOutcome::Ready => Ok(html),
            EnsureOutcome::Refetched => self.client.get(url, &[]).await,
        }
    }

    /// §10.1 — group by first-seen date, JSON with lowercase `staffgroupId`.
    pub async fn add_to_cart(&self, items: Vec<(String, String)>) -> CoreResult<()> {
        let info = self
            .user_info
            .lock()
            .unwrap()
            .clone()
            .ok_or(CoreError::NotLoggedIn)?;
        let mut order: Vec<String> = Vec::new();
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
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
            "staffgroupId": info.staff_group_id, // lowercase g (01 §10.1)
            "dates": dates,
        })
        .to_string();

        let resp = self.client.post_json(GOURMET_ADD_TO_CART_URL, body).await?;
        let parsed: serde_json::Value =
            serde_json::from_str(&resp).map_err(|e| CoreError::Parse {
                detail: e.to_string(),
            })?;
        if parsed.get("success").and_then(|v| v.as_bool()) != Some(true) {
            let message = parsed
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_string();
            return Err(CoreError::AddToCartFailed { detail: message });
        }
        Ok(())
    }

    /// §9.3 — confirm = exit edit mode when the page is in edit mode.
    pub async fn confirm_orders(&self) -> CoreResult<()> {
        let html = self.get_authenticated_html(GOURMET_ORDERS_URL).await?;
        let edit_mode = parser::extract_edit_mode(&html).unwrap_or_else(|| "True".to_string());
        if edit_mode == "False" {
            self.post_toggle(&html).await?;
        }
        Ok(())
    }

    /// Cancel each position (rebuilt orders §9.4). The rebuilt site renders the cancel form whenever
    /// an order exists — no edit-mode toggle required — and cancels via a tokenless urlencoded POST
    /// to `/Controller/AlaMyOrders/CancelPosition`. Re-fetches between cancels so a still-present
    /// position's form data is fresh.
    pub async fn cancel_orders(&self, position_ids: Vec<String>) -> CoreResult<()> {
        for position_id in &position_ids {
            let html = self.get_authenticated_html(GOURMET_ORDERS_URL).await?;
            let form = parser::extract_cancel_form_data(&html, position_id)?;
            self.client
                .post_urlencoded(
                    GOURMET_CANCEL_POSITION_URL,
                    vec![
                        ("cp_PositionId".into(), form.position_id),
                        (
                            format!("cp_EatingCycleId_{position_id}"),
                            form.eating_cycle_id,
                        ),
                        (format!("cp_Date_{position_id}"), form.date),
                    ],
                )
                .await?;
        }
        Ok(())
    }

    /// POST the edit-mode toggle, echoing the extracted editMode value (rebuilt orders §9.2).
    /// Tokenless urlencoded POST to `/Controller/AlaMyOrders/ToggleEditMode`.
    async fn post_toggle(&self, html: &str) -> CoreResult<String> {
        let edit_mode = parser::extract_edit_mode(html).unwrap_or_else(|| "True".to_string());
        self.client
            .post_urlencoded(
                GOURMET_TOGGLE_EDIT_MODE_URL,
                vec![("editMode".into(), edit_mode)],
            )
            .await
    }

    /// §10.2 — billing. Requires cached user info before ANY request; probe + no-refetch.
    pub async fn get_billings(&self, check_last_month_number: &str) -> CoreResult<Vec<Bill>> {
        let info = self
            .user_info
            .lock()
            .unwrap()
            .clone()
            .ok_or(CoreError::NotLoggedIn)?;
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
            serde_json::from_str(&resp).map_err(|e| CoreError::Parse {
                detail: e.to_string(),
            })?;

        let arr = parsed
            .get("Billings")
            .and_then(|v| v.as_array())
            .or_else(|| parsed.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(arr.iter().map(map_bill).collect())
    }

    /// §11 — best-effort logout; always clears local session.
    pub async fn logout(&self) -> CoreResult<()> {
        let _ = self.logout_inner().await;
        *self.user_info.lock().unwrap() = None;
        *self.credentials.lock().unwrap() = None;
        self.client.reset();
        Ok(())
    }

    async fn logout_inner(&self) -> CoreResult<()> {
        // Logout is now a tokenless empty POST to /Controller/AlaLogin/SubmitLogout (rebuilt site,
        // 2026-07-23). A prior GET keeps the Referer pointing at a real page.
        let _ = self.client.get(GOURMET_LOGIN_URL, &[]).await?;
        self.client
            .post_urlencoded(GOURMET_LOGOUT_URL, vec![])
            .await?;
        Ok(())
    }
}

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
        is_custom_menu: v
            .get("IsCustomMenu")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
    }
}

fn str_field(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{CapturingTransport, HttpResponse, Method, RequestBody};
    use std::sync::Arc;

    const LOGIN_PAGE: &str = include_str!("../../tests/fixtures/gourmet/login-page.html");
    const LOGIN_SUCCESS: &str = include_str!("../../tests/fixtures/gourmet/login-success.html");
    const LOGIN_FAILED: &str = include_str!("../../tests/fixtures/gourmet/login-failed.html");
    const MENUS_PAGE_0: &str = include_str!("../../tests/fixtures/gourmet/menus-page-0.html");
    const MENUS_PAGE_1: &str = include_str!("../../tests/fixtures/gourmet/menus-page-1.html");
    const ORDERS_PAGE: &str = include_str!("../../tests/fixtures/gourmet/orders-page.html");
    const ORDERS_EDIT: &str =
        include_str!("../../tests/fixtures/gourmet/orders-page-edit-mode.html");

    fn ok(body: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: vec![],
            body: body.into(),
        }
    }

    async fn logged_in_api(t: &Arc<CapturingTransport>) -> GourmetApi {
        t.queue_response(ok(LOGIN_PAGE));
        t.queue_response(ok(LOGIN_SUCCESS));
        let api = GourmetApi::new(t.clone());
        api.login(Credentials {
            username: "u".into(),
            password: "p".into(),
        })
        .await
        .unwrap();
        api
    }

    #[tokio::test]
    async fn login_posts_urlencoded_email_password_remember_and_caches_user_info() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok(LOGIN_PAGE)); // step 0 GET /start/ (not logged in)
        t.queue_response(ok(LOGIN_SUCCESS)); // credential POST → authenticated page
        let api = GourmetApi::new(t.clone());

        let info = api
            .login(Credentials {
                username: "u".into(),
                password: "p".into(),
            })
            .await
            .unwrap();
        assert_eq!(info.eater_id, "EATER-TEST-456");
        assert!(api.is_authenticated());

        let reqs = t.requests();
        assert_eq!(reqs.len(), 2); // no token extraction, no re-GET
        assert_eq!(reqs[1].method, Method::Post);
        assert_eq!(
            reqs[1].url,
            "https://alaclickneu.gourmet.at/Controller/AlaLogin/Submit"
        );
        // urlencoded (Form), exactly Email/Password/RememberMe in order — no ufprt/__ncforminfo.
        match &reqs[1].body {
            Some(RequestBody::Form(f)) => {
                let names: Vec<&str> = f.iter().map(|(k, _)| k.as_str()).collect();
                assert_eq!(names, ["Email", "Password", "RememberMe"]);
                assert_eq!(f[0].1, "u");
                assert_eq!(f[1].1, "p");
                assert_eq!(f[2].1, "false");
            }
            other => panic!("expected urlencoded Form body, got {other:?}"),
        }
        // Origin + Referer are still set on the POST.
        let hdr = |k: &str| {
            reqs[1]
                .headers
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(hdr("Origin"), Some("https://alaclickneu.gourmet.at"));
        assert_eq!(
            hdr("Referer"),
            Some("https://alaclickneu.gourmet.at/start/")
        );
    }

    #[tokio::test]
    async fn login_failure_raises() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok(LOGIN_PAGE));
        t.queue_response(ok(LOGIN_FAILED));
        let api = GourmetApi::new(t.clone());
        let err = api
            .login(Credentials {
                username: "u".into(),
                password: "bad".into(),
            })
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Login failed: invalid credentials or account blocked"
        );
    }

    #[tokio::test]
    async fn stale_session_triggers_pre_logout() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok(LOGIN_SUCCESS)); // step 0 GET (already logged in)
        t.queue_response(ok("<html>bye</html>")); // pre-logout POST (empty, tokenless)
        t.queue_response(ok(LOGIN_PAGE)); // re-GET /start/
        t.queue_response(ok(LOGIN_SUCCESS)); // credential POST
        let api = GourmetApi::new(t.clone());
        api.login(Credentials {
            username: "u".into(),
            password: "p".into(),
        })
        .await
        .unwrap();
        let reqs = t.requests();
        assert_eq!(reqs.len(), 4);
        // The pre-logout POST goes to SubmitLogout with an empty urlencoded body.
        assert_eq!(reqs[1].method, Method::Post);
        assert_eq!(
            reqs[1].url,
            "https://alaclickneu.gourmet.at/Controller/AlaLogin/SubmitLogout"
        );
        match &reqs[1].body {
            Some(RequestBody::Form(f)) => assert!(f.is_empty()),
            other => panic!("expected empty urlencoded body, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_menus_paginates_until_no_next_link() {
        let t = Arc::new(CapturingTransport::new());
        let api = logged_in_api(&t).await;
        t.queue_response(ok(MENUS_PAGE_0));
        t.queue_response(ok(MENUS_PAGE_1));
        let items = api.get_menus().await.unwrap();
        assert!(!items.is_empty());
        // No duplicate (id, day) pairs survive — guards against the broken-pagination dup bug.
        let mut keys: Vec<(String, String)> = items
            .iter()
            .map(|i| (i.id.clone(), i.day.clone()))
            .collect();
        let total = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), total, "menu items must be unique by (id, day)");

        let reqs = t.requests();
        assert_eq!(reqs[2].url, "https://alaclickneu.gourmet.at/de/menues/");
        assert_eq!(
            reqs[3].url,
            "https://alaclickneu.gourmet.at/de/menues/?page=1"
        );
        assert_eq!(reqs.len(), 4);
    }

    #[tokio::test]
    async fn get_orders_parses_after_session_check() {
        let t = Arc::new(CapturingTransport::new());
        let api = logged_in_api(&t).await;
        t.queue_response(ok(ORDERS_PAGE));
        let orders = api.get_orders(0).await.unwrap();
        assert!(!orders.is_empty());
        assert_eq!(
            t.requests()[2].url,
            "https://alaclickneu.gourmet.at/de/bestellungen/"
        );
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
        assert_eq!(
            post.url,
            "https://alaclickneu.gourmet.at/umbraco/api/AlaCartApi/AddToMenuesCart"
        );
        match &post.body {
            Some(RequestBody::Json(s)) => {
                let v: serde_json::Value = serde_json::from_str(s).unwrap();
                assert!(v.get("staffgroupId").is_some());
                let dates = v["dates"].as_array().unwrap();
                assert_eq!(dates.len(), 2);
                assert_eq!(dates[0]["date"], "02-10-2026");
                assert_eq!(dates[0]["menuIds"].as_array().unwrap().len(), 2);
            }
            _ => panic!("expected json"),
        }
    }

    #[tokio::test]
    async fn add_to_cart_requires_user_info() {
        let t = Arc::new(CapturingTransport::new());
        let api = GourmetApi::new(t.clone());
        let err = api
            .add_to_cart(vec![("m".into(), "2026-02-10".into())])
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "Not logged in");
    }

    #[tokio::test]
    async fn add_to_cart_failure_maps_message() {
        let t = Arc::new(CapturingTransport::new());
        let api = logged_in_api(&t).await;
        t.queue_response(ok(r#"{"success":false,"message":"boom"}"#));
        let err = api
            .add_to_cart(vec![("m".into(), "2026-02-10".into())])
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "Add to cart failed: boom");
    }

    #[tokio::test]
    async fn cancel_orders_posts_tokenless_urlencoded_to_controller() {
        let t = Arc::new(CapturingTransport::new());
        let api = logged_in_api(&t).await;
        t.queue_response(ok(ORDERS_EDIT)); // GET orders (cancel form + login markers, no edit dance)
        t.queue_response(ok("<html>cancelled</html>")); // cancel POST
        api.cancel_orders(vec!["POS-001".into()]).await.unwrap();

        let post = t
            .requests()
            .into_iter()
            .find(|r| r.url == GOURMET_CANCEL_POSITION_URL)
            .expect("a cancel POST to /Controller/AlaMyOrders/CancelPosition");
        assert_eq!(post.method, Method::Post);
        match &post.body {
            Some(RequestBody::Form(f)) => {
                let names: Vec<&str> = f.iter().map(|(k, _)| k.as_str()).collect();
                assert_eq!(
                    names,
                    [
                        "cp_PositionId",
                        "cp_EatingCycleId_POS-001",
                        "cp_Date_POS-001"
                    ]
                );
                assert_eq!(f[0].1, "POS-001");
                assert!(!names.contains(&"ufprt") && !names.contains(&"__ncforminfo"));
            }
            other => panic!("expected urlencoded Form body, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn confirm_orders_toggles_edit_mode_tokenless() {
        let t = Arc::new(CapturingTransport::new());
        let api = logged_in_api(&t).await;
        // editMode="False" ⇒ order is in edit mode (unconfirmed) ⇒ confirm toggles it.
        t.queue_response(ok(ORDERS_EDIT)); // GET orders
        t.queue_response(ok("<html>confirmed</html>")); // toggle POST
        api.confirm_orders().await.unwrap();

        let post = t
            .requests()
            .into_iter()
            .find(|r| r.url == GOURMET_TOGGLE_EDIT_MODE_URL)
            .expect("a toggle POST to /Controller/AlaMyOrders/ToggleEditMode");
        match &post.body {
            Some(RequestBody::Form(f)) => {
                let names: Vec<&str> = f.iter().map(|(k, _)| k.as_str()).collect();
                assert_eq!(names, ["editMode"]);
                assert_eq!(f[0].1, "False"); // echoes the extracted editMode value
            }
            other => panic!("expected urlencoded Form body, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_billings_requires_user_info_before_any_request() {
        let t = Arc::new(CapturingTransport::new());
        let api = GourmetApi::new(t.clone());
        let err = api.get_billings("0").await.unwrap_err();
        assert_eq!(err.to_string(), "Not logged in");
        assert_eq!(t.requests().len(), 0);
    }

    #[tokio::test]
    async fn get_billings_probes_then_posts_and_maps_wrapper() {
        let t = Arc::new(CapturingTransport::new());
        let api = logged_in_api(&t).await;
        t.queue_response(ok(LOGIN_SUCCESS)); // probe GET /start/
        t.queue_response(ok(
            r#"{"Billings":[{"BillNr":10001,"BillDate":"2026-02-10T12:00:00","Location":"Wien","Billing":4.5,"BillingItemInfo":[{"Id":"i1","ArticleId":"a1","Count":1,"Description":"Schnitzel","Total":5.5,"Subsidy":2.5,"DiscountValue":0.0,"IsCustomMenu":false}]}]}"#,
        ));
        let bills = api.get_billings("0").await.unwrap();
        assert_eq!(bills.len(), 1);
        assert_eq!(bills[0].bill_nr, 10001);
        assert_eq!(bills[0].location, "Wien");
        assert_eq!(bills[0].items[0].description, "Schnitzel");

        let post = t
            .requests()
            .into_iter()
            .find(|r| r.url == GOURMET_BILLING_URL)
            .unwrap();
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
        t.queue_response(ok("[]"));
        assert_eq!(api.get_billings("0").await.unwrap().len(), 0);
    }
}
