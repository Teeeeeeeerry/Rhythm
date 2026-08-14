//! RS-01–23：Resolver 端到端行为清单（manifest: docs/testing/behavior/resolver.md）。
//!
//! 零网络：stub 可执行脚本（tests/fixtures/fake_ytdlp.sh）经
//! `RHYTHM_YTDLP_PATH` 注入，按 URL 子串吐出预置 JSON/错误，测试覆盖
//! 进程调用→输出解析→缓存→失败落地全链路。所有测试持一把进程级锁
//! 串行执行：环境变量（RHYTHM_YTDLP_PATH / HOME / PATH）、yt-dlp 路径
//! 缓存与解析缓存均为进程全局。

mod common;

use rhythm_core::resolver::{ResolveErrorKind, YTDLP_ENV_OVERRIDE};
use std::path::Path;
use std::sync::Mutex;

static RESOLVER_E2E_LOCK: Mutex<()> = Mutex::new(());

const FAKE_YTDLP: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fake_ytdlp.sh");
const CALL_LOG_ENV: &str = "FAKE_YTDLP_CALL_LOG";

/// Point the resolver at the fake yt-dlp and a fresh call log.
fn setup_stub(dir: &Path) {
    std::env::set_var(YTDLP_ENV_OVERRIDE, FAKE_YTDLP);
    std::env::set_var(CALL_LOG_ENV, dir.join("calls.log"));
    // Never provision a real yt-dlp copy during tests.
    std::env::set_var("RHYTHM_NO_AUTO_INSTALL", "1");
}

fn call_count(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("calls.log"))
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

/// A per-test-unique URL whose stub scenario is dispatched on `url_tag`
/// (the fake yt-dlp matches the tag as a substring anywhere in the URL).
/// Uniqueness also keeps the process-global resolution cache from
/// serving one test's entry to another.
fn unique(url_tag: &str) -> String {
    format!("https://e2e.example.com/watch?v={url_tag}")
}

// ─── RS-01 空串/非 http ─────────────────────────────────────────────

#[test]
fn rs01_resolve_rejects_empty_and_non_http_without_subprocess() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    for input in ["", "   ", "file:///etc/passwd", "not a url", "ftp://x.com/a.mp3"] {
        let err = rhythm_core::resolver::resolve_url(input).unwrap_err();
        assert_eq!(err.kind, ResolveErrorKind::InvalidUrl, "input: {input:?}");
    }
    assert_eq!(call_count(dir.path()), 0, "no subprocess for invalid input");
}

// ─── RS-02/03/04/05 classify 变体 ───────────────────────────────────

#[test]
fn rs02_classify_youtube_variants() {
    use rhythm_core::{resolver::classify_url, SourceType};
    for url in [
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://youtu.be/dQw4w9WgXcQ?t=30",
        "https://youtube.com/shorts/abc123def45",
        "https://www.youtube.com/embed/dQw4w9WgXcQ",
        "https://music.youtube.com/watch?v=xyz789",
        "https://m.youtube.com/watch?v=dQw4w9WgXcQ",
    ] {
        assert_eq!(classify_url(url).unwrap(), SourceType::YouTube, "{url}");
    }
    // No scheme is rejected as non-http (the manifest's "无协议" variant
    // locks to current behavior: classify requires an http(s) prefix).
    assert_eq!(
        classify_url("youtube.com/watch?v=dQw4w9WgXcQ").unwrap_err().kind,
        ResolveErrorKind::InvalidUrl
    );
}

#[test]
fn rs03_classify_bilibili_variants() {
    use rhythm_core::{resolver::classify_url, SourceType};
    for url in [
        "https://www.bilibili.com/video/BV1GJ411x7h7",
        "https://www.bilibili.com/video/BV1GJ411x7h7?p=2",
        "https://m.bilibili.com/video/BV1xx411E7jJ",
        "https://b23.tv/abc1234",
    ] {
        assert_eq!(classify_url(url).unwrap(), SourceType::Bilibili, "{url}");
    }
}

