//! Signed manifests, version ordering and artifact hashing of `snackpilot_core::update`.
//!
//! Tests sign with a fixed seed key held only in this file; the production public
//! key never has a private half here.

use ed25519_dalek::{Signer, SigningKey};
use snackpilot_core::update::{
    parse_desktop_tag, sha256_hex, verify_manifest, Manifest, Sha256Hasher, UpdateError, Version,
    SCHEMA, UPDATE_PUBLIC_KEY,
};

const WINDOWS_MANIFEST: &str = include_str!("fixtures/update/manifest-windows-x64.json");
const MACOS_MANIFEST: &str = include_str!("fixtures/update/manifest-macos-arm64.json");
/// One real run of `tools/devops/make-update-manifest.sh` with a throwaway key.
const OPENSSL_MANIFEST: &str = include_str!("fixtures/update/openssl-manifest.json");
const OPENSSL_SIGNATURE: &str = include_str!("fixtures/update/openssl-manifest.json.sig");
const OPENSSL_PUBLIC_KEY: &str = include_str!("fixtures/update/openssl-public-key.hex");

fn test_key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

fn test_public_key() -> [u8; 32] {
    test_key().verifying_key().to_bytes()
}

fn sign_hex(bytes: &[u8]) -> String {
    let sig = test_key().sign(bytes);
    sig.to_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn verify_windows() -> Result<Manifest, UpdateError> {
    verify_manifest(
        WINDOWS_MANIFEST.as_bytes(),
        &sign_hex(WINDOWS_MANIFEST.as_bytes()),
        &test_public_key(),
    )
}

// @req REQ-UPD-03
#[test]
fn verify_accepts_signed_manifest() {
    let manifest = verify_windows().expect("signed manifest must verify");

    assert_eq!(manifest.schema, SCHEMA);
    assert_eq!(manifest.platform, "windows");
    assert_eq!(manifest.architecture, "x64");
    assert_eq!(manifest.version, "2.3.0");
    assert_eq!(manifest.build, "42");
    assert_eq!(manifest.minimum_os, "10.0.22000");
    assert_eq!(manifest.artifact, "SnackPilot-2.3.0-windows-x64-setup.exe");
    assert_eq!(manifest.size, 12345678);
    assert_eq!(manifest.tag, "desktop/v2.3.0");

    let macos = verify_manifest(
        MACOS_MANIFEST.as_bytes(),
        &sign_hex(MACOS_MANIFEST.as_bytes()),
        &test_public_key(),
    )
    .expect("signed macOS manifest must verify");
    assert_eq!(macos.platform, "macos");
    assert_eq!(macos.architecture, "arm64");
    assert_eq!(macos.minimum_os, "26.0");
}

// @req REQ-UPD-03 @req REQ-UPD-18
#[test]
fn openssl_signed_fixture_verifies_with_the_rust_verifier() {
    let hex = OPENSSL_PUBLIC_KEY.trim();
    assert_eq!(hex.len(), 64, "public key is 64 hex chars");
    let mut key = [0u8; 32];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("hex public key");
    }

    let manifest = verify_manifest(OPENSSL_MANIFEST.as_bytes(), OPENSSL_SIGNATURE, &key)
        .expect("the script's signature must verify with its public key");
    assert_eq!(manifest.schema, SCHEMA);
    assert_eq!(manifest.platform, "windows");
    assert_eq!(manifest.architecture, "x64");
    assert_eq!(manifest.version, "2.3.0");
    assert_eq!(manifest.tag, "desktop/v2.3.0");
    assert_eq!(manifest.minimum_os, "10.0.22000");
    assert_eq!(manifest.artifact, "SnackPilot-2.3.0-windows-x64-setup.exe");
    assert_eq!(
        manifest.notes_url,
        "https://github.com/radaiko/SnackPilot/releases/tag/desktop/v2.3.0"
    );

    // The script's file is ASCII, one compact line plus a newline.
    assert!(OPENSSL_MANIFEST.is_ascii());
    assert!(OPENSSL_MANIFEST.ends_with("}\n"));
    assert_eq!(OPENSSL_MANIFEST.lines().count(), 1);
    let sig = OPENSSL_SIGNATURE.trim();
    assert_eq!(sig.len(), 128);
    assert_eq!(sig, sig.to_lowercase());

    // One byte off either part is rejected exactly as in production.
    let mut flipped = OPENSSL_MANIFEST.as_bytes().to_vec();
    flipped[10] ^= 0x01;
    assert_eq!(
        verify_manifest(&flipped, OPENSSL_SIGNATURE, &key),
        Err(UpdateError::SignatureInvalid)
    );
    assert_eq!(
        verify_manifest(
            OPENSSL_MANIFEST.as_bytes(),
            OPENSSL_SIGNATURE,
            &test_public_key()
        ),
        Err(UpdateError::SignatureInvalid)
    );
}

