# WinHTTP Native Proxy Support — Design Document

**Issue:** [#10 — Desktop bug: Proxy settings not automatically detected if no environment variables are set](https://github.com/radaiko/SnackPilot/issues/10)

**Problem:** On Windows with PAC-file proxy + NTLM authentication, the Rust `reqwest` backend cannot:
1. Execute PAC files to resolve the correct proxy per URL
2. Authenticate via NTLM/Kerberos with the proxy server

`reqwest`'s `system-proxy` feature only reads static proxy strings from the Windows registry (`HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings\ProxyServer`). It does not read `AutoConfigURL` (PAC) or `AutoDetect` (WPAD), and has no NTLM proxy authentication support (open issue since 2019: seanmonstar/reqwest#498).

**Solution:** On Windows, replace `reqwest` with native WinHTTP API calls for scraping HTTP requests. WinHTTP handles PAC resolution, WPAD auto-detection, and NTLM/Kerberos proxy auth natively through the OS.

---

## Architecture

### Current Flow (all platforms)

```text
Axios → tauriAdapter → Tauri IPC → reqwest client → External sites
```

### New Flow

```text
[Windows]
  Axios → tauriAdapter → Tauri IPC → WinHTTP native client → External sites
                                      (PAC + NTLM handled by OS)

[macOS/Linux]
  Axios → tauriAdapter → Tauri IPC → reqwest client → External sites
                                      (env var proxy, unchanged)
```

### What Changes

| Component | Change |
|-----------|--------|
| `src-tauri/src/winhttp_client.rs` | **New** — WinHTTP-based HTTP client module |
| `src-tauri/src/lib.rs` | **Modified** — platform-conditional dispatch in `http_request` and `http_reset` |
| `src-tauri/Cargo.toml` | **Modified** — add `windows-sys` dependency (Windows only) |

### What Doesn't Change

| Component | Reason |
|-----------|--------|
| `tauriHttp.web.ts` | Same Tauri IPC interface; adapter unchanged |
| `gourmetClient.ts` / `ventopayClient.ts` | No change to TypeScript HTTP clients |
| macOS/Linux behavior | Still uses reqwest, no change |
| Velopack update commands | Still use reqwest blocking client (GitHub doesn't require NTLM) |

---

## WinHTTP Client Design

### Session Management

A `WinHttpProxy` struct holds a WinHTTP session handle (`HINTERNET`) that persists cookies across requests, analogous to reqwest's `Client` with a cookie jar.

```rust
#[cfg(target_os = "windows")]
struct WinHttpProxy {
    session: RwLock<HINTERNET>,  // WinHTTP session handle
}
```

Created with `WinHttpOpen` using `WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY`, which enables:
- Automatic PAC file resolution
- WPAD auto-detection
- NTLM/Kerberos proxy authentication via the logged-in user's credentials
- Static proxy from registry as fallback

### Request Flow

For each `http_request` Tauri command:

1. `WinHttpConnect(session, host, port)` — establish connection to target
2. `WinHttpOpenRequest(connection, method, path)` — create request handle
3. `WinHttpAddRequestHeaders(request, headers)` — set custom headers
4. For multipart: manually construct `multipart/form-data` body with boundary
5. For URL-encoded/JSON: pass body string directly
6. `WinHttpSendRequest(request, body)` — send the request
7. `WinHttpReceiveResponse(request)` — wait for response
8. `WinHttpQueryHeaders(request)` — read status code + response headers
9. `WinHttpReadData(request)` — read response body
10. Close request and connection handles

### Cookie Management

WinHTTP manages cookies automatically per session handle:
- Cookies from `Set-Cookie` headers are stored internally
- Cookies are sent automatically on subsequent requests to matching domains
- On logout (`http_reset`): close session handle and create a new one

Response `Set-Cookie` headers are still extracted and returned to TypeScript for Ventopay's manual cookie management.

### Multipart Form Data

WinHTTP has no built-in multipart support. A helper function constructs the `multipart/form-data` body:

```text
--{boundary}\r\n
Content-Disposition: form-data; name="{key}"\r\n
\r\n
{value}\r\n
--{boundary}--\r\n
```

The `Content-Type: multipart/form-data; boundary={boundary}` header is set manually.

### Error Handling

WinHTTP errors map to string errors returned via the Tauri command, matching the existing reqwest error handling pattern.

---

## Dependencies

### New (Windows only)

```toml
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.59", features = [
    "Win32_Networking_WinHttp",
    "Win32_Foundation",
] }
```

### Unchanged

- `reqwest` — still used on macOS/Linux and for Velopack updates on all platforms
- All other dependencies unchanged

---

## Platform Compilation Strategy

```rust
#[cfg(target_os = "windows")]
mod winhttp_client;

// In http_request command:
#[cfg(target_os = "windows")]
{ winhttp_client::execute_request(&request).await }

#[cfg(not(target_os = "windows"))]
{ reqwest_execute_request(&state, &request).await }
```

The Tauri command interface (`HttpRequest` / `HttpResponse` structs) stays identical. Only the implementation differs per platform.

---

## Testing Strategy

- Unit tests for multipart body construction (cross-platform, no WinHTTP dependency)
- Existing TypeScript tests pass unchanged (they test the scraping logic, not the HTTP transport)
- Manual testing on Windows with PAC proxy to verify end-to-end
- macOS CI continues to work (compiles without `windows-sys`, uses reqwest path)

---

## Proxy Capability Matrix (After Change)

| Capability | Windows (WinHTTP) | macOS/Linux (reqwest) |
|------------|-------------------|----------------------|
| No proxy | Yes | Yes |
| Env vars (HTTP_PROXY) | Yes | Yes |
| Registry/system proxy | Yes (native) | Yes (system-proxy feature) |
| PAC file | Yes (native) | No |
| WPAD auto-detect | Yes (native) | No |
| NTLM proxy auth | Yes (native, silent) | No |
| Kerberos proxy auth | Yes (native, silent) | No |
