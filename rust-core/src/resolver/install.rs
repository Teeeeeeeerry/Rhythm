//! Automatic yt-dlp provisioning.
//!
//! Rhythm needs yt-dlp to play YouTube / Bilibili links, but a first-time user
//! shouldn't have to install anything by hand. When no yt-dlp is found on the
//! system, the first resolution fetches the official standalone build into the
//! app's own data directory and uses that.
//!
//! The download is the official GitHub release asset, verified against the
//! `SHA2-256SUMS` file published alongside it. Set `RHYTHM_NO_AUTO_INSTALL=1`
//! to opt out and manage yt-dlp yourself.

use super::{ResolveErrorKind, ResolveFailure, ResolveResult};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime};

/// Opt out of automatic provisioning.
pub const NO_AUTO_INSTALL_ENV: &str = "RHYTHM_NO_AUTO_INSTALL";

/// Where the official standalone builds live.
const RELEASE_BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";

/// Checksum manifest published with every release.
const CHECKSUM_FILE: &str = "SHA2-256SUMS";

const CHECKSUM_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

/// How often the managed copy checks for a newer yt-dlp.
const UPDATE_INTERVAL: Duration = Duration::from_secs(7 * 24 * 3600);

/// Progress granularity for the UI: report every 512 KiB.
const PROGRESS_STEP: u64 = 512 * 1024;

// ─── Install status (polled by the UI) ──────────────────────────────

/// What the resolver is doing, so the UI can say more than "resolving…".
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum InstallStatus {
    /// Nothing in flight.
    Idle,
    /// Fetching the checksum manifest.
    Checking,
    /// Downloading the binary.
    Downloading { received: u64, total: Option<u64> },
    /// Hashing and smoke-testing the download.
    Verifying,
    /// Updating an existing managed copy.
    Updating,
    /// A usable yt-dlp is in place.
    Ready,
    /// Provisioning failed; the resolver falls back to an error.
    Failed { message: String },
}

static STATUS: LazyLock<Mutex<InstallStatus>> = LazyLock::new(|| Mutex::new(InstallStatus::Idle));

/// Serializes provisioning so concurrent resolutions don't download twice.
static INSTALL_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn set_status(status: InstallStatus) {
    *STATUS.lock().unwrap() = status;
}

/// Current provisioning status, for the UI.
pub fn status() -> InstallStatus {
    STATUS.lock().unwrap().clone()
}

// ─── Paths ──────────────────────────────────────────────────────────

/// Release asset for this platform.
fn asset_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "yt-dlp_macos"
    }
    #[cfg(windows)]
    {
        "yt-dlp.exe"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "yt-dlp_linux"
    }
}

/// File name of the managed binary.
fn binary_name() -> &'static str {
    if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    }
}

/// Directory Rhythm keeps its own tools in.
pub fn managed_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let base = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support/Rhythm");

    #[cfg(windows)]
    let base = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Rhythm");

    #[cfg(all(unix, not(target_os = "macos")))]
    let base = match std::env::var_os("XDG_DATA_HOME") {
        Some(data) => PathBuf::from(data).join("rhythm"),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".local/share/rhythm"),
    };

    Some(base.join("bin"))
}

/// Path of the managed yt-dlp, whether or not it exists yet.
pub fn managed_ytdlp_path() -> Option<PathBuf> {
    Some(managed_dir()?.join(binary_name()))
}

/// Marker file recording the last update check.
fn update_stamp_path() -> Option<PathBuf> {
    Some(managed_dir()?.join(".last-update-check"))
}

/// Has automatic provisioning been switched off?
pub fn auto_install_disabled() -> bool {
    std::env::var(NO_AUTO_INSTALL_ENV)
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            !(v.is_empty() || v == "0" || v == "false" || v == "no")
        })
        .unwrap_or(false)
}

// ─── Provisioning ───────────────────────────────────────────────────

