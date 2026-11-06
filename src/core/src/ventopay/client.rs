//! Ventopay HTTP primitives over the foundation Transport with the app-owned cookie jar
//! (02-ventopay-scraping §2). JSON POSTs, Origin/Referer, Bearer token, lastPageUrl.
use crate::error::CoreResult;
use crate::http::cookie_jar::CookieJar;
use crate::http::{HttpResponse, Method, Request, RequestBody, Transport};
use crate::ventopay::{VENTOPAY_ORIGIN, VENTOPAY_WEB_URL};
use std::sync::{Arc, Mutex};

pub struct VentopayClient {
    transport: Arc<dyn Transport>,
    jar: Mutex<CookieJar>,
    bearer: Mutex<Option<String>>,
    last_page_url: Mutex<String>,
}

impl VentopayClient {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self {
            transport,
            jar: Mutex::new(CookieJar::new()),
            bearer: Mutex::new(None),
            last_page_url: Mutex::new(String::new()),
        }
    }

    pub fn last_page_url(&self) -> String {
        self.last_page_url.lock().unwrap().clone()
    }

    /// Clears the cookie jar, the bearer token and lastPageUrl (logout, 02 §4).
    pub fn reset(&self) {
        self.jar.lock().unwrap().clear();
        self.bearer.lock().unwrap().take();
        self.last_page_url.lock().unwrap().clear();
    }

    /// Stores the access token sent as `authorization: Bearer …` on every later request (02 §2.5).
    pub fn set_bearer(&self, token: Option<String>) {
        *self.bearer.lock().unwrap() = token;
    }

    /// GET with optional query params. Records lastPageUrl (query-stripped).
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
            .send(Request {
                method: Method::Get,
                url: full,
                headers: vec![],
                body: None,
            })
            .await?;
        *self.last_page_url.lock().unwrap() = base;
        Ok(resp.body)
    }

    /// JSON POST with Origin + Referer (§2.4). The transport sets Content-Type.
    pub async fn post_json(&self, url: &str, body: String) -> CoreResult<String> {
        let headers = vec![
            ("Origin".to_string(), VENTOPAY_ORIGIN.to_string()),
            ("Referer".to_string(), self.referer(url)),
        ];
        let resp = self
            .send(Request {
                method: Method::Post,
                url: self.absolute_url(url),
                headers,
                body: Some(RequestBody::Json(body)),
            })
            .await?;
        Ok(resp.body)
    }

    /// DELETE with Origin + Referer (logout, §4).
    pub async fn delete(&self, url: &str) -> CoreResult<String> {
        let headers = vec![
            ("Origin".to_string(), VENTOPAY_ORIGIN.to_string()),
            ("Referer".to_string(), self.referer(url)),
        ];
        let resp = self
            .send(Request {
                method: Method::Delete,
                url: self.absolute_url(url),
                headers,
                body: None,
            })
            .await?;
        Ok(resp.body)
    }

    /// Inject the Cookie header from the jar and the Bearer token, send, capture Set-Cookie into
    /// the jar (§2.2), and FOLLOW 3xx redirects manually (up to 5). The transport is built with
    /// redirects OFF for Ventopay: reqwest's auto-follow hides intermediate `Set-Cookie` headers.
    async fn send(&self, req: Request) -> CoreResult<HttpResponse> {
        let mut next = req;
        let mut hops = 0u8;
        loop {
            if let Some(h) = self.jar.lock().unwrap().header() {
                next.headers.push(("Cookie".to_string(), h));
            }
            if let Some(t) = self.bearer.lock().unwrap().as_ref() {
                next.headers
                    .push(("authorization".to_string(), format!("Bearer {t}")));
            }
            let resp = self.transport.send(next).await?;
            let set_cookies: Vec<String> = resp
                .headers
                .iter()
                .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
                .map(|(_, v)| v.clone())
                .collect();
            if !set_cookies.is_empty() {
                self.jar.lock().unwrap().capture(&set_cookies);
            }
            // A 3xx with a Location → follow as a GET, carrying the just-captured cookies.
            // Non-redirects (and after 5 hops) return as-is.
            if (300..400).contains(&resp.status) && hops < 5 {
                if let Some(loc) = resp
                    .headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("location"))
                    .map(|(_, v)| v.clone())
                {
                    hops += 1;
                    next = Request {
                        method: Method::Get,
                        url: self.resolve_redirect(&loc),
                        headers: vec![],
                        body: None,
                    };
                    continue;
                }
            }
            return Ok(resp);
        }
    }

    /// Resolve a redirect `Location` (absolute, root-relative, or path-relative) against the
    /// web origin. Query strings are preserved (unlike `absolute_url`).
    fn resolve_redirect(&self, loc: &str) -> String {
        if loc.starts_with("http") {
            loc.to_string()
        } else if loc.starts_with('/') {
            format!("{VENTOPAY_WEB_URL}{loc}")
        } else {
            format!("{VENTOPAY_WEB_URL}/{loc}")
        }
    }

    fn absolute_url(&self, url: &str) -> String {
        if url.starts_with("http") {
            url.split('?').next().unwrap_or(url).to_string()
        } else {
            format!("{VENTOPAY_WEB_URL}/{}", url.trim_start_matches('/'))
        }
    }

    fn referer(&self, url: &str) -> String {
        let last = self.last_page_url.lock().unwrap();
        if last.is_empty() {
            url.to_string()
        } else {
            last.clone()
        }
    }
}

