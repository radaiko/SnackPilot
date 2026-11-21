//! Desktop updater host side: staging, macOS bundle trust and both handoffs
//! (design D1/D6; REQ-UPD-04…09).
//!
//! Everything here is a plain function over a [`CommandRunner`], so the whole
//! macOS sequence runs on Linux with a stub instead of `ditto`, `PlistBuddy`,
//! `codesign`, `stapler` and `spctl`. The tools are named by absolute path
//! (REQ-UPD-06), the staging directory is the only thing this module ever
//! deletes, and neither handoff replaces the running app (REQ-UPD-07). A pin
//! test (`src/ui/model/tests/pins.rs`, task 1.3) keeps `reqwest` out of this crate.
//!
//! @req REQ-UPD-04 @req REQ-UPD-05 @req REQ-UPD-06 @req REQ-UPD-07
//! @req REQ-UPD-08 @req REQ-UPD-09

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use snackpilot_model::{
    HandoffOutcome, PlatformError, UpdateHostInfo, VerifiedManifest, VerifiedUpdate,
};

use super::metadata::BUNDLE_ID;

/// The Developer ID team every macOS release is signed with (REQ-UPD-06).
pub const TEAM_ID: &str = "555V4MNLK3";
/// The one bundle a release ZIP may contain (REQ-UPD-05).
pub const APP_BUNDLE: &str = "SnackPilot.app";

// Apple tools by absolute path (REQ-UPD-06): no `PATH` entry can stand in for
// one of these.
const DITTO: &str = "/usr/bin/ditto";
const PLISTBUDDY: &str = "/usr/libexec/PlistBuddy";
const CODESIGN: &str = "/usr/bin/codesign";
const STAPLER: &str = "/usr/bin/xcrun";
const SPCTL: &str = "/usr/sbin/spctl";
const OPEN: &str = "/usr/bin/open";

/// Prefix of every Windows staging directory under the OS temp directory;
/// launch removes exactly these and nothing else.
pub const TEMP_STAGING_PREFIX: &str = "snackpilot-update-";

/// Runs one tool to completion and answers its combined output and exit
/// success. Production is [`SystemRunner`]; tests script a stub, so no test
/// starts a process.
pub trait CommandRunner {
    fn run(&self, program: &str, args: &[String]) -> io::Result<(bool, String)>;
}

/// The real runner: a direct exec (no shell), stdin closed.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, program: &str, args: &[String]) -> io::Result<(bool, String)> {
        let output = Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()?;
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok((output.status.success(), text))
    }
}

/// `macos`/`arm64` on Apple Silicon, `windows`/`x64` on 64-bit Windows, and
/// nothing anywhere else (REQ-UPD-04). Pure `cfg!`: never a network read.
pub fn supported_target() -> Option<(&'static str, &'static str)> {
    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        Some(("macos", "arm64"))
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        Some(("windows", "x64"))
    } else {
        None
    }
}

/// macOS staging: `~/Downloads/SnackPilot-{version}/`, created or emptied so a
/// retry of the same version never inherits an earlier file.
pub fn macos_staging_dir(home: &Path, version: &str) -> Result<PathBuf, PlatformError> {
    let dir = home.join("Downloads").join(format!("SnackPilot-{version}"));
    fresh_empty_dir(&dir)?;
    Ok(dir)
}

/// Windows staging: a new `snackpilot-update-…` directory under `temp`. The
/// nonce only separates two attempts at the same version, never two runs.
pub fn windows_staging_dir(
    temp: &Path,
    version: &str,
    nonce: u128,
) -> Result<PathBuf, PlatformError> {
    let dir = temp.join(format!("{TEMP_STAGING_PREFIX}{version}-{nonce:x}"));
    std::fs::create_dir(&dir).map_err(|e| PlatformError::System(e.to_string()))?;
    Ok(dir)
}

/// Makes `dir` exist and contain nothing, whatever it held before.
fn fresh_empty_dir(dir: &Path) -> Result<(), PlatformError> {
    let failed = |e: io::Error| PlatformError::System(e.to_string());
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(failed)?;
    }
    std::fs::create_dir_all(dir).map_err(failed)?;
    if std::fs::read_dir(dir).map_err(failed)?.next().is_some() {
        return Err(PlatformError::System("staging is not empty".into()));
    }
    Ok(())
}

/// The staging directory of a downloaded artifact: the ZIP's or installer's
/// own directory. Nothing else is ever removed for a failed verification.
pub fn staging_dir_of(artifact: &Path) -> io::Result<PathBuf> {
    artifact
        .parent()
        .map(Path::to_path_buf)
        .filter(|dir| !dir.as_os_str().is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "artifact has no directory"))
}

/// Removes `dir` and answers the typed failure, so every failing step of a
/// verification leaves nothing installable (REQ-UPD-04/05/06).
fn discard(dir: &Path, reason: &str) -> PlatformError {
    let _ = std::fs::remove_dir_all(dir);
    PlatformError::System(reason.to_string())
}

