#pragma once

#include "BehaviorPch.h"
#include "Bridge/L10nKeys.h"
#include "Bridge/MessageSpec.h"

#include <cstring>

namespace rhythm {

/// Windows-side runtime localization, mirroring the macOS `L10n` enum
/// (macos/Rhythm/Models/L10n.swift). All user-facing strings come from the
/// shared key table `contracts/l10n-keys.json` (generated `L10nKeys.h`) —
/// views never hardcode literals (CONTEXT.md L10n convention, #167 组).
///
/// Language resolution (README: 中英文界面, 跟随系统语言、支持手动切换):
/// a manual override persisted at HKCU\Software\Rhythm\AppLanguage wins;
/// otherwise the system UI language decides. The resolved value is cached
/// and invalidated by SetOverrideLanguage. This detection stays
/// platform-specific (macOS uses Locale.current) — the *copy* is shared.
namespace L10n {

// ─── Language resolution ────────────────────────────────────────────

/// Cache invalidation flag for the resolved language; internals only.
inline bool& isChineseComputed() {
    static bool computed = false;
    return computed;
}

/// Manual override ("zh", "en", ...), empty when following the system.
inline std::wstring OverrideLanguage() {
    HKEY key;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, L"Software\\Rhythm", 0, KEY_READ, &key) != ERROR_SUCCESS) {
        return {};
    }
    wchar_t buffer[16] = {};
    DWORD size = sizeof(buffer);
    DWORD type = 0;
    LONG result = RegQueryValueExW(key, L"AppLanguage", nullptr, &type,
                                   reinterpret_cast<LPBYTE>(buffer), &size);
    RegCloseKey(key);
    if (result != ERROR_SUCCESS || type != REG_SZ) return {};
    return buffer;
}

inline void SetOverrideLanguage(const std::wstring& code) {
    HKEY key;
    if (RegCreateKeyExW(HKEY_CURRENT_USER, L"Software\\Rhythm", 0, nullptr, 0,
                        KEY_WRITE, nullptr, &key, nullptr) != ERROR_SUCCESS) {
        return;
    }
    if (code.empty()) {
        RegDeleteValueW(key, L"AppLanguage");
    } else {
        RegSetValueExW(key, L"AppLanguage", 0, REG_SZ,
                       reinterpret_cast<const BYTE*>(code.c_str()),
                       static_cast<DWORD>((code.size() + 1) * sizeof(wchar_t)));
    }
    RegCloseKey(key);
    isChineseComputed() = false;
}

/// Whether the UI renders Chinese: manual override wins, otherwise the
/// system UI language decides (mirrors macOS `L10n.isChinese`).
inline bool IsChinese() {
    static bool cached = false;
    if (!isChineseComputed()) {
        auto override = OverrideLanguage();
        cached = override.empty()
            ? PRIMARYLANGID(GetUserDefaultUILanguage()) == LANG_CHINESE
            : override.rfind(L"zh", 0) == 0;
        isChineseComputed() = true;
    }
    return cached;
}

