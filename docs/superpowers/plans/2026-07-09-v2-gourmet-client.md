# SnackPilot v2 — Gourmet Client (Phase 1b-ii) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `gourmet::client` — the three HTTP request primitives (`get`, `post_form`, `post_json`) over the foundation `Transport`, with the exact Gourmet header rules (`Origin`/`Referer`, multipart vs JSON content types), `lastPageUrl` tracking, URL building, and `reset` — byte-for-byte per `docs/requirements/01-gourmet-scraping.md` §2, request-shape-tested with the `CapturingTransport`.

**Architecture:** `src/core/src/gourmet/client.rs` holds `GourmetClient`, a thin wrapper over an injected `Arc<dyn Transport>`. It builds `Request` values (method, absolute URL, headers, ordered body) and tracks `lastPageUrl` behind a `Mutex`. Production wires a `ReqwestTransport::new(true)` (cookie store on, Accept header, no UA — all provided by the foundation); tests wire a `CapturingTransport` and assert the exact captured requests. The client owns none of the scraping logic (that's `parser`) and none of the orchestration (that's `api`); it is purely "shape the request, record the referer".

**Tech Stack:** Rust 2021; foundation `http::{Transport, Request, HttpResponse, Method, RequestBody}`, `error::CoreResult`; `tokio` for async tests. No new dependencies.

## Global Constraints

- **Baseline:** v1.4.5 (`main` @ 6997c44); authoritative spec `docs/requirements/01-gourmet-scraping.md` §2.
- **Crate:** `src/core/` on the `v2` worktree `/Users/radaiko/dev/private/SnackPilot-v2`. Test with `cd src/core && cargo test`.
- **Constants:** `GOURMET_BASE_URL = "https://alaclickneu.gourmet.at"` and `GOURMET_ORIGIN = "https://alaclickneu.gourmet.at"` (01 §1). Add to `gourmet/mod.rs`.
- **GET (§2.1):** plain GET; optional query params appended to the URL; **no** Origin/Referer headers. After every GET, record `last_page_url` = the absolute request URL **without query params** (relative url → prefixed with base + `/` if needed).
- **post_form (§2.2):** `multipart/form-data` (NEVER url-encoded); fields in insertion order; headers `Origin: {GOURMET_ORIGIN}` and `Referer: {last_page_url or the url arg exactly as passed}`. Content-Type + boundary handled by the transport's multipart encoding.
- **post_json (§2.3):** `application/json`; headers `Origin` + `Referer` (same rule).
- **reset (§2.4):** clears only `last_page_url`; does NOT clear cookies.
- The foundation transport already adds `Accept: application/json, text/plain, */*` and no `User-Agent` on every request; the client MUST NOT re-add or override those.
- Commit after each green step. Conventional-commit, scope `core`.

---

## File structure (this plan)

```
src/core/src/gourmet/
├── mod.rs        # + constants (GOURMET_BASE_URL, GOURMET_ORIGIN) + pub mod client;
└── client.rs     # GourmetClient + tests
```

---

### Task 1: Constants + `GourmetClient::get` with `lastPageUrl` tracking

**Files:**
- Modify: `src/core/src/gourmet/mod.rs`
- Create: `src/core/src/gourmet/client.rs`
- Test: inline `#[cfg(test)]`

**Interfaces:**
- Consumes: `http::{Transport, Request, HttpResponse, Method}`, `error::CoreResult`.
- Produces:
  - consts `GOURMET_BASE_URL`, `GOURMET_ORIGIN` in `gourmet` module.
  - `pub struct GourmetClient { transport: Arc<dyn Transport>, last_page_url: Mutex<String> }`
  - `pub fn new(transport: Arc<dyn Transport>) -> Self`
  - `pub async fn get(&self, url: &str, params: &[(&str, &str)]) -> CoreResult<String>` — builds the absolute URL (query appended), sends `Method::Get` with no extra headers, records `last_page_url` (absolute, query-stripped), returns the body.
  - `fn absolute_url(&self, url: &str) -> String` and `fn last_page_of(&self, url: &str) -> String` helpers.

- [ ] **Step 1: Write the failing GET test**

Create `src/core/src/gourmet/client.rs` with a test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{CapturingTransport, HttpResponse, Method};
    use std::sync::Arc;

    fn ok_body(body: &str) -> HttpResponse {
        HttpResponse { status: 200, headers: vec![], body: body.into() }
    }

    #[tokio::test]
    async fn get_sends_bare_url_and_records_last_page() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok_body("<html>menus</html>"));
        let client = GourmetClient::new(t.clone());

        let body = client.get("https://alaclickneu.gourmet.at/menus/", &[]).await.unwrap();
        assert_eq!(body, "<html>menus</html>");

        let reqs = t.requests();
        assert_eq!(reqs[0].method, Method::Get);
        assert_eq!(reqs[0].url, "https://alaclickneu.gourmet.at/menus/");
        // no Origin/Referer on GET
        assert!(reqs[0].headers.iter().all(|(k, _)| k != "Origin" && k != "Referer"));
        assert_eq!(client.last_page_url(), "https://alaclickneu.gourmet.at/menus/");
    }

    #[tokio::test]
    async fn get_appends_query_but_last_page_strips_it() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok_body("p1"));
        let client = GourmetClient::new(t.clone());

        client.get("https://alaclickneu.gourmet.at/menus/", &[("page", "1")]).await.unwrap();
        assert_eq!(t.requests()[0].url, "https://alaclickneu.gourmet.at/menus/?page=1");
        assert_eq!(client.last_page_url(), "https://alaclickneu.gourmet.at/menus/");
    }
}
```

> A `last_page_url()` accessor is added for tests; keep it `pub` — the `api` layer also needs to read/compare it.

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib gourmet::client`
Expected: FAIL — `GourmetClient` not defined.

