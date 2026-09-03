//! MS-01~08：核心消息规格（manifest: docs/testing/behavior/l10n-keys.md）。
//!
//! 零接缝：纯函数，无 UI 框架依赖，确定性运行。
//! 历史回归：#120（播放失败分类）、#135（分类判断被写进中文分支，英文用户
//! 拿不到分类建议）——分派下沉到核心后，两端两语言不可能再分叉。

use rhythm_core::message::{
    playback_failure, resolve_failure, MessageLanguage, MessagePlatform, MessageSegment,
    MessageSpec,
};
use rhythm_core::message::resolver_status;
use rhythm_core::resolver::install::InstallStatus;
use rhythm_core::resolver::ResolveErrorKind;
use rhythm_core::HttpErrorKind;

/// 规格里第一个键段的键名（拼装形状的「标题」那一段）。
fn headline_key(spec: &MessageSpec) -> &str {
    spec.segments
        .iter()
        .find_map(|s| match s {
            MessageSegment::Key { key, .. } => Some(key.as_str()),
            _ => None,
        })
        .expect("规格里应当有一个键段")
}

/// 把键渲染成 `<键名>`，看得见拼装形状与占位符填充。
fn render(spec: &MessageSpec) -> String {
    spec.render(|key| format!("<{key}>"))
}

// ─── MS-01 播放失败分类到文案键 ─────────────────────────────────────

#[test]
fn ms01_playback_failure_maps_each_classification_to_its_key() {
    // 表驱动：每一种分类一行，含未识别分类走的回退分支。
    let cases: &[(Option<HttpErrorKind>, &str)] = &[
        (Some(HttpErrorKind::Expired), "playback_failed_expired"),
        (Some(HttpErrorKind::CdnRejected), "playback_failed_cdn_rejected"),
        (Some(HttpErrorKind::Other), "playback_failed_headline"),
        (None, "playback_failed_headline"),
    ];

    for (kind, expected_key) in cases {
        for language in [MessageLanguage::Chinese, MessageLanguage::English] {
            let spec = playback_failure(*kind, "detail", language);
            assert_eq!(
                headline_key(&spec),
                *expected_key,
                "{kind:?} / {language:?} 应当选中 {expected_key}"
            );
        }
    }
}

#[test]
fn ms01_playback_failure_takes_no_parameters() {
    let spec = playback_failure(Some(HttpErrorKind::Expired), "detail", MessageLanguage::English);
    for segment in &spec.segments {
        if let MessageSegment::Key { key, params } = segment {
            assert!(params.is_empty(), "{key} 不该带参数");
        }
    }
}

// ─── MS-02 中英拼装形状 ─────────────────────────────────────────────

#[test]
fn ms02_chinese_puts_the_detail_prefix_between_headline_and_detail() {
    let spec = playback_failure(Some(HttpErrorKind::Expired), "HTTP 403", MessageLanguage::Chinese);
    assert_eq!(
        render(&spec),
        "<playback_failed_expired>\n\n<detail_prefix_zh>\nHTTP 403"
    );
}

#[test]
fn ms02_english_appends_the_detail_directly() {
    let spec = playback_failure(Some(HttpErrorKind::Expired), "HTTP 403", MessageLanguage::English);
    assert_eq!(render(&spec), "<playback_failed_expired>\n\nHTTP 403");
}

#[test]
fn ms02_both_languages_pick_the_same_key() {
    // #135：分类结论不随语言变化。
    for kind in [
        Some(HttpErrorKind::Expired),
        Some(HttpErrorKind::CdnRejected),
        Some(HttpErrorKind::Other),
        None,
    ] {
        let zh = playback_failure(kind, "d", MessageLanguage::Chinese);
        let en = playback_failure(kind, "d", MessageLanguage::English);
        assert_eq!(headline_key(&zh), headline_key(&en), "{kind:?} 的键不该随语言变");
    }
}

// ─── MS-03 详情为空 ─────────────────────────────────────────────────

