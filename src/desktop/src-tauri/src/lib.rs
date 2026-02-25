use std::collections::HashMap;
use std::io::Write;
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

const ALLOWED_HTTP_HOSTS: [&str; 2] = [
    "alaclickneu.gourmet.at",
    "my.ventopay.com",
];

const KEYRING_SERVICE: &str = "SnackPilot";

fn validate_http_request(request: &HttpRequest) -> Result<(), String> {
    let parsed = reqwest::Url::parse(&request.url)
        .map_err(|e| format!("Invalid request URL: {}", e))?;
    if parsed.scheme() != "https" {
        return Err("Only https URLs are allowed".to_string());
    }
    let host = parsed.host_str()
        .ok_or_else(|| "Request URL must include a host".to_string())?;
    if !ALLOWED_HTTP_HOSTS.iter().any(|allowed| allowed.eq_ignore_ascii_case(host)) {
        return Err(format!("Host not allowed: {}", host));
    }
    Ok(())
}

fn keyring_entry(key: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, key).map_err(|e| e.to_string())
}

// --- Tauri commands ---

#[cfg(not(target_os = "windows"))]
#[tauri::command]
async fn http_request(
    state: tauri::State<'_, reqwest_http::HttpProxy>,
    request: HttpRequest,
) -> Result<HttpResponse, String> {
    validate_http_request(&request)?;
    reqwest_http::execute_request(&state, &request).await
}

#[cfg(target_os = "windows")]
#[tauri::command]
async fn http_request(
    state: tauri::State<'_, winhttp_client::WinHttpSession>,
    request: HttpRequest,
) -> Result<HttpResponse, String> {
    validate_http_request(&request)?;
    // WinHTTP is synchronous — run on a blocking thread to avoid blocking the async runtime
    let session = state.inner().clone();
    let result = tokio::task::spawn_blocking(move || {
        winhttp_client::execute_request(&session, &request)
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

#[tauri::command]
async fn secure_set_item(key: String, value: String) -> Result<(), String> {
    let entry = keyring_entry(&key)?;
    entry.set_password(&value).map_err(|e| e.to_string())
}

#[tauri::command]
async fn secure_get_item(key: String) -> Result<Option<String>, String> {
    let entry = keyring_entry(&key)?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
async fn secure_delete_item(key: String) -> Result<(), String> {
    let entry = keyring_entry(&key)?;
    match entry.delete_credential() {
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
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
            secure_set_item,
            secure_get_item,
            secure_delete_item,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_request(url: &str) -> HttpRequest {
        HttpRequest {
            url: url.to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
            form_data: None,
        }
    }

    #[test]
    fn allows_known_https_hosts() {
        assert!(validate_http_request(&build_request("https://alaclickneu.gourmet.at/start/")).is_ok());
        assert!(validate_http_request(&build_request("https://my.ventopay.com/mocca.website/Login.aspx")).is_ok());
    }

    #[test]
    fn rejects_non_https_urls() {
        let result = validate_http_request(&build_request("http://alaclickneu.gourmet.at/start/"));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_hosts() {
        let result = validate_http_request(&build_request("https://example.com/path"));
        assert!(result.is_err());
    }
}