// @req REQ-UPD-03
#[test]
fn verify_rejects_flipped_manifest_byte() {
    let mut bytes = WINDOWS_MANIFEST.as_bytes().to_vec();
    let sig = sign_hex(&bytes);
    bytes[0] ^= 0x01;
    assert_eq!(
        verify_manifest(&bytes, &sig, &test_public_key()),
        Err(UpdateError::SignatureInvalid)
    );
}

// @req REQ-UPD-03
#[test]
fn verify_rejects_flipped_signature_byte() {
    let bytes = WINDOWS_MANIFEST.as_bytes();
    let mut sig = sign_hex(bytes).into_bytes();
    let last = sig.len() - 1;
    sig[last] = if sig[last] == b'0' { b'1' } else { b'0' };
    let sig = String::from_utf8(sig).unwrap();
    assert_eq!(
        verify_manifest(bytes, &sig, &test_public_key()),
        Err(UpdateError::SignatureInvalid)
    );
}

// @req REQ-UPD-03
#[test]
fn verify_rejects_other_key() {
    let bytes = WINDOWS_MANIFEST.as_bytes();
    let other = SigningKey::from_bytes(&[9u8; 32])
        .verifying_key()
        .to_bytes();
    assert_eq!(
        verify_manifest(bytes, &sign_hex(bytes), &other),
        Err(UpdateError::SignatureInvalid)
    );
    assert_eq!(
        verify_manifest(bytes, &sign_hex(bytes), &UPDATE_PUBLIC_KEY),
        Err(UpdateError::SignatureInvalid)
    );
}

// @req REQ-UPD-03
#[test]
fn verify_rejects_malformed_signature_hex() {
    let bytes = WINDOWS_MANIFEST.as_bytes();
    let good = sign_hex(bytes);
    let cases = [
        String::new(),
        "0".repeat(126),
        "0".repeat(130),
        format!("{good}00"),
        format!(" {good}"), // leading whitespace is not allowed
        good[..126].to_string() + "zz",
        good.to_uppercase().replacen('A', "G", 1),
        "---".to_string(),
    ];
    for sig in cases {
        assert_eq!(
            verify_manifest(bytes, &sig, &test_public_key()),
            Err(UpdateError::SignatureInvalid),
            "{sig:?} must be rejected"
        );
    }
    // Optional trailing whitespace is accepted.
    for ws in ["\n", " ", "\r\n", "\n\n"] {
        assert!(
            verify_manifest(bytes, &format!("{good}{ws}"), &test_public_key()).is_ok(),
            "{ws:?} trailing whitespace must be accepted"
        );
    }
}

// @req REQ-UPD-03
#[test]
fn signature_is_checked_before_json_is_parsed() {
    // Not JSON at all, correctly signed: verification passes, parsing alone fails.
    let not_json = b"this is not a manifest";
    assert!(verify_manifest(not_json, &sign_hex(not_json), &test_public_key()).is_err());

    // Not JSON and not signed: the signature is what fails.
    assert_eq!(
        verify_manifest(not_json, &sign_hex(b"something else"), &test_public_key()),
        Err(UpdateError::SignatureInvalid)
    );

    // A manifest missing every field, signed: the signature is checked first, so
    // the rejection comes from parsing afterwards, never from a field check.
    let empty = b"{}";
    assert!(verify_manifest(empty, &sign_hex(empty), &test_public_key()).is_err());
}

// @req REQ-UPD-03
#[test]
fn schema_other_than_one_is_unsupported() {
    let bytes = WINDOWS_MANIFEST.replace("\"schema\":1", "\"schema\":2");
    assert!(bytes.contains("\"schema\":2"));
    assert_eq!(
        verify_manifest(
            bytes.as_bytes(),
            &sign_hex(bytes.as_bytes()),
            &test_public_key()
        ),
        Err(UpdateError::Unsupported)
    );
}

// @req REQ-UPD-01
#[test]
fn version_ordering_is_numeric() {
    let v = |s: &str| Version::parse(s).unwrap_or_else(|| panic!("{s} must parse"));
    assert!(v("2.10.0") > v("2.9.0"));
    assert!(v("2.9.1") > v("2.9.0"));
    assert!(v("10.0.0") > v("9.99.99"));
    assert_eq!(v("2.3.0"), v("2.3.0"));
    assert_eq!(v("2.3.0").to_string(), "2.3.0");

    assert_eq!(Version::parse("v2.3.0"), None);
    assert_eq!(Version::parse("2.3"), None);
    assert_eq!(Version::parse("2.3.0-rc1"), None);
    assert_eq!(Version::parse("02.3.0"), None);
}

