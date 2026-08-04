use crate::{RhythmError, SourceType, TrackInfo};
use regex::Regex;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Cached URL resolutions to avoid repeated yt-dlp calls.
/// Each entry stores the resolved info and the instant it was cached.
static RESOLVED_CACHE: LazyLock<Mutex<HashMap<String, CachedEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Maximum number of entries in the resolution cache.
const CACHE_MAX_CAPACITY: usize = 256;

/// Time-to-live for a cached resolution result.
const CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour

/// Timeout for the yt-dlp subprocess.
const YTDLP_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for the cheap `yt-dlp --version` probe used during discovery.
const YTDLP_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for asking the user's login shell where yt-dlp lives.
const LOGIN_SHELL_TIMEOUT: Duration = Duration::from_secs(8);

/// Environment variable that pins the yt-dlp binary location, bypassing
/// discovery entirely.
pub const YTDLP_ENV_OVERRIDE: &str = "RHYTHM_YTDLP_PATH";

/// Rotate the resolver log once it grows past this size.
const LOG_MAX_BYTES: u64 = 512 * 1024;

/// The result of resolving a URL.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedUrl {
    pub title: String,
    pub artist: Option<String>,
    pub stream_url: String,
    pub duration: f64,
    pub source_type: SourceType,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedEntry {
    resolved: ResolvedUrl,
    cached_at: Instant,
}

// ─── Failure reporting ──────────────────────────────────────────────

/// Why a URL failed to resolve.
///
/// The UI layers switch on `kind` to show a localized message and fall back
/// to `ResolveFailure::message` (English, with actionable detail) when they
/// don't recognise the variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolveErrorKind {
    /// Empty input, or not an http(s) URL.
    InvalidUrl,
    /// yt-dlp is not installed, or not reachable from this process.
    YtDlpMissing,
    /// yt-dlp did not finish within the timeout.
    Timeout,
    /// yt-dlp reported a network/TLS failure.
    Network,
    /// Private, deleted, geo-blocked, members-only, or sign-in gated.
    Unavailable,
    /// yt-dlp ran but produced no usable audio stream URL.
    NoAudioStream,
    /// yt-dlp is too old for the site's current extractor.
    YtDlpOutdated,
    /// Spawn failures, malformed JSON, and anything else unexpected.
    Internal,
}

/// A resolution failure with a machine-readable kind and a human message.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolveFailure {
    pub kind: ResolveErrorKind,
    pub message: String,
}

impl ResolveFailure {
    fn new(kind: ResolveErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ResolveFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ResolveFailure {}

impl From<ResolveFailure> for RhythmError {
    fn from(failure: ResolveFailure) -> Self {
        match failure.kind {
            ResolveErrorKind::InvalidUrl => RhythmError::InvalidInput(failure.message),
            ResolveErrorKind::Network | ResolveErrorKind::Timeout => {
                RhythmError::Network(failure.message)
            }
            _ => RhythmError::Resolution(failure.message),
        }
    }
}

/// Result alias for resolver operations.
pub type ResolveResult<T> = Result<T, ResolveFailure>;

/// Pattern matchers for known platforms.
///
/// YouTube: handles standard watch, short links (youtu.be), Shorts, Music, and
/// embed URLs with optional playlist / timestamp / other query params.
static YOUTUBE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(https?://)?(www\.|music\.|m\.)?(youtube\.com/watch\?v=|youtu\.be/|youtube\.com/shorts/|youtube\.com/embed/)[\w\-]{6,}"
    )
    .unwrap()
});

/// Bilibili: full video pages (including mobile subdomain) and b23.tv short links.
static BILIBILI_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(https?://)?(www\.|m\.)?bilibili\.com/video/BV[\w]+|b23\.tv/[\w]+").unwrap()
});

/// Direct audio URL: common container extensions, optionally followed by query params.
static DIRECT_AUDIO_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\.(mp3|flac|aac|ogg|opus|m4a|wav|wma|aiff|webm|weba)(\?.*)?$").unwrap()
});

// ─── Cache helpers ──────────────────────────────────────────────────

/// Evict the oldest entry if the cache is over capacity, then remove any
/// entries whose TTL has expired.
fn prune_cache(cache: &mut HashMap<String, CachedEntry>) {
    // Capacity pruning: remove the oldest entry while over the limit.
    while cache.len() > CACHE_MAX_CAPACITY {
        if let Some(oldest_key) = cache
            .iter()
            .min_by_key(|(_, e)| e.cached_at)
            .map(|(k, _)| k.clone())
        {
            cache.remove(&oldest_key);
        } else {
            break;
        }
    }

    // TTL pruning.
    let now = Instant::now();
    cache.retain(|_, entry| now.duration_since(entry.cached_at) < CACHE_TTL);
}

