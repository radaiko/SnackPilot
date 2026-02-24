# WinHTTP Native Proxy Support — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** On Windows, replace reqwest with native WinHTTP for scraping HTTP requests so PAC-file proxy resolution and NTLM/Kerberos authentication work automatically via the OS.

**Architecture:** Add a `winhttp_client.rs` module behind `#[cfg(target_os = "windows")]` that wraps WinHTTP API calls via the `windows-sys` crate. The Tauri `http_request` and `http_reset` commands dispatch to WinHTTP on Windows and reqwest on macOS/Linux. The TypeScript side is unchanged — same IPC interface.

**Tech Stack:** `windows-sys` 0.59+ (Microsoft-maintained FFI bindings), WinHTTP Win32 API, existing Tauri v2 + reqwest infrastructure.

**Design doc:** `docs/plans/2026-02-24-winhttp-proxy-design.md`

---

### Task 1: Add `windows-sys` dependency (Windows-only)

**Files:**
- Modify: `src/desktop/src-tauri/Cargo.toml`

**Step 1: Add the windows-sys dependency**

Append after the existing `[dependencies]` section:

```toml
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.59", features = ["Win32_Networking_WinHttp"] }
```

The `Win32_Networking_WinHttp` feature transitively enables `Win32_Foundation` (for `BOOL`, `GetLastError`).

**Step 2: Verify it compiles**

Run: `cd src/desktop/src-tauri && cargo check`
Expected: compiles successfully (on macOS the windows-sys dep is simply skipped).

**Step 3: Commit**

```bash
git add src/desktop/src-tauri/Cargo.toml
git commit -m "chore: add windows-sys dependency for WinHTTP support (#10)"
```

---

### Task 2: Create `winhttp_client.rs` — string helpers and session management

**Files:**
- Create: `src/desktop/src-tauri/src/winhttp_client.rs`

This task builds the foundation: wide-string conversion, handle RAII wrapper, and session creation/reset.

**Step 1: Create the module file with string helpers and session struct**

Create `src/desktop/src-tauri/src/winhttp_client.rs`:

```rust
//! WinHTTP-based HTTP client for Windows.
//!
//! Replaces reqwest for scraping requests on Windows to support:
//! - PAC file proxy auto-configuration
//! - WPAD auto-detection
//! - NTLM/Kerberos proxy authentication (via the logged-in user's credentials)
//!
//! Uses `WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY` which delegates all proxy
//! resolution and authentication to the Windows networking stack.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use std::sync::Mutex;

use windows_sys::Win32::Networking::WinHttp::*;

/// Convert a Rust string to a null-terminated UTF-16 wide string for Win32 APIs.
fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0u16))
        .collect()
}

/// Convert a UTF-16 wide string (with known byte length) to a Rust String.
/// `byte_len` is the size in bytes (not u16 code units) returned by WinHTTP.
fn from_wide(buf: &[u16], byte_len: usize) -> String {
    let char_len = byte_len / 2;
    // Trim trailing null if present
    let slice = if char_len > 0 && buf[char_len - 1] == 0 {
        &buf[..char_len - 1]
    } else {
        &buf[..char_len]
    };
    String::from_utf16_lossy(slice)
}

/// RAII wrapper for a WinHTTP HINTERNET handle.
/// Calls `WinHttpCloseHandle` on drop.
struct WinHandle(*mut std::ffi::c_void);

impl WinHandle {
    fn is_null(&self) -> bool {
        self.0.is_null()
    }

    fn as_ptr(&self) -> *mut std::ffi::c_void {
        self.0
    }
}

impl Drop for WinHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { WinHttpCloseHandle(self.0); }
        }
    }
}

// WinHandle must not be shared across threads — WinHTTP handles are
// single-threaded. We protect access via Mutex in WinHttpSession.
unsafe impl Send for WinHandle {}

/// Persistent WinHTTP session with automatic proxy and cookie support.
///
/// Analogous to reqwest's `Client` with a cookie jar.
/// The session handle persists cookies across requests.
/// On reset (logout), the session is destroyed and recreated.
pub struct WinHttpSession {
    session: Mutex<WinHandle>,
}

impl WinHttpSession {
    /// Create a new WinHTTP session with automatic proxy detection.
    pub fn new() -> Result<Self, String> {
        let session = Self::create_session()?;
        Ok(Self {
            session: Mutex::new(session),
        })
    }

    fn create_session() -> Result<WinHandle, String> {
        let agent = to_wide("SnackPilot/1.0");
        let handle = unsafe {
            WinHttpOpen(
                agent.as_ptr(),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                ptr::null(),  // no explicit proxy name
                ptr::null(),  // no proxy bypass list
                0,            // synchronous mode
            )
        };
        if handle.is_null() {
            return Err(format!(
                "WinHttpOpen failed (error {})",
                unsafe { windows_sys::Win32::Foundation::GetLastError() }
            ));
        }
        Ok(WinHandle(handle))
    }

    /// Reset the session (clears all cookies). Called on logout.
    pub fn reset(&self) -> Result<(), String> {
        let mut session = self.session.lock().map_err(|e| e.to_string())?;
        *session = Self::create_session()?;
        Ok(())
    }
}
```