/// 键表取值：当前语言下的文案（未命中键回退键名，键表校验拦截缺失）。
/// 键→访问器映射由脚本生成器维护（与 L10nKeys.h 同源）。
inline const wchar_t* Key(const char* key) {
    struct Entry { const char* name; const wchar_t* zh; const wchar_t* en; };
    static const Entry kTable[] = {
#define L10N_ENTRY(name) {#name, L10nKeys_zh_##name(), L10nKeys_en_##name()},
        L10N_ENTRY(add_to_playlist)
        L10N_ENTRY(back)
        L10N_ENTRY(buffering)
        L10N_ENTRY(by_artist_album)
        L10N_ENTRY(by_letter)
        L10N_ENTRY(cancel)
        L10N_ENTRY(create)
        L10N_ENTRY(delete_button)
        L10N_ENTRY(delete_confirm_message)
        L10N_ENTRY(delete_confirm_title)
        L10N_ENTRY(delete_from_library)
        L10N_ENTRY(detail_prefix_zh)
        L10N_ENTRY(export_failed)
        L10N_ENTRY(export_failed_title)
        L10N_ENTRY(export_invalid_tracks)
        L10N_ENTRY(export_m3u8)
        L10N_ENTRY(export_result_title)
        L10N_ENTRY(exported_tracks)
        L10N_ENTRY(import_all_failed)
        L10N_ENTRY(import_button)
        L10N_ENTRY(import_dir_empty)
        L10N_ENTRY(import_dir_failed)
        L10N_ENTRY(import_file_failed)
        L10N_ENTRY(import_file_unsupported)
        L10N_ENTRY(import_folder_tooltip)
        L10N_ENTRY(import_hint)
        L10N_ENTRY(import_m3u8)
        L10N_ENTRY(import_none_found)
        L10N_ENTRY(import_result_title)
        L10N_ENTRY(import_tooltip)
        L10N_ENTRY(import_some_failed)
        L10N_ENTRY(imported_tracks)
        L10N_ENTRY(importing)
        L10N_ENTRY(library_empty)
        L10N_ENTRY(library_tab)
        L10N_ENTRY(menu_next)
        L10N_ENTRY(menu_play_pause)
        L10N_ENTRY(menu_playback)
        L10N_ENTRY(menu_prev)
        L10N_ENTRY(menu_stop)
        L10N_ENTRY(menu_toggle_mode)
        L10N_ENTRY(mode_list_loop)
        L10N_ENTRY(mode_shuffle)
        L10N_ENTRY(mode_single_loop)
        L10N_ENTRY(mode_sequential)
        L10N_ENTRY(new_playlist)
        L10N_ENTRY(no_playlist_selected)
        L10N_ENTRY(no_playlists)
        L10N_ENTRY(not_playing)
        L10N_ENTRY(ok)
        L10N_ENTRY(play)
        L10N_ENTRY(play_mode_tooltip)
        L10N_ENTRY(playback_failed_cdn_rejected)
        L10N_ENTRY(playback_failed_expired)
        L10N_ENTRY(playback_failed_headline)
        L10N_ENTRY(playlist_empty)
        L10N_ENTRY(playlist_empty_hint)
        L10N_ENTRY(playlist_name)
        L10N_ENTRY(playlists_tab)
        L10N_ENTRY(remove_from_playlist)
        L10N_ENTRY(resolve_error_invalid_url)
        L10N_ENTRY(resolve_error_network)
        L10N_ENTRY(resolve_error_no_audio_stream)
        L10N_ENTRY(resolve_error_timeout)
        L10N_ENTRY(resolve_error_unavailable)
        L10N_ENTRY(resolve_error_yt_dlp_missing_windows)
        L10N_ENTRY(resolve_error_yt_dlp_outdated_windows)
        L10N_ENTRY(resolver_status_checking)
        L10N_ENTRY(resolver_status_downloading)
        L10N_ENTRY(resolver_status_downloading_unknown_total)
        L10N_ENTRY(resolver_status_failed)
        L10N_ENTRY(resolver_status_updating)
        L10N_ENTRY(resolver_status_verifying)
        L10N_ENTRY(search_placeholder)
        L10N_ENTRY(tag_bilibili)
        L10N_ENTRY(tag_link)
        L10N_ENTRY(tag_local)
        L10N_ENTRY(tag_youtube)
        L10N_ENTRY(track_count)
        L10N_ENTRY(tray_next)
        L10N_ENTRY(tray_pause)
        L10N_ENTRY(tray_play)
        L10N_ENTRY(tray_play_pause)
        L10N_ENTRY(tray_prev)
        L10N_ENTRY(tray_quit)
        L10N_ENTRY(tray_show)
        L10N_ENTRY(tray_stop)
        L10N_ENTRY(url_error_title)
        L10N_ENTRY(url_placeholder)
        L10N_ENTRY(url_play)
        L10N_ENTRY(url_resolve_failed)
        L10N_ENTRY(url_resolving)
        L10N_ENTRY(view)
        L10N_ENTRY(yt_dlp_install_command_windows)
#undef L10N_ENTRY
    };
    for (const auto& entry : kTable) {
        if (strcmp(entry.name, key) == 0) {
            return IsChinese() ? entry.zh : entry.en;
        }
    }
    return L"";
}

/// 替换 `{name}` 占位符（键表模板文案的格式化入口）。
inline std::wstring Fill(const wchar_t* templateText,
                         std::initializer_list<std::pair<const wchar_t*, std::wstring>> values) {
    std::wstring result = templateText;
    for (const auto& [name, value] : values) {
        std::wstring key = L"{";
        key += name;
        key += L"}";
        size_t pos;
        while ((pos = result.find(key)) != std::wstring::npos) {
            result.replace(pos, key.size(), value);
        }
    }
    return result;
}

/// 渲染核心的消息规格：按键取模板、按参数填占位符，顺序拼接。
/// 这是适配层剩下的全部职责（#228）。
inline std::wstring RenderMessageSpec(const std::vector<MessageSegment>& segments) {
    std::wstring out;
    for (const auto& segment : segments) {
        if (!segment.isKey) {
            out += segment.text;
            continue;
        }
        std::wstring text = Key(segment.key.c_str());
        for (const auto& [name, value] : segment.params) {
            std::wstring placeholder = L"{" + name + L"}";
            size_t pos;
            while ((pos = text.find(placeholder)) != std::wstring::npos) {
                text.replace(pos, placeholder.size(), value);
            }
        }
        out += text;
    }
    return out;
}

} // namespace L10n
} // namespace rhythm

