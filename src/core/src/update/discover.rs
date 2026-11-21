//! Release discovery over GitHub Releases (REQ-UPD-01..04, design D2).
//!
//! The request sequence is fixed and asserted: the release list, then the
//! manifest asset, then its signature. Every request is an anonymous GET.

use super::{parse_desktop_tag, verify_manifest, Manifest, UpdateError, Version};
use crate::http::{HttpResponse, Method, Request, Transport};
use serde::Deserialize;

/// The release list: one unauthenticated GET, never `releases/latest`.
pub const RELEASES_URL: &str =
    "https://api.github.com/repos/radaiko/SnackPilot/releases?per_page=100";

/// Every asset address must stay under this prefix (REQ-UPD-04).
pub const DOWNLOAD_PREFIX: &str = "https://github.com/radaiko/SnackPilot/releases/download/";

/// GitHub's documented JSON and raw media types; the only header sent.
const ACCEPT: &str = "application/vnd.github+json, application/json";

/// The running host: `platform` is `macos` or `windows`, `architecture` `arm64`
/// or `x64`, `os_version` the OS version when known (unknown never blocks).
pub struct HostTarget {
    pub platform: &'static str,
    pub architecture: &'static str,
    pub os_version: Option<String>,
}

/// Outcome of a check. The shape is design D2's; boxing `Candidate` would
/// change the public API the model is written against.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Discovery {
    UpToDate { latest: String },
    Available(Candidate),
}

/// A verified release: its signed manifest and the artifact address the release
/// list names (never the manifest's word for it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub manifest: Manifest,
    pub artifact_url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

/// Finds the newest applicable release and returns it, or `UpToDate`.
///
/// Failure order follows the signature rule: the signature over the exact
/// manifest bytes is checked before any field is read (REQ-UPD-03).
pub async fn discover(
    t: &dyn Transport,
    host: &HostTarget,
    installed: &str,
    key: &[u8; 32],
) -> Result<Discovery, UpdateError> {
    let installed = Version::parse(installed).ok_or(UpdateError::Unsupported)?;

    // 1. Release list; only `desktop/vX.Y.Z`, never draft or prerelease.
    let list = fetch(t, RELEASES_URL).await?;
    let releases: Vec<Release> = serde_json::from_str(&list).map_err(|_| UpdateError::NoRelease)?;
    let (tag, release) = releases
        .iter()
        .filter(|r| !r.draft && !r.prerelease)
        .filter_map(|r| Some((parse_desktop_tag(&r.tag_name)?, r)))
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.tag_name.cmp(&b.1.tag_name)))
        .ok_or(UpdateError::NoRelease)?;
    let artifact_name = format!(
        "SnackPilot-manifest-{}-{}.json",
        host.platform, host.architecture
    );
    let sig_name = format!("{artifact_name}.sig");

    // 2. and 3. Manifest asset, then its signature.
    let manifest_url = find_download_asset(&release.assets, &artifact_name)?;
    let sig_url = find_download_asset(&release.assets, &sig_name)?;
    let manifest_bytes = fetch(t, manifest_url).await?;
    let signature = fetch(t, sig_url).await?;

    let manifest = verify_manifest(manifest_bytes.as_bytes(), &signature, key)?;

    // Field checks, only on verified input.
    if !applies_to_host(&manifest, host, tag) {
        return Err(UpdateError::ManifestMismatch);
    }
    if !is_newer(&manifest.version, &installed) {
        return Ok(Discovery::UpToDate {
            latest: manifest.version,
        });
    }
    if let Some(host_os) = &host.os_version {
        if !os_reaches(&manifest.minimum_os, host_os) {
            return Err(UpdateError::Unsupported);
        }
    }
    let artifact_url = find_download_asset(&release.assets, &manifest.artifact)?.to_string();

    Ok(Discovery::Available(Candidate {
        manifest,
        artifact_url,
    }))
}

/// The asset's address, which must stay under [`DOWNLOAD_PREFIX`] and end with
/// the expected file name (REQ-UPD-04).
fn find_download_asset<'a>(assets: &'a [ReleaseAsset], name: &str) -> Result<&'a str, UpdateError> {
    assets
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.browser_download_url.as_str())
        .ok_or(UpdateError::NoManifest)
        .and_then(|url| {
            let suffix = format!("/{name}");
            if url.starts_with(DOWNLOAD_PREFIX)
                && url.ends_with(&suffix)
                && url.len() > DOWNLOAD_PREFIX.len() + suffix.len()
            {
                Ok(url)
            } else {
                Err(UpdateError::NoManifest)
            }
        })
}

/// Manifest fields agree with one another and with the host and release (REQ-UPD-02).
fn applies_to_host(manifest: &Manifest, host: &HostTarget, tag: Version) -> bool {
    manifest.platform == host.platform
        && manifest.architecture == host.architecture
        && parse_desktop_tag(&manifest.tag) == Some(tag)
        && Version::parse(&manifest.version) == Some(tag)
}

/// Strictly newer than what is installed, by numeric comparison.
fn is_newer(version: &str, installed: &Version) -> bool {
    Version::parse(version).is_some_and(|v| v > *installed)
}

/// True when `host` (a dotted numeric OS version) is at least `minimum`.
/// Anything unparseable does not block.
fn os_reaches(minimum: &str, host: &str) -> bool {
    let parse = |s: &str| -> Option<Vec<u64>> {
        let parts: Option<Vec<u64>> = s.split('.').map(|p| p.parse().ok()).collect();
        parts.filter(|p| !p.is_empty())
    };
    let (Some(min), Some(have)) = (parse(minimum), parse(host)) else {
        return true;
    };
    let len = min.len().max(have.len());
    let padded = |v: &[u64]| -> Vec<u64> {
        let mut out = v.to_vec();
        out.resize(len, 0);
        out
    };
    padded(&have) >= padded(&min)
}

/// One anonymous GET with `Accept` and nothing else.
async fn fetch(t: &dyn Transport, url: &str) -> Result<String, UpdateError> {
    let resp: HttpResponse = t
        .send(Request {
            method: Method::Get,
            url: url.to_string(),
            headers: vec![("Accept".to_string(), ACCEPT.to_string())],
            body: None,
        })
        .await
        .map_err(|_| UpdateError::Network)?;
    if resp.status != 200 {
        return Err(UpdateError::Network);
    }
    Ok(resp.body)
}
