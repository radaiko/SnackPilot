//! Signed update manifest (schema 1, design D4) and its Ed25519 verification.
//!
//! The signature covers the exact manifest bytes and is checked before JSON is
//! parsed (REQ-UPD-03): no field is ever read from unverified input.

use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

use super::UpdateError;

/// The only manifest schema this app understands.
pub const SCHEMA: u32 = 1;

/// A manifest as written by `tools/devops/make-update-manifest.sh`.
/// Unknown extra fields are ignored; every listed field is required.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub platform: String,
    pub architecture: String,
    pub version: String,
    pub build: String,
    pub minimum_os: String,
    pub artifact: String,
    pub size: u64,
    pub sha256: String,
    pub notes: String,
    pub notes_url: String,
    pub tag: String,
}

/// Verifies `signature_hex` over `bytes`, then parses the manifest.
///
/// `signature_hex` is ASCII hex of exactly 64 bytes with optional trailing
/// whitespace; anything else, including an invalid signature, is
/// `SignatureInvalid` (REQ-UPD-03). A schema other than 1 is `Unsupported`.
pub fn verify_manifest(
    bytes: &[u8],
    signature_hex: &str,
    key: &[u8; 32],
) -> Result<Manifest, UpdateError> {
    let signature = parse_signature(signature_hex)?;
    let verifying_key = VerifyingKey::from_bytes(key).map_err(|_| UpdateError::SignatureInvalid)?;
    verifying_key
        .verify_strict(bytes, &signature)
        .map_err(|_| UpdateError::SignatureInvalid)?;

    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|_| UpdateError::Unsupported)?;
    if manifest.schema != SCHEMA {
        return Err(UpdateError::Unsupported);
    }
    Ok(manifest)
}

/// Decodes 128 hex chars into a 64-byte signature; trailing whitespace is allowed.
fn parse_signature(s: &str) -> Result<Signature, UpdateError> {
    let trimmed = s.trim_end_matches(|c: char| c.is_ascii_whitespace());
    let hex = trimmed.as_bytes();
    if hex.len() != 128 {
        return Err(UpdateError::SignatureInvalid);
    }
    let mut raw = [0u8; 64];
    for (i, pair) in hex.chunks_exact(2).enumerate() {
        let hi = (pair[0] as char)
            .to_digit(16)
            .ok_or(UpdateError::SignatureInvalid)?;
        let lo = (pair[1] as char)
            .to_digit(16)
            .ok_or(UpdateError::SignatureInvalid)?;
        raw[i] = (hi * 16 + lo) as u8;
    }
    Ok(Signature::from_bytes(&raw))
}
