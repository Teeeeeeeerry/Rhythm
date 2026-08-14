//! RS-14/RS-21：yt-dlp 路径失效与安装开关（manifest:
//! docs/testing/behavior/resolver.md）。
//!
//! 独立测试二进制：`YTDLP_PATH` 与解析缓存都是进程全局，这两个场景
//! 需要从空缓存开始，与 resolver_e2e.rs 共享进程会互相粘滞。

use rhythm_core::resolver::{ResolveErrorKind, YTDLP_ENV_OVERRIDE};
use std::path::Path;
use std::sync::Mutex;

mod common;

static PATH_FAILURE_LOCK: Mutex<()> = Mutex::new(());

const FAKE_YTDLP: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fake_ytdlp.sh");

fn unique(url_tag: &str) -> String {
    format!("https://e2e.example.com/watch?v={url_tag}")
}

/// RS-14: a cached binary that stops being spawnable is forgotten and
/// re-discovered; the user gets `YtDlpMissing`, not a crash.
#[test]
fn rs14_spawn_failure_reports_missing_and_rechecks() {
    let _guard = PATH_FAILURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();

    // A private copy of the stub so we can delete it behind the resolver's
    // back after the path has been cached.
    let copy = dir.path().join("fake_ytdlp_copy.sh");
    std::fs::copy(FAKE_YTDLP, &copy).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&copy, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::env::set_var(YTDLP_ENV_OVERRIDE, &copy);
    std::env::set_var("RHYTHM_NO_AUTO_INSTALL", "1");

    // Same guard as rs21: the re-discovery phase walks hardcoded absolute
    // candidate paths, so a dev machine with a real yt-dlp there would
    // find it instead of reporting missing.
    for candidate in [
        "/opt/homebrew/bin/yt-dlp",
        "/usr/local/bin/yt-dlp",
        "/opt/local/bin/yt-dlp",
        "/usr/bin/yt-dlp",
        "/snap/bin/yt-dlp",
    ] {
        if Path::new(candidate).is_file() {
            eprintln!("rs14 skipped: real yt-dlp found at {candidate}");
            return;
        }
    }

    // First resolve succeeds and caches the copy's path.
    rhythm_core::resolver::resolve_url(&unique("success-full-rs14")).unwrap();

    // Delete the binary: the next resolve fails to spawn, reports
    // YtDlpMissing, and forgets the cached path.
    std::fs::remove_file(&copy).unwrap();
    let err = rhythm_core::resolver::resolve_url(&unique("success-full-rs14b")).unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::YtDlpMissing);
    assert!(
        err.message.contains("Failed to start yt-dlp"),
        "message must name the spawn failure"
    );

    // With no usable binary anywhere and auto-install disabled, the next
    // resolve re-discovers and reports the plain missing-binary error.
    // HOME is pointed at a temp dir so the managed copy / ~/bin candidates
    // on a developer machine (which hold a REAL yt-dlp) don't get found.
    std::env::set_var(YTDLP_ENV_OVERRIDE, "");
    let _path_guard = common::EnvGuard::set("PATH", "");
    let _home_guard = common::EnvGuard::set("HOME", dir.path().join("empty-home"));
    let err2 = rhythm_core::resolver::resolve_url(&unique("rs14-recheck")).unwrap_err();
    assert_eq!(err2.kind, ResolveErrorKind::YtDlpMissing);
}

/// RS-21: `RHYTHM_NO_AUTO_INSTALL=1` with no yt-dlp anywhere → the plain
/// missing-binary error, and no provisioning attempt.
#[test]
fn rs21_no_auto_install_yields_missing() {
    let _guard = PATH_FAILURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();

    // Guard: this test assumes the machine has no yt-dlp on the hardcoded
    // candidate paths (see candidate_ytdlp_paths); bail out with a note if
    // that ever stops being true on a dev machine.
    for candidate in [
        "/opt/homebrew/bin/yt-dlp",
        "/usr/local/bin/yt-dlp",
        "/opt/local/bin/yt-dlp",
        "/usr/bin/yt-dlp",
        "/snap/bin/yt-dlp",
    ] {
        if Path::new(candidate).is_file() {
            eprintln!("rs21 skipped: real yt-dlp found at {candidate}");
            return;
        }
    }

    std::env::set_var(YTDLP_ENV_OVERRIDE, "");
    std::env::set_var("RHYTHM_NO_AUTO_INSTALL", "1");
    let _path_guard = common::EnvGuard::set("PATH", "");
    let _home_guard = common::EnvGuard::set("HOME", dir.path().join("empty-home"));

    let err = rhythm_core::resolver::resolve_url(&unique("rs21-missing")).unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::YtDlpMissing);
}
