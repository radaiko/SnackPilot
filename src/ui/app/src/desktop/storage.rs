//! Durable desktop data directory (desktop-ui-redesign task 1.6).
//!
//! A shipping desktop build keeps its cache and preferences in per-user
//! application data, never in the development temp directory and never inside
//! the installed `.app` (REQ-DST-04):
//!
//! - macOS: `~/Library/Application Support/dev.radaiko.gourmetclient/`
//! - Windows: `%LOCALAPPDATA%\dev.radaiko.gourmetclient\`
//!
//! There is deliberately no fallback. A missing home/`LOCALAPPDATA`, an
//! unsupported host or an unwritable directory is a truthful startup failure,
//! because silently continuing in a temp or install location would lose the
//! user's settings on the next launch or update.
//!
//! @req REQ-DST-04

use std::fmt;
use std::path::{Path, PathBuf};

use super::metadata::BUNDLE_ID;

/// One level of the host path rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopOs {
    MacOs,
    Windows,
    Other,
}

impl DesktopOs {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            DesktopOs::MacOs
        } else if cfg!(target_os = "windows") {
            DesktopOs::Windows
        } else {
            DesktopOs::Other
        }
    }
}

/// The host values the resolution depends on, injected so tests can exercise
/// macOS and Windows rules on this Linux VM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPaths {
    pub os: DesktopOs,
    pub home: Option<PathBuf>,
    pub local_app_data: Option<PathBuf>,
}

impl HostPaths {
    pub fn current() -> Self {
        Self {
            os: DesktopOs::current(),
            home: std::env::var_os("HOME").map(PathBuf::from),
            local_app_data: std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        }
    }
}

/// Why the durable directory cannot be used. Every variant is fatal; the host
/// reports it instead of substituting another location (REQ-DST-04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    MissingHome,
    MissingLocalAppData,
    UnsupportedHost,
    Unwritable { path: PathBuf, reason: String },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::MissingHome => {
                write!(f, "desktop storage unavailable: HOME is not set")
            }
            StorageError::MissingLocalAppData => {
                write!(f, "desktop storage unavailable: LOCALAPPDATA is not set")
            }
            StorageError::UnsupportedHost => write!(
                f,
                "desktop storage unavailable: this host has no shipping data location"
            ),
            StorageError::Unwritable { path, reason } => write!(
                f,
                "desktop storage unavailable: {} is not writable ({reason})",
                path.display()
            ),
        }
    }
}

impl std::error::Error for StorageError {}

/// The durable per-user data directory for `paths`, without touching the
/// filesystem. macOS uses Application Support under the user's home; Windows
/// uses LOCALAPPDATA. Anything else is not a supported shipping host.
pub fn data_dir(paths: &HostPaths) -> Result<PathBuf, StorageError> {
    match paths.os {
        DesktopOs::MacOs => paths
            .home
            .as_ref()
            .map(|home| {
                home.join("Library")
                    .join("Application Support")
                    .join(BUNDLE_ID)
            })
            .ok_or(StorageError::MissingHome),
        DesktopOs::Windows => paths
            .local_app_data
            .as_ref()
            .map(|local| local.join(BUNDLE_ID))
            .ok_or(StorageError::MissingLocalAppData),
        DesktopOs::Other => Err(StorageError::UnsupportedHost),
    }
}

/// Create the directory if needed and prove it accepts a write. No probe is
/// left behind; a failure is reported, never turned into a temp fallback.
pub fn ensure_writable(dir: &Path) -> Result<(), StorageError> {
    std::fs::create_dir_all(dir).map_err(|error| StorageError::Unwritable {
        path: dir.to_path_buf(),
        reason: error.to_string(),
    })?;
    let probe = dir.join(format!(".snackpilot-write-probe-{}", std::process::id()));
    std::fs::write(&probe, b"probe").map_err(|error| StorageError::Unwritable {
        path: dir.to_path_buf(),
        reason: error.to_string(),
    })?;
    std::fs::remove_file(&probe).map_err(|error| StorageError::Unwritable {
        path: dir.to_path_buf(),
        reason: error.to_string(),
    })
}

/// Resolve and prepare the durable directory for this host.
pub fn prepare(paths: &HostPaths) -> Result<PathBuf, StorageError> {
    let dir = data_dir(paths)?;
    ensure_writable(&dir)?;
    Ok(dir)
}

/// The directory the desktop host boots with. A normal launch always uses the
/// durable per-user directory; the isolated debug evidence harness keeps its
/// explicit temp subdirectory so captures cannot pollute real user data.
pub fn launch_storage() -> Result<PathBuf, StorageError> {
    let base = if crate::evidence::requested() {
        std::env::temp_dir().join("snackpilot-desktop")
    } else {
        prepare(&HostPaths::current())?
    };
    Ok(crate::evidence::storage_dir(base))
}