/// Windows: the artifact is the staged installer, named as the signed manifest
/// says. There is no bundle trust to add.
pub fn verify_windows_artifact(
    artifact: &Path,
    manifest: &VerifiedManifest,
) -> Result<VerifiedUpdate, PlatformError> {
    let dir = staging_dir_of(artifact).map_err(|e| PlatformError::System(e.to_string()))?;
    let named = artifact.file_name().and_then(|n| n.to_str()) == Some(manifest.artifact.as_str());
    if !named || !artifact.is_file() {
        return Err(discard(&dir, "not the staged installer"));
    }
    Ok(VerifiedUpdate {
        version: manifest.version.clone(),
        artifact: artifact.to_path_buf(),
        prepared: None,
    })
}

/// macOS: extract into the staging directory, then prove identity and trust in
/// the order of REQ-UPD-05/06; any failure deletes the whole staging directory.
pub fn verify_macos_artifact<R: CommandRunner + ?Sized>(
    runner: &R,
    artifact: &Path,
    manifest: &VerifiedManifest,
) -> Result<VerifiedUpdate, PlatformError> {
    let dir = staging_dir_of(artifact).map_err(|e| PlatformError::System(e.to_string()))?;
    match verify_macos_steps(runner, &dir, artifact, manifest) {
        Ok(bundle) => Ok(VerifiedUpdate {
            version: manifest.version.clone(),
            artifact: artifact.to_path_buf(),
            prepared: Some(bundle),
        }),
        Err(reason) => Err(discard(&dir, &reason)),
    }
}

/// The sequence itself, answering the bundle path or the failing step's name.
fn verify_macos_steps<R: CommandRunner + ?Sized>(
    runner: &R,
    dir: &Path,
    artifact: &Path,
    manifest: &VerifiedManifest,
) -> Result<PathBuf, String> {
    let extracted = dir.join("extracted");
    let text = |p: &Path| p.to_string_lossy().into_owned();

    // 1. Extract.
    run_ok(
        runner,
        DITTO,
        &["-x".into(), "-k".into(), text(artifact), text(&extracted)],
        "ditto -x -k",
    )?;

    // 2. Exactly one SnackPilot.app.
    let bundle = single_app_bundle(&extracted)?;
    let bundle_arg = text(&bundle);

    // 3. Bundle identity and version (REQ-UPD-05).
    let plist = format!("{bundle_arg}/Contents/Info.plist");
    let identifier = plist_value(runner, &plist, "CFBundleIdentifier")?;
    if identifier != BUNDLE_ID {
        return Err(format!("bundle identifier {identifier}"));
    }
    let version = plist_value(runner, &plist, "CFBundleShortVersionString")?;
    if version != manifest.version {
        return Err(format!("bundle version {version}"));
    }

    // 4. Signature, team and authority (REQ-UPD-06).
    run_ok(
        runner,
        CODESIGN,
        &[
            "--verify".into(),
            "--deep".into(),
            "--strict".into(),
            bundle_arg.clone(),
        ],
        "codesign --verify",
    )?;
    let (ok, report) = runner
        .run(
            CODESIGN,
            &["-dv".into(), "--verbose=4".into(), bundle_arg.clone()],
        )
        .map_err(|e| format!("codesign -dv: {e}"))?;
    if !ok || !report_is_release_signed(&report) {
        return Err("codesign -dv team or authority".into());
    }

    // 5. Stapled ticket, then the Gatekeeper assessment.
    run_ok(
        runner,
        STAPLER,
        &["stapler".into(), "validate".into(), bundle_arg.clone()],
        "stapler validate",
    )?;
    run_ok(
        runner,
        SPCTL,
        &[
            "--assess".into(),
            "--type".into(),
            "execute".into(),
            bundle_arg,
        ],
        "spctl --assess",
    )?;

    Ok(bundle)
}

fn run_ok<R: CommandRunner + ?Sized>(
    runner: &R,
    program: &str,
    args: &[String],
    step: &str,
) -> Result<(), String> {
    match runner.run(program, args) {
        Ok((true, _)) => Ok(()),
        Ok((false, _)) => Err(format!("{step} failed")),
        Err(e) => Err(format!("{step}: {e}")),
    }
}

/// The one `SnackPilot.app` in the extraction: at its root, or inside its
/// single top-level directory.
fn single_app_bundle(extracted: &Path) -> Result<PathBuf, String> {
    fn apps_in(dir: &Path) -> io::Result<Vec<PathBuf>> {
        let mut apps = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() && path.file_name().and_then(|n| n.to_str()) == Some(APP_BUNDLE) {
                apps.push(path);
            }
        }
        Ok(apps)
    }
    let failed = |e: io::Error| e.to_string();
    // Every bundle in the extraction counts, whether at its root or beside a
    // root one: two of them are never one release.
    let mut found = apps_in(extracted).map_err(failed)?;
    for entry in std::fs::read_dir(extracted).map_err(failed)? {
        let path = entry.map_err(failed)?.path();
        if path.is_dir() {
            found.extend(apps_in(&path).map_err(failed)?);
        }
    }
    match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err("zip contains no SnackPilot.app".into()),
        _ => Err("zip contains more than one SnackPilot.app".into()),
    }
}