// ─── Subprocess helper ──────────────────────────────────────────────

struct CommandOutput {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

enum RunError {
    Spawn(std::io::Error),
    Timeout,
    Io(std::io::Error),
}

/// Run a command, killing it if it outruns `timeout`.
///
/// Both pipes are drained on their own threads: yt-dlp's `--print-json`
/// payload comfortably exceeds the OS pipe buffer, so a wait-then-read
/// approach would deadlock on a full pipe.
fn run_with_timeout(mut command: Command, timeout: Duration) -> Result<CommandOutput, RunError> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(RunError::Spawn)?;

    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let stdout_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(pipe) = stdout_pipe.as_mut() {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(pipe) = stderr_pipe.as_mut() {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    // Kill the process so it can't linger after we return.
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(RunError::Timeout);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(RunError::Io(e)),
        }
    };

    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();

    Ok(CommandOutput {
        status,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

// ─── yt-dlp discovery ───────────────────────────────────────────────

/// Cached location of the yt-dlp binary.
///
/// Only successful lookups are cached, so installing yt-dlp while Rhythm is
/// running takes effect without a restart.
static YTDLP_PATH: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));

/// Name to hand to the OS PATH lookup.
const YTDLP_BIN: &str = "yt-dlp";

fn env_dir(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_dir())
}

/// Expand `parent/<each subdirectory>/tail...` into existing file paths.
///
/// Used for versioned Python install prefixes (`~/Library/Python/3.12/bin`,
/// `%APPDATA%\Python\Python312\Scripts`) that can't be hardcoded.
fn scan_versioned_dirs(parent: &Path, tail: &[&str]) -> Vec<PathBuf> {
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
fn candidate_ytdlp_paths() -> Vec<PathBuf> {
    let mut candidates = env_override_path().into_iter().collect::<Vec<_>>();

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
fn candidate_ytdlp_paths() -> Vec<PathBuf> {
    let mut candidates = env_override_path().into_iter().collect::<Vec<_>>();

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
fn env_override_path() -> Option<PathBuf> {
    let raw = std::env::var(YTDLP_ENV_OVERRIDE).ok()?;
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
}

/// Does this path run and answer `--version`?
fn probe_ytdlp(path: &Path) -> bool {
    ytdlp_version_at(path).is_some()
}

/// Ask a yt-dlp binary for its version string.
fn ytdlp_version_at(path: &Path) -> Option<String> {
    let mut command = Command::new(path);
    command.arg("--version");
    let output = run_with_timeout(command, YTDLP_PROBE_TIMEOUT).ok()?;
    if !output.status.success() {
        return None;
    }
    let version = output.stdout.trim().to_string();
    (!version.is_empty()).then_some(version)
}

/// Last resort: ask the user's login shell, which knows about pyenv, asdf,
/// conda, and other prefixes we can't enumerate.
#[cfg(not(windows))]
fn ytdlp_from_login_shell() -> Option<PathBuf> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut command = Command::new(&shell);
    command.args(["-lc", "command -v yt-dlp"]);

    let output = run_with_timeout(command, LOGIN_SHELL_TIMEOUT).ok()?;
    if !output.status.success() {
        return None;
    }

    let path = PathBuf::from(output.stdout.trim());
    (path.is_file() && probe_ytdlp(&path)).then_some(path)
}

#[cfg(windows)]
fn ytdlp_from_login_shell() -> Option<PathBuf> {
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

fn discover_ytdlp() -> Option<PathBuf> {
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
fn forget_ytdlp_path() {
    *YTDLP_PATH.lock().unwrap() = None;
}

/// Return a user-friendly error when yt-dlp cannot be found.
fn ytdlp_missing_error() -> ResolveFailure {
    ResolveFailure::new(
        ResolveErrorKind::YtDlpMissing,
        format!(
            "yt-dlp was not found on this system.\n\n\
             Install it to play YouTube / Bilibili links:\n  \
             macOS:   brew install yt-dlp\n  \
             Windows: winget install yt-dlp   or   pip install yt-dlp\n\n\
             Already installed? A GUI app does not inherit your shell's PATH, \
             so point Rhythm at the binary directly by setting {YTDLP_ENV_OVERRIDE} \
             to its full path (find it with: which yt-dlp)."
        ),
    )
}

/// Machine-readable snapshot of the resolver environment, for bug reports.
pub fn diagnostics() -> serde_json::Value {
    let path = ytdlp_path();
    serde_json::json!({
        "ytdlp_path": path.as_ref().map(|p| p.display().to_string()),
        "ytdlp_version": path.as_deref().and_then(ytdlp_version_at),
        "ytdlp_env_override": std::env::var(YTDLP_ENV_OVERRIDE).ok(),
        "path_env": std::env::var("PATH").ok(),
        "log_file": log_file_path().map(|p| p.display().to_string()),
    })
}

// ─── Failure logging ────────────────────────────────────────────────

/// Where resolver failures are appended, per platform convention.
pub fn log_file_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let dir = env_dir("HOME")?.join("Library/Logs/Rhythm");

    #[cfg(windows)]
    let dir = env_dir("LOCALAPPDATA")?.join(r"Rhythm\logs");

    #[cfg(all(not(target_os = "macos"), not(windows)))]
    let dir = match std::env::var_os("XDG_STATE_HOME") {
        Some(state) => PathBuf::from(state).join("rhythm"),
        None => env_dir("HOME")?.join(".local/state/rhythm"),
    };

    Some(dir.join("resolver.log"))
}

/// Append a failure to the resolver log. Best-effort: logging problems never
/// affect the resolution result.
fn log_failure(url: &str, failure: &ResolveFailure, stderr: &str) {
    log::warn!("resolver: {url} failed ({:?}): {}", failure.kind, failure.message);

    let Some(path) = log_file_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }

    // Rotate rather than grow without bound.
    if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > LOG_MAX_BYTES {
        let _ = std::fs::remove_file(&path);
    }

    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    else {
        return;
    };

    let mut entry = format!(
        "[{}] {:?}\n  url: {}\n  message: {}\n",
        format_utc(SystemTime::now()),
        failure.kind,
        url,
        failure.message.replace('\n', "\n    ")
    );
    if !stderr.trim().is_empty() {
        entry.push_str("  yt-dlp stderr:\n");
        for line in stderr.trim().lines() {
            entry.push_str("    ");
            entry.push_str(line);
            entry.push('\n');
        }
    }
    entry.push('\n');

    let _ = file.write_all(entry.as_bytes());
}

/// Format a timestamp as `YYYY-MM-DD HH:MM:SS UTC` without a date-time crate.
fn format_utc(time: SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (year, month, day) = civil_from_days(secs.div_euclid(86_400));
    let time_of_day = secs.rem_euclid(86_400);

    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        time_of_day / 3600,
        (time_of_day % 3600) / 60,
        time_of_day % 60
    )
}

/// Days since the Unix epoch → (year, month, day). Howard Hinnant's
/// `civil_from_days` algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;

    (if month <= 2 { year + 1 } else { year }, month, day)
}