**Step 2: Wire the module into lib.rs (compile-gated)**

In `src/desktop/src-tauri/src/lib.rs`, add at the top (after the existing `use` statements):

```rust
#[cfg(target_os = "windows")]
mod winhttp_client;
```

**Step 3: Verify it compiles**

Run: `cd src/desktop/src-tauri && cargo check`
Expected: compiles successfully. On macOS the `winhttp_client` module is skipped entirely.

**Step 4: Commit**

```bash
git add src/desktop/src-tauri/src/winhttp_client.rs src/desktop/src-tauri/src/lib.rs
git commit -m "feat: add WinHTTP session management module (#10)"
```

---

### Task 3: Implement multipart form-data body builder

**Files:**
- Modify: `src/desktop/src-tauri/src/winhttp_client.rs`

WinHTTP has no built-in multipart support. Add a helper that constructs the `multipart/form-data` body and returns the boundary string (needed for the `Content-Type` header).

**Step 1: Add the multipart builder function**

Append to `winhttp_client.rs`, after the `WinHttpSession` impl block:

```rust
/// Build a multipart/form-data body from key-value pairs.
/// Returns `(body_bytes, boundary_string)`.
///
/// The caller must set the Content-Type header to:
///   `multipart/form-data; boundary={boundary}`
fn build_multipart_body(fields: &HashMap<String, String>) -> (Vec<u8>, String) {
    let boundary = format!("----WinHttpBoundary{:016x}", {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    });

    let mut body = Vec::new();
    for (key, value) in fields {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n", key).as_bytes(),
        );
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    (body, boundary)
}
```

**Step 2: Verify it compiles**

Run: `cd src/desktop/src-tauri && cargo check`
Expected: compiles (function unused warning is fine for now).

**Step 3: Commit**

```bash
git add src/desktop/src-tauri/src/winhttp_client.rs
git commit -m "feat: add multipart/form-data body builder for WinHTTP (#10)"
```

---

### Task 4: Implement the core `execute_request` function

**Files:**
- Modify: `src/desktop/src-tauri/src/winhttp_client.rs`

This is the main function that takes an `HttpRequest` and returns an `HttpResponse` using WinHTTP. It handles URL parsing, request construction, header management, body sending, response reading, and Set-Cookie extraction.

**Step 1: Add URL parsing helper**

Append to `winhttp_client.rs`:

```rust
/// Parse a URL string into (scheme_is_https, host, port, path_and_query).
fn parse_url(url: &str) -> Result<(bool, String, u16, String), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL '{}': {}", url, e))?;

    let is_https = parsed.scheme() == "https";
    let host = parsed.host_str()
        .ok_or_else(|| format!("No host in URL: {}", url))?
        .to_string();
    let port = parsed.port().unwrap_or(if is_https { 443 } else { 80 });

    // path + query, e.g. "/menus?page=0"
    let mut path_and_query = parsed.path().to_string();
    if let Some(query) = parsed.query() {
        path_and_query.push('?');
        path_and_query.push_str(query);
    }

    Ok((is_https, host, port, path_and_query))
}
```

Note: This requires the `url` crate. Check if it's already a transitive dependency of reqwest (it is — reqwest re-exports `reqwest::Url` which is `url::Url`). Add it explicitly to `Cargo.toml` under `[target.'cfg(windows)'.dependencies]`:

```toml
url = "2"
```

**Step 2: Add the `execute_request` function**

Append to `winhttp_client.rs`. This function implements the full WinHTTP request lifecycle:

