# SnackPilot v2 — Ventopay Client (Phase 1c-ii) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `ventopay::client` — the Ventopay HTTP primitives (`get`, `post_form`) over the foundation `Transport`, with the **app-owned `CookieJar`** (capture `Set-Cookie` from each response, inject a single `Cookie` header on each request), `application/x-www-form-urlencoded` POST bodies, the `Origin`/`Referer` rules, `lastPageUrl` tracking, and `reset` — byte-for-byte per `docs/requirements/02-ventopay-scraping.md` §2, request/cookie-shape-tested with the `CapturingTransport`.

**Architecture:** `src/core/src/ventopay/client.rs` holds `VentopayClient` over an `Arc<dyn Transport>` plus a `Mutex<CookieJar>` (the foundation jar) and a `Mutex<String>` lastPageUrl. Every request passes through one `send` helper that injects the `Cookie` header before sending and captures `Set-Cookie` from the response after. Production wires `ReqwestTransport::new(false)` — reqwest's own cookie store **disabled** so the app jar is the only one (02 §2.1); tests wire a `CapturingTransport`, queue responses with `set-cookie` headers, and assert both the captured request headers/body and the jar behavior.

**Tech Stack:** Rust 2021; foundation `http::{Transport, Request, HttpResponse, Method, RequestBody}`, `http::cookie_jar::CookieJar`, `error::CoreResult`, `ventopay` constants; `tokio` for async tests. No new dependencies.

## Global Constraints

- **Baseline:** v1.4.5 (`main` @ 6997c44); authoritative spec `docs/requirements/02-ventopay-scraping.md` §2.
- **Crate:** `src/core/` on the `v2` worktree `/Users/radaiko/dev/private/SnackPilot-v2`. Test with `cd src/core && cargo test`.
- **Cookie jar (§2.2):** the client injects one `Cookie: n1=v1; n2=v2` header (insertion order) from the jar when non-empty, and captures every `Set-Cookie` response header into the jar. Only ONE cookie jar is active — the transport's reqwest cookie store is disabled in production (§2.1, ban rule #8). The jar semantics (parse before `;`, split first `=`, ignore attributes, insertion-order, no header when empty) live in the foundation `CookieJar`, already tested.
- **GET (§2.3):** plain GET with optional query params appended; **no** Origin/Referer; records `last_page_url` = absolute request URL without query. Query values keep `.` unencoded (dates are `dd.MM.yyyy`).
- **post_form (§2.4):** `application/x-www-form-urlencoded` (`RequestBody::Form`, field order preserved); headers `Origin: https://my.ventopay.com` and `Referer: {last_page_url or the POST url}`.
- **reset:** clears BOTH the jar and `last_page_url` (used on logout, 02 §4).
- The foundation transport adds `Accept: application/json, text/plain, */*` and no `User-Agent` on every request; the client MUST NOT re-add or override those.
- Commit after each green step. Conventional-commit, scope `core`.

---

## File structure (this plan)

```
src/core/src/ventopay/
├── mod.rs      # + pub mod client;
└── client.rs   # VentopayClient + tests
```

---

### Task 1: `VentopayClient::get` with cookie capture/injection + `lastPageUrl`

**Files:**
- Modify: `src/core/src/ventopay/mod.rs`
- Create: `src/core/src/ventopay/client.rs`
- Test: inline `#[cfg(test)]`

**Interfaces:**
- Consumes: `http::{Transport, Request, HttpResponse, Method, RequestBody}`, `http::cookie_jar::CookieJar`, `error::CoreResult`, `ventopay` constants.
- Produces:
  - `pub struct VentopayClient { transport: Arc<dyn Transport>, jar: Mutex<CookieJar>, last_page_url: Mutex<String> }`
  - `pub fn new(transport: Arc<dyn Transport>) -> Self`
  - `pub fn last_page_url(&self) -> String`, `pub fn reset(&self)`
  - `pub async fn get(&self, url: &str, params: &[(&str, &str)]) -> CoreResult<String>`
  - private `async fn send(&self, req: Request) -> CoreResult<HttpResponse>` (cookie inject + capture), `fn absolute_url(&self, &str) -> String`.