// ─── Public API ─────────────────────────────────────────────────────

/// Detect the type of URL and its source platform.
pub fn classify_url(url: &str) -> ResolveResult<SourceType> {
    // Basic sanity check: must look like an HTTP(S) URL.
    let trimmed = url.trim();
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err(ResolveFailure::new(
            ResolveErrorKind::InvalidUrl,
            "Please enter a valid URL starting with http:// or https://",
        ));
    }

    if YOUTUBE_PATTERN.is_match(trimmed) {
        Ok(SourceType::YouTube)
    } else if BILIBILI_PATTERN.is_match(trimmed) {
        Ok(SourceType::Bilibili)
    } else if DIRECT_AUDIO_PATTERN.is_match(trimmed) {
        Ok(SourceType::DirectUrl)
    } else {
        // Could still be a direct URL without a recognised audio extension.
        // Default to treating it as a yt-dlp target in case it's an
        // unsupported video site that yt-dlp still understands.
        Ok(SourceType::YouTube)
    }
}

/// Resolve a URL to a playable audio stream.
///
/// Uses yt-dlp for YouTube / Bilibili, direct fetch for audio URLs.
/// Results are cached for one hour (up to 256 entries). Failures carry a
/// [`ResolveErrorKind`] so the UI can explain what went wrong, and are
/// appended to the resolver log.
pub fn resolve_url(url: &str) -> ResolveResult<ResolvedUrl> {
    let trimmed = url.trim();

    // Basic sanity check.
    if trimmed.is_empty() {
        return Err(ResolveFailure::new(
            ResolveErrorKind::InvalidUrl,
            "URL is empty",
        ));
    }
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err(ResolveFailure::new(
            ResolveErrorKind::InvalidUrl,
            "Please enter a valid URL starting with http:// or https://",
        ));
    }

    // Check cache (return clone so we don't hold the lock across I/O).
    {
        let mut cache = RESOLVED_CACHE.lock().unwrap();
        prune_cache(&mut cache);
        if let Some(entry) = cache.get(trimmed) {
            if entry.cached_at.elapsed() < CACHE_TTL {
                return Ok(entry.resolved.clone());
            }
        }
    }

    let source_type = classify_url(trimmed)?;

    let resolved = match source_type {
        SourceType::DirectUrl => resolve_direct_url(trimmed)?,
        _ => resolve_with_ytdlp(trimmed, &source_type)?,
    };

    // Insert into cache.
    {
        let mut cache = RESOLVED_CACHE.lock().unwrap();
        cache.insert(
            trimmed.to_string(),
            CachedEntry {
                resolved: resolved.clone(),
                cached_at: Instant::now(),
            },
        );
        // Prune after insert in case we just went over capacity.
        prune_cache(&mut cache);
    }

    Ok(resolved)
}

