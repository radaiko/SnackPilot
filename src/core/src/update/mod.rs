//! Desktop update: release discovery, manifest verification and artifact
//! integrity (design D2). Not exported over UniFFI; mobile code never calls it.
//!
//! Every failure is an [`UpdateError`]; [`UpdateError::user_message`] is the
//! only text a person sees, and it carries no URL, hash or key.

pub mod key;
pub mod manifest;
pub mod version;

pub use key::UPDATE_PUBLIC_KEY;
pub use manifest::{verify_manifest, Manifest, SCHEMA};
pub use version::{parse_desktop_tag, Version};

use sha2::{Digest, Sha256};
use thiserror::Error;

/// Update failures. Own enum, so `CoreError` and its UniFFI shape stay untouched.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UpdateError {
    #[error("network")]
    Network,
    #[error("no release")]
    NoRelease,
    #[error("no manifest")]
    NoManifest,
    #[error("invalid signature")]
    SignatureInvalid,
    #[error("manifest mismatch")]
    ManifestMismatch,
    #[error("unsupported")]
    Unsupported,
    #[error("size mismatch")]
    SizeMismatch,
    #[error("hash mismatch")]
    HashMismatch,
    #[error("io")]
    Io,
}

impl UpdateError {
    /// One plain German sentence for the Updates detail. Never a URL, hash or key.
    pub fn user_message(&self) -> &'static str {
        match self {
            UpdateError::Network => "Keine Verbindung zu GitHub.",
            UpdateError::NoRelease => "Es wurde keine passende Version gefunden.",
            UpdateError::NoManifest => "Diese Version passt nicht zu diesem Gerät.",
            UpdateError::SignatureInvalid => "Das Update ist nicht echt signiert.",
            UpdateError::ManifestMismatch => "Das Update passt nicht zur gefundenen Version.",
            UpdateError::Unsupported => "Diese Update-Art wird nicht unterstützt.",
            UpdateError::SizeMismatch => "Das Update hat eine unerwartete Größe.",
            UpdateError::HashMismatch => "Das Update wurde unterwegs verändert.",
            UpdateError::Io => "Die Datei konnte nicht geschrieben werden.",
        }
    }
}

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256Hasher::new();
    hasher.update(bytes);
    hasher.finish_hex()
}

/// Streaming SHA-256, for artifacts too large to hold in memory (REQ-UPD-04).
pub struct Sha256Hasher(Sha256);

impl Sha256Hasher {
    pub fn new() -> Self {
        Self(Sha256::new())
    }

    pub fn update(&mut self, chunk: &[u8]) {
        self.0.update(chunk);
    }

    /// Lowercase hex of everything fed so far.
    pub fn finish_hex(self) -> String {
        self.0
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

impl Default for Sha256Hasher {
    fn default() -> Self {
        Self::new()
    }
}