/// Download and install the official yt-dlp build.
///
/// Callers must hold nothing but the install lock: this blocks for as long as
/// the download takes.
pub fn install(reason_update: bool) -> ResolveResult<PathBuf> {
    let _guard = INSTALL_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let dir = managed_dir().ok_or_else(|| {
        ResolveFailure {
            kind: ResolveErrorKind::Internal,
            message: "Could not determine where to install yt-dlp (no home directory)".to_string(),
        }
    })?;
    let target = dir.join(binary_name());

    std::fs::create_dir_all(&dir).map_err(|e| ResolveFailure {
        kind: ResolveErrorKind::Internal,
        message: format!("Could not create {}: {e}", dir.display()),
    })?;

    set_status(if reason_update {
        InstallStatus::Updating
    } else {
        InstallStatus::Checking
    });

    let result = download_and_verify(&target);

    match &result {
        Ok(_) => {
            set_status(InstallStatus::Ready);
            touch_update_stamp();
            log::info!("resolver: installed yt-dlp at {}", target.display());
        }
        Err(failure) => {
            set_status(InstallStatus::Failed {
                message: failure.message.clone(),
            });
            log::warn!("resolver: yt-dlp install failed: {}", failure.message);
        }
    }

    result
}

fn download_and_verify(target: &Path) -> ResolveResult<PathBuf> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("Rhythm/", env!("CARGO_PKG_VERSION")))
        .timeout(DOWNLOAD_TIMEOUT)
        .build()
        .map_err(|e| network_failure(format!("Could not create HTTP client: {e}")))?;

    // 1. Expected checksum for this platform's asset.
    let expected = fetch_expected_checksum(&client)?;

    // 2. Stream the binary to a temporary file next to the target, hashing as
    //    we go so a corrupt download never lands in place.
    let temp = target.with_extension("download");
    let (actual, size) = stream_to_file(&client, &temp)?;

    set_status(InstallStatus::Verifying);

    if !actual.eq_ignore_ascii_case(&expected) {
        let _ = std::fs::remove_file(&temp);
        return Err(ResolveFailure {
            kind: ResolveErrorKind::Internal,
            message: format!(
                "Downloaded yt-dlp failed checksum verification \
                 (expected {expected}, got {actual}). The download was discarded."
            ),
        });
    }

    // 3. Make it executable before it takes its final name.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o755)).map_err(|e| {
            ResolveFailure {
                kind: ResolveErrorKind::Internal,
                message: format!("Could not mark yt-dlp executable: {e}"),
            }
        })?;
    }

    // Windows refuses to rename onto an existing file.
    let _ = std::fs::remove_file(target);
    std::fs::rename(&temp, target).map_err(|e| ResolveFailure {
        kind: ResolveErrorKind::Internal,
        message: format!("Could not install yt-dlp to {}: {e}", target.display()),
    })?;

    // 4. Smoke-test: a binary that won't report its version is no use.
    //
    //    This also warms the PyInstaller unpack cache, which costs several
    //    seconds exactly once — better to pay it here, inside a step the UI
    //    already labels as "verifying", than on the user's first link.
    let version = ytdlp_version_within(target, super::YTDLP_FIRST_RUN_TIMEOUT)
        .ok_or_else(|| ResolveFailure {
            kind: ResolveErrorKind::Internal,
            message: format!(
                "Installed yt-dlp at {} but it did not run. \
                 Try installing yt-dlp yourself (brew install yt-dlp).",
                target.display()
            ),
        })?;

    log::info!("resolver: yt-dlp {version} ready ({size} bytes)");
    Ok(target.to_path_buf())
}

/// Read the expected SHA-256 for this platform's asset from the release's
/// checksum manifest.
fn fetch_expected_checksum(client: &reqwest::blocking::Client) -> ResolveResult<String> {
    let url = format!("{RELEASE_BASE}/{CHECKSUM_FILE}");
    let response = client
        .get(&url)
        .timeout(CHECKSUM_TIMEOUT)
        .send()
        .map_err(|e| network_failure(format!("Could not reach {url}: {e}")))?;

    if !response.status().is_success() {
        return Err(network_failure(format!(
            "Checksum manifest returned HTTP {}",
            response.status()
        )));
    }

    let body = response
        .text()
        .map_err(|e| network_failure(format!("Could not read checksum manifest: {e}")))?;

    parse_checksum(&body, asset_name()).ok_or_else(|| ResolveFailure {
        kind: ResolveErrorKind::Internal,
        message: format!("Release checksums do not list {}", asset_name()),
    })
}

