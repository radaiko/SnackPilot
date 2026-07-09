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
        Self {
            transport,
            last_page_url: Mutex::new(String::new()),
        }
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

    /// multipart/form-data POST with Origin+Referer (§2.2). Fields keep insertion order.
    pub async fn post_form(&self, url: &str, fields: Vec<(String, String)>) -> CoreResult<String> {
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

    /// Referer = lastPageUrl if a GET has happened, else the url arg exactly as passed (§2.2).
    fn referer(&self, url: &str) -> String {
        let last = self.last_page_url.lock().unwrap();
        if last.is_empty() {
            url.to_string()
        } else {
            last.clone()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{CapturingTransport, HttpResponse, Method, RequestBody};
    use std::sync::Arc;

    fn ok_body(body: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: vec![],
            body: body.into(),
        }
    }

    #[tokio::test]
    async fn get_sends_bare_url_and_records_last_page() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok_body("<html>menus</html>"));
        let client = GourmetClient::new(t.clone());

        let body = client
            .get("https://alaclickneu.gourmet.at/menus/", &[])
            .await
            .unwrap();
        assert_eq!(body, "<html>menus</html>");

        let reqs = t.requests();
        assert_eq!(reqs[0].method, Method::Get);
        assert_eq!(reqs[0].url, "https://alaclickneu.gourmet.at/menus/");
        assert!(reqs[0]
            .headers
            .iter()
            .all(|(k, _)| k != "Origin" && k != "Referer"));
        assert_eq!(
            client.last_page_url(),
            "https://alaclickneu.gourmet.at/menus/"
        );
    }

    #[tokio::test]
    async fn get_appends_query_but_last_page_strips_it() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok_body("p1"));
        let client = GourmetClient::new(t.clone());

        client
            .get("https://alaclickneu.gourmet.at/menus/", &[("page", "1")])
            .await
            .unwrap();
        assert_eq!(
            t.requests()[0].url,
            "https://alaclickneu.gourmet.at/menus/?page=1"
        );
        assert_eq!(
            client.last_page_url(),
            "https://alaclickneu.gourmet.at/menus/"
        );
    }

    #[tokio::test]
    async fn post_form_carries_origin_referer_and_ordered_multipart() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok_body("login page")); // GET response
        t.queue_response(ok_body("<html>ok</html>")); // POST response
        let client = GourmetClient::new(t.clone());

        client
            .get("https://alaclickneu.gourmet.at/start/", &[])
            .await
            .unwrap();
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

        let post = &t.requests()[1];
        assert_eq!(post.method, Method::Post);
        let hdr = |k: &str| {
            post.headers
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(hdr("Origin"), Some("https://alaclickneu.gourmet.at"));
        assert_eq!(
            hdr("Referer"),
            Some("https://alaclickneu.gourmet.at/start/")
        );
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
            .post_json(
                "https://alaclickneu.gourmet.at/umbraco/api/x",
                "{\"a\":1}".into(),
            )
            .await
            .unwrap();

        let post = &t.requests()[0];
        let hdr = |k: &str| {
            post.headers
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(hdr("Origin"), Some("https://alaclickneu.gourmet.at"));
        assert_eq!(
            hdr("Referer"),
            Some("https://alaclickneu.gourmet.at/umbraco/api/x")
        );
        match &post.body {
            Some(RequestBody::Json(s)) => assert_eq!(s, "{\"a\":1}"),
            _ => panic!("expected json"),
        }
    }

    #[tokio::test]
    async fn reset_clears_last_page_url() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok_body("x"));
        let client = GourmetClient::new(t.clone());
        client
            .get("https://alaclickneu.gourmet.at/start/", &[])
            .await
            .unwrap();
        assert!(!client.last_page_url().is_empty());
        client.reset();
        assert_eq!(client.last_page_url(), "");
    }
}