// ─── Named accessors (generated, #371) ───────────────────────────────
//
// One accessor per windows key, generated from contracts/l10n-keys.json by
// scripts/gen-l10n.py - the module's call surface is the generated set, so a
// referenced-but-missing accessor (TrayQuit, once) cannot happen again, and
// check-l10n-keys.py compares exactly this surface with the key table.
// Only functions that fill parameters or dispatch on a kind stay hand-written
// below; they build on the generated accessors.
#include "L10nAccessors.h"

namespace rhythm {
namespace L10n {

// ─── Import feedback (WA-23, mirrors the macOS import alert) ─────────

inline std::wstring ImportedTracks(int32_t count) {
    return Fill(ImportedTracksTemplate().c_str(),
                {{L"count", std::to_wstring(count)}, {L"s", count == 1 ? L"" : L"s"}});
}
inline std::wstring ImportSomeFailed(int32_t imported, int32_t failed) {
    return Fill(ImportSomeFailedTemplate().c_str(),
                {{L"imported", std::to_wstring(imported)}, {L"failed", std::to_wstring(failed)}});
}

// ─── Export feedback (#352, the same shape as the import alert) ─────

inline std::wstring ExportedTracks(int32_t count) {
    return Fill(ExportedTracksTemplate().c_str(),
                {{L"count", std::to_wstring(count)}, {L"s", count == 1 ? L"" : L"s"}});
}
inline std::wstring ExportFailed(int32_t code) {
    return Fill(ExportFailedTemplate().c_str(), {{L"code", std::to_wstring(code)}});
}
/// Bad track data is a bug, not something a retry fixes (#321 review).
inline std::wstring ExportInvalidTracks(int32_t code) {
    return Fill(ExportInvalidTracksTemplate().c_str(), {{L"code", std::to_wstring(code)}});
}

// ─── Source tags ────────────────────────────────────────────────────

inline std::wstring SourceTag(const std::wstring& sourceType) {
    if (sourceType == L"local")      return TagLocal();
    if (sourceType == L"youtube")    return TagYoutube();
    if (sourceType == L"bilibili")   return TagBilibili();
    if (sourceType == L"direct_url") return TagLink();
    return L"";
}

// ─── Resolver provisioning status ───────────────────────────────────

/// 阶段分派、字节到 MB 的换算与「已收 / 总量」的格式化都在核心
/// （#231/#232），本层只填模板；静默阶段与核心不可用时都是空串。
inline std::wstring ResolverStatusText(const std::wstring& phase, int64_t received, int64_t total) {
    return RenderMessageSpec(ResolverStatusSpec(phase, received, total));
}

// ─── URL resolution failure ─────────────────────────────────────────

/// Chinese users get a translated headline plus the engine detail; English
/// users get the engine message verbatim (mirrors macOS `urlResolveError`).
///
/// 分类到文案键的分派、中英拼装形状、平台差异选键（winget 而非 brew）
/// 都在核心（#229/#230），本层只填模板；核心不可用时退回引擎原文。
///
/// #374: a failure that carries nothing to show (no payload from the core,
/// or no detail and no localized headline) falls back to the key table's
/// `url_resolve_failed` instead of an empty dialog (macOS `ResolveError.unknown`).
inline std::wstring UrlResolveError(const std::wstring& kind, const std::wstring& detail) {
    auto spec = ResolveFailureSpec(kind, detail, IsChinese());
    auto text = spec.empty() ? detail : RenderMessageSpec(spec);
    return text.empty() ? UrlResolveFailed() : text;
}

// ─── Playback failure (#120 classification) ──────────────────────────

/// Explain a playback failure (as opposed to a resolution failure).
///
/// `kind` 是核心对 HTTP 失败的分类（#120）。分类到文案键的分派、中英拼装
/// 形状都在核心（#227/#228），本层只填模板；核心不可用时退回引擎原文，
/// 不再本地重写一套分派（mirrors macOS `playbackFailed`）。
inline std::wstring PlaybackFailed(const std::wstring& kind, const std::wstring& detail) {
    auto spec = PlaybackFailureSpec(kind, detail, IsChinese());
    if (spec.empty()) return detail;
    return RenderMessageSpec(spec);
}

// ─── Directory import result (#375/#376) ──────────────────────────────

/// 目录导入结果的文案。三态分派（有导入/全部失败/没找到文件）与
/// 「有导入」时的单复数参数都在核心（#375），本层只渲染规格。
inline std::wstring ImportDirectoryResult(int32_t imported, int32_t failed) {
    return RenderMessageSpec(ImportDirectoryResultSpec(imported, failed));
}

} // namespace L10n
} // namespace rhythm
