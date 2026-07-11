//! Production Transport on reqwest. Shared config for both services: explicit Accept header on
//! every request, a mobile-Safari User-Agent, redirect limit 5, status 200-399 = success
//! (docs/architecture §3.1; 01 §2, 02 §2.1). Per-service cookie behavior is layered above.
use crate::error::{CoreError, CoreResult};
use crate::http::{HttpResponse, Method, Request, RequestBody, Transport};
use std::future::Future;
use std::pin::Pin;

const ACCEPT: &str = "application/json, text/plain, */*";

/// A realistic current mobile-Safari User-Agent. v1 relied on the platform default UA (non-empty);
/// reqwest sends none by default, and an absent UA is a common bot-detection trigger (01 §10.2),
/// so we send a plausible browser UA instead of nothing. Single value for both platforms.
const USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_6 like Mac OS X) \
AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.6 Mobile/15E148 Safari/604.1";

pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    /// `cookie_store` = true for the Gourmet client (reqwest manages cookies + carries them across
    /// redirects), false for Ventopay (which manages its own jar above this layer).
    ///
    /// `follow_redirects` = true for Gourmet (reqwest's cookie store captures the login-302 cookie
    /// while auto-following). Ventopay passes false: reqwest's auto-follow hides intermediate 3xx
    /// `Set-Cookie` headers, so with a manual jar the login-302 session cookie would be dropped
    /// (verified by tests/wire_audit). VentopayClient follows redirects itself, capturing each hop.
    pub fn new(cookie_store: bool, follow_redirects: bool) -> CoreResult<Self> {
        let policy = if follow_redirects {
            reqwest::redirect::Policy::limited(5)
        } else {
            reqwest::redirect::Policy::none()
        };
        let client = reqwest::Client::builder()
            .cookie_store(cookie_store)
            .redirect(policy)
            .user_agent(USER_AGENT)
            // Ceilings (NOT throttling — safe w.r.t. the no-delay rule): a stalled connection
            // (captive-portal Wi-Fi, VPN handoff) otherwise hangs the login/menu spinner forever.
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| CoreError::Http {
                detail: e.to_string(),
            })?;
        Ok(Self { client })
    }
}

impl Transport for ReqwestTransport {
    fn send<'a>(
        &'a self,
        req: Request,
    ) -> Pin<Box<dyn Future<Output = CoreResult<HttpResponse>> + Send + 'a>> {
        Box::pin(async move {
            let method = match req.method {
                Method::Get => reqwest::Method::GET,
                Method::Post => reqwest::Method::POST,
            };
            let mut rb = self
                .client
                .request(method, &req.url)
                .header("Accept", ACCEPT);
            for (k, v) in &req.headers {
                rb = rb.header(k.as_str(), v.as_str());
            }
            rb = match req.body {
                None => rb,
                Some(RequestBody::Json(s)) => rb.header("Content-Type", "application/json").body(s),
                Some(RequestBody::Form(fields)) => {
                    // preserve field order; reqwest .form() takes a slice of pairs.
                    rb.form(&fields)
                }
                Some(RequestBody::Multipart(fields)) => {
                    let mut form = reqwest::multipart::Form::new();
                    for (k, v) in fields {
                        form = form.text(k, v);
                    }
                    rb.multipart(form)
                }
            };
            let resp = rb.send().await.map_err(|e| CoreError::Http {
                detail: e.to_string(),
            })?;
            let status = resp.status().as_u16();
            if status >= 400 {
                return Err(CoreError::Http {
                    detail: format!("HTTP {status}"),
                });
            }
            let headers = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let body = resp.text().await.map_err(|e| CoreError::Http {
                detail: e.to_string(),
            })?;
            Ok(HttpResponse {
                status,
                headers,
                body,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{Method, Request, Transport};
    use std::io::{Read, Write};
    use std::net::TcpListener;

    // Minimal one-shot HTTP/1.1 server: captures the raw request, replies 200.
    fn spawn_capturing_server() -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let _ = tx.send(String::from_utf8_lossy(&buf[..n]).to_string());
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi");
            }
        });
        (format!("http://{addr}/path"), rx)
    }

    #[tokio::test]
    async fn sends_accept_and_mobile_user_agent() {
        let (url, rx) = spawn_capturing_server();
        let t = ReqwestTransport::new(true, true).unwrap();
        let resp = t
            .send(Request {
                method: Method::Get,
                url,
                headers: vec![],
                body: None,
            })
            .await
            .unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, "hi");

        let raw = rx.recv().unwrap();
        assert!(
            raw.contains("accept: application/json, text/plain, */*")
                || raw.contains("Accept: application/json, text/plain, */*"),
            "missing Accept header in:\n{raw}"
        );
        // A non-empty, browser-like UA must be present (absent UA is a bot signal, 01 §10.2).
        assert!(
            raw.to_lowercase().contains("user-agent: mozilla/5.0"),
            "expected the mobile-Safari User-Agent in:\n{raw}"
        );
    }

    #[tokio::test]
    async fn status_400_maps_to_http_error() {
        // server that replies 404
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let mut b = [0u8; 1024];
                let _ = s.read(&mut b);
                let _ = s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
            }
        });
        let t = ReqwestTransport::new(true, true).unwrap();
        let out = t
            .send(Request {
                method: Method::Get,
                url: format!("http://{addr}/x"),
                headers: vec![],
                body: None,
            })
            .await;
        assert!(matches!(out, Err(crate::error::CoreError::Http { .. })));
    }
}
