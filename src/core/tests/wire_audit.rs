//! Wire-level audit of the REAL reqwest transport against a local mock server (no real accounts).
//!
//! The unit tests use the CapturingTransport *fake*, which only shows headers our code sets. This
//! harness drives the production `ReqwestTransport` at a loopback server that records the exact
//! bytes on the wire, so we can confirm — before touching a real Gourmet/Ventopay account — that
//! v2's traffic matches v1's known-good sequence. It answers the two ban-critical unknowns:
//!
//!   1. User-Agent: v1 sent the platform default (non-empty); v2 must not look bot-like.
//!   2. Ventopay session cookie across the login 302 under cookie_store=false + auto-redirect:
//!      reqwest does NOT surface a `Set-Cookie` set on an intermediate 3xx it follows, so a
//!      session cookie delivered on the 302 would be silently dropped and the session lost.
//!
//! Run:  cargo test --test wire_audit -- --nocapture --test-threads=1

use snackpilot_core::http::reqwest_transport::ReqwestTransport;
use snackpilot_core::http::{Method, Request, RequestBody, Transport};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
struct Captured {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl Captured {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Loopback HTTP/1.1 server. Records each request and replies per path. Reads until a brief idle
/// timeout so it works for both Content-Length and chunked bodies (reqwest multipart).
fn spawn_server() -> (String, Arc<Mutex<Vec<Captured>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");
    let log = Arc::new(Mutex::new(Vec::<Captured>::new()));
    let log2 = log.clone();
    let base2 = base.clone();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let mut stream = match conn {
                Ok(s) => s,
                Err(_) => continue,
            };
            stream
                .set_read_timeout(Some(Duration::from_millis(300)))
                .ok();
            let mut buf = Vec::new();
            let mut tmp = [0u8; 2048];
            loop {
                match stream.read(&mut tmp) {
                    Ok(0) => break,
                    Ok(n) => buf.extend_from_slice(&tmp[..n]),
                    Err(_) => break, // idle timeout → request fully received
                }
            }
            let split = buf
                .windows(4)
                .position(|w| w == b"\r\n\r\n")
                .unwrap_or(buf.len());
            let head = String::from_utf8_lossy(&buf[..split]).to_string();
            let body = String::from_utf8_lossy(buf.get(split + 4..).unwrap_or(&[])).to_string();
            let mut lines = head.split("\r\n");
            let reqline = lines.next().unwrap_or("");
            let mut parts = reqline.split_whitespace();
            let method = parts.next().unwrap_or("").to_string();
            let path = parts.next().unwrap_or("").to_string();
            let headers: Vec<(String, String)> = lines
                .filter_map(|l| {
                    let mut it = l.splitn(2, ": ");
                    Some((it.next()?.to_string(), it.next()?.to_string()))
                })
                .collect();
            log2.lock().unwrap().push(Captured {
                method,
                path: path.clone(),
                headers,
                body,
            });
            let _ = stream.write_all(response_for(&path, &base2).as_bytes());
        }
    });
    (base, log)
}