// @req REQ-UPD-01
#[test]
fn desktop_tag_shape_is_exact() {
    assert_eq!(parse_desktop_tag("desktop/v1.2.3"), Version::parse("1.2.3"));
    for bad in [
        "desktop/v1.2.3-rc.1",
        "desktop/v1.2",
        "desktop/v1.2.3.4",
        "desktop/1.2.3",
        "ios/v1.2.3",
        "v1.2.3",
        "desktop/v",
        "desktop/v01.2.3",
    ] {
        assert_eq!(
            parse_desktop_tag(bad),
            None,
            "{bad:?} must not be a desktop tag"
        );
    }
}

// @req REQ-UPD-04
#[test]
fn sha256_known_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    // Streaming in chunks equals hashing at once.
    let mut hasher = Sha256Hasher::new();
    hasher.update(b"a");
    hasher.update(b"b");
    hasher.update(b"c");
    assert_eq!(hasher.finish_hex(), sha256_hex(b"abc"));
    assert_eq!(Sha256Hasher::default().finish_hex(), sha256_hex(b""));
}

// @req REQ-UPD-03
#[test]
fn production_key_is_not_placeholder_or_test_key() {
    assert_eq!(
        UPDATE_PUBLIC_KEY,
        [
            0x5b, 0x03, 0x0e, 0x29, 0x46, 0xe5, 0xda, 0x06, 0xd1, 0x79, 0xfa, 0x78, 0x65, 0x04,
            0xe2, 0x1f, 0x43, 0x53, 0xb9, 0x3b, 0x47, 0x18, 0x7b, 0x3e, 0xc7, 0xdf, 0x25, 0x79,
            0xfd, 0xfe, 0xdb, 0x44,
        ]
    );
    assert_ne!(UPDATE_PUBLIC_KEY, [0u8; 32]);
    assert_ne!(UPDATE_PUBLIC_KEY, test_public_key());
}

// @req REQ-UPD-03
#[test]
fn user_messages_are_plain_german_without_url_hash_or_key() {
    let all = [
        UpdateError::Network,
        UpdateError::NoRelease,
        UpdateError::NoManifest,
        UpdateError::SignatureInvalid,
        UpdateError::ManifestMismatch,
        UpdateError::Unsupported,
        UpdateError::SizeMismatch,
        UpdateError::HashMismatch,
        UpdateError::Io,
    ];
    for err in all {
        let msg = err.user_message();
        assert!(!msg.trim().is_empty(), "{err:?} needs a message");
        assert!(!msg.contains("http"), "{err:?}: {msg}");
        assert!(!msg.contains('/'), "{err:?}: {msg}");
        assert!(
            msg.chars().count() > 10 && msg.chars().count() < 100,
            "{err:?}: {msg}"
        );
    }
}

// ─── Release discovery, verified download and fakes (task 1.2) ─────────────────

mod discovery {
    use ed25519_dalek::{Signer, SigningKey};
    use snackpilot_core::http::{CapturingTransport, HttpResponse, Method};
    use snackpilot_core::update::{
        discover, download_verified, sha256_hex, Candidate, Discovery, Downloader, FakeDownloader,
        HostTarget, Manifest, ReqwestDownloader, UpdateError, DOWNLOAD_PREFIX, RELEASES_URL,
    };
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    const RELEASES_LIST: &str = include_str!("fixtures/update/releases-list.json");
    const RELEASES_NO_MANIFEST: &str = include_str!("fixtures/update/releases-no-manifest.json");
    const WINDOWS_MANIFEST: &str = include_str!("fixtures/update/manifest-windows-x64.json");

    const MANIFEST_URL: &str = "https://github.com/radaiko/SnackPilot/releases/download/desktop/v2.3.0/SnackPilot-manifest-windows-x64.json";
    const SIGNATURE_URL: &str = "https://github.com/radaiko/SnackPilot/releases/download/desktop/v2.3.0/SnackPilot-manifest-windows-x64.json.sig";
    const ARTIFACT_URL: &str = "https://github.com/radaiko/SnackPilot/releases/download/desktop/v2.3.0/SnackPilot-2.3.0-windows-x64-setup.exe";

    fn test_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn test_public_key() -> [u8; 32] {
        test_key().verifying_key().to_bytes()
    }