/// One plist entry through PlistBuddy; a missing key or a failed tool is an
/// error, never an empty identity.
fn plist_value<R: CommandRunner + ?Sized>(
    runner: &R,
    plist: &str,
    entry: &str,
) -> Result<String, String> {
    let args = ["-c".into(), format!("Print :{entry}"), plist.to_string()];
    match runner.run(PLISTBUDDY, &args) {
        Ok((true, value)) => Ok(value.trim().to_string()),
        Ok((false, _)) => Err(format!("{entry} missing")),
        Err(e) => Err(format!("{entry}: {e}")),
    }
}

/// `codesign -dv --verbose=4` output: the team identifier and every
/// `Authority=` line, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodesignReport {
    pub team: Option<String>,
    pub authorities: Vec<String>,
}

/// Parses the `codesign -dv --verbose=4` report. `Authority=` lines may appear
/// before or after `TeamIdentifier=`, and the first authority is the leaf.
pub fn codesign_report(output: &str) -> CodesignReport {
    let mut report = CodesignReport {
        team: None,
        authorities: Vec::new(),
    };
    for line in output.lines() {
        let line = line.trim();
        if let Some(team) = line.strip_prefix("TeamIdentifier=") {
            let team = team.trim();
            if !team.is_empty() && team != "not set" {
                report.team = Some(team.to_string());
            }
        } else if let Some(authority) = line.strip_prefix("Authority=") {
            report.authorities.push(authority.trim().to_string());
        }
    }
    report
}

/// Whether the report names the release team and a Developer ID Application
/// authority (REQ-UPD-06).
pub fn report_is_release_signed(output: &str) -> bool {
    let report = codesign_report(output);
    report.team.as_deref() == Some(TEAM_ID)
        && report
            .authorities
            .iter()
            .any(|a| a.starts_with("Developer ID Application"))
}

/// Windows handoff (REQ-UPD-08): spawn the installer once, from its staging
/// directory, with no arguments at all, then ask the app to quit.
pub fn hand_off_windows(
    update: &VerifiedUpdate,
    spawn: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<HandoffOutcome, PlatformError> {
    spawn(&update.artifact).map_err(|e| PlatformError::System(e.to_string()))?;
    Ok(HandoffOutcome::QuitRequested)
}

/// macOS handoff (REQ-UPD-09): reveal the extracted app in Finder; the running
/// app keeps running and its own bundle is never touched.
pub fn hand_off_macos<R: CommandRunner + ?Sized>(
    runner: &R,
    update: &VerifiedUpdate,
) -> Result<HandoffOutcome, PlatformError> {
    let bundle = update
        .prepared
        .as_ref()
        .ok_or_else(|| PlatformError::System("no extracted bundle".into()))?;
    run_ok(
        runner,
        OPEN,
        &["-R".into(), bundle.to_string_lossy().into_owned()],
        "open -R",
    )
    .map_err(PlatformError::System)?;
    Ok(HandoffOutcome::Revealed)
}

/// The launch hook: on Windows, remove the staging directories an earlier run
/// left under the OS temp directory. Anywhere else it does nothing, so
/// `~/Downloads` is never touched (REQ-UPD-04).
pub fn cleanup_stale_staging_at_launch() {
    if supported_target().map(|(platform, _)| platform) == Some("windows") {
        let _ = cleanup_stale_staging(&std::env::temp_dir());
    }
}

/// Removes stale `snackpilot-update-*` directories under `temp` and nothing
/// else: no other name, no file, no path outside `temp` (REQ-UPD-04).
pub fn cleanup_stale_staging(temp: &Path) -> io::Result<Vec<PathBuf>> {
    let mut removed = Vec::new();
    for entry in std::fs::read_dir(temp)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name.starts_with(TEMP_STAGING_PREFIX) && path.is_dir() {
            std::fs::remove_dir_all(&path)?;
            removed.push(path);
        }
    }
    Ok(removed)
}

/// The supported host this process runs on, or the typed refusal; never a
/// network read. macOS also needs `$HOME` for `~/Downloads`.
fn supported_platform() -> Result<&'static str, PlatformError> {
    supported_target()
        .map(|(platform, _)| platform)
        .ok_or_else(unsupported)
}

/// macOS staging root: the home directory whose `Downloads` receives the
/// staging directory.
fn macos_home() -> Result<PathBuf, PlatformError> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(unsupported)
}

