//! 消息规格：核心把具名分类翻译成「文案键 + 参数」（#216 组，ticket #227）。
//!
//! 分类到文案键这一跳过去在 macOS 与 Windows 各写一份，两份已经漂了
//! （#135 就是这个形状）。本模块把它收进核心，与分类本身放在一起：
//! 输入是核心已有的具名枚举，输出是一条 [`MessageSpec`]——按顺序拼接的
//! 若干段，每段要么是键表里的一条文案（可带占位符参数），要么是原样
//! 输出的字面量（引擎原文、分隔符）。
//!
//! 双端适配层因此只剩两件事：按键取模板、按参数填占位符。选哪个键、
//! 中英各拼成什么形状，全部在这里决定。语言解析（各平台的系统语言与
//! 手动覆盖机制）仍是平台特异的，由调用方解析后作为 [`MessageLanguage`]
//! 传入。

use std::collections::BTreeMap;

use crate::HttpErrorKind;

/// 渲染文案时的语言。核心据此决定拼装形状（中文多一段「详细信息：」前缀）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageLanguage {
    Chinese,
    English,
}

impl MessageLanguage {
    /// 由平台适配层解析出的语言标识（"zh" / "zh-Hans" / "en" …）判定。
    /// 非中文一律按英文处理——键表只有中英两栏。
    pub fn from_code(code: &str) -> Self {
        if code.starts_with("zh") {
            Self::Chinese
        } else {
            Self::English
        }
    }
}

/// 消息规格的一段。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "segment", rename_all = "snake_case")]
pub enum MessageSegment {
    /// 键表中的一条文案；`params` 是它的 `{占位符}` 取值。
    Key {
        key: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        params: BTreeMap<String, String>,
    },
    /// 原样输出的字面量（引擎原文、换行分隔符）。
    Literal { text: String },
}

impl MessageSegment {
    /// 无参数的键段。
    pub fn key(key: &str) -> Self {
        Self::Key {
            key: key.to_string(),
            params: BTreeMap::new(),
        }
    }

    /// 带参数的键段。
    pub fn key_with(key: &str, params: &[(&str, &str)]) -> Self {
        Self::Key {
            key: key.to_string(),
            params: params
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        }
    }

    /// 字面量段。
    pub fn literal(text: &str) -> Self {
        Self::Literal {
            text: text.to_string(),
        }
    }
}

/// 一条消息规格。段为空表示这一状态不显示任何文案（静默）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MessageSpec {
    pub segments: Vec<MessageSegment>,
}

impl MessageSpec {
    fn new(segments: Vec<MessageSegment>) -> Self {
        Self { segments }
    }

    /// 不显示任何文案。
    pub fn silent() -> Self {
        Self::new(Vec::new())
    }

    /// 是否为静默规格。
    pub fn is_silent(&self) -> bool {
        self.segments.is_empty()
    }

    /// 用给定的模板取值函数渲染成最终字符串。双端适配层做的正是这件事，
    /// 核心测试用它锁定拼装形状。
    pub fn render(&self, template: impl Fn(&str) -> String) -> String {
        let mut out = String::new();
        for segment in &self.segments {
            match segment {
                MessageSegment::Key { key, params } => {
                    let mut text = template(key);
                    for (name, value) in params {
                        text = text.replace(&format!("{{{name}}}"), value);
                    }
                    out.push_str(&text);
                }
                MessageSegment::Literal { text } => out.push_str(text),
            }
        }
        out
    }
}

/// 「标题 + 引擎详情」这一拼装形状：中文在详情前多一段「详细信息：」，
/// 英文直接接详情；详情为空时只留标题。
fn headline_with_detail(
    headline: MessageSegment,
    detail: &str,
    language: MessageLanguage,
) -> MessageSpec {
    if detail.is_empty() {
        return MessageSpec::new(vec![headline]);
    }
    match language {
        MessageLanguage::Chinese => MessageSpec::new(vec![
            headline,
            MessageSegment::literal("\n\n"),
            MessageSegment::key("detail_prefix_zh"),
            MessageSegment::literal("\n"),
            MessageSegment::literal(detail),
        ]),
        MessageLanguage::English => MessageSpec::new(vec![
            headline,
            MessageSegment::literal("\n\n"),
            MessageSegment::literal(detail),
        ]),
    }
}

/// 播放失败的文案规格（#120 分类）。
///
/// `kind` 是核心对 HTTP 失败的分类：链接真过期（`Expired`）保留「重新
/// 粘贴」建议；CDN 拒绝了仍然有效的链接（`CdnRejected`）给出换网络的
/// 建议——重贴在那里毫无意义。其它分类与非 HTTP 失败（`None`）走泛化
/// 提示。
pub fn playback_failure(
    kind: Option<HttpErrorKind>,
    detail: &str,
    language: MessageLanguage,
) -> MessageSpec {
    let headline = match kind {
        Some(HttpErrorKind::Expired) => MessageSegment::key("playback_failed_expired"),
        Some(HttpErrorKind::CdnRejected) => MessageSegment::key("playback_failed_cdn_rejected"),
        _ => MessageSegment::key("playback_failed_headline"),
    };
    headline_with_detail(headline, detail, language)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_from_code_treats_non_chinese_as_english() {
        assert_eq!(MessageLanguage::from_code("zh-Hans"), MessageLanguage::Chinese);
        assert_eq!(MessageLanguage::from_code("zh"), MessageLanguage::Chinese);
        assert_eq!(MessageLanguage::from_code("en-US"), MessageLanguage::English);
        assert_eq!(MessageLanguage::from_code(""), MessageLanguage::English);
    }

    #[test]
    fn render_fills_placeholders_and_keeps_literals() {
        let spec = MessageSpec::new(vec![
            MessageSegment::key_with("t", &[("a", "1")]),
            MessageSegment::literal("|"),
        ]);
        assert_eq!(spec.render(|k| format!("<{k}:{{a}}>")), "<t:1>|");
    }

    #[test]
    fn spec_round_trips_through_json() {
        let spec = playback_failure(Some(HttpErrorKind::Expired), "boom", MessageLanguage::Chinese);
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<MessageSpec>(&json).unwrap(), spec);
    }
}