    fn sign_hex(bytes: &[u8]) -> String {
        test_key()
            .sign(bytes)
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    fn windows_host() -> HostTarget {
        HostTarget {
            platform: "windows",
            architecture: "x64",
            os_version: Some("10.0.22631".into()),
        }
    }

    fn ok(body: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: vec![],
            body: body.to_string(),
        }
    }

    /// A release list entry with one asset, for the mismatch cases.
    fn release(tag: &str, assets: &[(&str, Option<&str>)]) -> String {
        let assets: Vec<serde_json::Value> = assets
            .iter()
            .map(|(name, url)| {
                serde_json::json!({
                    "name": name,
                    "browser_download_url": url.map(|u| u.to_string()).unwrap_or_else(|| {
                        format!("{DOWNLOAD_PREFIX}{tag}/{name}")
                    }),
                })
            })
            .collect();
        serde_json::json!({
            "tag_name": tag,
            "draft": false,
            "prerelease": false,
            "assets": assets,
        })
        .to_string()
    }

    fn manifest_json(mutate: impl Fn(&mut serde_json::Value)) -> String {
        let mut v: serde_json::Value = serde_json::from_str(WINDOWS_MANIFEST).unwrap();
        mutate(&mut v);
        serde_json::to_string(&v).unwrap()
    }

    fn queue_manifest(t: &CapturingTransport, manifest: &str, signature: &str) {
        t.queue_response(ok(RELEASES_LIST));
        t.queue_response(ok(manifest));
        t.queue_response(ok(signature));
    }

    // @req REQ-UPD-01
    #[tokio::test]
    async fn discover_picks_highest_desktop_tag() {
        let t = CapturingTransport::new();
        let manifest = manifest_json(|_| {});
        queue_manifest(&t, &manifest, &sign_hex(manifest.as_bytes()));

        let found = discover(&t, &windows_host(), "2.2.0", &test_public_key())
            .await
            .expect("a signed release must be offered");
        let Discovery::Available(candidate) = found else {
            panic!("expected an available update, got {found:?}");
        };
        assert_eq!(candidate.manifest.tag, "desktop/v2.3.0");
        assert_eq!(candidate.manifest.version, "2.3.0");
        assert_eq!(candidate.artifact_url, ARTIFACT_URL);

        let urls: Vec<String> = t.requests().iter().map(|r| r.url.clone()).collect();
        assert_eq!(urls, vec![RELEASES_URL, MANIFEST_URL, SIGNATURE_URL]);
    }

    // @req REQ-UPD-01
    #[tokio::test]
    async fn discover_ignores_draft_prerelease_and_foreign_tags() {
        // releases-list.json holds the draft v2.2.0, the prereleases v2.1.1 and v1.4.2,
        // ios/android/`v`/badge tags and the foreign `macos/` and `desktop/0.0.9` shapes.
        // Only desktop/v2.3.0 is a published `desktop/vX.Y.Z` release.
        let t = CapturingTransport::new();
        let manifest = manifest_json(|_| {});
        queue_manifest(&t, &manifest, &sign_hex(manifest.as_bytes()));

        let Discovery::Available(candidate) =
            discover(&t, &windows_host(), "2.1.0", &test_public_key())
                .await
                .expect("real candidate")
        else {
            panic!("expected an available update");
        };
        assert_eq!(candidate.manifest.tag, "desktop/v2.3.0");

        // Without v2.3.0 the highest published tag is desktop/v2.1.0, not the
        // draft v2.2.0 or the prereleases v2.1.1 / v1.4.2.
        let without: Vec<serde_json::Value> =
            serde_json::from_str::<Vec<serde_json::Value>>(RELEASES_LIST)
                .unwrap()
                .into_iter()
                .filter(|r| r["tag_name"] != "desktop/v2.3.0")
                .collect();
        let t = CapturingTransport::new();
        t.queue_response(ok(&serde_json::to_string(&without).unwrap()));
        assert_eq!(
            discover(&t, &windows_host(), "2.0.0", &test_public_key()).await,
            Err(UpdateError::NoManifest),
            "desktop/v2.1.0 carries no manifest"
        );
        assert_eq!(t.requests().len(), 1);

        // Numeric ordering, not lexical: v2.10.0 beats v2.9.0, prerelease v2.11.0 is skipped.
        let mut eleven: serde_json::Value =
            serde_json::from_str(&release("desktop/v2.11.0", &[])).unwrap();
        eleven["prerelease"] = true.into();
        let list = format!(
            "[{},{},{}]",
            release("desktop/v2.9.0", &[]),
            release("desktop/v2.10.0", &[]),
            eleven
        );
        let t = CapturingTransport::new();
        t.queue_response(ok(&list));
        assert_eq!(
            discover(&t, &windows_host(), "2.0.0", &test_public_key()).await,
            Err(UpdateError::NoManifest)
        );
        assert_eq!(
            t.requests().len(),
            1,
            "a missing manifest asset is decided from the list alone"
        );
    }