/// `DesktopPlatform::update_target`: `Some` only on macOS arm64 and Windows
/// x64, never a network read (REQ-UPD-04).
pub fn desktop_update_target() -> Option<UpdateHostInfo> {
    let (platform, architecture) = supported_target()?;
    let os_version = match platform {
        "windows" => {
            let root = std::env::var_os("SystemRoot").map(PathBuf::from);
            windows_os_version(&SystemRunner, root.as_deref())
        }
        _ => None,
    };
    Some(UpdateHostInfo {
        platform,
        architecture,
        os_version,
        installed_version: super::metadata::product_version(),
        installed_build: super::metadata::product_build(),
    })
}

/// `DesktopPlatform::update_staging_dir`.
pub fn desktop_staging_dir(version: &str) -> Result<PathBuf, PlatformError> {
    match supported_platform()? {
        "macos" => macos_staging_dir(&macos_home()?, version),
        _ => {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos());
            windows_staging_dir(&std::env::temp_dir(), version, nonce)
        }
    }
}

/// `DesktopPlatform::verify_update_artifact`.
pub fn desktop_verify_artifact(
    artifact: &Path,
    manifest: &VerifiedManifest,
) -> Result<VerifiedUpdate, PlatformError> {
    match supported_platform()? {
        "macos" => verify_macos_artifact(&SystemRunner, artifact, manifest),
        _ => verify_windows_artifact(artifact, manifest),
    }
}

/// `DesktopPlatform::hand_off_update`: reveal on macOS, spawn-then-quit on
/// Windows.
pub fn desktop_hand_off(update: &VerifiedUpdate) -> Result<HandoffOutcome, PlatformError> {
    match supported_platform()? {
        "macos" => hand_off_macos(&SystemRunner, update),
        _ => hand_off_windows(update, spawn_installer),
    }
}

/// The typed refusal of a host without update support.
fn unsupported() -> PlatformError {
    PlatformError::System("unsupported host".into())
}