- [ ] **Step 1: Write failing tests**

Create `src/core/src/ventopay/client.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{CapturingTransport, HttpResponse, Method};
    use std::sync::Arc;

    fn resp_with_cookie(body: &str, set_cookie: Option<&str>) -> HttpResponse {
        let headers = match set_cookie {
            Some(c) => vec![("set-cookie".to_string(), c.to_string())],
            None => vec![],
        };
        HttpResponse { status: 200, headers, body: body.into() }
    }

    #[tokio::test]
    async fn get_records_last_page_and_captures_cookie() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(resp_with_cookie("login", Some("ASP.NET_SessionId=abc; path=/")));
        let client = VentopayClient::new(t.clone());

        let body = client.get(crate::ventopay::VENTOPAY_LOGIN_URL, &[]).await.unwrap();
        assert_eq!(body, "login");
        let req = &t.requests()[0];
        assert_eq!(req.method, Method::Get);
        assert_eq!(req.url, "https://my.ventopay.com/mocca.website/Login.aspx");
        // first request carries no Cookie (jar was empty)
        assert!(req.headers.iter().all(|(k, _)| k != "Cookie"));
        assert_eq!(client.last_page_url(), "https://my.ventopay.com/mocca.website/Login.aspx");
    }

    #[tokio::test]
    async fn second_request_injects_captured_cookie() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(resp_with_cookie("a", Some("ASP.NET_SessionId=abc; path=/")));
        t.queue_response(resp_with_cookie("b", None));
        let client = VentopayClient::new(t.clone());

        client.get(crate::ventopay::VENTOPAY_LOGIN_URL, &[]).await.unwrap();
        client.get(crate::ventopay::VENTOPAY_TRANSACTIONS_URL, &[]).await.unwrap();

        let second = &t.requests()[1];
        let cookie = second.headers.iter().find(|(k, _)| k == "Cookie").map(|(_, v)| v.as_str());
        assert_eq!(cookie, Some("ASP.NET_SessionId=abc"));
    }

    #[tokio::test]
    async fn get_query_keeps_dots_and_strips_from_last_page() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(resp_with_cookie("x", None));
        let client = VentopayClient::new(t.clone());
        client
            .get(crate::ventopay::VENTOPAY_TRANSACTIONS_URL,
                 &[("fromDate", "01.02.2026"), ("untilDate", "28.02.2026")])
            .await
            .unwrap();
        assert_eq!(
            t.requests()[0].url,
            "https://my.ventopay.com/mocca.website/Transaktionen.aspx?fromDate=01.02.2026&untilDate=28.02.2026"
        );
        assert_eq!(
            client.last_page_url(),
            "https://my.ventopay.com/mocca.website/Transaktionen.aspx"
        );
    }
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib ventopay::client`
Expected: FAIL — `VentopayClient` not defined.

- [ ] **Step 3: Implement**

Add to `src/core/src/ventopay/mod.rs`: `pub mod client;` (keep `pub mod parser;`).

Create the top of `src/core/src/ventopay/client.rs`:

```rust
//! Ventopay HTTP primitives over the foundation Transport with the app-owned cookie jar
//! (02-ventopay-scraping §2). url-encoded POSTs, Origin/Referer, lastPageUrl.
use crate::error::CoreResult;
use crate::http::cookie_jar::CookieJar;
use crate::http::{HttpResponse, Method, Request, RequestBody, Transport};
use crate::ventopay::VENTOPAY_ORIGIN;
use std::sync::{Arc, Mutex};

pub struct VentopayClient {
    transport: Arc<dyn Transport>,
    jar: Mutex<CookieJar>,
    last_page_url: Mutex<String>,
}

impl VentopayClient {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self {
            transport,
            jar: Mutex::new(CookieJar::new()),
            last_page_url: Mutex::new(String::new()),
        }
    }

    pub fn last_page_url(&self) -> String {
        self.last_page_url.lock().unwrap().clone()
    }

    /// Clears BOTH the cookie jar and lastPageUrl (logout, 02 §4).
    pub fn reset(&self) {
        self.jar.lock().unwrap().clear();
        self.last_page_url.lock().unwrap().clear();
    }

    /// GET with optional query params. No Origin/Referer. Records lastPageUrl (query-stripped).
    pub async fn get(&self, url: &str, params: &[(&str, &str)]) -> CoreResult<String> {
        let base = self.absolute_url(url);
        let full = if params.is_empty() {
            base.clone()
        } else {
            let qs = params
                .iter()
                .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            format!("{base}?{qs}")
        };
        let resp = self
            .send(Request { method: Method::Get, url: full, headers: vec![], body: None })
            .await?;
        *self.last_page_url.lock().unwrap() = base;
        Ok(resp.body)
    }

    /// Inject the Cookie header from the jar, send, then capture Set-Cookie into the jar (§2.2).
    async fn send(&self, mut req: Request) -> CoreResult<HttpResponse> {
        if let Some(h) = self.jar.lock().unwrap().header() {
            req.headers.push(("Cookie".to_string(), h));
        }
        let resp = self.transport.send(req).await?;
        let set_cookies: Vec<String> = resp
            .headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
            .map(|(_, v)| v.clone())
            .collect();
        if !set_cookies.is_empty() {
            self.jar.lock().unwrap().capture(&set_cookies);
        }
        Ok(resp)
    }

    fn absolute_url(&self, url: &str) -> String {
        if url.starts_with("http") {
            url.split('?').next().unwrap_or(url).to_string()
        } else {
            format!("{}/{}", crate::ventopay::VENTOPAY_BASE_URL, url.trim_start_matches('/'))
        }
    }
}

/// Query encoding leaving RFC3986 unreserved chars (incl. '.') unencoded — Ventopay date
/// params are `dd.MM.yyyy` and the '.' must NOT be percent-encoded (02 §2.3).
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
```

> Note: `VENTOPAY_ORIGIN` import is used by Task 2's `post_form`; add it now so the file compiles once `post_form` lands, or add the import in Task 2. If Rust warns "unused import" after Task 1, move the `use crate::ventopay::VENTOPAY_ORIGIN;` line into Task 2.

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib ventopay::client`
Expected: the three GET/cookie tests PASS. (If `VENTOPAY_ORIGIN` triggers an unused-import warning that fails a `-D warnings` build, defer that import to Task 2.)

- [ ] **Step 5: Commit**

```bash
git add src/core/src/ventopay
git commit -m "feat(core): ventopay::client — get with cookie capture/injection + lastPageUrl"
```

---

### Task 2: `post_form` (url-encoded, Origin/Referer)

**Files:**
- Modify: `src/core/src/ventopay/client.rs`

**Interfaces:**
- Produces:
  - `pub async fn post_form(&self, url: &str, fields: Vec<(String, String)>) -> CoreResult<String>` — `Method::Post`, `RequestBody::Form(fields)` (order preserved), headers `Origin` + `Referer` (lastPageUrl or the url arg).
  - private `fn referer(&self, url: &str) -> String`.

- [ ] **Step 1: Write failing test**

Add to `tests`:

```rust
use crate::http::RequestBody;