    // @req REQ-UPD-01
    #[tokio::test]
    async fn discover_request_sequence_is_exact() {
        let t = CapturingTransport::new();
        let manifest = manifest_json(|_| {});
        queue_manifest(&t, &manifest, &sign_hex(manifest.as_bytes()));
        discover(&t, &windows_host(), "2.2.0", &test_public_key())
            .await
            .unwrap();

        let requests = t.requests();
        assert_eq!(requests.len(), 3, "exactly three requests");
        for (i, req) in requests.iter().enumerate() {
            assert_eq!(req.method, Method::Get, "request {i}");
            assert_eq!(req.body, None, "request {i} carries no body");
            assert_eq!(
                req.headers,
                vec![(
                    "Accept".to_string(),
                    "application/vnd.github+json, application/json".to_string()
                )],
                "request {i} carries only GitHub's documented Accept header"
            );
            assert!(
                !req.url.to_lowercase().contains("gourmet")
                    && !req.url.to_lowercase().contains("ventopay"),
                "request {i} must not target a scraping host: {}",
                req.url
            );
        }
        assert_eq!(requests[0].url, RELEASES_URL);
        assert_eq!(
            requests[0].url,
            "https://api.github.com/repos/radaiko/SnackPilot/releases?per_page=100"
        );
        assert_eq!(requests[1].url, MANIFEST_URL);
        assert_eq!(requests[2].url, SIGNATURE_URL);
    }

    // @req REQ-UPD-02
    #[tokio::test]
    async fn discover_up_to_date_when_not_newer() {
        let t = CapturingTransport::new();
        let manifest = manifest_json(|_| {});
        queue_manifest(&t, &manifest, &sign_hex(manifest.as_bytes()));
        assert_eq!(
            discover(&t, &windows_host(), "2.3.0", &test_public_key()).await,
            Ok(Discovery::UpToDate {
                latest: "2.3.0".into()
            })
        );
        assert_eq!(t.requests().len(), 3);

        let t = CapturingTransport::new();
        queue_manifest(&t, &manifest, &sign_hex(manifest.as_bytes()));
        assert_eq!(
            discover(&t, &windows_host(), "2.4.0", &test_public_key()).await,
            Ok(Discovery::UpToDate {
                latest: "2.3.0".into()
            })
        );
    }

    // @req REQ-UPD-02
    #[tokio::test]
    async fn discover_rejects_manifest_for_other_platform_or_tag() {
        for (name, patch) in [
            ("platform", "{\"platform\":\"macos\"}"),
            ("architecture", "{\"architecture\":\"arm64\"}"),
            ("tag", "{\"tag\":\"desktop/v2.4.0\"}"),
            ("version", "{\"version\":\"2.4.0\"}"),
        ] {
            let t = CapturingTransport::new();
            let manifest = manifest_json(|v| {
                let patch: serde_json::Value = serde_json::from_str(patch).unwrap();
                for (k, val) in patch.as_object().unwrap() {
                    v[k] = val.clone();
                }
            });
            queue_manifest(&t, &manifest, &sign_hex(manifest.as_bytes()));
            assert_eq!(
                discover(&t, &windows_host(), "2.2.0", &test_public_key()).await,
                Err(UpdateError::ManifestMismatch),
                "manifest with another {name} must be rejected"
            );
        }
    }

    // @req REQ-UPD-04
    #[tokio::test]
    async fn discover_rejects_asset_url_outside_download_prefix() {
        // The release lists the artifact address; one that leaves the prefix or names
        // another file is rejected before any download.
        for bad in [
            ARTIFACT_URL.replace(DOWNLOAD_PREFIX, "https://evil.example/"),
            ARTIFACT_URL.replace("SnackPilot-2.3.0-windows-x64-setup.exe", "Other.exe"),
            ARTIFACT_URL.replace("https://github.com/", "https://github.com.evil.example/"),
            "http://github.com/radaiko/SnackPilot/releases/download/desktop/v2.3.0/SnackPilot-2.3.0-windows-x64-setup.exe".to_string(),
        ] {
            let t = CapturingTransport::new();
            let manifest = manifest_json(|_| {});
            let list = release(
                "desktop/v2.3.0",
                &[
                    ("SnackPilot-manifest-windows-x64.json", Some(MANIFEST_URL)),
                    ("SnackPilot-manifest-windows-x64.json.sig", Some(SIGNATURE_URL)),
                    ("SnackPilot-2.3.0-windows-x64-setup.exe", Some(&bad)),
                ],
            );
            t.queue_response(ok(&list));
            t.queue_response(ok(&manifest));
            t.queue_response(ok(&sign_hex(manifest.as_bytes())));
            assert!(
                discover(&t, &windows_host(), "2.2.0", &test_public_key()).await.is_err(),
                "{bad} must be rejected"
            );
        }
    }