```rust
use crate::{HttpRequest, HttpResponse};

/// Execute an HTTP request using WinHTTP.
///
/// This is called from the `http_request` Tauri command on Windows.
/// Returns the same `HttpResponse` struct as the reqwest path.
pub fn execute_request(
    session: &WinHttpSession,
    request: &HttpRequest,
) -> Result<HttpResponse, String> {
    let (is_https, host, port, path_and_query) = parse_url(&request.url)?;

    let session_guard = session.session.lock().map_err(|e| e.to_string())?;
    if session_guard.is_null() {
        return Err("WinHTTP session not initialized".into());
    }

    // 1. Connect to host
    let host_wide = to_wide(&host);
    let connection = WinHandle(unsafe {
        WinHttpConnect(
            session_guard.as_ptr(),
            host_wide.as_ptr(),
            port,
            0, // reserved
        )
    });
    if connection.is_null() {
        return Err(format!(
            "WinHttpConnect to {}:{} failed (error {})",
            host, port,
            unsafe { windows_sys::Win32::Foundation::GetLastError() }
        ));
    }

    // 2. Create request
    let method_wide = to_wide(&request.method.to_uppercase());
    let path_wide = to_wide(&path_and_query);
    let flags = if is_https { WINHTTP_FLAG_SECURE } else { 0 };

    let req_handle = WinHandle(unsafe {
        WinHttpOpenRequest(
            connection.as_ptr(),
            method_wide.as_ptr(),
            path_wide.as_ptr(),
            ptr::null(),              // HTTP version (null = HTTP/1.1)
            ptr::null(),              // referrer (null = none)
            ptr::null(),              // accept types (null = no restriction)
            flags,
        )
    });
    if req_handle.is_null() {
        return Err(format!(
            "WinHttpOpenRequest failed (error {})",
            unsafe { windows_sys::Win32::Foundation::GetLastError() }
        ));
    }

    // 3. Prepare body and headers
    let (body_bytes, extra_headers) = prepare_body(request)?;

    // 4. Add custom headers from request + any generated headers (e.g. Content-Type for multipart)
    let mut all_headers = String::new();
    for (key, value) in &request.headers {
        all_headers.push_str(&format!("{}: {}\r\n", key, value));
    }
    all_headers.push_str(&extra_headers);

    if !all_headers.is_empty() {
        let headers_wide = to_wide(&all_headers);
        let ok = unsafe {
            WinHttpAddRequestHeaders(
                req_handle.as_ptr(),
                headers_wide.as_ptr(),
                u32::MAX, // auto-compute length
                WINHTTP_ADDREQ_FLAG_ADD | WINHTTP_ADDREQ_FLAG_REPLACE,
            )
        };
        if ok == 0 {
            return Err(format!(
                "WinHttpAddRequestHeaders failed (error {})",
                unsafe { windows_sys::Win32::Foundation::GetLastError() }
            ));
        }
    }

    // 5. Send request
    let body_ptr = if body_bytes.is_empty() {
        ptr::null()
    } else {
        body_bytes.as_ptr() as *const std::ffi::c_void
    };
    let body_len = body_bytes.len() as u32;

    let ok = unsafe {
        WinHttpSendRequest(
            req_handle.as_ptr(),
            ptr::null(),  // no additional headers at send time
            0,
            body_ptr,
            body_len,
            body_len,     // total content length
            0,            // context (unused in sync mode)
        )
    };
    if ok == 0 {
        return Err(format!(
            "WinHttpSendRequest failed (error {})",
            unsafe { windows_sys::Win32::Foundation::GetLastError() }
        ));
    }

    // 6. Receive response
    let ok = unsafe {
        WinHttpReceiveResponse(req_handle.as_ptr(), ptr::null_mut())
    };
    if ok == 0 {
        return Err(format!(
            "WinHttpReceiveResponse failed (error {})",
            unsafe { windows_sys::Win32::Foundation::GetLastError() }
        ));
    }

    // 7. Read status code
    let status = query_status_code(&req_handle)?;

    // 8. Read response headers
    let (headers, set_cookies) = query_response_headers(&req_handle)?;

    // 9. Read response body
    let body = read_response_body(&req_handle)?;

    // 10. Determine final URL (WinHTTP follows redirects automatically;
    //     we return the original URL since we don't have easy access to the final one)
    let resp_url = request.url.clone();

    Ok(HttpResponse {
        status,
        headers,
        set_cookies,
        body,
        url: resp_url,
    })
}

/// Prepare the request body bytes and any extra headers (e.g. Content-Type for multipart).
fn prepare_body(request: &HttpRequest) -> Result<(Vec<u8>, String), String> {
    if let Some(ref form_data) = request.form_data {
        // Multipart form data (used by Gourmet forms)
        let (body, boundary) = build_multipart_body(form_data);
        let header = format!("Content-Type: multipart/form-data; boundary={}\r\n", boundary);
        Ok((body, header))
    } else if let Some(ref body) = request.body {
        // String body (URL-encoded or JSON) — already has Content-Type from request.headers
        Ok((body.as_bytes().to_vec(), String::new()))
    } else {
        // No body (GET request)
        Ok((Vec::new(), String::new()))
    }
}

/// Query the HTTP status code from a completed WinHTTP request.
fn query_status_code(req: &WinHandle) -> Result<u16, String> {
    let mut status: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let ok = unsafe {
        WinHttpQueryHeaders(
            req.as_ptr(),
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            ptr::null(),
            &mut status as *mut u32 as *mut std::ffi::c_void,
            &mut size,
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(format!(
            "WinHttpQueryHeaders (status code) failed (error {})",
            unsafe { windows_sys::Win32::Foundation::GetLastError() }
        ));
    }
    Ok(status as u16)
}

/// Query all response headers and extract Set-Cookie headers separately.
fn query_response_headers(
    req: &WinHandle,
) -> Result<(HashMap<String, String>, Vec<String>), String> {
    // First call: get required buffer size
    let mut buf_size: u32 = 0;
    unsafe {
        WinHttpQueryHeaders(
            req.as_ptr(),
            WINHTTP_QUERY_RAW_HEADERS_CRLF,
            ptr::null(),
            ptr::null_mut(),
            &mut buf_size,
            ptr::null_mut(),
        );
    }
    // Expected: returns FALSE with ERROR_INSUFFICIENT_BUFFER, buf_size now set

    if buf_size == 0 {
        return Ok((HashMap::new(), Vec::new()));
    }

    // Second call: read headers into buffer
    let char_count = (buf_size as usize) / 2;
    let mut buf: Vec<u16> = vec![0u16; char_count];
    let ok = unsafe {
        WinHttpQueryHeaders(
            req.as_ptr(),
            WINHTTP_QUERY_RAW_HEADERS_CRLF,
            ptr::null(),
            buf.as_mut_ptr() as *mut std::ffi::c_void,
            &mut buf_size,
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(format!(
            "WinHttpQueryHeaders (raw) failed (error {})",
            unsafe { windows_sys::Win32::Foundation::GetLastError() }
        ));
    }

    let raw = from_wide(&buf, buf_size as usize);
    let mut headers = HashMap::new();
    let mut set_cookies = Vec::new();

    // Parse "Header-Name: value\r\n" lines (skip the first status line)
    for line in raw.split("\r\n").skip(1) {
        if line.is_empty() {
            continue;
        }
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim().to_lowercase();
            let value = line[colon_pos + 1..].trim().to_string();

            if name == "set-cookie" {
                set_cookies.push(value);
            } else {
                headers.insert(name, value);
            }
        }
    }

    Ok((headers, set_cookies))
}

/// Read the entire response body as a UTF-8 string.
fn read_response_body(req: &WinHandle) -> Result<String, String> {
    let mut all_bytes = Vec::new();
    let mut buf = [0u8; 8192];

    loop {
        let mut bytes_read: u32 = 0;
        let ok = unsafe {
            WinHttpReadData(
                req.as_ptr(),
                buf.as_mut_ptr() as *mut std::ffi::c_void,
                buf.len() as u32,
                &mut bytes_read,
            )
        };
        if ok == 0 {
            return Err(format!(
                "WinHttpReadData failed (error {})",
                unsafe { windows_sys::Win32::Foundation::GetLastError() }
            ));
        }
        if bytes_read == 0 {
            break; // End of response
        }
        all_bytes.extend_from_slice(&buf[..bytes_read as usize]);
    }

    String::from_utf8(all_bytes)
        .map_err(|e| format!("Response body is not valid UTF-8: {}", e))
}
```