// ─── Direct URL resolution ──────────────────────────────────────────

/// Simple resolution for direct audio URLs.
fn resolve_direct_url(url: &str) -> ResolveResult<ResolvedUrl> {
    let filename = url
        .rsplit('/')
        .next()
        .and_then(|s| s.split('?').next())
        .unwrap_or("Unknown");

    let title = urlencoding_if_needed(filename).to_string();

    Ok(ResolvedUrl {
        title,
        artist: None,
        stream_url: url.to_string(),
        duration: 0.0, // Unknown until playback starts
        source_type: SourceType::DirectUrl,
        thumbnail_url: None,
    })
}

// ─── yt-dlp resolution ──────────────────────────────────────────────

/// Resolve using the yt-dlp binary with a timeout.
///
/// The binary is located by [`ytdlp_path`] rather than by PATH alone, and the
/// subprocess is killed if it outruns `YTDLP_TIMEOUT`.
fn resolve_with_ytdlp(url: &str, source_type: &SourceType) -> ResolveResult<ResolvedUrl> {
    let binary = match ytdlp_path() {
        Some(path) => path,
        None => {
            let failure = ytdlp_missing_error();
            log_failure(url, &failure, "");
            return Err(failure);
        }
    };

    let mut command = Command::new(&binary);
    command.args([
        "-f",
        "bestaudio/best",
        "--no-playlist",
        "--print-json",
        "--no-download",
        "--ignore-errors",
        "--no-check-certificates",
        url,
    ]);

    let output = match run_with_timeout(command, YTDLP_TIMEOUT) {
        Ok(output) => output,
        Err(RunError::Spawn(e)) => {
            // The cached path went stale (upgrade, uninstall) — drop it so the
            // next attempt re-discovers the binary.
            forget_ytdlp_path();
            let failure = ResolveFailure::new(
                ResolveErrorKind::YtDlpMissing,
                format!(
                    "Failed to start yt-dlp at {}: {e}\n\n{}",
                    binary.display(),
                    ytdlp_missing_error().message
                ),
            );
            log_failure(url, &failure, "");
            return Err(failure);
        }
        Err(RunError::Timeout) => {
            let failure = ResolveFailure::new(
                ResolveErrorKind::Timeout,
                format!(
                    "URL resolution timed out after {} seconds. \
                     Check your network connection and try again.",
                    YTDLP_TIMEOUT.as_secs()
                ),
            );
            log_failure(url, &failure, "");
            return Err(failure);
        }
        Err(RunError::Io(e)) => {
            let failure = ResolveFailure::new(
                ResolveErrorKind::Internal,
                format!("yt-dlp process error: {e}"),
            );
            log_failure(url, &failure, "");
            return Err(failure);
        }
    };

    if !output.status.success() {
        let failure = ytdlp_failure_from_stderr(&output.stderr, output.status);
        log_failure(url, &failure, &output.stderr);
        return Err(failure);
    }

    let trimmed_json = output.stdout.trim();
    if trimmed_json.is_empty() {
        // `--ignore-errors` makes yt-dlp exit 0 with an empty payload for
        // videos it could not touch, so the stderr still holds the reason.
        let failure = if output.stderr.trim().is_empty() {
            ResolveFailure::new(
                ResolveErrorKind::Unavailable,
                "yt-dlp returned no output. The URL may be private, geo-blocked, or deleted.",
            )
        } else {
            ytdlp_failure_from_stderr(&output.stderr, output.status)
        };
        log_failure(url, &failure, &output.stderr);
        return Err(failure);
    }

    let json: serde_json::Value = match serde_json::from_str(trimmed_json) {
        Ok(json) => json,
        Err(e) => {
            let failure = ResolveFailure::new(
                ResolveErrorKind::Internal,
                format!("Failed to parse yt-dlp output (unexpected JSON format): {e}"),
            );
            log_failure(url, &failure, &output.stderr);
            return Err(failure);
        }
    };

    // ── Extract fields with fallback chains ─────────────────────────

    let title = json["title"]
        .as_str()
        .or_else(|| json["fulltitle"].as_str())
        .or_else(|| json["alt_title"].as_str())
        .unwrap_or("Unknown Title")
        .to_string();

    let artist = json["uploader"]
        .as_str()
        .or_else(|| json["channel"].as_str())
        .or_else(|| json["artist"].as_str())
        .or_else(|| json["creator"].as_str())
        .or_else(|| json["uploader_id"].as_str())
        .map(|s| s.to_string());

    // Duration: accept both numeric and string representations.
    let duration = parse_duration_seconds(&json);

    // Stream URL: try several known locations in the yt-dlp JSON schema.
    let stream_url = match extract_stream_url(&json, url) {
        Ok(stream_url) => stream_url,
        Err(failure) => {
            log_failure(url, &failure, &output.stderr);
            return Err(failure);
        }
    };

    let thumbnail_url = json["thumbnail"].as_str().map(|s| s.to_string());

    Ok(ResolvedUrl {
        title,
        artist,
        stream_url,
        duration,
        source_type: source_type.clone(),
        thumbnail_url,
    })
}

