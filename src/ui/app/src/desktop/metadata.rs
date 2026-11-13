//! Desktop release metadata (desktop-ui-redesign task 1.6).
//!
//! The common semver/build inputs are the iOS project settings `make ship`
//! already bumps; the packaging step may override them at compile time through
//! `SNACKPILOT_PRODUCT_VERSION` / `SNACKPILOT_BUILD`. About and the rail footer
//! read this module, so a release never shows the app crate's `0.0.0`
//! placeholder (REQ-UI-14).
//!
//! The bundle constants below are what `tools/devops/package-macos.sh` writes
//! into `Info.plist.in` and what the native inspection verifies: stable
//! identity, one arm64 slice and a macOS 26.0 floor (REQ-DST-01).
//!
//! @req REQ-DST-01 @req REQ-UI-14

/// Stable desktop identity; also the Application Support directory name.
pub const BUNDLE_ID: &str = "dev.radaiko.gourmetclient";
/// Release CPU architecture (Apple Silicon only, REQ-DST-01).
pub const ARCHITECTURE: &str = "aarch64";
/// Minimum supported macOS for the release app.
pub const MINIMUM_MACOS_VERSION: &str = "26.0";
/// Executable name inside `SnackPilot.app/Contents/MacOS/`.
pub const EXECUTABLE_NAME: &str = "SnackPilot";
/// Icon resource name (`SnackPilot.icns`) inside `Contents/Resources/`.
pub const ICON_NAME: &str = "SnackPilot";

/// The common release inputs (iOS project settings are the shared semver
/// source used by `make ship`). Embedded so About works from an installed
/// bundle without a checkout.
const COMMON_RELEASE_INPUTS: &str = include_str!("../../../ios/project.yml");

/// The plist the packaging step fills in. Embedded here so the metadata tests
/// pin the placeholders the script has to substitute.
#[cfg(test)]
const INFO_PLIST_TEMPLATE: &str = include_str!("../../../desktop/macos/Info.plist.in");

/// The version/build pair every desktop surface reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseMetadata {
    pub version: String,
    pub build: String,
}

/// First `key: value` in a YAML fragment, quotes trimmed.
pub fn yaml_value(yaml: &str, key: &str) -> Option<String> {
    yaml.lines().find_map(|line| {
        let value = line.trim().strip_prefix(key)?.trim_start();
        let value = value.strip_prefix(':')?.trim();
        (!value.is_empty()).then(|| value.trim_matches('"').to_string())
    })
}

/// Resolve one key: a non-placeholder compile-time override wins, otherwise
/// the shared project input.
fn release_metadata(key: &str, env_value: Option<&'static str>, yaml: &str) -> Option<String> {
    if let Some(value) = env_value {
        let value = value.trim();
        if !value.is_empty() && value != "0.0.0" && value != "0" {
            return Some(value.to_string());
        }
    }
    yaml_value(yaml, key).filter(|value| value != "0.0.0" && value != "0")
}

/// Runtime product version: the common release metadata from the shared
/// inputs (or a packaging override), explicit debug metadata when it is
/// missing. Never the placeholder `0.0.0` (REQ-UI-14).
pub fn product_version() -> String {
    release_metadata(
        "MARKETING_VERSION",
        option_env!("SNACKPILOT_PRODUCT_VERSION"),
        COMMON_RELEASE_INPUTS,
    )
    .or_else(|| cfg!(debug_assertions).then(|| "Entwicklung".to_string()))
    .unwrap_or_else(|| "unbekannt".to_string())
}

/// Runtime build metadata, same rules as [`product_version`].
pub fn product_build() -> String {
    release_metadata(
        "CURRENT_PROJECT_VERSION",
        option_env!("SNACKPILOT_BUILD"),
        COMMON_RELEASE_INPUTS,
    )
    .or_else(|| cfg!(debug_assertions).then(|| "debug".to_string()))
    .unwrap_or_else(|| "unbekannt".to_string())
}

/// The pair About, the rail footer and the release receipt share.
pub fn release() -> ReleaseMetadata {
    ReleaseMetadata {
        version: product_version(),
        build: product_build(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        release, release_metadata, yaml_value, ARCHITECTURE, BUNDLE_ID, EXECUTABLE_NAME, ICON_NAME,
        INFO_PLIST_TEMPLATE, MINIMUM_MACOS_VERSION,
    };

    /// The shared project input supplies a real version/build, so About and
    /// the rail never fall back to the app crate's `0.0.0` (REQ-UI-14).
    /// @req REQ-UI-14
    #[test]
    fn release_metadata_is_not_a_placeholder() {
        let metadata = release();
        assert_ne!(metadata.version, "0.0.0");
        assert!(!metadata.version.is_empty());
        assert_ne!(metadata.build, "0");
        assert!(!metadata.build.is_empty());
        assert_eq!(
            yaml_value("a: 1\nMARKETING_VERSION: \"2.1.0\"\n", "MARKETING_VERSION").as_deref(),
            Some("2.1.0")
        );
        assert_eq!(
            yaml_value("CURRENT_PROJECT_VERSION: 35", "CURRENT_PROJECT_VERSION").as_deref(),
            Some("35")
        );
    }

    /// A `0.0.0`/`0` override is ignored in favour of the common input, and a
    /// `0.0.0` input resolves to nothing rather than being reported.
    /// @req REQ-UI-14
    #[test]
    fn placeholders_are_filtered_from_release_metadata() {
        assert_eq!(
            release_metadata(
                "MARKETING_VERSION",
                Some("0.0.0"),
                "MARKETING_VERSION: \"2.1.0\"",
            )
            .as_deref(),
            Some("2.1.0")
        );
        assert_eq!(
            release_metadata(
                "CURRENT_PROJECT_VERSION",
                Some("0"),
                "CURRENT_PROJECT_VERSION: 7"
            )
            .as_deref(),
            Some("7")
        );
        assert_eq!(
            release_metadata("MARKETING_VERSION", None, "MARKETING_VERSION: \"0.0.0\""),
            None
        );
        assert_eq!(
            release_metadata(
                "CURRENT_PROJECT_VERSION",
                None,
                "CURRENT_PROJECT_VERSION: 0"
            ),
            None
        );
    }

    /// The packaging template carries every value the release identity needs,
    /// and the Rust constants agree with the stable bundle identity
    /// (REQ-DST-01).
    /// @req REQ-DST-01
    #[test]
    fn bundle_metadata_matches_the_packaging_template() {
        assert_eq!(BUNDLE_ID, "dev.radaiko.gourmetclient");
        assert_eq!(ARCHITECTURE, "aarch64");
        assert_eq!(MINIMUM_MACOS_VERSION, "26.0");
        assert_eq!(EXECUTABLE_NAME, "SnackPilot");
        assert_eq!(ICON_NAME, "SnackPilot");
        for placeholder in [
            "@BUNDLE_ID@",
            "@EXECUTABLE@",
            "@ICON@",
            "@VERSION@",
            "@BUILD@",
            "@MINIMUM_SYSTEM_VERSION@",
        ] {
            assert!(
                INFO_PLIST_TEMPLATE.contains(placeholder),
                "Info.plist.in is missing {placeholder}"
            );
        }
    }
}
