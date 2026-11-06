//! Pins for the gpui dependency set (REQ-UI-02) and the no-HTTP-client rule (REQ-UI-08).
//! Parsed with std only: adding a TOML crate here would need its own approval.

use std::fs;
use std::path::{Path, PathBuf};

fn ui_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Dependency names listed in a manifest's [dependencies] sections.
fn dependency_names(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line == "[dependencies]" || line == "[dev-dependencies]";
            continue;
        }
        if in_deps {
            if let Some((name, _)) = line.split_once('=') {
                let name = name.trim().trim_matches('"');
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        }
    }
    names
}

// @req REQ-UI-02
#[test]
fn gpui_dependencies_are_pinned() {
    let root = ui_root();
    let workspace = read(&root.join("Cargo.toml"));

    assert!(
        workspace.contains(r#"package = "gpui-pre", version = "=0.3.4""#),
        "gpui-pre must be pinned to exactly =0.3.4"
    );
    assert!(
        workspace.contains(r#"gpui-kit = { git = "https://github.com/longbridge/gpui-kit", rev = "7d9efcd2069f9eaa6eb3ba6345aac4aa7d87c9f7""#),
        "gpui-kit must be pinned to git rev 7d9efcd2069f9eaa6eb3ba6345aac4aa7d87c9f7"
    );
    assert!(
        workspace.contains(r#"package = "gpui-pre-mobile", git = "https://github.com/longbridge/gpui-mobile", rev = "0b882efdac7f524e0bb0b1d4c886b2aa752f9f20""#),
        "gpui-pre-mobile must be pinned to git rev 0b882efdac7f524e0bb0b1d4c886b2aa752f9f20"
    );

    // The lockfile must resolve exactly one gpui-pre, at the pinned version.
    let lock = read(&root.join("Cargo.lock"));
    let gpui_pre: Vec<String> = lock
        .split("[[package]]")
        .filter(|block| block.contains(r#"name = "gpui-pre""#))
        .map(|block| {
            block
                .lines()
                .find_map(|l| l.trim().strip_prefix(r#"version = "#))
                .unwrap_or_default()
                .trim_matches('"')
                .to_string()
        })
        .collect();
    assert_eq!(
        gpui_pre,
        vec!["0.3.4"],
        "Cargo.lock must contain exactly one gpui-pre at 0.3.4, found {gpui_pre:?}"
    );
}

// @req REQ-UI-08
#[test]
fn model_manifest_lists_no_http_client() {
    let manifest = read(&ui_root().join("model/Cargo.toml"));
    let names = dependency_names(&manifest);
    for forbidden in [
        "reqwest",
        "hyper",
        "hyper-util",
        "hyper-tls",
        "ureq",
        "awc",
        "surf",
        "isahc",
    ] {
        assert!(
            !names.iter().any(|n| n == forbidden),
            "snackpilot-model must not depend on HTTP client crate {forbidden} (REQ-UI-08)"
        );
    }
}