/// Turn yt-dlp's stderr into an actionable failure.
fn ytdlp_failure_from_stderr(stderr: &str, status: std::process::ExitStatus) -> ResolveFailure {
    let detail = summarize_stderr(stderr);
    let kind = classify_ytdlp_stderr(stderr);

    let advice = match kind {
        ResolveErrorKind::YtDlpOutdated => {
            "\n\nyt-dlp looks out of date for this site. Update it:\n  \
             brew upgrade yt-dlp   (macOS)\n  \
             pip install -U yt-dlp (pip installs)\n  \
             yt-dlp -U             (standalone builds)"
        }
        ResolveErrorKind::Unavailable => {
            "\n\nThe video may be private, deleted, age-restricted, \
             members-only, or blocked in your region."
        }
        ResolveErrorKind::Network => {
            "\n\nCheck your network connection, proxy, or VPN and try again."
        }
        _ => "",
    };

    let message = if detail.is_empty() {
        format!(
            "yt-dlp exited with status {status} and no error output.{advice}"
        )
    } else {
        format!("yt-dlp failed: {detail}{advice}")
    };

    ResolveFailure::new(kind, message)
}

/// Map yt-dlp's stderr onto a failure kind.
fn classify_ytdlp_stderr(stderr: &str) -> ResolveErrorKind {
    let text = stderr.to_lowercase();

    let matches = |needles: &[&str]| needles.iter().any(|n| text.contains(n));

    // Check "update yt-dlp" style breakage before the generic buckets: those
    // messages often also mention "unable to download".
    if matches(&[
        "nsig extraction failed",
        "signature extraction failed",
        "please report this issue",
        "update to the latest version",
        "yt-dlp is out of date",
        "unable to extract player",
        "unsupported url",
    ]) {
        return ResolveErrorKind::YtDlpOutdated;
    }

    if matches(&[
        "confirm you're not a bot",
        "confirm you are not a bot",
        "private video",
        "video unavailable",
        "this video is unavailable",
        "removed by the uploader",
        "account associated with this video has been terminated",
        "members-only",
        "age-restricted",
        "sign in to view",
        // Covers both "not available in your country" and yt-dlp's
        // "The uploader has not made this video available in your country".
        "available in your country",
        "geo restricted",
        "geo-restricted",
        "blocked it in your country",
        "requires a subscription",
        "login required",
        "cookies",
    ]) {
        return ResolveErrorKind::Unavailable;
    }

    if matches(&[
        "unable to download webpage",
        "urlopen error",
        "temporary failure in name resolution",
        "name or service not known",
        "connection refused",
        "connection reset",
        "connection timed out",
        "network is unreachable",
        "read timed out",
        "ssl",
        "certificate verify failed",
        "proxy",
    ]) {
        return ResolveErrorKind::Network;
    }

    ResolveErrorKind::Internal
}

/// Keep the tail of yt-dlp's stderr — the last lines carry the actual error,
/// and the full text goes to the log file anyway.
fn summarize_stderr(stderr: &str) -> String {
    const MAX_LINES: usize = 4;
    const MAX_CHARS: usize = 600;

    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    let tail = if lines.len() > MAX_LINES {
        &lines[lines.len() - MAX_LINES..]
    } else {
        &lines[..]
    };

    let mut summary = tail.join("\n");
    if summary.chars().count() > MAX_CHARS {
        summary = summary.chars().take(MAX_CHARS).collect::<String>() + "…";
    }
    summary
}