#[test]
fn ms03_empty_detail_leaves_only_the_headline() {
    for language in [MessageLanguage::Chinese, MessageLanguage::English] {
        let spec = playback_failure(Some(HttpErrorKind::CdnRejected), "", language);
        assert_eq!(spec.segments.len(), 1, "{language:?} 只该剩标题一段");
        assert_eq!(render(&spec), "<playback_failed_cdn_rejected>");
    }
}

// ─── MS-04 解析失败分类到文案键 ─────────────────────────────────────

#[test]
fn ms04_resolve_failure_maps_each_classification_to_its_key() {
    // 表驱动：每一种解析错误分类一行（macOS 键；平台差异见 MS-05）。
    let cases: &[(ResolveErrorKind, &str)] = &[
        (ResolveErrorKind::InvalidUrl, "resolve_error_invalid_url"),
        (ResolveErrorKind::YtDlpMissing, "resolve_error_yt_dlp_missing"),
        (ResolveErrorKind::Timeout, "resolve_error_timeout"),
        (ResolveErrorKind::Network, "resolve_error_network"),
        (ResolveErrorKind::Unavailable, "resolve_error_unavailable"),
        (ResolveErrorKind::NoAudioStream, "resolve_error_no_audio_stream"),
        (ResolveErrorKind::YtDlpOutdated, "resolve_error_yt_dlp_outdated"),
    ];

    for (kind, expected_key) in cases {
        let spec = resolve_failure(
            Some(*kind),
            "engine detail",
            MessageLanguage::Chinese,
            MessagePlatform::MacOs,
        );
        assert_eq!(headline_key(&spec), *expected_key, "{kind:?} 应当选中 {expected_key}");
    }
}

#[test]
fn ms04_unrecognised_classification_falls_back_to_the_engine_detail() {
    // Internal 与未识别分类：引擎原文就是全部已知信息，不猜文案。
    for kind in [Some(ResolveErrorKind::Internal), None] {
        let spec = resolve_failure(
            kind,
            "engine detail",
            MessageLanguage::Chinese,
            MessagePlatform::MacOs,
        );
        assert_eq!(render(&spec), "engine detail", "{kind:?} 应当回退引擎原文");
    }
}

#[test]
fn ms04_english_returns_the_engine_detail_verbatim() {
    // 键表的英文栏对解析失败条目是空的：原文本身就是可行动的信息。
    let spec = resolve_failure(
        Some(ResolveErrorKind::Timeout),
        "engine detail",
        MessageLanguage::English,
        MessagePlatform::MacOs,
    );
    assert_eq!(render(&spec), "engine detail");
}

#[test]
fn ms04_chinese_keeps_the_headline_plus_detail_shape() {
    let spec = resolve_failure(
        Some(ResolveErrorKind::Timeout),
        "engine detail",
        MessageLanguage::Chinese,
        MessagePlatform::MacOs,
    );
    assert_eq!(
        render(&spec),
        "<resolve_error_timeout>\n\n<detail_prefix_zh>\nengine detail"
    );
}

// ─── MS-05 平台差异键 ───────────────────────────────────────────────

#[test]
fn ms05_platform_diff_keys_come_from_the_platform_marker() {
    // 安装命令在两个平台不同：规格带平台标记选键，适配层不再分叉。
    let cases: &[(ResolveErrorKind, &str, &str)] = &[
        (
            ResolveErrorKind::YtDlpMissing,
            "resolve_error_yt_dlp_missing",
            "resolve_error_yt_dlp_missing_windows",
        ),
        (
            ResolveErrorKind::YtDlpOutdated,
            "resolve_error_yt_dlp_outdated",
            "resolve_error_yt_dlp_outdated_windows",
        ),
    ];

    for (kind, mac_key, win_key) in cases {
        let mac = resolve_failure(Some(*kind), "d", MessageLanguage::Chinese, MessagePlatform::MacOs);
        let win = resolve_failure(
            Some(*kind),
            "d",
            MessageLanguage::Chinese,
            MessagePlatform::Windows,
        );
        assert_eq!(headline_key(&mac), *mac_key);
        assert_eq!(headline_key(&win), *win_key);
    }
}