- [ ] **Step 3: Implement constants + `get`**

Add to `src/core/src/gourmet/mod.rs`:

```rust
pub mod client;
pub mod parser;

/// 01-gourmet-scraping §1.
pub const GOURMET_BASE_URL: &str = "https://alaclickneu.gourmet.at";
pub const GOURMET_ORIGIN: &str = "https://alaclickneu.gourmet.at";
```

Create the top of `src/core/src/gourmet/client.rs`:

```rust
//! Gourmet HTTP request primitives over the foundation Transport (01-gourmet-scraping §2).
//! Owns header rules (Origin/Referer), multipart/JSON encoding selection, and lastPageUrl.
use crate::error::CoreResult;
use crate::gourmet::{GOURMET_BASE_URL, GOURMET_ORIGIN};
use crate::http::{Method, Request, RequestBody, Transport};
use std::sync::{Arc, Mutex};

pub struct GourmetClient {
    transport: Arc<dyn Transport>,
    last_page_url: Mutex<String>,
}

impl GourmetClient {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self { transport, last_page_url: Mutex::new(String::new()) }
    }

    pub fn last_page_url(&self) -> String {
        self.last_page_url.lock().unwrap().clone()
    }

    /// Clears lastPageUrl only; cookies are NOT cleared (01 §2.4).
    pub fn reset(&self) {
        self.last_page_url.lock().unwrap().clear();
    }

    /// GET with optional query params. Records lastPageUrl (absolute, query-stripped). (§2.1)
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
            .transport
            .send(Request { method: Method::Get, url: full, headers: vec![], body: None })
            .await?;
        *self.last_page_url.lock().unwrap() = base;
        Ok(resp.body)
    }

    /// Absolute form of a possibly-relative URL (01 §2.1 lastPageUrl prefixing).
    fn absolute_url(&self, url: &str) -> String {
        if url.starts_with("http") {
            // strip any existing query for the canonical page URL.
            url.split('?').next().unwrap_or(url).to_string()
        } else if let Some(stripped) = url.strip_prefix('/') {
            format!("{GOURMET_BASE_URL}/{stripped}")
        } else {
            format!("{GOURMET_BASE_URL}/{url}")
        }
    }
}

/// Minimal percent-encoding for query values: leave RFC3986 unreserved chars, encode the rest.
/// (Gourmet only ever sends `page=N`; this keeps '.'/'-'/'_'/'~' unencoded to match v1's
/// axios default serializer, 01 §2.1.)
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

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib gourmet::client`
Expected: both GET tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/src/gourmet
git commit -m "feat(core): gourmet::client — get + lastPageUrl tracking + constants"
```

---

### Task 2: `post_form` and `post_json` with Origin/Referer

**Files:**
- Modify: `src/core/src/gourmet/client.rs`

**Interfaces:**
- Produces:
  - `pub async fn post_form(&self, url: &str, fields: Vec<(String, String)>) -> CoreResult<String>` — `Method::Post`, `RequestBody::Multipart(fields)` (order preserved), headers `Origin` + `Referer` per rule.
  - `pub async fn post_json(&self, url: &str, body: String) -> CoreResult<String>` — `Method::Post`, `RequestBody::Json(body)`, headers `Origin` + `Referer`. (Content-Type is set by the transport for `Json`.)
  - `fn referer(&self, url: &str) -> String` helper: `last_page_url` if non-empty, else the `url` arg exactly as passed (§2.2 fallback caveat).

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module:

```rust
use crate::http::RequestBody;

#[tokio::test]
async fn post_form_carries_origin_referer_and_ordered_multipart() {
    let t = Arc::new(CapturingTransport::new());
    t.queue_response(ok_body("<html>ok</html>"));
    let client = GourmetClient::new(t.clone());
    // simulate a prior GET so Referer resolves to the page URL
    t.queue_response(ok_body("login page"));
    client.get("https://alaclickneu.gourmet.at/start/", &[]).await.unwrap();

    // re-queue the POST response (queue is FIFO; the GET consumed the first)
    let t2 = Arc::new(CapturingTransport::new());
    t2.queue_response(ok_body("login page"));
    t2.queue_response(ok_body("<html>ok</html>"));
    let client = GourmetClient::new(t2.clone());
    client.get("https://alaclickneu.gourmet.at/start/", &[]).await.unwrap();
    client
        .post_form(
            "https://alaclickneu.gourmet.at/start/",
            vec![
                ("Username".into(), "u".into()),
                ("Password".into(), "p".into()),
                ("RememberMe".into(), "false".into()),
            ],
        )
        .await
        .unwrap();

    let post = &t2.requests()[1];
    assert_eq!(post.method, Method::Post);
    let hdr = |k: &str| post.headers.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    assert_eq!(hdr("Origin"), Some("https://alaclickneu.gourmet.at"));
    assert_eq!(hdr("Referer"), Some("https://alaclickneu.gourmet.at/start/"));
    match &post.body {
        Some(RequestBody::Multipart(f)) => {
            assert_eq!(f[0].0, "Username");
            assert_eq!(f[2], ("RememberMe".to_string(), "false".to_string()));
        }
        _ => panic!("expected multipart"),
    }
}