/// Try every known field where yt-dlp may place the audio stream URL.
fn extract_stream_url(json: &serde_json::Value, original_url: &str) -> ResolveResult<String> {
    // 1. Direct "url" field (most common).
    if let Some(u) = json["url"].as_str() {
        return Ok(u.to_string());
    }

    // 2. "requested_formats" array — pick the first entry that has a "url".
    if let Some(formats) = json["requested_formats"].as_array() {
        for fmt in formats {
            if let Some(u) = fmt["url"].as_str() {
                return Ok(u.to_string());
            }
        }
    }

    // 3. "formats" array — sometimes yt-dlp returns a flat formats list
    //    when --print-json is used with certain extractors.
    if let Some(formats) = json["formats"].as_array() {
        // Prefer audio-only formats, then fall back to any.
        for fmt in formats {
            let is_audio = fmt["vcodec"].as_str() == Some("none")
                || fmt["acodec"].as_str().map_or(false, |a| a != "none");
            if is_audio {
                if let Some(u) = fmt["url"].as_str() {
                    return Ok(u.to_string());
                }
            }
        }
        // Fallback: first format with any url.
        for fmt in formats {
            if let Some(u) = fmt["url"].as_str() {
                return Ok(u.to_string());
            }
        }
    }

    // 4. "manifest_url" or "m3u8" HLS playlist — usable for streaming.
    if let Some(u) = json["manifest_url"].as_str() {
        return Ok(u.to_string());
    }

    Err(ResolveFailure::new(
        ResolveErrorKind::NoAudioStream,
        format!("No audio stream URL found in yt-dlp output for: {original_url}"),
    ))
}

/// Parse duration in seconds from yt-dlp JSON.
///
/// yt-dlp can return `duration` as a number, a numeric string, or even
/// a `duration_string` like "3:45".
fn parse_duration_seconds(json: &serde_json::Value) -> f64 {
    // Numeric duration field.
    if let Some(d) = json["duration"].as_f64() {
        if d > 0.0 {
            return d;
        }
    }
    // Duration as a string of digits.
    if let Some(s) = json["duration"].as_str() {
        if let Ok(d) = s.parse::<f64>() {
            if d > 0.0 {
                return d;
            }
        }
    }
    // "duration_string" like "3:45" or "1:02:30".
    if let Some(s) = json["duration_string"].as_str() {
        if let Some(total) = parse_hh_mm_ss(s) {
            return total;
        }
    }
    0.0
}

/// Parse a human-readable duration like "3:45", "1:02:30", or "45" into
/// total seconds.
fn parse_hh_mm_ss(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        1 => parts[0].parse::<f64>().ok(),
        2 => {
            let min: f64 = parts[0].parse().ok()?;
            let sec: f64 = parts[1].parse().ok()?;
            Some(min * 60.0 + sec)
        }
        3 => {
            let hr: f64 = parts[0].parse().ok()?;
            let min: f64 = parts[1].parse().ok()?;
            let sec: f64 = parts[2].parse().ok()?;
            Some(hr * 3600.0 + min * 60.0 + sec)
        }
        _ => None,
    }
}

// ─── Track conversion ────────────────────────────────────────────────

/// Build a TrackInfo from a resolved URL.
pub fn resolved_to_track(resolved: &ResolvedUrl, original_url: &str) -> TrackInfo {
    TrackInfo {
        id: None,
        file_path: None,
        source_type: resolved.source_type.clone(),
        source_url: Some(original_url.to_string()),
        title: resolved.title.clone(),
        artist: resolved.artist.clone(),
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        genre: None,
        year: None,
        duration: resolved.duration,
        format: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_size: None,
        date_added: None,
        last_played: None,
        play_count: 0,
        artwork_path: resolved.thumbnail_url.clone(),
        is_available: true,
    }
}