    // @req REQ-UPD-02
    #[tokio::test]
    async fn discover_without_manifest_asset_is_no_manifest() {
        // releases-no-manifest.json: the newest published desktop release (v2.4.1)
        // ships only platform artifacts, so the check stops after the list.
        let t = CapturingTransport::new();
        t.queue_response(ok(RELEASES_NO_MANIFEST));
        assert_eq!(
            discover(&t, &windows_host(), "2.0.0", &test_public_key()).await,
            Err(UpdateError::NoManifest)
        );
        assert_eq!(
            t.requests().len(),
            1,
            "nothing is fetched for a missing manifest"
        );

        // A release that carries its manifest assets: exactly three requests.
        let t = CapturingTransport::new();
        t.queue_response(ok(RELEASES_LIST));
        t.queue_response(ok("manifest bytes"));
        t.queue_response(ok("signature bytes"));
        assert_eq!(
            discover(&t, &windows_host(), "2.0.0", &test_public_key()).await,
            Err(UpdateError::SignatureInvalid)
        );
        assert_eq!(t.requests().len(), 3);
    }

    // @req REQ-UPD-03
    #[tokio::test]
    async fn discover_rejects_invalid_signature_without_parsing() {
        for bad_signature in [String::new(), "0".repeat(128), "zz".repeat(64)] {
            let t = CapturingTransport::new();
            let manifest = manifest_json(|_| {});
            queue_manifest(&t, &manifest, &bad_signature);
            assert_eq!(
                discover(&t, &windows_host(), "2.2.0", &test_public_key()).await,
                Err(UpdateError::SignatureInvalid)
            );
        }

        // Signed by another key.
        let t = CapturingTransport::new();
        let manifest = manifest_json(|_| {});
        let other = SigningKey::from_bytes(&[9u8; 32]);
        queue_manifest(
            &t,
            &manifest,
            &other
                .sign(manifest.as_bytes())
                .to_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        );
        assert_eq!(
            discover(&t, &windows_host(), "2.2.0", &test_public_key()).await,
            Err(UpdateError::SignatureInvalid)
        );

        // Validly signed bytes that are not a manifest: rejected after the signature passed.
        let t = CapturingTransport::new();
        let not_json = "not a manifest at all";
        queue_manifest(&t, not_json, &sign_hex(not_json.as_bytes()));
        assert_eq!(
            discover(&t, &windows_host(), "2.2.0", &test_public_key()).await,
            Err(UpdateError::Unsupported)
        );
    }

    // @req REQ-UPD-02
    #[tokio::test]
    async fn discover_minimum_os_blocks_the_offer() {
        let t = CapturingTransport::new();
        let manifest = manifest_json(|v| v["minimum_os"] = "10.0.99999".into());
        queue_manifest(&t, &manifest, &sign_hex(manifest.as_bytes()));
        assert_eq!(
            discover(&t, &windows_host(), "2.2.0", &test_public_key()).await,
            Err(UpdateError::Unsupported)
        );

        // Unknown host OS does not block the offer.
        let t = CapturingTransport::new();
        let manifest = manifest_json(|v| v["minimum_os"] = "10.0.99999".into());
        queue_manifest(&t, &manifest, &sign_hex(manifest.as_bytes()));
        let unknown = HostTarget {
            platform: "windows",
            architecture: "x64",
            os_version: None,
        };
        assert!(matches!(
            discover(&t, &unknown, "2.2.0", &test_public_key()).await,
            Ok(Discovery::Available(_))
        ));

        // Equal minimum OS is met.
        let t = CapturingTransport::new();
        let manifest = manifest_json(|v| v["minimum_os"] = "10.0.22000".into());
        queue_manifest(&t, &manifest, &sign_hex(manifest.as_bytes()));
        let exact = HostTarget {
            os_version: Some("10.0.22000".into()),
            ..windows_host()
        };
        assert!(matches!(
            discover(&t, &exact, "2.2.0", &test_public_key()).await,
            Ok(Discovery::Available(_))
        ));
    }

