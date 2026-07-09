//! Ventopay HTTP primitives over the foundation Transport with the app-owned cookie jar
//! (02-ventopay-scraping §2). url-encoded POSTs, Origin/Referer, lastPageUrl.
use crate::error::CoreResult;
use crate::http::cookie_jar::CookieJar;
use crate::http::{HttpResponse, Method, Request, RequestBody, Transport};
use crate::ventopay::{VENTOPAY_BASE_URL, VENTOPAY_ORIGIN};
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

    /// url-encoded POST with Origin + Referer (§2.4). Field order preserved.
    pub async fn post_form(&self, url: &str, fields: Vec<(String, String)>) -> CoreResult<String> {
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
            format!("{}/{}", VENTOPAY_BASE_URL, url.trim_start_matches('/'))
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
            Some("ASP.NET_SessionId=abc; path=/"),
        ));
        let client = VentopayClient::new(t.clone());

        let body = client
            .get(crate::ventopay::VENTOPAY_LOGIN_URL, &[])
            .await
            .unwrap();
        assert_eq!(body, "login");
        let req = &t.requests()[0];
        assert_eq!(req.method, Method::Get);
        assert_eq!(req.url, "https://my.ventopay.com/mocca.website/Login.aspx");
        assert!(req.headers.iter().all(|(k, _)| k != "Cookie"));
        assert_eq!(
            client.last_page_url(),
            "https://my.ventopay.com/mocca.website/Login.aspx"
        );
    }

    #[tokio::test]
    async fn second_request_injects_captured_cookie() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(resp_with_cookie("a", Some("ASP.NET_SessionId=abc; path=/")));
        t.queue_response(resp_with_cookie("b", None));
        let client = VentopayClient::new(t.clone());

        client
            .get(crate::ventopay::VENTOPAY_LOGIN_URL, &[])
            .await
            .unwrap();
        client
            .get(crate::ventopay::VENTOPAY_TRANSACTIONS_URL, &[])
            .await
            .unwrap();

        let second = &t.requests()[1];
        let cookie = second
            .headers
            .iter()
            .find(|(k, _)| k == "Cookie")
            .map(|(_, v)| v.as_str());
        assert_eq!(cookie, Some("ASP.NET_SessionId=abc"));
    }

    #[tokio::test]
    async fn get_query_keeps_dots_and_strips_from_last_page() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(resp_with_cookie("x", None));
        let client = VentopayClient::new(t.clone());
        client
            .get(
                crate::ventopay::VENTOPAY_TRANSACTIONS_URL,
                &[("fromDate", "01.02.2026"), ("untilDate", "28.02.2026")],
            )
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

    #[tokio::test]
    async fn post_form_is_urlencoded_with_origin_and_referer() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(resp_with_cookie("login page", None)); // GET
        t.queue_response(resp_with_cookie("<html>Ausloggen.aspx</html>", None)); // POST
        let client = VentopayClient::new(t.clone());
        client
            .get(crate::ventopay::VENTOPAY_LOGIN_URL, &[])
            .await
            .unwrap();
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
        let hdr = |k: &str| {
            post.headers
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(hdr("Origin"), Some("https://my.ventopay.com"));
        assert_eq!(
            hdr("Referer"),
            Some("https://my.ventopay.com/mocca.website/Login.aspx")
        );
        match &post.body {
            Some(RequestBody::Form(f)) => {
                assert_eq!(f[0].0, "__VIEWSTATE");
                assert_eq!(f[2], ("BtnLogin".to_string(), "Login".to_string()));
            }
            _ => panic!("expected url-encoded form"),
        }
    }
}