**Step 3: Verify it compiles**

Run: `cd src/desktop/src-tauri && cargo check`
Expected: compiles (on macOS, the module is skipped entirely via `#[cfg]`).

**Step 4: Commit**

```bash
git add src/desktop/src-tauri/src/winhttp_client.rs src/desktop/src-tauri/Cargo.toml
git commit -m "feat: implement WinHTTP execute_request with full request lifecycle (#10)"
```

---

### Task 5: Wire WinHTTP into the Tauri commands (platform dispatch)

**Files:**
- Modify: `src/desktop/src-tauri/src/lib.rs`

Replace the existing `HttpProxy`-based `http_request` and `http_reset` commands with platform-conditional dispatch: WinHTTP on Windows, reqwest on macOS/Linux.

**Step 1: Refactor lib.rs for platform dispatch**

The key changes to `lib.rs`:

1. Keep `HttpRequest` and `HttpResponse` structs (shared interface).
2. Keep `HttpProxy` (reqwest) but gate it with `#[cfg(not(target_os = "windows"))]`.
3. On Windows, manage `WinHttpSession` as the Tauri state instead.
4. Keep `ProxyAwareHttpSource` and Velopack commands unchanged (they always use reqwest).

Replace the entire `lib.rs` with:

```rust
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use velopack::*;

#[cfg(target_os = "windows")]
mod winhttp_client;

// --- Reqwest-based HTTP client (macOS/Linux only) ---

#[cfg(not(target_os = "windows"))]
mod reqwest_http {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Shared HTTP client with cookie persistence for scraping external sites.
    pub struct HttpProxy {
        pub client: RwLock<reqwest::Client>,
    }

    impl HttpProxy {
        pub fn new() -> Self {
            Self {
                client: RwLock::new(Self::build_client()),
            }
        }

        pub fn build_client() -> reqwest::Client {
            let jar = Arc::new(reqwest::cookie::Jar::default());
            reqwest::Client::builder()
                .cookie_provider(jar)
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .expect("Failed to create HTTP client")
        }
    }

    pub async fn execute_request(
        state: &tauri::State<'_, HttpProxy>,
        request: &super::HttpRequest,
    ) -> Result<super::HttpResponse, String> {
        let client = state.client.read().await.clone();

        let mut req = match request.method.to_uppercase().as_str() {
            "GET" => client.get(&request.url),
            "POST" => client.post(&request.url),
            _ => return Err(format!("Unsupported method: {}", request.method)),
        };

        for (k, v) in &request.headers {
            req = req.header(k.as_str(), v.as_str());
        }

        if let Some(ref form_data) = request.form_data {
            let mut form = reqwest::multipart::Form::new();
            for (k, v) in form_data {
                form = form.text(k.clone(), v.clone());
            }
            req = req.multipart(form);
        } else if let Some(ref body) = request.body {
            req = req.body(body.clone());
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;

        let status = resp.status().as_u16();
        let resp_url = resp.url().to_string();

        let mut headers = HashMap::new();
        let mut set_cookies = Vec::new();
        for (name, value) in resp.headers() {
            let v = value.to_str().unwrap_or("").to_string();
            if name.as_str() == "set-cookie" {
                set_cookies.push(v);
            } else {
                headers.insert(name.to_string(), v);
            }
        }

        let body = resp.text().await.map_err(|e| e.to_string())?;

        Ok(super::HttpResponse {
            status,
            headers,
            set_cookies,
            body,
            url: resp_url,
        })
    }
}

// --- Shared types (same IPC interface on all platforms) ---

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequest {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    /// URL-encoded or JSON body string
    pub body: Option<String>,
    /// Key-value pairs sent as multipart/form-data
    pub form_data: Option<HashMap<String, String>>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub set_cookies: Vec<String>,
    pub body: String,
    pub url: String,
}

// --- Tauri commands ---

#[cfg(not(target_os = "windows"))]
#[tauri::command]
async fn http_request(
    state: tauri::State<'_, reqwest_http::HttpProxy>,
    request: HttpRequest,
) -> Result<HttpResponse, String> {
    reqwest_http::execute_request(&state, &request).await
}

#[cfg(target_os = "windows")]
#[tauri::command]
async fn http_request(
    state: tauri::State<'_, winhttp_client::WinHttpSession>,
    request: HttpRequest,
) -> Result<HttpResponse, String> {
    // WinHTTP is synchronous — run on a blocking thread to avoid blocking the async runtime
    let session_ptr = state.inner().clone();
    let result = tokio::task::spawn_blocking(move || {
        winhttp_client::execute_request(&session_ptr, &request)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;
    result
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
async fn http_reset(state: tauri::State<'_, reqwest_http::HttpProxy>) -> Result<(), String> {
    let mut client = state.client.write().await;
    *client = reqwest_http::HttpProxy::build_client();
    Ok(())
}

#[cfg(target_os = "windows")]
#[tauri::command]
async fn http_reset(
    state: tauri::State<'_, winhttp_client::WinHttpSession>,
) -> Result<(), String> {
    state.reset()
}

// --- Velopack update commands (always use reqwest, all platforms) ---

/// Custom Velopack update source that uses reqwest's blocking client.
/// This respects the OS certificate store (rustls-tls-native-roots) and
/// system proxy settings (system-proxy), unlike Velopack's built-in
/// HttpSource which uses ureq + webpki-roots.
#[derive(Clone)]
struct ProxyAwareHttpSource {
    url: String,
}

impl ProxyAwareHttpSource {
    fn new(url: &str) -> Self {
        Self { url: url.to_owned() }
    }

    fn build_blocking_client() -> Result<reqwest::blocking::Client, Error> {
        reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| Error::Generic(e.to_string()))
    }
}

impl sources::UpdateSource for ProxyAwareHttpSource {
    fn get_release_feed(
        &self,
        channel: &str,
        app: &bundle::Manifest,
        staged_user_id: &str,
    ) -> Result<VelopackAssetFeed, Error> {
        let releases_name = format!("releases.{}.json", channel);
        let path = self.url.trim_end_matches('/').to_owned() + "/";
        let base = reqwest::Url::parse(&path)
            .map_err(|e| Error::Generic(e.to_string()))?;
        let mut releases_url = base.join(&releases_name)
            .map_err(|e| Error::Generic(e.to_string()))?;
        releases_url.set_query(Some(
            &format!("localVersion={}&id={}&stagingId={}", app.version, app.id, staged_user_id),
        ));

        let client = Self::build_blocking_client()?;
        let json = client
            .get(releases_url.as_str())
            .send()
            .map_err(|e| Error::Generic(format!("Failed to fetch release feed: {}", e)))?
            .text()
            .map_err(|e| Error::Generic(format!("Failed to read release feed: {}", e)))?;

        let feed: VelopackAssetFeed = serde_json::from_str(&json)
            .map_err(|e| Error::Generic(format!("Failed to parse release feed: {}", e)))?;
        Ok(feed)
    }

    fn download_release_entry(
        &self,
        asset: &VelopackAsset,
        local_file: &str,
        progress_sender: Option<Sender<i16>>,
    ) -> Result<(), Error> {
        let path = self.url.trim_end_matches('/').to_owned() + "/";
        let base = reqwest::Url::parse(&path)
            .map_err(|e| Error::Generic(e.to_string()))?;
        let asset_url = base.join(&asset.FileName)
            .map_err(|e| Error::Generic(e.to_string()))?;

        let client = Self::build_blocking_client()?;
        let resp = client
            .get(asset_url.as_str())
            .send()
            .map_err(|e| Error::Generic(format!("Failed to download update: {}", e)))?;

        let mut file = std::fs::File::create(local_file)
            .map_err(|e| Error::Generic(format!("Failed to create file: {}", e)))?;

        let bytes = resp.bytes()
            .map_err(|e| Error::Generic(format!("Failed to read update: {}", e)))?;
        file.write_all(&bytes)
            .map_err(|e| Error::Generic(format!("Failed to write file: {}", e)))?;

        if let Some(sender) = &progress_sender {
            let _ = sender.send(100);
        }

        Ok(())
    }

    fn clone_boxed(&self) -> Box<dyn sources::UpdateSource> {
        Box::new(self.clone())
    }
}

const UPDATE_URL: &str = "https://github.com/radaiko/SnackPilot/releases/latest/download";

#[tauri::command]
async fn check_for_updates() -> Result<Option<String>, String> {
    let source = ProxyAwareHttpSource::new(UPDATE_URL);
    let um = UpdateManager::new(source, None, None).map_err(|e| e.to_string())?;

    match um.check_for_updates().map_err(|e| e.to_string())? {
        UpdateCheck::UpdateAvailable(info) => {
            Ok(Some(info.TargetFullRelease.Version.clone()))
        }
        _ => Ok(None),
    }
}

#[tauri::command]
async fn download_update() -> Result<Option<String>, String> {
    let source = ProxyAwareHttpSource::new(UPDATE_URL);
    let um = UpdateManager::new(source, None, None).map_err(|e| e.to_string())?;

    match um.check_for_updates().map_err(|e| e.to_string())? {
        UpdateCheck::UpdateAvailable(info) => {
            um.download_updates(&info, None)
                .map_err(|e| e.to_string())?;
            Ok(Some(info.TargetFullRelease.Version.clone()))
        }
        _ => Ok(None),
    }
}

#[tauri::command]
async fn install_update() -> Result<(), String> {
    let source = ProxyAwareHttpSource::new(UPDATE_URL);
    let um = UpdateManager::new(source, None, None).map_err(|e| e.to_string())?;

    if let UpdateCheck::UpdateAvailable(info) =
        um.check_for_updates().map_err(|e| e.to_string())?
    {
        um.apply_updates_and_restart(&info.TargetFullRelease)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(not(target_os = "windows"))]
    let builder = builder.manage(reqwest_http::HttpProxy::new());

    #[cfg(target_os = "windows")]
    let builder = builder.manage(
        winhttp_client::WinHttpSession::new().expect("Failed to create WinHTTP session")
    );

    builder
        .invoke_handler(tauri::generate_handler![
            check_for_updates,
            download_update,
            install_update,
            http_request,
            http_reset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Important:** For the Windows `http_request` command, `WinHttpSession` must implement `Clone` because `tokio::task::spawn_blocking` requires the closure to be `'static`. Update `WinHttpSession` to use `Arc<Mutex<...>>` internally so `Clone` is cheap:

In `winhttp_client.rs`, change:

```rust
pub struct WinHttpSession {
    session: Mutex<WinHandle>,
}
```

to:

```rust
#[derive(Clone)]
pub struct WinHttpSession {
    session: Arc<Mutex<WinHandle>>,
}
```

And update `new()` accordingly:

```rust
pub fn new() -> Result<Self, String> {
    let session = Self::create_session()?;
    Ok(Self {
        session: Arc::new(Mutex::new(session)),
    })
}
```

Add `use std::sync::Arc;` to the imports in `winhttp_client.rs`.

**Step 2: Add `url` crate to windows-only dependencies**

In `Cargo.toml`, update the Windows dependencies section:

```toml
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.59", features = ["Win32_Networking_WinHttp"] }
url = "2"
```

**Step 3: Verify it compiles**

Run: `cd src/desktop/src-tauri && cargo check`
Expected: compiles on macOS (reqwest path). The Windows path compiles when building on Windows.

**Step 4: Commit**

```bash
git add src/desktop/src-tauri/src/lib.rs src/desktop/src-tauri/src/winhttp_client.rs src/desktop/src-tauri/Cargo.toml
git commit -m "feat: wire WinHTTP into Tauri commands with platform dispatch (#10)"
```

---

### Task 6: Run existing TypeScript tests to verify no regressions