#[test]
fn rs04_classify_direct_audio_with_query_and_m4s() {
    use rhythm_core::{resolver::classify_url, SourceType};
    for url in [
        "https://cdn.example.com/track.mp3",
        "https://cdn.example.com/track.flac?token=abc",
        "https://example.com/audio.opus",
        // DASH segment — must stay DirectUrl, never handed to yt-dlp (#23).
        "https://cdn.example.com/video.m4s?range=0-1000",
    ] {
        assert_eq!(classify_url(url).unwrap(), SourceType::DirectUrl, "{url}");
    }
}

#[test]
fn rs05_classify_unknown_http_falls_back_to_youtube() {
    use rhythm_core::{resolver::classify_url, SourceType};
    assert_eq!(
        classify_url("https://some.unknown.site/video/123").unwrap(),
        SourceType::YouTube,
        "unrecognized http URLs default to yt-dlp handling"
    );
}

// ─── RS-06 直链 resolve ─────────────────────────────────────────────

#[test]
fn rs06_direct_url_resolves_locally() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    let resolved = rhythm_core::resolver::resolve_url("https://example.com/song.mp3?token=x").unwrap();
    assert_eq!(resolved.title, "song.mp3", "title = file name, query stripped");
    assert_eq!(resolved.stream_url, "https://example.com/song.mp3?token=x");
    assert_eq!(resolved.duration, 0.0);
    assert_eq!(call_count(dir.path()), 0, "direct URLs never spawn yt-dlp");
}

// ─── RS-07 字段提取 fallback 链 ─────────────────────────────────────

#[test]
fn rs07_title_artist_duration_fallbacks() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    // Full fields.
    let full = rhythm_core::resolver::resolve_url(&unique("success-full")).unwrap();
    assert_eq!(full.title, "Full Title");
    assert_eq!(full.artist.as_deref(), Some("Full Uploader"));
    assert_eq!(full.duration, 125.0);
    assert_eq!(full.thumbnail_url.as_deref(), Some("https://example.com/t.jpg"));

    // title → fulltitle.
    let alt = rhythm_core::resolver::resolve_url(&unique("success-alt-title")).unwrap();
    assert_eq!(alt.title, "Alt Title");

    // No title anywhere → "Unknown Title".
    let none = rhythm_core::resolver::resolve_url(&unique("success-no-title")).unwrap();
    assert_eq!(none.title, "Unknown Title");

    // artist → channel.
    let ch = rhythm_core::resolver::resolve_url(&unique("success-artist-channel")).unwrap();
    assert_eq!(ch.artist.as_deref(), Some("Channel Name"));

    // artist → creator (no channel).
    let cr = rhythm_core::resolver::resolve_url(&unique("success-artist-creator")).unwrap();
    assert_eq!(cr.artist.as_deref(), Some("Creator Name"));

    // A non-numeric string duration ("3:45") does not parse as f64 and
    // falls through to 0 (locked behavior).
    let ds = rhythm_core::resolver::resolve_url(&unique("success-duration-string")).unwrap();
    assert_eq!(ds.duration, 0.0);

    // A numeric string duration parses like the numeric form.
    let ns = rhythm_core::resolver::resolve_url(&unique("success-duration-numericstring")).unwrap();
    assert_eq!(ns.duration, 125.0);

    // duration_string "1:02:30" → 3750 seconds.
    let dstr = rhythm_core::resolver::resolve_url(&unique("success-duration-durationstring")).unwrap();
    assert_eq!(dstr.duration, 3750.0);
}

// ─── RS-08 stream URL 提取优先级 ────────────────────────────────────

