//! MS-01~03：核心消息规格（manifest: docs/testing/behavior/l10n-keys.md）。
//!
//! 零接缝：纯函数，无 UI 框架依赖，确定性运行。
//! 历史回归：#120（播放失败分类）、#135（分类判断被写进中文分支，英文用户
//! 拿不到分类建议）——分派下沉到核心后，两端两语言不可能再分叉。

use rhythm_core::message::{playback_failure, MessageLanguage, MessageSegment, MessageSpec};
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