**Files:** None modified — verification only.

**Step 1: Run the full TypeScript test suite**

Run: `cd src/app && npm test`
Expected: ALL PASS (178+ tests). These tests don't touch the Rust layer — they test parsers, stores, and utilities — so they must pass unchanged.

**Step 2: Verify macOS cargo build succeeds**

Run: `cd src/desktop/src-tauri && cargo build`
Expected: builds successfully using the reqwest path.

---

### Task 7: Manual verification on Windows

**This task requires a Windows machine.**

**Step 1: Build on Windows**

Run: `cd src/desktop && npm run build`
Expected: Tauri build completes, producing a `.msi` installer.

**Step 2: Test without proxy**

1. Launch the app
2. Log in to both Gourmet and Ventopay
3. Navigate menus, view orders, check billing
4. Verify all functionality works identically to the reqwest version

**Step 3: Test with PAC-file proxy + NTLM auth**

1. Configure Windows system proxy with a PAC file URL
2. Ensure no `HTTP_PROXY` / `HTTPS_PROXY` env vars are set
3. Launch the app
4. Log in — should work through the proxy with automatic NTLM auth
5. Navigate menus, view orders, check billing

**Step 4: Verify logout/reset**

1. Log in successfully
2. Log out (triggers `http_reset`)
3. Log in again — should work (session was properly reset)