#[test]
fn rs08_stream_url_priority() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    // requested_formats first entry wins when there is no top-level url.
    let req = rhythm_core::resolver::resolve_url(&unique("success-requested-formats-rs08")).unwrap();
    assert_eq!(req.stream_url, "https://cdn.example.com/first.m4a");

    // formats: audio-only preferred.
    let fmts = rhythm_core::resolver::resolve_url(&unique("success-formats-audio")).unwrap();
    assert_eq!(fmts.stream_url, "https://cdn.example.com/audio.m4a");

    // No audio-only format → fall back to the first format with any url.
    let fb = rhythm_core::resolver::resolve_url(&unique("success-formats-fallback")).unwrap();
    assert_eq!(fb.stream_url, "https://cdn.example.com/video.mp4");

    // manifest_url fallback.
    let manifest = rhythm_core::resolver::resolve_url(&unique("success-manifest")).unwrap();
    assert_eq!(manifest.stream_url, "https://cdn.example.com/index.m3u8");

    // Nothing at all → NoAudioStream.
    let err = rhythm_core::resolver::resolve_url(&unique("success-no-stream")).unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::NoAudioStream);
}

// ─── RS-09 headers 提取 ─────────────────────────────────────────────

#[test]
fn rs09_headers_extraction() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    // Top-level headers (Bilibili Referer shape).
    let top = rhythm_core::resolver::resolve_url(&unique("success-full-rs09")).unwrap();
    assert_eq!(top.http_headers.get("Referer").map(String::as_str), Some("https://www.bilibili.com"));
    assert_eq!(top.http_headers.get("User-Agent").map(String::as_str), Some("Mozilla/5.0"));

    // Format-level headers override the top-level set.
    let fmt = rhythm_core::resolver::resolve_url(&unique("success-headers-format-rs09")).unwrap();
    assert_eq!(fmt.http_headers.get("Referer").map(String::as_str), Some("https://format.example.com"));

    // requested_formats entry carries its own headers.
    let req = rhythm_core::resolver::resolve_url(&unique("success-requested-formats-rs08")).unwrap();
    assert_eq!(req.http_headers.get("Referer").map(String::as_str), Some("https://example.com/page"));
}

// ─── RS-10/11 缓存 ──────────────────────────────────────────────────

#[test]
fn rs10_cache_hit_skips_subprocess() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    let url = unique("success-full-rs10");
    let first = rhythm_core::resolver::resolve_url(&url).unwrap();
    assert_eq!(call_count(dir.path()), 1);

    let second = rhythm_core::resolver::resolve_url(&url).unwrap();
    assert_eq!(call_count(dir.path()), 1, "second resolve must hit the cache");
    assert_eq!(second.title, first.title);
    assert_eq!(second.stream_url, first.stream_url);
}

// ─── RS-12/15/25 由已有单测覆盖（RS-38–41/35/43–44），见清单标注 ───

// ─── RS-13 失败写日志 ───────────────────────────────────────────────

#[test]
fn rs13_failure_appends_to_log_and_rotates() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    // Point HOME at the temp dir so resolver.log lands there.
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let _home_guard = common::EnvGuard::set("HOME", &home);

    let url = unique("fail-unavailable");
    let err = rhythm_core::resolver::resolve_url(&url).unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::Unavailable);

    let log = home.join("Library/Logs/Rhythm/resolver.log");
    let content = std::fs::read_to_string(&log).expect("resolver.log must exist");
    assert!(content.contains(&url), "log entry must carry the url");
    assert!(content.contains("Unavailable"), "log entry must carry the kind");

    // Rotation: pre-fill over 512 KiB → the next failure starts a fresh file.
    std::fs::write(&log, vec![b'x'; 513 * 1024]).unwrap();
    let _ = rhythm_core::resolver::resolve_url(&unique("fail-network")).unwrap_err();
    let rotated = std::fs::read_to_string(&log).unwrap();
    assert!(rotated.len() < 513 * 1024, "oversized log must be replaced");
    assert!(rotated.contains("Network"));
}

#[test]
fn rs13_log_io_failure_does_not_affect_result() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    // A file where the log directory must be created → create_dir_all fails
    // → logging is skipped, resolution still reports its failure.
    let blocker = dir.path().join("home");
    std::fs::write(&blocker, b"x").unwrap(); // blocks Library/Logs/Rhythm mkdir
    let _home_guard = common::EnvGuard::set("HOME", &blocker);

    let err = rhythm_core::resolver::resolve_url(&unique("fail-unavailable")).unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::Unavailable);
}

