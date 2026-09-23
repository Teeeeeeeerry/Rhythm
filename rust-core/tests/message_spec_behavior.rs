//! MS-01~11：核心消息规格（manifest: docs/testing/behavior/l10n-keys.md）。
//!
//! 零接缝：纯函数，无 UI 框架依赖，确定性运行。
//! 历史回归：#120（播放失败分类）、#135（分类判断被写进中文分支，英文用户
//! 拿不到分类建议）——分派下沉到核心后，两端两语言不可能再分叉。

use rhythm_core::message::{
    import_batch_result, import_directory_result, import_file_result, playback_failure,
    resolve_failure, MessageLanguage, MessagePlatform, MessageSegment, MessageSpec,
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

// ─── MS-09 目录导入结果分类到文案键 ─────────────────────────────────

#[test]
fn ms09_directory_import_maps_each_count_combination_to_its_key() {
    // 表驱动：「有导入」「全部失败」「没找到文件」三种组合各一行（#375）。
    let cases: &[(i32, i32, &str)] = &[
        (2, 0, "imported_tracks"),  // 有导入
        (0, 3, "import_dir_failed"), // 全部失败
        (0, 0, "import_dir_empty"), // 没找到文件
    ];
    for (imported, failed, expected_key) in cases {
        let spec = import_directory_result(*imported, *failed);
        assert_eq!(
            headline_key(&spec),
            *expected_key,
            "imported={imported} failed={failed} 应当选中 {expected_key}"
        );
    }
}

#[test]
fn ms09_has_import_wins_even_when_some_paths_also_failed() {
    // 有导入优先于失败——不是全有全无。
    let spec = import_directory_result(1, 5);
    assert_eq!(headline_key(&spec), "imported_tracks");
}

#[test]
fn ms09_imported_tracks_carries_count_and_pluralization_params() {
    let spec = import_directory_result(1, 0);
    match &spec.segments[0] {
        MessageSegment::Key { params, .. } => {
            assert_eq!(params.get("count").map(String::as_str), Some("1"));
            assert_eq!(params.get("s").map(String::as_str), Some(""));
        }
        other => panic!("expected a key segment, got {other:?}"),
    }

    let spec = import_directory_result(2, 0);
    match &spec.segments[0] {
        MessageSegment::Key { params, .. } => {
            assert_eq!(params.get("count").map(String::as_str), Some("2"));
            assert_eq!(params.get("s").map(String::as_str), Some("s"));
        }
        other => panic!("expected a key segment, got {other:?}"),
    }
}

#[test]
fn ms09_failed_and_empty_keys_take_no_parameters() {
    for spec in [import_directory_result(0, 4), import_directory_result(0, 0)] {
        for segment in &spec.segments {
            if let MessageSegment::Key { key, params } = segment {
                assert!(params.is_empty(), "{key} 不该带参数");
            }
        }
    }
}

// ─── MS-10 单文件导入结果分类到文案键 ───────────────────────────────

#[test]
fn ms10_file_import_maps_each_count_combination_to_its_key() {
    // 表驱动：「有导入」「格式不支持」「读取失败」三种组合各一行（#377）。
    let cases: &[(i32, i32, &str)] = &[
        (1, 0, "imported_tracks"),          // 有导入
        (0, 1, "import_file_unsupported"),  // 格式不支持
        (0, 0, "import_file_failed"),       // 读取失败
    ];
    for (imported, unsupported, expected_key) in cases {
        let spec = import_file_result(*imported, *unsupported);
        assert_eq!(
            headline_key(&spec),
            *expected_key,
            "imported={imported} unsupported={unsupported} 应当选中 {expected_key}"
        );
    }
}

#[test]
fn ms10_unsupported_and_failed_are_distinct_keys() {
    // 「不支持」与「读取失败」是用户唯一能据以行动的区分，不折成一种失败。
    let unsupported = import_file_result(0, 1);
    let failed = import_file_result(0, 0);
    assert_ne!(headline_key(&unsupported), headline_key(&failed));
}

#[test]
fn ms10_has_import_wins_even_when_the_rest_are_unsupported() {
    let spec = import_file_result(1, 5);
    assert_eq!(headline_key(&spec), "imported_tracks");
}

#[test]
fn ms10_imported_tracks_carries_count_and_pluralization_params() {
    let spec = import_file_result(1, 0);
    match &spec.segments[0] {
        MessageSegment::Key { params, .. } => {
            assert_eq!(params.get("count").map(String::as_str), Some("1"));
            assert_eq!(params.get("s").map(String::as_str), Some(""));
        }
        other => panic!("expected a key segment, got {other:?}"),
    }
}

// ─── MS-11 批量导入结果分类到文案键 ─────────────────────────────────

/// 规格里第一个键段的参数名，按字典序。
fn param_names(spec: &MessageSpec) -> Vec<&str> {
    match &spec.segments[0] {
        MessageSegment::Key { params, .. } => params.keys().map(String::as_str).collect(),
        other => panic!("expected a key segment, got {other:?}"),
    }
}

#[test]
fn ms11_batch_import_maps_each_count_combination_to_its_key() {
    // 表驱动：「全部成功」「部分成功」「全部失败」「没找到支持的文件」四种组合各一行（#379）。
    let cases: &[(i32, i32, &str)] = &[
        (3, 0, "imported_tracks"),    // 全部成功
        (2, 1, "import_some_failed"), // 部分成功
        (0, 2, "import_all_failed"),  // 全部失败
        (0, 0, "import_none_found"),  // 没找到支持的文件
    ];
    for (imported, failed, expected_key) in cases {
        let spec = import_batch_result(*imported, *failed);
        assert_eq!(
            headline_key(&spec),
            *expected_key,
            "imported={imported} failed={failed} 应当选中 {expected_key}"
        );
    }
}

#[test]
fn ms11_partial_success_carries_both_counts_in_one_key() {
    // 成功与失败两个数字出现在同一句话里：一个键段，恰好两个参数。
    let spec = import_batch_result(2, 1);
    assert_eq!(spec.segments.len(), 1);
    assert_eq!(param_names(&spec), ["failed", "imported"]);
    match &spec.segments[0] {
        MessageSegment::Key { params, .. } => {
            assert_eq!(params.get("imported").map(String::as_str), Some("2"));
            assert_eq!(params.get("failed").map(String::as_str), Some("1"));
        }
        other => panic!("expected a key segment, got {other:?}"),
    }
}

#[test]
fn ms11_all_success_carries_count_and_pluralization_params() {
    let spec = import_batch_result(1, 0);
    assert_eq!(param_names(&spec), ["count", "s"]);
    match &spec.segments[0] {
        MessageSegment::Key { params, .. } => {
            assert_eq!(params.get("count").map(String::as_str), Some("1"));
            assert_eq!(params.get("s").map(String::as_str), Some(""));
        }
        other => panic!("expected a key segment, got {other:?}"),
    }
    assert_eq!(param_names(&import_batch_result(3, 0)), ["count", "s"]);
}

#[test]
fn ms11_all_failed_and_none_found_take_no_parameters() {
    for spec in [import_batch_result(0, 2), import_batch_result(0, 0)] {
        assert_eq!(spec.segments.len(), 1);
        assert!(param_names(&spec).is_empty(), "{:?} 不该带参数", spec.segments[0]);
    }
}