#[test]
fn ms05_platform_marker_does_not_touch_the_shared_keys() {
    // 没有平台差异的分类：两个平台选同一个键。
    for kind in [
        ResolveErrorKind::InvalidUrl,
        ResolveErrorKind::Timeout,
        ResolveErrorKind::Network,
        ResolveErrorKind::Unavailable,
        ResolveErrorKind::NoAudioStream,
    ] {
        let mac = resolve_failure(Some(kind), "d", MessageLanguage::Chinese, MessagePlatform::MacOs);
        let win = resolve_failure(Some(kind), "d", MessageLanguage::Chinese, MessagePlatform::Windows);
        assert_eq!(headline_key(&mac), headline_key(&win), "{kind:?} 不该有平台差异");
    }
}

// ─── MS-06 解析器阶段到文案键 ───────────────────────────────────────

#[test]
fn ms06_each_phase_maps_to_its_key() {
    let cases: &[(InstallStatus, &str)] = &[
        (InstallStatus::Checking, "resolver_status_checking"),
        (InstallStatus::Verifying, "resolver_status_verifying"),
        (InstallStatus::Updating, "resolver_status_updating"),
        (
            InstallStatus::Failed {
                message: "boom".into(),
            },
            "resolver_status_failed",
        ),
        (
            InstallStatus::Downloading {
                received: 1_048_576,
                total: Some(2_097_152),
            },
            "resolver_status_downloading",
        ),
    ];

    for (status, expected_key) in cases {
        let spec = resolver_status(status);
        assert_eq!(headline_key(&spec), *expected_key, "{status:?} 应当选中 {expected_key}");
    }
}

#[test]
fn ms06_quiet_phases_produce_an_empty_spec() {
    // 空闲与就绪没有值得告诉用户的事。
    for status in [InstallStatus::Idle, InstallStatus::Ready] {
        assert!(resolver_status(&status).is_silent(), "{status:?} 应当静默");
    }
}

// ─── MS-07 下载进度的两种形态 ───────────────────────────────────────

#[test]
fn ms07_download_with_total_reports_received_and_total() {
    let spec = resolver_status(&InstallStatus::Downloading {
        received: 1_048_576,
        total: Some(2_097_152),
    });
    assert_eq!(headline_key(&spec), "resolver_status_downloading");
    assert_eq!(
        render(&spec),
        "<resolver_status_downloading>",
        "键段本身不含参数以外的内容"
    );
    match &spec.segments[0] {
        MessageSegment::Key { params, .. } => {
            assert_eq!(params.get("received").map(String::as_str), Some("1.0"));
            assert_eq!(params.get("total").map(String::as_str), Some("2.0"));
        }
        other => panic!("expected a key segment, got {other:?}"),
    }
}

#[test]
fn ms07_download_without_total_reports_only_the_received_size() {
    // 服务端未给出总量：另一个键，只有已收量一个参数。
    for total in [None, Some(0)] {
        let spec = resolver_status(&InstallStatus::Downloading {
            received: 3_145_728,
            total,
        });
        assert_eq!(headline_key(&spec), "resolver_status_downloading_unknown_total");
        match &spec.segments[0] {
            MessageSegment::Key { params, .. } => {
                assert_eq!(params.get("received").map(String::as_str), Some("3.0"));
                assert!(params.get("total").is_none(), "无总量时不该产出总量参数");
            }
            other => panic!("expected a key segment, got {other:?}"),
        }
    }
}

// ─── MS-08 字节到 MB 的换算 ─────────────────────────────────────────

#[test]
fn ms08_byte_to_megabyte_conversion_keeps_one_decimal() {
    let cases: &[(u64, &str)] = &[
        (0, "0.0"),
        (1_048_576, "1.0"),
        (1_572_864, "1.5"),
        (41_943_040, "40.0"),
    ];
    for (bytes, expected) in cases {
        let spec = resolver_status(&InstallStatus::Downloading {
            received: *bytes,
            total: None,
        });
        match &spec.segments[0] {
            MessageSegment::Key { params, .. } => {
                assert_eq!(params.get("received").map(String::as_str), Some(*expected));
            }
            other => panic!("expected a key segment, got {other:?}"),
        }
    }
}