#[cfg(test)]
mod tests {
    use super::BUNDLE_ID;
    use super::{data_dir, ensure_writable, prepare, DesktopOs, HostPaths, StorageError};
    use std::path::PathBuf;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "snackpilot-desktop-storage-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn mac(home: impl Into<PathBuf>) -> HostPaths {
        HostPaths {
            os: DesktopOs::MacOs,
            home: Some(home.into()),
            local_app_data: Some(PathBuf::from("/should-not-be-used")),
        }
    }

    fn windows(local: impl Into<PathBuf>) -> HostPaths {
        HostPaths {
            os: DesktopOs::Windows,
            home: Some(PathBuf::from("/should-not-be-used")),
            local_app_data: Some(local.into()),
        }
    }

    /// macOS resolves the durable Application Support directory under the
    /// user's home, independent of where the app is installed (REQ-DST-04).
    /// @req REQ-DST-04
    #[test]
    fn macos_uses_application_support_under_home() {
        let dir = data_dir(&mac("/Users/ada")).unwrap();
        assert_eq!(
            dir,
            PathBuf::from("/Users/ada/Library/Application Support").join(BUNDLE_ID)
        );
        let app_bundle = PathBuf::from("/Applications/SnackPilot.app");
        assert!(
            !dir.starts_with(&app_bundle),
            "user data must live outside the installed bundle"
        );
        assert!(
            !dir.starts_with(std::env::temp_dir()),
            "user data must not live in the development temp directory"
        );
        assert!(
            !dir.to_string_lossy().contains(".app"),
            "the install location is not a storage fallback"
        );
    }

    /// Windows resolves LOCALAPPDATA, not the home directory, and never a temp
    /// or install path (REQ-DST-04).
    /// @req REQ-DST-04
    #[test]
    fn windows_uses_local_app_data() {
        let dir = data_dir(&windows("/Users/ada/AppData/Local")).unwrap();
        assert_eq!(
            dir,
            PathBuf::from("/Users/ada/AppData/Local").join(BUNDLE_ID)
        );
        assert!(!dir.starts_with("/Users/ada/Library"));
        assert!(!dir.starts_with(std::env::temp_dir()));
    }

    /// A missing home or LOCALAPPDATA is a truthful failure, not a fallback.
    /// @req REQ-DST-04
    #[test]
    fn missing_host_variables_fail_closed() {
        assert_eq!(
            data_dir(&HostPaths {
                os: DesktopOs::MacOs,
                home: None,
                local_app_data: Some(PathBuf::from("/local")),
            }),
            Err(StorageError::MissingHome)
        );
        assert_eq!(
            data_dir(&HostPaths {
                os: DesktopOs::Windows,
                home: Some(PathBuf::from("/home")),
                local_app_data: None,
            }),
            Err(StorageError::MissingLocalAppData)
        );
        assert_eq!(
            data_dir(&HostPaths {
                os: DesktopOs::Other,
                home: Some(PathBuf::from("/home")),
                local_app_data: Some(PathBuf::from("/local")),
            }),
            Err(StorageError::UnsupportedHost)
        );
    }

    /// An unwritable data location reports its path and reason; nothing is
    /// created in a temp directory instead (REQ-DST-04).
    /// @req REQ-DST-04
    #[test]
    fn unwritable_location_fails_truthfully() {
        let root = temp_root("unwritable");
        // A file where a directory is required: create_dir_all fails even for
        // a privileged user, so this covers the unwritable-home class.
        let blocker = root.join("home-is-a-file");
        std::fs::write(&blocker, b"not a directory").unwrap();
        let error = prepare(&mac(&blocker)).unwrap_err();
        match error {
            StorageError::Unwritable { path, .. } => {
                assert!(path.starts_with(&blocker), "error names the real path");
            }
            other => panic!("expected Unwritable, got {other:?}"),
        }
        assert_eq!(
            std::fs::read_dir(&root).unwrap().count(),
            1,
            "a failed resolution writes nothing beside the blocking file"
        );
    }

    /// Preparing an existing directory keeps its contents, so an installer
    /// replacing the app cannot overwrite the user's cached data or settings.
    /// @req REQ-DST-04
    #[test]
    fn existing_data_survives_install_and_relaunch() {
        let root = temp_root("durable");
        let paths = mac(root.join("home"));
        let dir = prepare(&paths).unwrap();
        std::fs::write(dir.join("prefs.json"), b"{\"accent\":\"orange\"}").unwrap();

        // A replacement app bundle installs elsewhere; a relaunch resolves the
        // same per-user directory and still finds the file.
        let replacement_bundle = root.join("Applications/SnackPilot.app");
        std::fs::create_dir_all(&replacement_bundle).unwrap();
        assert!(!dir.starts_with(&replacement_bundle));

        let again = prepare(&paths).unwrap();
        assert_eq!(again, dir);
        assert_eq!(
            std::fs::read_to_string(again.join("prefs.json")).unwrap(),
            "{\"accent\":\"orange\"}"
        );
        assert!(
            !again
                .join(format!(".snackpilot-write-probe-{}", std::process::id()))
                .exists(),
            "the writability probe never remains in user data"
        );
    }

    /// The probe itself fails on a read-only directory (POSIX mode), which is
    /// the other shape of an unwritable home.
    /// @req REQ-DST-04
    #[cfg(unix)]
    #[test]
    fn read_only_directory_is_not_writable() {
        use std::os::unix::fs::PermissionsExt;
        let root = temp_root("readonly");
        let dir = root.join("data");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let error = ensure_writable(&dir).unwrap_err();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(error, StorageError::Unwritable { .. }));
    }
}