/// Pull `<sha256>  <name>` out of a SHA2-256SUMS manifest.
fn parse_checksum(manifest: &str, asset: &str) -> Option<String> {
    manifest.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?;
        // Manifests may prefix names with '*' (binary mode).
        let name = name.trim_start_matches('*');
        (name == asset && hash.len() == 64).then(|| hash.to_string())
    })
}

/// Download the asset to `path`, returning its hex SHA-256 and byte count.
fn stream_to_file(
    client: &reqwest::blocking::Client,
    path: &Path,
) -> ResolveResult<(String, u64)> {
    let url = format!("{RELEASE_BASE}/{}", asset_name());
    let mut response = client
        .get(&url)
        .send()
        .map_err(|e| network_failure(format!("Could not download yt-dlp from {url}: {e}")))?;

    if !response.status().is_success() {
        return Err(network_failure(format!(
            "Download of {url} returned HTTP {}",
            response.status()
        )));
    }

    let total = response.content_length();
    set_status(InstallStatus::Downloading { received: 0, total });

    let mut file = std::fs::File::create(path).map_err(|e| ResolveFailure {
        kind: ResolveErrorKind::Internal,
        message: format!("Could not write to {}: {e}", path.display()),
    })?;

    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut received: u64 = 0;
    let mut last_reported: u64 = 0;

    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|e| network_failure(format!("Download interrupted: {e}")))?;
        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read]).map_err(|e| ResolveFailure {
            kind: ResolveErrorKind::Internal,
            message: format!("Could not write to {}: {e}", path.display()),
        })?;

        received += read as u64;
        if received - last_reported >= PROGRESS_STEP {
            last_reported = received;
            set_status(InstallStatus::Downloading { received, total });
        }
    }

    file.flush().map_err(|e| ResolveFailure {
        kind: ResolveErrorKind::Internal,
        message: format!("Could not flush {}: {e}", path.display()),
    })?;

    Ok((hex(hasher.finalize().as_slice()), received))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn network_failure(message: String) -> ResolveFailure {
    ResolveFailure {
        kind: ResolveErrorKind::Network,
        message: format!(
            "{message}\n\nRhythm could not download yt-dlp automatically. \
             Check your network, or install it yourself:\n  \
             macOS:   brew install yt-dlp\n  \
             Windows: winget install yt-dlp"
        ),
    }
}

// ─── Updating ───────────────────────────────────────────────────────

fn touch_update_stamp() {
    let Some(path) = update_stamp_path() else {
        return;
    };
    let _ = std::fs::write(&path, format!("{:?}", SystemTime::now()));
}

/// Has it been long enough to look for a newer yt-dlp?
fn update_check_due() -> bool {
    let Some(path) = update_stamp_path() else {
        return false;
    };
    match std::fs::metadata(&path).and_then(|m| m.modified()) {
        Ok(modified) => modified
            .elapsed()
            .map(|age| age >= UPDATE_INTERVAL)
            .unwrap_or(true),
        // No stamp yet: treat as due so a copy installed long ago refreshes.
        Err(_) => true,
    }
}

/// Refresh the managed copy in the background if it hasn't been checked in a
/// while. yt-dlp breaks whenever a site changes, so a stale copy is a
/// liability — but the user should never wait for this.
pub fn maybe_update_in_background(binary: &Path) {
    let Some(managed) = managed_ytdlp_path() else {
        return;
    };
    // Only touch copies we own — never a Homebrew or system install.
    if binary != managed || !update_check_due() {
        return;
    }

    std::thread::spawn(move || {
        // Re-check inside the thread: another resolution may have just run.
        if !update_check_due() {
            return;
        }
        log::info!("resolver: checking for a newer yt-dlp");
        let _ = install(true);
    });
}

/// Update the managed copy now, e.g. after a site rejected the current
/// version. Returns the binary path when the update succeeded.
pub fn update_now() -> ResolveResult<PathBuf> {
    install(true)
}

// ─── Tests ──────────────────────────────────────────────────────────

