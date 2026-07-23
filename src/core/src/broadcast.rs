//! Broadcast banner — a single operator-controlled message fetched from a public GitHub gist,
//! shown to all users when non-empty (e.g. "a breaking change happened, update the app").
//!
//! The gist's raw endpoint always serves the latest revision as `text/plain`. An empty (or
//! whitespace-only) gist means "no broadcast". Fetching is best-effort: any network/HTTP error
//! yields `None`, never an error the UI must handle — a missing banner is the correct fallback.
use crate::http::{Method, Request, Transport};

/// Raw content of the broadcast gist (latest revision). Owner: radaiko.
pub const BROADCAST_GIST_URL: &str =
    "https://gist.githubusercontent.com/radaiko/f021da975573b90872cc49dd861880f2/raw";

/// Reduce a raw gist body to a broadcast message: trimmed, or `None` when empty.
pub fn parse_broadcast(body: &str) -> Option<String> {
    let msg = body.trim();
    if msg.is_empty() {
        None
    } else {
        Some(msg.to_string())
    }
}

/// GET the gist and return the trimmed message, or `None` when empty or on any transport/HTTP
/// error. Never returns an error — the banner is purely informational.
///
/// The raw-gist CDN (Fastly) caches per-URL for ~1 minute, so `Cache-Control: no-cache` alone is
/// not enough to see a just-edited gist. `cache_buster` (a unique value per call, e.g. the current
/// epoch-ms) is appended as a query param so every fetch is a distinct URL → guaranteed cache miss
/// → the gist's *current* content. Called on every app start / foreground, so an edited or emptied
/// gist reflects immediately.
pub async fn fetch_broadcast(transport: &dyn Transport, cache_buster: &str) -> Option<String> {
    let url = format!("{BROADCAST_GIST_URL}?cb={cache_buster}");
    let resp = transport
        .send(Request {
            method: Method::Get,
            url,
            headers: vec![
                ("Cache-Control".to_string(), "no-cache".to_string()),
                ("Pragma".to_string(), "no-cache".to_string()),
            ],
            body: None,
        })
        .await
        .ok()?;
    parse_broadcast(&resp.body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{CapturingTransport, HttpResponse};
    use std::sync::Arc;

    fn ok(body: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: vec![],
            body: body.into(),
        }
    }

    #[test]
    fn parse_trims_and_keeps_nonempty() {
        assert_eq!(
            parse_broadcast("  Update verfügbar!\n"),
            Some("Update verfügbar!".to_string())
        );
    }

    #[test]
    fn parse_empty_and_whitespace_yield_none() {
        assert_eq!(parse_broadcast(""), None);
        assert_eq!(parse_broadcast("   \n\t  "), None);
    }

    #[tokio::test]
    async fn fetch_returns_message_from_body() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok("Neue Version verfügbar."));
        let msg = fetch_broadcast(t.as_ref(), "12345").await;
        assert_eq!(msg, Some("Neue Version verfügbar.".to_string()));
        // GET to the gist raw URL with a cache-busting query so each focus/start gets fresh data.
        let reqs = t.requests();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].method, Method::Get);
        assert_eq!(reqs[0].url, format!("{BROADCAST_GIST_URL}?cb=12345"));
        assert!(
            reqs[0]
                .headers
                .iter()
                .any(|(k, v)| k == "Cache-Control" && v == "no-cache"),
            "broadcast fetch must send Cache-Control: no-cache"
        );
    }

    #[tokio::test]
    async fn fetch_empty_gist_is_none() {
        let t = Arc::new(CapturingTransport::new());
        t.queue_response(ok("\n"));
        assert_eq!(fetch_broadcast(t.as_ref(), "1").await, None);
    }

    #[tokio::test]
    async fn fetch_transport_error_is_none() {
        // No queued response → CapturingTransport surfaces an error → best-effort None.
        let t = Arc::new(CapturingTransport::new());
        assert_eq!(fetch_broadcast(t.as_ref(), "1").await, None);
    }
}