/// RS-12: unknown stderr lands on Internal (the "未知 → Internal"
/// classification, exercised end-to-end through the stub).
#[test]
fn rs12_unknown_stderr_is_internal() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    let err = rhythm_core::resolver::resolve_url(&unique("fail-unknown")).unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::Internal);
}

// ─── RS-16/20 受管副本与 outdated ───────────────────────────────────

/// RS-20: a non-managed copy that reports outdated gets upgrade advice —
/// no auto-update attempt (that path only applies to Rhythm's own copy,
/// which needs a managed install + network; noted in the manifest).
#[test]
fn rs20_non_managed_outdated_reports_with_upgrade_advice() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    let err = rhythm_core::resolver::resolve_url(&unique("fail-outdated")).unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::YtDlpOutdated);
    assert!(
        err.message.contains("brew upgrade yt-dlp"),
        "non-managed copies must get upgrade advice, not an auto-update"
    );
}

// ─── RS-17 resolved_to_track ────────────────────────────────────────

#[test]
fn rs17_resolved_to_track_keeps_page_url_and_thumbnail() {
    let resolved = rhythm_core::resolver::ResolvedUrl {
        title: "Resolved".into(),
        artist: Some("Artist".into()),
        stream_url: "https://cdn.example.com/expiring.m4a".into(),
        duration: 90.0,
        source_type: rhythm_core::SourceType::YouTube,
        thumbnail_url: Some("https://i.example.com/t.jpg".into()),
        http_headers: Default::default(),
    };

    let track = rhythm_core::resolver::resolved_to_track(
        &resolved,
        "https://www.youtube.com/watch?v=original",
    );
    assert_eq!(track.source_url.as_deref(), Some("https://www.youtube.com/watch?v=original"), "page URL, not the CDN link");
    assert_eq!(track.artwork_path.as_deref(), Some("https://i.example.com/t.jpg"));
    assert_eq!(track.title, "Resolved");
    assert_eq!(track.artist.as_deref(), Some("Artist"));
}

// ─── RS-18/19 空输出与非法 JSON ─────────────────────────────────────

#[test]
fn rs18_empty_output_failures() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    // Exit 0, no stdout, no stderr → Unavailable.
    let err = rhythm_core::resolver::resolve_url(&unique("empty-output")).unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::Unavailable);

    // Exit 0, no stdout, stderr carries the reason → classified from stderr.
    let err = rhythm_core::resolver::resolve_url(&unique("empty-with-stderr")).unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::Unavailable);
}

#[test]
fn rs19_invalid_json_is_internal() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    let err = rhythm_core::resolver::resolve_url(&unique("bad-json")).unwrap_err();
    assert_eq!(err.kind, ResolveErrorKind::Internal);
    assert!(err.message.contains("Failed to parse"));
}

// ─── RS-22 RHYTHM_YTDLP_PATH 覆盖：已有单测 RS-33/34 覆盖，见清单 ───

// ─── RS-23 直链标题百分号解码（红测禁用，挂 #80）────────────────────

/// 期望：直链标题按 URL 百分号编码规则解码（`My Song.mp3`）。
/// 现状 `urlencoding_if_needed` 为 no-op，标题保留 `%20`（rhythm#80）。
/// 修复后本测试自动转真断言。
#[test]
#[ignore = "rhythm#80 直链标题不解码百分号编码 — https://github.com/Teeeeeeeerry/Rhythm/issues/80"]
fn rs23_direct_url_title_decodes_percent_encoding() {
    let _guard = RESOLVER_E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    setup_stub(dir.path());

    let resolved = rhythm_core::resolver::resolve_url("https://example.com/My%20Song.mp3").unwrap();
    assert_eq!(resolved.title, "My Song.mp3", "percent-encoded file names must decode");
}