// ─── yt-dlp discovery (ticket #187 split: moved from resolver/mod.rs) ──
// ─── yt-dlp discovery ───────────────────────────────────────────────

/// Cached location of the yt-dlp binary.
///
/// Only successful lookups are cached, so installing yt-dlp while Rhythm is
/// running takes effect without a restart.
static YTDLP_PATH: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));

/// Name to hand to the OS PATH lookup.
const YTDLP_BIN: &str = "yt-dlp";

pub fn env_dir(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_dir())
}

/// Expand `parent/<each subdirectory>/tail...` into existing file paths.
///
/// Used for versioned Python install prefixes (`~/Library/Python/3.12/bin`,
/// `%APPDATA%\Python\Python312\Scripts`) that can't be hardcoded.
pub fn scan_versioned_dirs(parent: &Path, tail: &[&str]) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let mut candidate = entry.path();
        for part in tail {
            candidate.push(part);
        }
        if candidate.is_file() {
            found.push(candidate);
        }
    }
    found
}

/// Absolute locations to probe before falling back to a PATH lookup.
///
/// A GUI app launched from Finder/Dock inherits launchd's minimal PATH
/// (`/usr/bin:/bin:/usr/sbin:/sbin`), which excludes Homebrew, MacPorts, and
/// pip prefixes — so `Command::new("yt-dlp")` fails even when the very same
/// binary resolves fine in a terminal. (#21)
#[cfg(not(windows))]
pub fn candidate_ytdlp_paths() -> Vec<PathBuf> {
    let mut candidates = env_override_path().into_iter().collect::<Vec<_>>();

    // Rhythm's own copy, provisioned on first use.
    candidates.extend(managed_ytdlp_path());

    candidates.extend(
        [
            "/opt/homebrew/bin/yt-dlp", // Homebrew on Apple Silicon
            "/usr/local/bin/yt-dlp",    // Homebrew on Intel, manual installs
            "/opt/local/bin/yt-dlp",    // MacPorts
            "/usr/bin/yt-dlp",          // distro packages
            "/snap/bin/yt-dlp",
        ]
        .iter()
        .map(PathBuf::from),
    );

    if let Some(home) = env_dir("HOME") {
        candidates.push(home.join(".local/bin/yt-dlp"));
        candidates.push(home.join("bin/yt-dlp"));
        // pip --user on macOS: ~/Library/Python/<version>/bin/yt-dlp
        candidates.extend(scan_versioned_dirs(
            &home.join("Library/Python"),
            &["bin", "yt-dlp"],
        ));
    }

    candidates
}

#[cfg(windows)]
pub fn candidate_ytdlp_paths() -> Vec<PathBuf> {
    let mut candidates = env_override_path().into_iter().collect::<Vec<_>>();

    // Rhythm's own copy, provisioned on first use.
    candidates.extend(managed_ytdlp_path());

    if let Some(local) = env_dir("LOCALAPPDATA") {
        // winget / Microsoft Store shim
        candidates.push(local.join(r"Microsoft\WindowsApps\yt-dlp.exe"));
        candidates.push(local.join(r"yt-dlp\yt-dlp.exe"));
        // pip on a per-user Python: %LOCALAPPDATA%\Programs\Python\Python3xx\Scripts
        candidates.extend(scan_versioned_dirs(
            &local.join(r"Programs\Python"),
            &["Scripts", "yt-dlp.exe"],
        ));
    }

    if let Some(roaming) = env_dir("APPDATA") {
        // pip --user: %APPDATA%\Python\Python3xx\Scripts
        candidates.extend(scan_versioned_dirs(
            &roaming.join("Python"),
            &["Scripts", "yt-dlp.exe"],
        ));
    }

    if let Some(profile) = env_dir("USERPROFILE") {
        candidates.push(profile.join(r"scoop\shims\yt-dlp.exe"));
    }

    candidates.push(PathBuf::from(r"C:\ProgramData\chocolatey\bin\yt-dlp.exe"));

    if let Some(program_files) = env_dir("ProgramFiles") {
        candidates.push(program_files.join(r"yt-dlp\yt-dlp.exe"));
    }

    candidates
}