fn response_for(path: &str, base: &str) -> String {
    if path.contains("start") {
        // Gourmet login page — sets a session cookie reqwest (cookie_store=true) should reuse.
        "HTTP/1.1 200 OK\r\nSet-Cookie: GBID=gourmet-session; Path=/\r\nConnection: close\r\nContent-Length: 5\r\n\r\nstart".to_string()
    } else if path.contains("login-302") {
        // Ventopay login → 302 carrying the session cookie ON the redirect (the risky case).
        format!(
            "HTTP/1.1 302 Found\r\nLocation: {base}/landing\r\nSet-Cookie: ASP.NET_SessionId=vento-session; path=/; HttpOnly\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
        )
    } else if path.contains("landing") {
        "HTTP/1.1 200 OK\r\nSet-Cookie: FINALCK=final; Path=/\r\nConnection: close\r\nContent-Length: 6\r\n\r\nlanded".to_string()
    } else {
        "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\nok".to_string()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wire_audit() {
    let (base, log) = spawn_server();

    // ---- GOURMET transport (reqwest manages cookies) ----
    let g = ReqwestTransport::new(true, true).unwrap();
    g.send(Request {
        method: Method::Get,
        url: format!("{base}/start/"),
        headers: vec![],
        body: None,
    })
    .await
    .unwrap();
    // A follow-up request: reqwest should resend the GBID cookie it captured from /start.
    g.send(Request {
        method: Method::Get,
        url: format!("{base}/menus"),
        headers: vec![],
        body: None,
    })
    .await
    .unwrap();
    // Multipart login-style POST — inspect the on-wire encoding + Content-Type.
    g.send(Request {
        method: Method::Post,
        url: format!("{base}/start/"),
        headers: vec![("Origin".into(), "https://alaclickneu.gourmet.at".into())],
        body: Some(RequestBody::Multipart(vec![
            ("Username".into(), "user".into()),
            ("RememberMe".into(), "false".into()),
        ])),
    })
    .await
    .unwrap();

    // ---- VENTOPAY transport (cookie_store=false; own jar above) ----
    let v = ReqwestTransport::new(false, false).unwrap();
    let vento_login = v
        .send(Request {
            method: Method::Post,
            url: format!("{base}/login-302"),
            headers: vec![],
            body: Some(RequestBody::Form(vec![(
                "TxtUsername".into(),
                "user".into(),
            )])),
        })
        .await
        .unwrap();

    let reqs = log.lock().unwrap().clone();

    // ================= REPORT =================
    println!("\n==================== WIRE AUDIT ====================");
    for (i, r) in reqs.iter().enumerate() {
        println!("\n── request #{i}: {} {}", r.method, r.path);
        for (k, val) in &r.headers {
            println!("     {k}: {val}");
        }
        if !r.body.is_empty() {
            let b = r.body.replace('\r', "\\r").replace('\n', "\\n");
            let b = if b.len() > 400 { &b[..400] } else { &b };
            println!("     [body] {b}");
        }
    }
    println!("\n── Ventopay login response headers (AFTER following the 302):");
    for (k, val) in &vento_login.headers {
        println!("     {k}: {val}");
    }

    let vento_302_cookie_preserved = vento_login
        .headers
        .iter()
        .any(|(k, val)| k.eq_ignore_ascii_case("set-cookie") && val.contains("ASP.NET_SessionId"));
    println!("\n==================== FINDINGS ====================");
    println!(
        "User-Agent present (mobile Safari) on all requests: {}",
        reqs.iter().all(|r| r
            .header("user-agent")
            .is_some_and(|ua| ua.starts_with("Mozilla/5.0")))
    );
    println!(
        "Accept header present: {}",
        reqs.iter()
            .all(|r| r.header("accept") == Some("application/json, text/plain, */*"))
    );
    println!(
        "Gourmet cookie_store resends session cookie: {}",
        reqs.iter()
            .find(|r| r.path.contains("menus"))
            .and_then(|r| r.header("cookie"))
            .map(|c| c.contains("GBID=gourmet-session"))
            .unwrap_or(false)
    );
    println!(
        "Ventopay 302 Set-Cookie preserved in response (CRITICAL): {vento_302_cookie_preserved}"
    );
    println!("===================================================\n");

    // ---- Invariants that MUST hold (locked) ----
    for r in &reqs {
        assert!(
            r.header("user-agent")
                .is_some_and(|ua| ua.starts_with("Mozilla/5.0")),
            "missing/altered User-Agent on {} {} (absent UA is a bot signal, 01 §10.2): {:?}",
            r.method,
            r.path,
            r.header("user-agent")
        );
        assert_eq!(
            r.header("accept"),
            Some("application/json, text/plain, */*"),
            "missing/altered Accept on {} {}",
            r.method,
            r.path
        );
    }
    // Gourmet: reqwest's cookie store must carry the session cookie onto later requests.
    let menus = reqs.iter().find(|r| r.path.contains("menus")).unwrap();
    assert!(
        menus
            .header("cookie")
            .is_some_and(|c| c.contains("GBID=gourmet-session")),
        "Gourmet cookie_store did not resend the session cookie: {:?}",
        menus.header("cookie")
    );
    // Multipart encoding must be multipart/form-data (NOT urlencoded — ban-critical, 01 §2.2).
    let mp = reqs
        .iter()
        .find(|r| r.method == "POST" && r.path.contains("start"))
        .unwrap();
    assert!(
        mp.header("content-type")
            .is_some_and(|c| c.starts_with("multipart/form-data")),
        "Gourmet form POST must be multipart/form-data, got {:?}",
        mp.header("content-type")
    );
    assert!(
        mp.body.contains("RememberMe") && mp.body.contains("false"),
        "multipart body missing fields"
    );
    // Post-fix: with redirects OFF for Ventopay, the transport now surfaces the 302's Set-Cookie
    // (the VentopayClient follows + captures it manually — proven by the client test below).
    assert!(
        vento_302_cookie_preserved,
        "Ventopay transport must surface the login-302 Set-Cookie (redirects off)"
    );
}

/// End-to-end proof of the fix: the real VentopayClient must capture the session cookie set on the
/// login 302, follow the redirect to the landing page, and resend the cookie on later requests.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ventopay_client_keeps_302_session_cookie() {
    use snackpilot_core::ventopay::client::VentopayClient;

    let (base, log) = spawn_server();
    let transport = Arc::new(ReqwestTransport::new(false, false).unwrap());
    let client = VentopayClient::new(transport);

    // Login POST → server replies 302 (Set-Cookie ASP.NET_SessionId) → client follows to /landing.
    let body = client
        .post_form(
            &format!("{base}/login-302"),
            vec![("TxtUsername".into(), "user".into())],
        )
        .await
        .unwrap();
    assert_eq!(
        body, "landed",
        "client must follow the 302 to the landing page"
    );

    // A subsequent request must carry the captured session cookie.
    client.get(&format!("{base}/echo"), &[]).await.unwrap();

    let reqs = log.lock().unwrap().clone();
    let echo = reqs
        .iter()
        .rev()
        .find(|r| r.path.contains("echo"))
        .expect("echo request");
    let cookie = echo.header("cookie").unwrap_or("");
    println!("\n[ventopay client] follow-up request Cookie: {cookie}");
    assert!(
        cookie.contains("ASP.NET_SessionId=vento-session"),
        "login-302 session cookie must be captured + resent, got: {cookie:?}"
    );
}