/// Query encoding leaving RFC3986 unreserved chars unencoded (02 §2.3). Ventopay date params
/// are ISO `YYYY-MM-DDTHH:mm:ss`; the ':' becomes %3A, matching the web app's
/// encodeURIComponent serializer.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{CapturingTransport, HttpResponse, Method, RequestBody};
    use std::sync::Arc;

    fn resp_with_cookie(body: &str, set_cookie: Option<&str>) -> HttpResponse {
        let headers = match set_cookie {
            Some(c) => vec![("set-cookie".to_string(), c.to_string())],
            None => vec![],
        };
        HttpResponse {
            status: 200,
            headers,
            body: body.into(),
        }
    }

    #[tokio::test]
    async fn get_records_last_page_and_captures_cookie() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(resp_with_cookie(
            "login",
            Some("RefreshToken=abc; path=/; secure; httponly"),
        ));
        let client = VentopayClient::new(t.clone());

        let body = client
            .get(crate::ventopay::VENTOPAY_LOGIN_URL, &[])
            .await
            .unwrap();
        assert_eq!(body, "login");
        let req = &t.requests()[0];
        assert_eq!(req.method, Method::Get);
        assert_eq!(
            req.url,
            "https://xperience-trumpf.ventopay.cloud/auth/login"
        );
        assert!(req.headers.iter().all(|(k, _)| k != "Cookie"));
        assert_eq!(
            client.last_page_url(),
            "https://xperience-trumpf.ventopay.cloud/auth/login"
        );
    }

    #[tokio::test]
    async fn second_request_injects_captured_cookie() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(resp_with_cookie(
            "a",
            Some("RefreshToken=abc; path=/; secure"),
        ));
        t.queue_response(resp_with_cookie("b", None));
        let client = VentopayClient::new(t.clone());

        client
            .get(crate::ventopay::VENTOPAY_LOGIN_URL, &[])
            .await
            .unwrap();
        client
            .get(crate::ventopay::VENTOPAY_ACCESS_TOKEN_URL, &[])
            .await
            .unwrap();

        let second = &t.requests()[1];
        let cookie = second
            .headers
            .iter()
            .find(|(k, _)| k == "Cookie")
            .map(|(_, v)| v.as_str());
        assert_eq!(cookie, Some("RefreshToken=abc"));
    }

    #[tokio::test]
    async fn get_query_percent_encodes_colon_in_iso_dates() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(resp_with_cookie("x", None));
        let client = VentopayClient::new(t.clone());
        client
            .get(
                crate::ventopay::VENTOPAY_TRANSACTIONS_URL,
                &[
                    ("fromDate", "2026-02-01T00:00:00"),
                    ("untilDate", "2026-02-28T23:59:59"),
                ],
            )
            .await
            .unwrap();
        assert_eq!(
            t.requests()[0].url,
            "https://gw-trumpf.ventopay.cloud/Customers/v1/person/bons?fromDate=2026-02-01T00%3A00%3A00&untilDate=2026-02-28T23%3A59%3A59"
        );
        assert_eq!(
            client.last_page_url(),
            "https://gw-trumpf.ventopay.cloud/Customers/v1/person/bons"
        );
    }

    #[tokio::test]
    async fn post_json_is_json_with_origin_and_referer() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(resp_with_cookie("ok", None)); // GET seeds lastPageUrl
        t.queue_response(resp_with_cookie("{}", None)); // POST
        let client = VentopayClient::new(t.clone());
        client
            .get(crate::ventopay::VENTOPAY_WEB_URL, &[])
            .await
            .unwrap();
        client
            .post_json(
                crate::ventopay::VENTOPAY_LOGIN_URL,
                r#"{"username":"u","password":"p"}"#.into(),
            )
            .await
            .unwrap();

        let post = &t.requests()[1];
        assert_eq!(post.method, Method::Post);
        let hdr = |k: &str| {
            post.headers
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(
            hdr("Origin"),
            Some("https://xperience-trumpf.ventopay.cloud")
        );
        assert_eq!(
            hdr("Referer"),
            Some("https://xperience-trumpf.ventopay.cloud")
        );
        match &post.body {
            Some(RequestBody::Json(b)) => assert!(b.contains("\"username\":\"u\"")),
            _ => panic!("expected json body"),
        }
    }

    #[tokio::test]
    async fn bearer_is_attached_after_set() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(resp_with_cookie("{}", None));
        let client = VentopayClient::new(t.clone());
        client.set_bearer(Some("TOK".into()));
        client
            .get(crate::ventopay::VENTOPAY_TRANSACTIONS_URL, &[])
            .await
            .unwrap();
        let req = &t.requests()[0];
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "authorization" && v == "Bearer TOK"));
        client.reset();
        assert!(client.bearer.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_sends_origin_and_referer() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(resp_with_cookie("", None));
        let client = VentopayClient::new(t.clone());
        client
            .delete(crate::ventopay::VENTOPAY_LOGOUT_URL)
            .await
            .unwrap();
        let req = &t.requests()[0];
        assert_eq!(req.method, Method::Delete);
        assert!(req.headers.iter().any(|(k, _)| k == "Origin"));
    }
}