    /// A candidate whose artifact is `bytes`.
    fn candidate(artifact: &str, bytes: &[u8]) -> Candidate {
        let manifest: Manifest = serde_json::from_str(&manifest_json(|v| {
            v["artifact"] = artifact.into();
            v["size"] = bytes.len().into();
            v["sha256"] = sha256_hex(bytes).into();
        }))
        .unwrap();
        Candidate {
            manifest,
            artifact_url: ARTIFACT_URL.into(),
        }
    }

    type Progress = Arc<Mutex<Vec<(u64, Option<u64>)>>>;

    fn progress() -> (Progress, impl Fn(u64, Option<u64>) + Send + Sync) {
        let log: Progress = Arc::default();
        let sink = log.clone();
        let record = move |done: u64, total: Option<u64>| sink.lock().unwrap().push((done, total));
        (log, record)
    }

    fn dirs(root: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(root)
            .map(|it| it.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default()
    }

    // @req REQ-UPD-04
    #[tokio::test]
    async fn download_verified_accepts_matching_file_and_reports_progress() {
        let staging = tempfile::tempdir().unwrap();
        let mut d = FakeDownloader::new();
        d.push(&[b"hello ".as_slice(), b"artifact"].concat());
        let (log, record) = progress();

        let path = download_verified(
            &d,
            &candidate("SnackPilot-2.3.0-windows-x64-setup.exe", b"hello artifact"),
            staging.path(),
            &record,
        )
        .await
        .expect("matching size and hash must be accepted");

        assert_eq!(
            path,
            staging
                .path()
                .join("SnackPilot-2.3.0-windows-x64-setup.exe")
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"hello artifact");
        let names = dirs(staging.path());
        assert_eq!(names, vec![path], "no .part file may remain");
        let calls = d.calls();
        assert_eq!(calls.len(), 1, "exactly one download");
        assert_eq!(
            d.calls()[0].max_bytes,
            14,
            "the cap is the signed manifest size (14 bytes)"
        );
        assert_eq!(calls[0].url, ARTIFACT_URL);
        assert_eq!(
            calls[0].dest,
            staging
                .path()
                .join("SnackPilot-2.3.0-windows-x64-setup.exe.part"),
            "writes <artifact>.part"
        );
        assert!(!log.lock().unwrap().is_empty());
    }

    // @req REQ-UPD-04
    #[tokio::test]
    async fn download_removes_staging_on_hash_mismatch() {
        let staging = tempfile::tempdir().unwrap();
        let mut d = FakeDownloader::new();
        d.push(b"hello artiFACT");
        let (_, record) = progress();
        assert_eq!(
            download_verified(
                &d,
                &candidate("SnackPilot-2.3.0-windows-x64-setup.exe", b"hello artifact"),
                staging.path(),
                &record,
            )
            .await,
            Err(UpdateError::HashMismatch)
        );
        assert!(dirs(staging.path()).is_empty(), "staging must be emptied");
    }

    // @req REQ-UPD-04
    #[tokio::test]
    async fn download_removes_staging_on_size_mismatch_and_overrun() {
        // Short by one byte, hash therefore also wrong.
        let staging = tempfile::tempdir().unwrap();
        let mut d = FakeDownloader::new();
        d.push(b"hello artifac");
        let (_, record) = progress();
        assert_eq!(
            download_verified(
                &d,
                &candidate("a.exe", b"hello artifact"),
                staging.path(),
                &record,
            )
            .await,
            Err(UpdateError::SizeMismatch)
        );
        assert!(dirs(staging.path()).is_empty());

        // Longer than the signed manifest size: the downloader's cap trips.
        let staging = tempfile::tempdir().unwrap();
        let mut d = FakeDownloader::new();
        d.push(b"hello artifact!");
        let (_, record) = progress();
        assert_eq!(
            download_verified(
                &d,
                &candidate("a.exe", b"hello artifact"),
                staging.path(),
                &record,
            )
            .await,
            Err(UpdateError::SizeMismatch)
        );
        assert!(dirs(staging.path()).is_empty());
    }

    // @req REQ-UPD-04
    #[tokio::test]
    async fn download_rejects_non_empty_staging() {
        let staging = tempfile::tempdir().unwrap();
        std::fs::write(staging.path().join("leftover"), b"x").unwrap();
        let mut d = FakeDownloader::new();
        d.push(b"hello artifact");
        let (_, record) = progress();
        assert_eq!(
            download_verified(
                &d,
                &candidate("a.exe", b"hello artifact"),
                staging.path(),
                &record,
            )
            .await,
            Err(UpdateError::Io)
        );
        assert!(d.calls().is_empty(), "nothing may be downloaded");
        assert!(staging.path().join("leftover").exists());
    }

    /// One-shot plain loopback server, the style of `reqwest_transport.rs`'s
    /// tests. Replies with `body`, with or without `Content-Length`.
    fn loopback_server(body: &'static [u8], with_length: bool) -> String {
        loopback_response(body, with_length, 200)
    }

    fn loopback_response(body: &'static [u8], with_length: bool, status: u16) -> String {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let _ = s.read(&mut [0u8; 2048]);
                let head = if with_length {
                    format!(
                        "HTTP/1.1 {status} X\r\nContent-Length: {}\r\n\r\n",
                        body.len()
                    )
                } else {
                    format!("HTTP/1.1 {status} X\r\nConnection: close\r\n\r\n")
                };
                let _ = s.write_all(head.as_bytes());
                let _ = s.write_all(body);
            }
        });
        format!("http://{addr}/artifact")
    }

    // @req REQ-UPD-04
    #[tokio::test]
    async fn reqwest_downloader_streams_from_a_local_server() {
        // The test seam: `new()` is https only, so the loopback server is reached
        // through an explicitly configured client. `new()` itself is checked below.
        let client = reqwest::Client::builder()
            .https_only(false)
            .build()
            .unwrap();
        let body: &'static [u8] = b"0123456789abcdef";

        // Content-Length present: progress carries the total and ends at the size.
        let url = loopback_server(body, true);
        let staging = tempfile::tempdir().unwrap();
        let dest = staging.path().join("artifact.zip");
        let (log, record) = progress();
        ReqwestDownloader::with_client(client.clone())
            .download(&url, &dest, 1024, &record)
            .await
            .expect("streaming download must succeed");
        assert_eq!(std::fs::read(&dest).unwrap(), body);
        let seen = log.lock().unwrap().clone();
        assert!(!seen.is_empty(), "progress must be reported");
        assert_eq!(seen.last().map(|p| p.0), Some(16));
        assert!(
            seen.iter().all(|p| p.1 == Some(16)),
            "Content-Length must be reported: {seen:?}"
        );
        assert!(
            seen.windows(2).all(|w| w[0].0 <= w[1].0),
            "progress never goes backwards: {seen:?}"
        );

        // Absent: the download completes and the total is unknown.
        let url = loopback_server(body, false);
        let dest = staging.path().join("nolength.zip");
        let (log, record) = progress();
        ReqwestDownloader::with_client(client.clone())
            .download(&url, &dest, 1024, &record)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), body);
        assert!(log.lock().unwrap().iter().all(|p| p.1.is_none()));

        // Over the cap: stops at max_bytes with SizeMismatch.
        let url = loopback_server(body, true);
        let dest = staging.path().join("capped.zip");
        let (_, record) = progress();
        assert_eq!(
            ReqwestDownloader::with_client(client.clone())
                .download(&url, &dest, 8, &record)
                .await,
            Err(UpdateError::SizeMismatch)
        );

        // A non-200 answer is a network failure.
        let url = loopback_response(b"denied", true, 500);
        assert_eq!(
            ReqwestDownloader::with_client(client)
                .download(&url, &staging.path().join("denied.zip"), 1024, &record)
                .await,
            Err(UpdateError::Network)
        );

        // Production's `new()` keeps the https-only rule: a plain http address is
        // refused before any connection.
        assert_eq!(
            ReqwestDownloader::new()
                .download(
                    "http://127.0.0.1:9/a.zip",
                    &staging.path().join("plain.zip"),
                    1024,
                    &record
                )
                .await,
            Err(UpdateError::Unsupported)
        );
    }

    // @req REQ-UPD-01
    #[test]
    fn update_urls_name_only_github_hosts() {
        for url in [RELEASES_URL, DOWNLOAD_PREFIX] {
            assert!(url.starts_with("https://"), "{url}");
            assert!(!url.to_lowercase().contains("gourmet"), "{url}");
            assert!(!url.to_lowercase().contains("ventopay"), "{url}");
        }
        assert_eq!(
            RELEASES_URL,
            "https://api.github.com/repos/radaiko/SnackPilot/releases?per_page=100"
        );
        assert!(!RELEASES_URL.contains("releases/latest"));
        assert_eq!(
            DOWNLOAD_PREFIX,
            "https://github.com/radaiko/SnackPilot/releases/download/"
        );
    }
}
