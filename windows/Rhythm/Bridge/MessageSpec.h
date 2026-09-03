#pragma once

#include "pch.h"
#include <rhythm_core.h>
#include <nlohmann/json.hpp>

#include <utility>

namespace rhythm {

/// UTF-8 与宽串互转，定义在 Bridge/RhythmCore.cpp。在此声明是为了让消息
/// 层不必包含 RhythmCore.h——后者包含 L10n.h，而 L10n.h 包含本文件。
std::wstring Utf8ToWide(const std::string& s);
std::string WideToUtf8(const std::wstring& ws);

/// 核心消息规格的一段：键表条目（可带占位符参数）或原样输出的字面量。
/// 选哪个键、拼成什么形状由核心决定（#227），本层只负责取模板与填充。
struct MessageSegment {
    bool isKey = false;
    std::string key;
    std::vector<std::pair<std::wstring, std::wstring>> params;
    std::wstring text;
};

/// 解析核心返回的规格 JSON；格式异常时返回空规格（不显示任何文案）。
inline std::vector<MessageSegment> ParseMessageSpec(const std::string& specJson) {
    using json = nlohmann::json;
    std::vector<MessageSegment> segments;
    try {
        auto parsed = json::parse(specJson);
        for (const auto& item : parsed.value("segments", json::array())) {
            MessageSegment segment;
            if (item.value("segment", std::string{}) == "key") {
                segment.isKey = true;
                segment.key = item.value("key", std::string{});
                if (item.contains("params") && item["params"].is_object()) {
                    for (const auto& [name, value] : item["params"].items()) {
                        segment.params.emplace_back(Utf8ToWide(name),
                                                    Utf8ToWide(value.get<std::string>()));
                    }
                }
            } else {
                segment.text = Utf8ToWide(item.value("text", std::string{}));
            }
            segments.push_back(std::move(segment));
        }
    } catch (const json::exception&) {
        return {};
    }
    return segments;
}

/// 取一条播放失败的消息规格。`kind` 是核心的 #120 分类值（非 HTTP 失败
/// 为空），`language` 是本端解析出的语言标识。
inline std::vector<MessageSegment> PlaybackFailureSpec(const std::wstring& kind,
                                                       const std::wstring& detail,
                                                       bool chinese) {
    auto kindUtf8 = WideToUtf8(kind);
    auto detailUtf8 = WideToUtf8(detail);
    char* json = rhythm_message_playback_failure(kindUtf8.c_str(), detailUtf8.c_str(),
                                                 chinese ? "zh" : "en");
    if (!json) return {};
    std::string owned(json);
    rhythm_free_string(json);
    return ParseMessageSpec(owned);
}

/// 取一条解析失败的消息规格。平台差异（yt-dlp 安装命令）由核心按构建
/// 目标选键（#229），本端只解析出语言标识。
inline std::vector<MessageSegment> ResolveFailureSpec(const std::wstring& kind,
                                                      const std::wstring& detail,
                                                      bool chinese) {
    auto kindUtf8 = WideToUtf8(kind);
    auto detailUtf8 = WideToUtf8(detail);
    char* json = rhythm_message_resolve_failure(kindUtf8.c_str(), detailUtf8.c_str(),
                                                chinese ? "zh" : "en");
    if (!json) return {};
    std::string owned(json);
    rhythm_free_string(json);
    return ParseMessageSpec(owned);
}

/// 取一条解析器供给状态的消息规格。阶段分派、字节到 MB 的换算与
/// 「已收 / 总量」的格式化都在核心（#231）；静默阶段返回空规格。
inline std::vector<MessageSegment> ResolverStatusSpec(const std::wstring& phase,
                                                      int64_t received, int64_t total) {
    auto phaseUtf8 = WideToUtf8(phase);
    char* json = rhythm_message_resolver_status(phaseUtf8.c_str(), received, total);
    if (!json) return {};
    std::string owned(json);
    rhythm_free_string(json);
    return ParseMessageSpec(owned);
}

} // namespace rhythm
