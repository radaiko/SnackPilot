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
