//! Test double for [`Downloader`] (design D2), public so model tests can script
//! downloads without a network. Like a real downloader it streams in chunks and
//! fails with `SizeMismatch` as soon as the bytes exceed `max_bytes`.

use super::{Downloader, UpdateError};
use std::collections::VecDeque;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Mutex;

/// One recorded [`Downloader::download`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeDownloadCall {
    pub url: String,
    pub dest: PathBuf,
    pub max_bytes: u64,
}

/// Scripted downloader: one queued byte sequence per call, streamed in 4 KiB chunks.
#[derive(Default)]
pub struct FakeDownloader {
    bodies: Mutex<VecDeque<Vec<u8>>>,
    calls: Mutex<Vec<FakeDownloadCall>>,
}

impl FakeDownloader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues the bytes the next call writes.
    pub fn push(&mut self, body: &[u8]) {
        self.bodies.lock().unwrap().push_back(body.to_vec());
    }

    /// Every call so far, in order.
    pub fn calls(&self) -> Vec<FakeDownloadCall> {
        self.calls.lock().unwrap().clone()
    }
}

impl Downloader for FakeDownloader {
    fn download<'a>(
        &'a self,
        url: &'a str,
        dest: &'a Path,
        max_bytes: u64,
        progress: &'a (dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<(), UpdateError>> + Send + 'a>> {
        Box::pin(async move {
            self.calls.lock().unwrap().push(FakeDownloadCall {
                url: url.to_string(),
                dest: dest.to_path_buf(),
                max_bytes,
            });
            let body = self
                .bodies
                .lock()
                .unwrap()
                .pop_front()
                .ok_or(UpdateError::Network)?;
            let total = body.len() as u64;
            let mut file = Vec::new();
            for chunk in body.chunks(4096) {
                file.extend_from_slice(chunk);
                if file.len() as u64 > max_bytes {
                    return Err(UpdateError::SizeMismatch);
                }
                std::fs::write(dest, &file).map_err(|_| UpdateError::Io)?;
                progress(file.len() as u64, Some(total));
            }
            Ok(())
        })
    }
}
