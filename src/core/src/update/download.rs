//! Verified artifact download (REQ-UPD-04, design D2).
//!
//! [`download_verified`] is pure over the [`Downloader`] trait, so the model can
//! script it. [`ReqwestDownloader`] is the production implementation with its own
//! client, deliberately separate from the scraping [`crate::http`] transport.

use super::{Candidate, Sha256Hasher, UpdateError};
use std::fs::File;
use std::future::Future;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;

/// Streams `url` into `dest`, reporting `(bytes so far, total when known)` and
/// failing with `SizeMismatch` rather than writing more than `max_bytes`.
pub trait Downloader: Send + Sync {
    fn download<'a>(
        &'a self,
        url: &'a str,
        dest: &'a Path,
        max_bytes: u64,
        progress: &'a (dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<(), UpdateError>> + Send + 'a>>;
}

/// Downloads `c` into `staging`, verifies size and SHA-256, then renames the
/// `.part` file to the manifest artifact name.
///
/// `staging` must be an absent or empty directory. On any error the whole
/// `staging` directory is removed, so nothing installable is left behind.
pub async fn download_verified(
    d: &dyn Downloader,
    c: &Candidate,
    staging: &Path,
    progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Result<PathBuf, UpdateError> {
    match std::fs::read_dir(staging) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(UpdateError::Io);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(UpdateError::Io),
    }

    match download_into(d, c, staging, progress).await {
        Ok(path) => Ok(path),
        Err(err) => {
            cleanup(staging);
            Err(err)
        }
    }
}

async fn download_into(
    d: &dyn Downloader,
    c: &Candidate,
    staging: &Path,
    progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
) -> Result<PathBuf, UpdateError> {
    std::fs::create_dir_all(staging).map_err(|_| UpdateError::Io)?;
    let part = staging.join(format!("{}.part", c.manifest.artifact));
    let path = staging.join(&c.manifest.artifact);

    d.download(&c.artifact_url, &part, c.manifest.size, progress)
        .await?;

    let (size, hash) = file_size_and_hash(&part)?;
    if size != c.manifest.size {
        return Err(UpdateError::SizeMismatch);
    }
    if hash != c.manifest.sha256.trim().to_ascii_lowercase() {
        return Err(UpdateError::HashMismatch);
    }
    std::fs::rename(&part, &path).map_err(|_| UpdateError::Io)?;
    Ok(path)
}

fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

fn file_size_and_hash(path: &Path) -> Result<(u64, String), UpdateError> {
    let mut file = File::open(path).map_err(|_| UpdateError::Io)?;
    let mut hasher = Sha256Hasher::new();
    let mut size = 0u64;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|_| UpdateError::Io)?;
        if n == 0 {
            break;
        }
        size += n as u64;
        hasher.update(&buf[..n]);
    }
    Ok((size, hasher.finish_hex()))
}

/// Production [`Downloader`]: its own client (https only, redirect limit 5,
/// 15 s connect timeout, no total timeout, 30 s per chunk), streaming to disk.
///
/// `with_client` exists for tests only (a loopback server is plain http);
/// production must use `new()`, which enforces the https-only rule.
pub struct ReqwestDownloader {
    client: reqwest::Client,
}

/// A stalled connection fails here instead of hanging a download forever.
const CHUNK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

impl ReqwestDownloader {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::limited(5))
            .connect_timeout(std::time::Duration::from_secs(15))
            .user_agent("SnackPilot-desktop-update")
            .build()
            .expect("static TLS and redirect configuration cannot fail");
        Self { client }
    }

    /// Test seam: an already configured client, bypassing `new`'s https-only
    /// rule so a loopback `TcpListener` can be measured. Never use in production.
    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }
}

impl Default for ReqwestDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Downloader for ReqwestDownloader {
    fn download<'a>(
        &'a self,
        url: &'a str,
        dest: &'a Path,
        max_bytes: u64,
        progress: &'a (dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<(), UpdateError>> + Send + 'a>> {
        Box::pin(async move {
            let mut resp = self.client.get(url).send().await.map_err(|e| {
                // `https_only` refuses a non-https address inside the client.
                if e.is_builder() {
                    UpdateError::Unsupported
                } else {
                    UpdateError::Network
                }
            })?;
            if !resp.status().is_success() {
                return Err(UpdateError::Network);
            }
            let total = resp.content_length();

            let mut file = File::create(dest).map_err(|_| UpdateError::Io)?;
            let mut done = 0u64;
            loop {
                let next = tokio::time::timeout(CHUNK_TIMEOUT, resp.chunk()).await;
                let chunk = match next {
                    Ok(Ok(Some(chunk))) => chunk,
                    Ok(Ok(None)) => break,
                    Ok(Err(_)) | Err(_) => return Err(UpdateError::Network),
                };
                done += chunk.len() as u64;
                if done > max_bytes {
                    return Err(UpdateError::SizeMismatch);
                }
                file.write_all(&chunk).map_err(|_| UpdateError::Io)?;
                progress(done, total);
            }
            file.flush().map_err(|_| UpdateError::Io)?;
            Ok(())
        })
    }
}
