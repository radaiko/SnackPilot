use crate::error::{CoreError, CoreResult};
use crate::http::{HttpResponse, Request, Transport};
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

/// Test double: records outgoing requests in order, replies with queued responses (§9.2).
pub struct CapturingTransport {
    requests: Mutex<Vec<Request>>,
    responses: Mutex<std::collections::VecDeque<HttpResponse>>,
}

impl CapturingTransport {
    pub fn new() -> Self {
        Self {
            requests: Mutex::new(vec![]),
            responses: Mutex::new(Default::default()),
        }
    }
    pub fn queue_response(&self, resp: HttpResponse) {
        self.responses.lock().unwrap().push_back(resp);
    }
    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

impl Default for CapturingTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for CapturingTransport {
    fn send<'a>(
        &'a self,
        req: Request,
    ) -> Pin<Box<dyn Future<Output = CoreResult<HttpResponse>> + Send + 'a>> {
        Box::pin(async move {
            self.requests.lock().unwrap().push(req);
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| CoreError::Http {
                    detail: "no queued response".into(),
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{HttpResponse, Method, Request, RequestBody, Transport};

    #[tokio::test]
    async fn capturing_transport_records_requests_and_returns_queued_bodies() {
        let t = CapturingTransport::new();
        t.queue_response(HttpResponse {
            status: 200,
            headers: vec![],
            body: "OK".into(),
        });

        let req = Request {
            method: Method::Post,
            url: "https://example.test/x".into(),
            headers: vec![("Accept".into(), "application/json, text/plain, */*".into())],
            body: Some(RequestBody::Multipart(vec![
                ("Username".into(), "u".into()),
                ("Password".into(), "p".into()),
            ])),
        };
        let resp = t.send(req).await.unwrap();
        assert_eq!(resp.body, "OK");

        let captured = t.requests();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].url, "https://example.test/x");
        // field order preserved
        match &captured[0].body {
            Some(RequestBody::Multipart(fields)) => {
                assert_eq!(fields[0].0, "Username");
                assert_eq!(fields[1].0, "Password");
            }
            _ => panic!("expected multipart"),
        }
    }

    #[tokio::test]
    async fn capturing_transport_errors_when_queue_empty() {
        let t = CapturingTransport::new();
        let req = Request {
            method: Method::Get,
            url: "u".into(),
            headers: vec![],
            body: None,
        };
        assert!(t.send(req).await.is_err());
    }
}