#[tokio::test]
async fn post_json_sets_json_body_and_referer_fallback() {
    let t = Arc::new(CapturingTransport::new());
    t.queue_response(ok_body("{}"));
    let client = GourmetClient::new(t.clone());
    // no prior GET → Referer falls back to the url arg exactly as passed
    client
        .post_json("https://alaclickneu.gourmet.at/umbraco/api/x", "{\"a\":1}".into())
        .await
        .unwrap();

    let post = &t.requests()[0];
    let hdr = |k: &str| post.headers.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    assert_eq!(hdr("Origin"), Some("https://alaclickneu.gourmet.at"));
    assert_eq!(hdr("Referer"), Some("https://alaclickneu.gourmet.at/umbraco/api/x"));
    match &post.body {
        Some(RequestBody::Json(s)) => assert_eq!(s, "{\"a\":1}"),
        _ => panic!("expected json"),
    }
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib gourmet::client`
Expected: FAIL — `post_form`/`post_json` not defined.

- [ ] **Step 3: Implement `post_form`, `post_json`, `referer`**

Add these methods inside `impl GourmetClient`:

```rust
    /// multipart/form-data POST with Origin+Referer (§2.2). Fields keep insertion order.
    pub async fn post_form(
        &self,
        url: &str,
        fields: Vec<(String, String)>,
    ) -> CoreResult<String> {
        let headers = vec![
            ("Origin".to_string(), GOURMET_ORIGIN.to_string()),
            ("Referer".to_string(), self.referer(url)),
        ];
        let resp = self
            .transport
            .send(Request {
                method: Method::Post,
                url: self.absolute_url(url),
                headers,
                body: Some(RequestBody::Multipart(fields)),
            })
            .await?;
        Ok(resp.body)
    }

    /// application/json POST with Origin+Referer (§2.3).
    pub async fn post_json(&self, url: &str, body: String) -> CoreResult<String> {
        let headers = vec![
            ("Origin".to_string(), GOURMET_ORIGIN.to_string()),
            ("Referer".to_string(), self.referer(url)),
        ];
        let resp = self
            .transport
            .send(Request {
                method: Method::Post,
                url: self.absolute_url(url),
                headers,
                body: Some(RequestBody::Json(body)),
            })
            .await?;
        Ok(resp.body)
    }

    /// Referer = lastPageUrl if a GET has happened, else the url arg exactly as passed (§2.2).
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

Run: `cd src/core && cargo test --lib gourmet::client`
Expected: all client tests PASS.

- [ ] **Step 5: Full crate test + fmt/clippy + commit**

Run: `cd src/core && cargo test && cargo fmt && cargo clippy --all-targets`
Expected: all tests pass; clippy clean. (Run `cargo fmt` then re-test if `--check` would fail.)

```bash
git add src/core/src/gourmet
git commit -m "feat(core): gourmet::client — post_form/post_json with Origin/Referer rules"
```

---

## Post-plan: next Gourmet sub-plan

- **Phase 1b-iii — `gourmet::api`**: orchestration wiring `client` + `parser` — login (stale-session pre-logout, §6), `ensureSession` (§7), `getMenus` pagination (§8.1), `getOrders` (§9), `addToCart` (§10.1), `confirmOrders`/`cancelOrders` edit-mode loop (§9.3/§9.4), `getBillings` (§10.2), logout (§11). Full request-sequence tests per 06-testing §6.1 using `CapturingTransport` with queued fixture bodies.

## Self-review notes

- **Spec coverage:** §2.1 (get + lastPageUrl), §2.2 (multipart + Origin/Referer + fallback caveat), §2.3 (json + Origin/Referer), §2.4 (reset), §1 (constants). The `Accept`/no-UA/redirect/cookie config is the foundation transport's job (already tested there) — deliberately not re-tested here.
- **Type consistency:** `GourmetClient::{new,get,post_form,post_json,reset,last_page_url}` are the surface `gourmet::api` will consume; `RequestBody::Multipart` preserves field order (foundation contract); `absolute_url`/`referer`/`encode` are private helpers.
- **Multipart Referer note:** the request-shape assertion checks the header the client sets; the transport's boundary/Content-Type generation for multipart is covered by the foundation reqwest test, not re-asserted here (CapturingTransport records the `RequestBody::Multipart` directly, so field order and values are asserted, not byte boundaries — matches 06-testing §9.2).
