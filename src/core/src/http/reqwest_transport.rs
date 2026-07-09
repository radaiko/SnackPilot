//! Production Transport on reqwest. Shared config for both services: explicit Accept
//! header on every request, NO User-Agent, redirect limit 5, status 200-399 = success
//! (docs/architecture §3.1; 01 §2, 02 §2.1). Per-service cookie behavior is layered above.
use crate::error::{CoreError, CoreResult};
use crate::http::{HttpResponse, Method, Request, RequestBody, Transport};
use std::future::Future;
use std::pin::Pin;

const ACCEPT: &str = "application/json, text/plain, */*";

pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    /// `cookie_store` = true for the Gourmet client (reqwest manages cookies),
    /// false for Ventopay (which manages its own jar above this layer).
    pub fn new(cookie_store: bool) -> CoreResult<Self> {
        let client = reqwest::Client::builder()
            .cookie_store(cookie_store)
            .redirect(reqwest::redirect::Policy::limited(5))
            // reqwest sets no default UA when we don't call .user_agent(); leave it absent.
            .build()
            .map_err(|e| CoreError::Http {
                message: e.to_string(),
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
                message: e.to_string(),
            })?;
            let status = resp.status().as_u16();
            if status >= 400 {
                return Err(CoreError::Http {
                    message: format!("HTTP {status}"),
                });
            }
            let headers = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let body = resp.text().await.map_err(|e| CoreError::Http {
                message: e.to_string(),
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
    async fn sends_accept_header_and_no_user_agent() {
        let (url, rx) = spawn_capturing_server();
        let t = ReqwestTransport::new(true).unwrap();
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
        assert!(
            !raw.to_lowercase().contains("user-agent:"),
            "reqwest sent a User-Agent, must be none:\n{raw}"
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
        let t = ReqwestTransport::new(true).unwrap();
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