/// Simple URL decoding helper.
fn urlencoding_if_needed(s: &str) -> String {
    if s.contains('%') {
        // Try to decode percent-encoded strings.
        s.to_string() // Simplified — use `urlencoding` crate in production
    } else {
        s.to_string()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── classify_url ─────────────────────────────────────────────

    #[test]
    fn test_classify_youtube() {
        let result = classify_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert_eq!(result, SourceType::YouTube);

        let result = classify_url("https://youtu.be/dQw4w9WgXcQ").unwrap();
        assert_eq!(result, SourceType::YouTube);

        let result = classify_url("https://youtube.com/shorts/abc123def45").unwrap();
        assert_eq!(result, SourceType::YouTube);

        let result = classify_url("https://music.youtube.com/watch?v=xyz789").unwrap();
        assert_eq!(result, SourceType::YouTube);

        let result = classify_url("https://www.youtube.com/embed/dQw4w9WgXcQ").unwrap();
        assert_eq!(result, SourceType::YouTube);
    }

    #[test]
    fn test_classify_bilibili() {
        let result = classify_url("https://www.bilibili.com/video/BV1GJ411x7h7").unwrap();
        assert_eq!(result, SourceType::Bilibili);

        let result = classify_url("https://m.bilibili.com/video/BV1xx411E7jJ").unwrap();
        assert_eq!(result, SourceType::Bilibili);

        let result = classify_url("https://b23.tv/abc1234").unwrap();
        assert_eq!(result, SourceType::Bilibili);
    }

    #[test]
    fn test_classify_direct_audio() {
        let result = classify_url("https://example.com/music/song.mp3").unwrap();
        assert_eq!(result, SourceType::DirectUrl);

        let result = classify_url("https://cdn.example.com/track.flac?token=abc").unwrap();
        assert_eq!(result, SourceType::DirectUrl);

        let result = classify_url("https://example.com/audio.opus").unwrap();
        assert_eq!(result, SourceType::DirectUrl);
    }

    #[test]
    fn test_classify_rejects_non_http() {
        let err = classify_url("not-a-url").unwrap_err();
        assert_eq!(err.kind, ResolveErrorKind::InvalidUrl);

        let err = classify_url("ftp://example.com/song.mp3").unwrap_err();
        assert_eq!(err.kind, ResolveErrorKind::InvalidUrl);
    }

    #[test]
    fn test_resolve_rejects_empty_and_non_http() {
        assert_eq!(
            resolve_url("   ").unwrap_err().kind,
            ResolveErrorKind::InvalidUrl
        );
        assert_eq!(
            resolve_url("file:///tmp/song.mp3").unwrap_err().kind,
            ResolveErrorKind::InvalidUrl
        );
    }

    // ── Duration parsing ─────────────────────────────────────────

    #[test]
    fn test_parse_hh_mm_ss() {
        assert_eq!(parse_hh_mm_ss("45"), Some(45.0));
        assert_eq!(parse_hh_mm_ss("3:45"), Some(225.0));
        assert_eq!(parse_hh_mm_ss("1:02:30"), Some(3750.0));
        assert_eq!(parse_hh_mm_ss("0:05"), Some(5.0));
        assert_eq!(parse_hh_mm_ss(""), None);
        assert_eq!(parse_hh_mm_ss("abc"), None);
    }

    // ── yt-dlp discovery ─────────────────────────────────────────

    #[test]
    fn test_candidate_paths_include_gui_unreachable_prefixes() {
        let candidates = candidate_ytdlp_paths();
        let as_strings: Vec<String> = candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect();

        // The whole point of #21: a Finder-launched app has none of these on
        // PATH, so they must be probed explicitly.
        #[cfg(not(windows))]
        {
            assert!(as_strings.iter().any(|p| p == "/opt/homebrew/bin/yt-dlp"));
            assert!(as_strings.iter().any(|p| p == "/usr/local/bin/yt-dlp"));
        }
        #[cfg(windows)]
        assert!(as_strings
            .iter()
            .any(|p| p.contains("WindowsApps") || p.contains("chocolatey")));
    }

    #[test]
    fn test_env_override_is_probed_first() {
        // Serialized by the mutex in `env_lock` — see note below.
        let _guard = env_lock().lock().unwrap();

        let previous = std::env::var(YTDLP_ENV_OVERRIDE).ok();
        std::env::set_var(YTDLP_ENV_OVERRIDE, "/custom/prefix/yt-dlp");

        let first = candidate_ytdlp_paths().first().cloned();
        assert_eq!(first, Some(PathBuf::from("/custom/prefix/yt-dlp")));

        match previous {
            Some(value) => std::env::set_var(YTDLP_ENV_OVERRIDE, value),
            None => std::env::remove_var(YTDLP_ENV_OVERRIDE),
        }
    }

    #[test]
    fn test_env_override_ignores_blank_value() {
        let _guard = env_lock().lock().unwrap();

        let previous = std::env::var(YTDLP_ENV_OVERRIDE).ok();
        std::env::set_var(YTDLP_ENV_OVERRIDE, "   ");
        assert_eq!(env_override_path(), None);

        match previous {
            Some(value) => std::env::set_var(YTDLP_ENV_OVERRIDE, value),
            None => std::env::remove_var(YTDLP_ENV_OVERRIDE),
        }
    }

    /// Tests that mutate process environment must not run concurrently.
    fn env_lock() -> &'static Mutex<()> {
        static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
        &LOCK
    }

    // ── Subprocess timeout ───────────────────────────────────────

    #[test]
    #[cfg(not(windows))]
    fn test_run_with_timeout_kills_slow_process() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "sleep 5"]);

        let started = Instant::now();
        let result = run_with_timeout(command, Duration::from_millis(300));

        assert!(matches!(result, Err(RunError::Timeout)));
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "timeout should return promptly, took {:?}",
            started.elapsed()
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn test_run_with_timeout_captures_large_stdout() {
        // Larger than a typical 64 KiB pipe buffer: catches the deadlock that
        // a wait-then-read implementation would hit.
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "head -c 200000 /dev/zero | tr '\\0' 'x'"]);

        let output = run_with_timeout(command, Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("command should finish"));

        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 200_000);
    }

    #[test]
    fn test_run_with_timeout_reports_spawn_failure() {
        let command = Command::new("/nonexistent/rhythm-test-binary");
        assert!(matches!(
            run_with_timeout(command, Duration::from_secs(1)),
            Err(RunError::Spawn(_))
        ));
    }

    // ── stderr classification ────────────────────────────────────

    #[test]
    fn test_classify_stderr_outdated() {
        assert_eq!(
            classify_ytdlp_stderr("ERROR: nsig extraction failed: Some formats may be missing"),
            ResolveErrorKind::YtDlpOutdated
        );
        assert_eq!(
            classify_ytdlp_stderr("ERROR: Unable to extract player response; please report this issue"),
            ResolveErrorKind::YtDlpOutdated
        );
    }

    #[test]
    fn test_classify_stderr_unavailable() {
        assert_eq!(
            classify_ytdlp_stderr("ERROR: Private video. Sign in if you've been granted access"),
            ResolveErrorKind::Unavailable
        );
        assert_eq!(
            classify_ytdlp_stderr(
                "ERROR: Sign in to confirm you're not a bot. Use --cookies-from-browser"
            ),
            ResolveErrorKind::Unavailable
        );
        assert_eq!(
            classify_ytdlp_stderr("ERROR: The uploader has not made this video available in your country"),
            ResolveErrorKind::Unavailable
        );
    }

    #[test]
    fn test_classify_stderr_network() {
        assert_eq!(
            classify_ytdlp_stderr("ERROR: Unable to download webpage: <urlopen error [Errno 8]>"),
            ResolveErrorKind::Network
        );
        assert_eq!(
            classify_ytdlp_stderr("ERROR: certificate verify failed: unable to get local issuer"),
            ResolveErrorKind::Network
        );
    }

    #[test]
    fn test_classify_stderr_unknown_is_internal() {
        assert_eq!(
            classify_ytdlp_stderr("ERROR: something entirely new went wrong"),
            ResolveErrorKind::Internal
        );
    }

    #[test]
    fn test_missing_binary_message_mentions_env_override() {
        let failure = ytdlp_missing_error();
        assert_eq!(failure.kind, ResolveErrorKind::YtDlpMissing);
        assert!(failure.message.contains(YTDLP_ENV_OVERRIDE));
        assert!(failure.message.contains("brew install yt-dlp"));
    }

    #[test]
    fn test_summarize_stderr_keeps_tail() {
        let stderr = (1..=10)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let summary = summarize_stderr(&stderr);

        assert!(summary.starts_with("line 7"));
        assert!(summary.ends_with("line 10"));
        assert_eq!(summary.lines().count(), 4);
    }

    #[test]
    fn test_summarize_stderr_truncates_long_lines() {
        let stderr = "x".repeat(2000);
        let summary = summarize_stderr(&stderr);
        assert!(summary.chars().count() <= 601);
        assert!(summary.ends_with('…'));
    }

    // ── Failure conversion ───────────────────────────────────────

    #[test]
    fn test_failure_maps_to_rhythm_error() {
        let invalid: RhythmError =
            ResolveFailure::new(ResolveErrorKind::InvalidUrl, "bad url").into();
        assert!(matches!(invalid, RhythmError::InvalidInput(_)));

        let timeout: RhythmError =
            ResolveFailure::new(ResolveErrorKind::Timeout, "too slow").into();
        assert!(matches!(timeout, RhythmError::Network(_)));

        let missing: RhythmError =
            ResolveFailure::new(ResolveErrorKind::YtDlpMissing, "not installed").into();
        assert!(matches!(missing, RhythmError::Resolution(_)));
    }

    #[test]
    fn test_failure_serializes_snake_case_kind() {
        let json = serde_json::to_string(&ResolveFailure::new(
            ResolveErrorKind::YtDlpMissing,
            "not installed",
        ))
        .unwrap();
        assert!(json.contains("\"kind\":\"yt_dlp_missing\""));
    }

    // ── Timestamp formatting ─────────────────────────────────────

    #[test]
    fn test_format_utc() {
        let epoch = UNIX_EPOCH;
        assert_eq!(format_utc(epoch), "1970-01-01 00:00:00 UTC");

        // 2026-08-05 03:04:05 UTC
        let t = UNIX_EPOCH + Duration::from_secs(1_785_899_045);
        assert_eq!(format_utc(t), "2026-08-05 03:04:05 UTC");

        // Leap day.
        let leap = UNIX_EPOCH + Duration::from_secs(1_709_164_800);
        assert_eq!(format_utc(leap), "2024-02-29 00:00:00 UTC");
    }
}
