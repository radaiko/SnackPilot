//! HTTP transport abstraction. All request-shape guarantees live in the gourmet/ventopay
//! clients ABOVE this trait, so tests pin exact bytes with a fake (docs/architecture §3.1, §9.2).
use crate::error::CoreResult;
use std::future::Future;
use std::pin::Pin;

pub mod cookie_jar;
pub mod fake;
pub mod reqwest_transport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequestBody {
    /// multipart/form-data — field order preserved (Gourmet, 01 §2.2).
    Multipart(Vec<(String, String)>),
    /// application/x-www-form-urlencoded — insertion order (Ventopay, 02 §2.4).
    Form(Vec<(String, String)>),
    /// application/json — pre-serialized body.
    Json(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<RequestBody>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// Object-safe async transport. Implementations: reqwest (production), CapturingTransport
/// (tests). Boxed future keeps `dyn Transport` usable without the async-trait crate.
pub trait Transport: Send + Sync {
    fn send<'a>(
        &'a self,
        req: Request,
    ) -> Pin<Box<dyn Future<Output = CoreResult<HttpResponse>> + Send + 'a>>;
}