/// Honour `RHYTHM_YTDLP_PATH` when it points at something.
pub fn env_override_path() -> Option<PathBuf> {
    let raw = std::env::var(super::YTDLP_ENV_OVERRIDE).ok()?;
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
}

/// Does this path run and answer `--version`?
pub fn probe_ytdlp(path: &Path) -> bool {
    ytdlp_version_at(path).is_some()
}

/// Ask a yt-dlp binary for its version string.
pub fn ytdlp_version_at(path: &Path) -> Option<String> {
    ytdlp_version_within(path, super::YTDLP_PROBE_TIMEOUT)
}

/// Ask a yt-dlp binary for its version, allowing a caller-chosen budget.
pub(crate) fn ytdlp_version_within(path: &Path, timeout: Duration) -> Option<String> {
    let mut command = std::process::Command::new(path);
    command.arg("--version");
    let output = super::run_with_timeout(command, timeout).ok()?;
    if !output.status.success() {
        return None;
    }
    let version = output.stdout.trim().to_string();
    (!version.is_empty()).then_some(version)
}

/// Last resort: ask the user's login shell, which knows about pyenv, asdf,
/// conda, and other prefixes we can't enumerate.
#[cfg(not(windows))]
pub fn ytdlp_from_login_shell() -> Option<PathBuf> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut command = std::process::Command::new(&shell);
    command.args(["-lc", "command -v yt-dlp"]);

    let output = super::run_with_timeout(command, super::LOGIN_SHELL_TIMEOUT).ok()?;
    if !output.status.success() {
        return None;
    }

    let path = PathBuf::from(output.stdout.trim());
    (path.is_file() && probe_ytdlp(&path)).then_some(path)
}

#[cfg(windows)]
pub fn ytdlp_from_login_shell() -> Option<PathBuf> {
    None
}

/// Locate the yt-dlp binary, caching the result for subsequent resolutions.
pub fn ytdlp_path() -> Option<PathBuf> {
    if let Some(cached) = YTDLP_PATH.lock().unwrap().clone() {
        return Some(cached);
    }

    let found = discover_ytdlp()?;
    log::info!("resolver: using yt-dlp at {}", found.display());
    *YTDLP_PATH.lock().unwrap() = Some(found.clone());
    Some(found)
}

pub fn discover_ytdlp() -> Option<PathBuf> {
    for candidate in candidate_ytdlp_paths() {
        if candidate.is_file() && probe_ytdlp(&candidate) {
            return Some(candidate);
        }
    }

    // PATH lookup — succeeds when Rhythm was started from a shell.
    if probe_ytdlp(Path::new(YTDLP_BIN)) {
        return Some(PathBuf::from(YTDLP_BIN));
    }

    ytdlp_from_login_shell()
}

/// Forget the cached yt-dlp location so the next call re-discovers it.
pub fn forget_ytdlp_path() {
    *YTDLP_PATH.lock().unwrap() = None;
}

/// Locate yt-dlp, provisioning Rhythm's own copy when the system has none.
///
/// A first-time user should be able to paste a link and have it play, so a
/// missing yt-dlp is something to fix rather than report — unless the user
/// opted out with `RHYTHM_NO_AUTO_INSTALL`.
pub fn ensure_ytdlp() -> ResolveResult<PathBuf> {
    if let Some(path) = ytdlp_path() {
        maybe_update_in_background(&path);
        return Ok(path);
    }

    if auto_install_disabled() {
        return Err(ytdlp_missing_error());
    }

    let installed = install(false)?;
    *YTDLP_PATH.lock().unwrap() = Some(installed.clone());
    Ok(installed)
}

/// Is this the copy Rhythm installed itself?
pub fn is_managed(binary: &Path) -> bool {
    managed_ytdlp_path()
        .map(|managed| managed == binary)
        .unwrap_or(false)
}

/// Return a user-friendly error when yt-dlp cannot be found.
pub fn ytdlp_missing_error() -> ResolveFailure {
    ResolveFailure::new(
        ResolveErrorKind::YtDlpMissing,
        format!(
            "yt-dlp was not found on this system.\n\n\
             Install it to play YouTube / Bilibili links:\n  \
             macOS:   brew install yt-dlp\n  \
             Windows: winget install yt-dlp   or   pip install yt-dlp\n\n\
             Already installed? A GUI app does not inherit your shell's PATH, \
             so point Rhythm at the binary directly by setting {YTDLP_ENV_OVERRIDE} \
             to its full path (find it with: which yt-dlp).",
            YTDLP_ENV_OVERRIDE = super::YTDLP_ENV_OVERRIDE,
        ),
    )
}