/// Starts the installer exactly as it is: no arguments, no shell, no working
/// directory changes, and never elevated (REQ-UPD-08).
fn spawn_installer(installer: &Path) -> io::Result<()> {
    let mut child = Command::new(installer)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Host OS version for the minimum-OS check: `cmd /C ver` parsed to
/// `10.0.<build>`; unparseable means unknown, which does not block.
pub fn windows_os_version<R: CommandRunner + ?Sized>(
    runner: &R,
    system_root: Option<&Path>,
) -> Option<String> {
    let cmd = system_root
        .unwrap_or_else(|| Path::new("C:\\Windows"))
        .join("System32")
        .join("cmd.exe");
    let (ok, output) = runner
        .run(&cmd.to_string_lossy(), &["/C".into(), "ver".into()])
        .ok()?;
    ok.then(|| parse_windows_ver(&output)).flatten()
}

/// `Microsoft Windows [Version 10.0.22631.4317]` becomes `10.0.22631`.
pub fn parse_windows_ver(output: &str) -> Option<String> {
    for token in output.split_whitespace() {
        let token = token.trim_end_matches(']');
        let version = token
            .rsplit_once("[Version ")
            .map_or(token, |(_, rest)| rest);
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() >= 3
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        {
            return Some(format!("{}.{}.{}", parts[0], parts[1], parts[2]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    const VERSION: &str = "2.3.0";
    const ZIP: &str = "SnackPilot-2.3.0-macos-arm64.zip";
    const INSTALLER: &str = "SnackPilot-2.3.0-windows-x64-setup.exe";

    /// `codesign -dv --verbose=4` output of a genuine release bundle.
    const GOOD_REPORT: &str = "Executable=SnackPilot\n\
        Identifier=dev.radaiko.gourmetclient\n\
        Authority=Developer ID Application: Patrick Radschiner (555V4MNLK3)\n\
        Authority=Developer ID Certification Authority\n\
        Authority=Apple Root CA\n\
        TeamIdentifier=555V4MNLK3\n\
        Runtime Version=26.0.0\n";

    fn manifest(platform: &str, architecture: &str, artifact: &str) -> VerifiedManifest {
        VerifiedManifest {
            schema: 1,
            platform: platform.into(),
            architecture: architecture.into(),
            version: VERSION.into(),
            build: "42".into(),
            minimum_os: "26.0".into(),
            artifact: artifact.into(),
            size: 3,
            sha256: "00".repeat(32),
            notes: "Fehler behoben.".into(),
            notes_url: "https://github.com/radaiko/SnackPilot/releases/tag/desktop/v2.3.0".into(),
            tag: "desktop/v2.3.0".into(),
        }
    }

    fn mac_manifest() -> VerifiedManifest {
        manifest("macos", "arm64", ZIP)
    }

    fn unique(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "snackpilot-app-update-{tag}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// One scripted tool answer: exit success and output text.
    type Answer = (bool, String);
    type Calls = Arc<Mutex<Vec<(String, Vec<String>)>>>;

    /// Answers each tool call from a script, in order, and records every call.
    struct StubRunner {
        answers: Mutex<VecDeque<Answer>>,
        calls: Calls,
    }

    impl StubRunner {
        fn new(answers: Vec<Answer>) -> (Self, Calls) {
            let calls = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    answers: Mutex::new(VecDeque::from(answers)),
                    calls: calls.clone(),
                },
                calls,
            )
        }

        fn programs(&self) -> Vec<String> {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .map(|(program, _)| program.clone())
                .collect()
        }
    }

    impl CommandRunner for StubRunner {
        fn run(&self, program: &str, args: &[String]) -> io::Result<(bool, String)> {
            self.calls
                .lock()
                .unwrap()
                .push((program.to_string(), args.to_vec()));
            let answer = self
                .answers
                .lock()
                .unwrap()
                .pop_front()
                .expect("no scripted answer left");
            Ok(answer)
        }
    }

    fn ok(text: &str) -> Answer {
        (true, text.to_string())
    }

    fn failed() -> Answer {
        (false, String::new())
    }

    /// The seven answers of a passing macOS run.
    fn mac_answers(identifier: &str, plist_version: &str, report: &str) -> Vec<Answer> {
        vec![
            ok(""),
            ok(identifier),
            ok(plist_version),
            ok(""),
            ok(report),
            ok("The validate action worked!"),
            ok("accepted"),
        ]
    }

    /// A staging directory holding the ZIP, and the extraction `ditto` leaves
    /// with `apps` `SnackPilot.app` bundles side by side.
    fn staged(tag: &str, apps: usize) -> (PathBuf, PathBuf) {
        let dir = unique(tag);
        let zip = dir.join(ZIP);
        std::fs::write(&zip, b"zip").unwrap();
        let extracted = dir.join("extracted");
        std::fs::create_dir_all(&extracted).unwrap();
        for i in 0..apps {
            let parent = if i == 0 {
                extracted.clone()
            } else {
                extracted.join(format!("copy{i}"))
            };
            std::fs::create_dir_all(parent.join(APP_BUNDLE)).unwrap();
        }
        (dir, zip)
    }

    /// @req REQ-UPD-06 The macOS sequence runs in design D6 order, every tool
    /// called by absolute path: extract, identity, version, verify, report,
    /// staple, assess.
    #[test]
    fn macos_sequence_runs_in_order_with_absolute_paths() {
        let (dir, zip) = staged("order", 1);
        let (runner, calls) = StubRunner::new(mac_answers(BUNDLE_ID, VERSION, GOOD_REPORT));
        let verified = verify_macos_artifact(&runner, &zip, &mac_manifest()).unwrap();

        let bundle = dir.join("extracted").join(APP_BUNDLE);
        assert_eq!(verified.prepared.as_deref(), Some(bundle.as_path()));
        assert_eq!(verified.artifact, zip);
        assert_eq!(verified.version, VERSION);
        assert!(dir.exists(), "a passing run keeps the staging directory");

        assert_eq!(
            runner.programs(),
            [DITTO, PLISTBUDDY, PLISTBUDDY, CODESIGN, CODESIGN, STAPLER, SPCTL]
        );
        let calls = calls.lock().unwrap().clone();
        for (program, _) in &calls {
            assert!(program.starts_with('/'), "{program} is a PATH lookup");
        }
        let path = bundle.to_string_lossy().into_owned();
        assert_eq!(
            calls[0].1,
            [
                "-x",
                "-k",
                zip.to_str().unwrap(),
                dir.join("extracted").to_str().unwrap()
            ]
        );
        assert_eq!(calls[1].1[1], "Print :CFBundleIdentifier");
        assert_eq!(calls[2].1[1], "Print :CFBundleShortVersionString");
        assert_eq!(
            calls[3].1,
            ["--verify", "--deep", "--strict", path.as_str()]
        );
        assert_eq!(calls[4].1, ["-dv", "--verbose=4", path.as_str()]);
        assert_eq!(calls[5].1, ["stapler", "validate", path.as_str()]);
        assert_eq!(calls[6].1, ["--assess", "--type", "execute", path.as_str()]);
    }

    /// @req REQ-UPD-06 A bundle signed by another team fails at the report
    /// step, and the staging directory goes with it.
    #[test]
    fn macos_wrong_team_is_rejected() {
        let report = GOOD_REPORT.replace(TEAM_ID, "ABCDE12345");
        let (dir, zip) = staged("wrong-team", 1);
        let (runner, _calls) = StubRunner::new(mac_answers(BUNDLE_ID, VERSION, &report));
        assert!(verify_macos_artifact(&runner, &zip, &mac_manifest()).is_err());
        assert_eq!(runner.programs().len(), 5, "nothing runs after the report");
        assert!(
            !dir.exists(),
            "a rejected bundle leaves no staging directory"
        );
    }

    /// @req REQ-UPD-06 Without a stapled ticket the sequence stops at
    /// `stapler validate` and nothing stays behind.
    #[test]
    fn macos_missing_staple_is_rejected() {
        let (dir, zip) = staged("no-staple", 1);
        let mut answers = mac_answers(BUNDLE_ID, VERSION, GOOD_REPORT);
        answers[5] = failed();
        let (runner, _calls) = StubRunner::new(answers);
        assert!(verify_macos_artifact(&runner, &zip, &mac_manifest()).is_err());
        assert_eq!(runner.programs().last().map(String::as_str), Some(STAPLER));
        assert!(!dir.exists());
    }

    /// @req REQ-UPD-05 Another bundle identifier or version fails the identity
    /// step, before any signature or trust check runs.
    #[test]
    fn macos_wrong_bundle_id_or_version_is_rejected() {
        let (dir, zip) = staged("identity", 1);
        let (runner, _calls) =
            StubRunner::new(mac_answers("dev.radaiko.other", VERSION, GOOD_REPORT));
        assert!(
            verify_macos_artifact(&runner, &zip, &mac_manifest()).is_err(),
            "another bundle identifier must be rejected"
        );
        assert_eq!(runner.programs(), [DITTO, PLISTBUDDY]);
        assert!(!dir.exists());

        let (dir, zip) = staged("identity-version", 1);
        let (runner, _calls) = StubRunner::new(mac_answers(BUNDLE_ID, "2.2.0", GOOD_REPORT));
        assert!(
            verify_macos_artifact(&runner, &zip, &mac_manifest()).is_err(),
            "another bundle version must be rejected"
        );
        assert_eq!(runner.programs(), [DITTO, PLISTBUDDY, PLISTBUDDY]);
        assert!(!dir.exists());
    }

    /// @req REQ-UPD-05 A failing tool at any step removes the extraction and
    /// the ZIP alike, so nothing installable is left behind.
    #[test]
    fn macos_failure_removes_staging() {
        for failing in 0..7 {
            let (dir, zip) = staged("failure", 1);
            let mut answers = mac_answers(BUNDLE_ID, VERSION, GOOD_REPORT);
            answers[failing] = failed();
            let (runner, _calls) = StubRunner::new(answers);
            assert!(
                verify_macos_artifact(&runner, &zip, &mac_manifest()).is_err(),
                "step {failing} must fail"
            );
            assert!(!zip.exists(), "step {failing}: the ZIP is deleted");
            assert!(
                !dir.exists(),
                "step {failing}: the staging directory is deleted"
            );
        }

        // A runner that cannot start a tool fails the same way.
        struct Broken;
        impl CommandRunner for Broken {
            fn run(&self, _program: &str, _args: &[String]) -> io::Result<(bool, String)> {
                Err(io::Error::new(io::ErrorKind::NotFound, "tool missing"))
            }
        }
        let (dir, zip) = staged("failure-broken", 1);
        assert!(verify_macos_artifact(&Broken, &zip, &mac_manifest()).is_err());
        assert!(!dir.exists());
    }

    /// @req REQ-UPD-05 Exactly one `SnackPilot.app`: none or two fails right
    /// after the extraction.
    #[test]
    fn macos_requires_exactly_one_app_bundle() {
        for apps in [0, 2] {
            let (dir, zip) = staged("count", apps);
            let (runner, _calls) = StubRunner::new(mac_answers(BUNDLE_ID, VERSION, GOOD_REPORT));
            assert!(
                verify_macos_artifact(&runner, &zip, &mac_manifest()).is_err(),
                "{apps} bundles must be rejected"
            );
            assert_eq!(runner.programs(), [DITTO], "only the extraction ran");
            assert!(!dir.exists());
        }

        // One bundle inside a single top-level folder also counts as one.
        let (dir, zip) = staged("nested", 0);
        std::fs::create_dir_all(dir.join("extracted").join("SnackPilot").join(APP_BUNDLE)).unwrap();
        let (runner, _calls) = StubRunner::new(mac_answers(BUNDLE_ID, VERSION, GOOD_REPORT));
        assert!(verify_macos_artifact(&runner, &zip, &mac_manifest()).is_ok());
        assert_eq!(runner.programs().len(), 7);
    }

    /// @req REQ-UPD-06 The `codesign -dv --verbose=4` parser reads the team
    /// and every authority line, and only the release team with a Developer ID
    /// Application authority counts as release-signed.
    #[test]
    fn codesign_output_parser_extracts_team_and_authority() {
        let report = codesign_report(GOOD_REPORT);
        assert_eq!(report.team.as_deref(), Some(TEAM_ID));
        assert_eq!(
            report.authorities,
            [
                "Developer ID Application: Patrick Radschiner (555V4MNLK3)",
                "Developer ID Certification Authority",
                "Apple Root CA"
            ]
        );
        assert!(report_is_release_signed(GOOD_REPORT));

        // Ad hoc: no team, no authority.
        let adhoc =
            "Identifier=dev.radaiko.gourmetclient\nSignature=adhoc\nTeamIdentifier=not set\n";
        assert_eq!(codesign_report(adhoc).team, None);
        assert!(!report_is_release_signed(adhoc));

        // Another team on a Developer ID signature, and this team on an Apple
        // Development signature, both fail.
        assert!(!report_is_release_signed(
            &GOOD_REPORT.replace(TEAM_ID, "ABCDE12345")
        ));
        assert!(!report_is_release_signed(
            &GOOD_REPORT.replace("Developer ID Application:", "Apple Development:")
        ));
        assert!(!report_is_release_signed(""));
    }

    /// @req REQ-UPD-08 The Windows handoff starts the verified installer once,
    /// from its staging directory, and asks for the quit only after it started.
    #[test]
    fn windows_handoff_spawns_installer_with_no_arguments() {
        let dir = unique("win-spawn");
        let installer = dir.join(INSTALLER);
        std::fs::write(&installer, b"exe").unwrap();
        let verified =
            verify_windows_artifact(&installer, &manifest("windows", "x64", INSTALLER)).unwrap();
        assert_eq!(verified.prepared, None, "an installer needs no preparation");

        let spawned = Mutex::new(Vec::new());
        hand_off_windows(&verified, |path| {
            spawned.lock().unwrap().push(path.to_path_buf());
            Ok(())
        })
        .unwrap();
        assert_eq!(*spawned.lock().unwrap(), [installer]);

        // A file that is not the staged installer is never started.
        let other = dir.join("SnackPilot-2.3.0-windows-x64-setup(1).exe");
        std::fs::write(&other, b"exe").unwrap();
        assert!(verify_windows_artifact(&other, &manifest("windows", "x64", INSTALLER)).is_err());
        assert!(!other.exists(), "the invented staging is discarded");
    }

    /// @req REQ-UPD-08 The outcome is `QuitRequested`, and only a started
    /// installer produces it; a failed spawn has no outcome at all.
    #[test]
    fn windows_handoff_quits_after_spawn() {
        let dir = unique("win-quit");
        let installer = dir.join(INSTALLER);
        std::fs::write(&installer, b"exe").unwrap();
        let update = VerifiedUpdate {
            version: VERSION.into(),
            artifact: installer.clone(),
            prepared: None,
        };
        let order = Mutex::new(Vec::new());
        let outcome = hand_off_windows(&update, |path| {
            order
                .lock()
                .unwrap()
                .push(format!("spawn {}", path.display()));
            Ok(())
        })
        .unwrap();
        order.lock().unwrap().push(format!("{outcome:?}"));
        let order = order.lock().unwrap();
        assert_eq!(order[0], format!("spawn {}", installer.display()));
        assert_eq!(
            order[1], "QuitRequested",
            "the quit request follows a started installer, never precedes it"
        );

        let failed = hand_off_windows(&update, |_| {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"))
        });
        assert!(matches!(failed, Err(PlatformError::System(_))));
    }

    /// @req REQ-UPD-07 A failed handoff keeps the verified files so the user
    /// can retry, and a passing one deletes nothing either.
    #[test]
    fn handoff_failure_keeps_verified_files() {
        let dir = unique("keep");
        let installer = dir.join(INSTALLER);
        std::fs::write(&installer, b"exe").unwrap();
        let update = VerifiedUpdate {
            version: VERSION.into(),
            artifact: installer.clone(),
            prepared: None,
        };
        assert!(hand_off_windows(&update, |_| Err(io::Error::other("no"))).is_err());
        assert!(
            installer.exists(),
            "a failed Windows handoff deletes nothing"
        );

        // macOS: `open -R` on the extracted bundle; the app keeps running.
        let (staging, zip) = staged("reveal", 1);
        let bundle = staging.join("extracted").join(APP_BUNDLE);
        let mac = VerifiedUpdate {
            version: VERSION.into(),
            artifact: zip.clone(),
            prepared: Some(bundle.clone()),
        };
        let (runner, calls) = StubRunner::new(vec![ok("")]);
        assert_eq!(
            hand_off_macos(&runner, &mac).unwrap(),
            HandoffOutcome::Revealed
        );
        assert_eq!(
            calls.lock().unwrap().clone(),
            [(
                OPEN.to_string(),
                vec!["-R".into(), bundle.to_string_lossy().into_owned()]
            )]
        );
        assert!(
            bundle.exists() && zip.exists(),
            "the reveal deletes nothing"
        );

        let (runner, _calls) = StubRunner::new(vec![failed()]);
        assert!(matches!(
            hand_off_macos(&runner, &mac),
            Err(PlatformError::System(_))
        ));
        assert!(
            bundle.exists() && zip.exists(),
            "a failed reveal deletes nothing"
        );

        // Without an extracted bundle there is nothing to reveal.
        let unprepared = VerifiedUpdate {
            prepared: None,
            ..mac
        };
        assert!(hand_off_macos(&SystemRunner, &unprepared).is_err());
    }

    /// @req REQ-UPD-04 Only macOS arm64 and Windows x64 are supported hosts;
    /// off them every hook is closed. The Windows OS version parses or is
    /// unknown, which never blocks an offer.
    #[test]
    fn update_target_is_none_off_supported_hosts() {
        let supported = (cfg!(target_os = "macos") && cfg!(target_arch = "aarch64"))
            || (cfg!(target_os = "windows") && cfg!(target_arch = "x86_64"));
        assert_eq!(supported_target().is_some(), supported);
        if !supported {
            assert_eq!(desktop_update_target(), None);
            assert!(desktop_staging_dir(VERSION).is_err());
            assert!(desktop_verify_artifact(Path::new("/nonexistent/x"), &mac_manifest()).is_err());
            assert!(desktop_hand_off(&VerifiedUpdate {
                version: VERSION.into(),
                artifact: PathBuf::from("/nonexistent/x"),
                prepared: None,
            })
            .is_err());
        }

        assert_eq!(
            parse_windows_ver("Microsoft Windows [Version 10.0.22631.4317]"),
            Some("10.0.22631".into())
        );
        assert_eq!(
            parse_windows_ver("Microsoft Windows [Version 6.3.9600]"),
            Some("6.3.9600".into())
        );
        assert_eq!(parse_windows_ver("command not found"), None);
        assert_eq!(parse_windows_ver(""), None);
    }

    /// @req REQ-UPD-04 Staging is a fresh empty directory: macOS under
    /// `~/Downloads/SnackPilot-{version}/` (an earlier attempt's files gone),
    /// Windows a new temp directory per attempt.
    #[test]
    fn staging_dir_is_fresh_and_empty() {
        let home = unique("home");
        let dir = macos_staging_dir(&home, VERSION).unwrap();
        assert_eq!(
            dir,
            home.join("Downloads").join(format!("SnackPilot-{VERSION}"))
        );
        assert!(dir.is_dir());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
        std::fs::write(dir.join("leftover.zip"), b"x").unwrap();
        assert_eq!(macos_staging_dir(&home, VERSION).unwrap(), dir);
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            0,
            "a retry of the same version starts empty"
        );

        let temp = unique("temp");
        let first = windows_staging_dir(&temp, VERSION, 1).unwrap();
        let second = windows_staging_dir(&temp, VERSION, 2).unwrap();
        assert_ne!(first, second, "each attempt gets its own directory");
        for dir in [&first, &second] {
            let name = dir.file_name().unwrap().to_str().unwrap();
            assert!(name.starts_with(TEMP_STAGING_PREFIX), "{name}");
            assert!(name.contains(VERSION));
            assert_eq!(std::fs::read_dir(dir).unwrap().count(), 0);
        }
        assert!(
            windows_staging_dir(&temp, VERSION, 1).is_err(),
            "a taken name is never reused"
        );
    }

    /// @req REQ-UPD-04 Launch cleanup removes `snackpilot-update-*` directories
    /// under the temp directory and nothing else: no other name, no file, no
    /// sibling directory, and nothing under a `~/Downloads` staging root.
    #[test]
    fn stale_staging_cleanup_only_touches_snackpilot_update_dirs() {
        let temp = unique("cleanup");
        let stale = temp.join(format!("{TEMP_STAGING_PREFIX}{VERSION}-1"));
        let stale_old = temp.join(format!("{TEMP_STAGING_PREFIX}earlier"));
        let unrelated = temp.join("downloads");
        let file = temp.join(format!("{TEMP_STAGING_PREFIX}file"));
        for dir in [&stale, &stale_old, &unrelated] {
            std::fs::create_dir_all(dir.join("inner")).unwrap();
        }
        std::fs::write(stale.join("inner").join("zip"), b"x").unwrap();
        std::fs::write(&file, b"x").unwrap();

        let mut removed = cleanup_stale_staging(&temp).unwrap();
        removed.sort();
        let mut expected = vec![stale.clone(), stale_old.clone()];
        expected.sort();
        assert_eq!(removed, expected);
        assert!(!stale.exists() && !stale_old.exists());
        assert!(
            unrelated.join("inner").exists(),
            "an unrelated directory stays"
        );
        assert!(file.exists(), "a file of that name stays");
        assert!(temp.exists(), "the temp directory itself stays");

        // A verified staging in a `~/Downloads` root is never cleanup material:
        // the function only looks in the root it is given.
        let home = unique("cleanup-home");
        let downloads = macos_staging_dir(&home, VERSION).unwrap();
        std::fs::write(downloads.join("staged.zip"), b"x").unwrap();
        cleanup_stale_staging(&unique("cleanup-other")).unwrap();
        assert!(downloads.join("staged.zip").exists());
    }
}