#[tokio::test]
async fn post_form_is_urlencoded_with_origin_and_referer() {
    let t = Arc::new(CapturingTransport::new());
    t.queue_response(resp_with_cookie("login page", None)); // GET
    t.queue_response(resp_with_cookie("<html>Ausloggen.aspx</html>", None)); // POST
    let client = VentopayClient::new(t.clone());
    client.get(crate::ventopay::VENTOPAY_LOGIN_URL, &[]).await.unwrap();
    client
        .post_form(
            crate::ventopay::VENTOPAY_LOGIN_URL,
            vec![
                ("__VIEWSTATE".into(), "vs".into()),
                ("TxtUsername".into(), "u".into()),
                ("BtnLogin".into(), "Login".into()),
            ],
        )
        .await
        .unwrap();

    let post = &t.requests()[1];
    assert_eq!(post.method, Method::Post);
    let hdr = |k: &str| post.headers.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    assert_eq!(hdr("Origin"), Some("https://my.ventopay.com"));
    assert_eq!(hdr("Referer"), Some("https://my.ventopay.com/mocca.website/Login.aspx"));
    match &post.body {
        Some(RequestBody::Form(f)) => {
            assert_eq!(f[0].0, "__VIEWSTATE");
            assert_eq!(f[2], ("BtnLogin".to_string(), "Login".to_string()));
        }
        _ => panic!("expected url-encoded form"),
    }
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib ventopay::client`
Expected: FAIL — `post_form` not defined.

- [ ] **Step 3: Implement**

Add these methods inside `impl VentopayClient` (and ensure `use crate::ventopay::VENTOPAY_ORIGIN;` is present):

```rust
    /// url-encoded POST with Origin + Referer (§2.4). Field order preserved.
    pub async fn post_form(
        &self,
        url: &str,
        fields: Vec<(String, String)>,
    ) -> CoreResult<String> {
        let headers = vec![
            ("Origin".to_string(), VENTOPAY_ORIGIN.to_string()),
            ("Referer".to_string(), self.referer(url)),
        ];
        let resp = self
            .send(Request {
                method: Method::Post,
                url: self.absolute_url(url),
                headers,
                body: Some(RequestBody::Form(fields)),
            })
            .await?;
        Ok(resp.body)
    }

    fn referer(&self, url: &str) -> String {
        let last = self.last_page_url.lock().unwrap();
        if last.is_empty() {
            url.to_string()
        } else {
            last.clone()
        }
    }
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib ventopay::client`
Expected: all client tests PASS.

- [ ] **Step 5: Full suite + fmt/clippy + commit**

Run: `cd src/core && cargo test && cargo fmt && cargo clippy --all-targets`
Expected: full suite green; clippy clean.

```bash
git add src/core/src/ventopay
git commit -m "feat(core): ventopay::client — url-encoded post_form with Origin/Referer"
```

---

## Post-plan: next Ventopay sub-plan

- **Phase 1c-iii — `ventopay::api`**: login (GET Login.aspx → extract state → 11-field POST → verify), `ensure_session` (single re-login, no jar reset), `get_transactions` (`dd.MM.yyyy` date params, expiry-retry-once), logout (GET Ausloggen.aspx, swallow, reset). Sequence tests via `CapturingTransport` + fixture bodies.

## Self-review notes

- **Spec coverage:** §2.2 (jar inject/capture), §2.3 (GET + lastPageUrl + dot-safe query), §2.4 (url-encoded POST + Origin/Referer). The jar's parse semantics are the foundation `CookieJar` (already tested). Production disabling reqwest's cookie store (§2.1) is a wiring choice at facade construction (`ReqwestTransport::new(false)`), noted for 1f.
- **Type consistency:** `VentopayClient::{new,get,post_form,reset,last_page_url}` are the surface `ventopay::api` (1c-iii) consumes; `send`/`absolute_url`/`referer`/`encode` are private. `CookieJar`, `RequestBody::Form`, and the constants come from prior phases.
- **Multi-`Set-Cookie` note:** `CapturingTransport`/`HttpResponse` model headers as a `Vec`, so multiple `set-cookie` entries are captured (the `send` helper filters all case-insensitively) — matching v1's array handling (02 §2.2).