/// Machine-readable snapshot of the resolver environment, for bug reports.
pub fn diagnostics() -> serde_json::Value {
    let path = ytdlp_path();
    serde_json::json!({
        "ytdlp_path": path.as_ref().map(|p| p.display().to_string()),
        "ytdlp_version": path.as_deref().and_then(ytdlp_version_at),
        "ytdlp_env_override": std::env::var(super::YTDLP_ENV_OVERRIDE).ok(),
        "path_env": std::env::var("PATH").ok(),
        "log_file": super::log_file_path().map(|p| p.display().to_string()),
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_checksum_picks_matching_asset() {
        let manifest = "\
aaaa000000000000000000000000000000000000000000000000000000000001  yt-dlp
bbbb000000000000000000000000000000000000000000000000000000000002  yt-dlp.exe
cccc000000000000000000000000000000000000000000000000000000000003  yt-dlp_macos
dddd000000000000000000000000000000000000000000000000000000000004  yt-dlp_linux
";
        assert_eq!(
            parse_checksum(manifest, "yt-dlp_macos").as_deref(),
            Some("cccc000000000000000000000000000000000000000000000000000000000003")
        );
        assert_eq!(
            parse_checksum(manifest, "yt-dlp.exe").as_deref(),
            Some("bbbb000000000000000000000000000000000000000000000000000000000002")
        );
        assert_eq!(parse_checksum(manifest, "yt-dlp_absent"), None);
    }

    #[test]
    fn test_parse_checksum_handles_binary_mode_prefix() {
        let manifest =
            "eeee000000000000000000000000000000000000000000000000000000000005 *yt-dlp_macos\n";
        assert!(parse_checksum(manifest, "yt-dlp_macos").is_some());
    }

    #[test]
    fn test_parse_checksum_rejects_malformed_hash() {
        let manifest = "notahash  yt-dlp_macos\n";
        assert_eq!(parse_checksum(manifest, "yt-dlp_macos"), None);
    }

    #[test]
    fn test_asset_matches_platform() {
        #[cfg(target_os = "macos")]
        assert_eq!(asset_name(), "yt-dlp_macos");
        #[cfg(windows)]
        assert_eq!(asset_name(), "yt-dlp.exe");
    }

    #[test]
    fn test_managed_path_is_under_app_data() {
        let path = managed_ytdlp_path().expect("home directory");
        assert!(path.ends_with(binary_name()));
        #[cfg(target_os = "macos")]
        assert!(path
            .to_string_lossy()
            .contains("Library/Application Support/Rhythm/bin"));
    }

    #[test]
    fn test_auto_install_opt_out() {
        let previous = std::env::var(NO_AUTO_INSTALL_ENV).ok();

        std::env::set_var(NO_AUTO_INSTALL_ENV, "1");
        assert!(auto_install_disabled());

        std::env::set_var(NO_AUTO_INSTALL_ENV, "0");
        assert!(!auto_install_disabled());

        std::env::remove_var(NO_AUTO_INSTALL_ENV);
        assert!(!auto_install_disabled());

        if let Some(value) = previous {
            std::env::set_var(NO_AUTO_INSTALL_ENV, value);
        }
    }

    #[test]
    fn test_status_serializes_with_phase_tag() {
        let json = serde_json::to_string(&InstallStatus::Downloading {
            received: 1024,
            total: Some(2048),
        })
        .unwrap();
        assert!(json.contains("\"phase\":\"downloading\""));
        assert!(json.contains("\"received\":1024"));

        let idle = serde_json::to_string(&InstallStatus::Idle).unwrap();
        assert_eq!(idle, "{\"phase\":\"idle\"}");
    }

    #[test]
    fn test_hex_encoding() {
        assert_eq!(hex(&[0x00, 0x0f, 0xff]), "000fff");
    }
}
