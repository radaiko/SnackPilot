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
use std::sync::{Arc, Mutex};

use windows_sys::Win32::Networking::WinHttp::*;

use crate::{HttpRequest, HttpResponse};

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
    let char_len = (byte_len / 2).min(buf.len());
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
#[derive(Clone)]
pub struct WinHttpSession {
    session: Arc<Mutex<WinHandle>>,
}

impl WinHttpSession {
    /// Create a new WinHTTP session with automatic proxy detection.
    pub fn new() -> Result<Self, String> {
        let session = Self::create_session()?;
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
        })
    }

    fn create_session() -> Result<WinHandle, String> {
        let agent = to_wide(&format!("SnackPilot/{}", env!("CARGO_PKG_VERSION")));
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

/// Execute an HTTP request using WinHTTP.
///
/// This is called from the `http_request` Tauri command on Windows.
/// Returns the same `HttpResponse` struct as the reqwest path.
///
/// Note: All requests are serialized through the session Mutex, which means
/// only one request can be in-flight at a time. This matches the app's
/// sequential scraping pattern and ensures cookie consistency.
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
        // Sanitize CRLF to prevent header injection
        let safe_key = key.replace(['\r', '\n'], "");
        let safe_value = value.replace(['\r', '\n'], "");
        if safe_key.is_empty() {
            continue;
        }
        all_headers.push_str(&format!("{}: {}\r\n", safe_key, safe_value));
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
