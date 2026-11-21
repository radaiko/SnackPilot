//! `X.Y.Z` versions and the `desktop/vX.Y.Z` tag namespace (design D2, decision A).
//!
//! Digits only: no leading `v`, no pre-release or build suffix, no leading zeros
//! beyond a bare `0`. Comparison is numeric per component, so `2.10.0 > 2.9.0`.

use std::cmp::Ordering;

const TAG_PREFIX: &str = "desktop/v";

/// A parsed `X.Y.Z` version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    /// Parses exactly `X.Y.Z`. Returns `None` for anything else, including a
    /// leading `v`, whitespace, pre-release suffixes and non-ASCII digits.
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split('.');
        let major = component(parts.next()?)?;
        let minor = component(parts.next()?)?;
        let patch = component(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }
        Some(Version {
            major,
            minor,
            patch,
        })
    }
}

/// One numeric component: at least one ASCII digit, and no leading zero.
fn component(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if s.len() > 1 && s.starts_with('0') {
        return None;
    }
    s.parse().ok()
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Parses a release tag, accepting only the exact shape `desktop/vX.Y.Z`.
pub fn parse_desktop_tag(tag: &str) -> Option<Version> {
    Version::parse(tag.strip_prefix(TAG_PREFIX)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    // @req REQ-UPD-01
    #[test]
    fn version_rejects_other_shapes() {
        for bad in [
            "",
            "v1.2.3",
            "1.2",
            "1.2.3.4",
            "1.2.3-rc1",
            "1.2.3+b",
            " 1.2.3",
            "1.2.x",
            "01.2.3",
            "1..3",
            "-1.2.3",
            "1.2.3\n",
        ] {
            assert_eq!(Version::parse(bad), None, "{bad:?} must not parse");
        }
    }

    // @req REQ-UPD-01
    #[test]
    fn tag_rejects_every_other_namespace() {
        for bad in [
            "ios/v1.2.3",
            "macos/v1.2.3",
            "windows/v1.2.3",
            "desktop/v1.2.3-rc1",
            "desktop/1.2.3",
            "desktop/vv1.2.3",
            "desktop/v1.2",
            "Desktop/v1.2.3",
        ] {
            assert_eq!(parse_desktop_tag(bad), None, "{bad:?} must not parse");
        }
        assert!(parse_desktop_tag("desktop/v1.2.3").is_some());
    }
}
