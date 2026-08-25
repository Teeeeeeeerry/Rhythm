#pragma once

#include "pch.h"
#include "Bridge/L10nKeys.h"

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
        L10N_ENTRY(export_m3u8)
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
        L10N_ENTRY(import_some_failed)
        L10N_ENTRY(imported_tracks)
        L10N_ENTRY(importing)
        L10N_ENTRY(library_empty)
        L10N_ENTRY(library_empty_hint)
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

// ─── Tab / Sidebar ──────────────────────────────────────────────────

inline std::wstring LibraryTab() { return Key("library_tab"); }
inline std::wstring PlaylistsTab() { return Key("playlists_tab"); }

// ─── Main window ────────────────────────────────────────────────────

inline std::wstring ImportFolderTooltip() { return Key("import_folder_tooltip"); }
inline std::wstring SearchPlaceholder() { return Key("search_placeholder"); }
inline std::wstring ByArtistAlbum() { return Key("by_artist_album"); }
inline std::wstring ByLetter() { return Key("by_letter"); }
inline std::wstring LibraryEmpty() { return Key("library_empty_hint"); }

// ─── Playlist ───────────────────────────────────────────────────────

inline std::wstring NewPlaylist() { return Key("new_playlist"); }
inline std::wstring PlaylistEmpty() { return Key("no_playlists"); }
inline std::wstring PlaylistNamePlaceholder() { return Key("playlist_name"); }
inline std::wstring Create() { return Key("create"); }
inline std::wstring Cancel() { return Key("cancel"); }
inline std::wstring ImportM3U8() { return Key("import_m3u8"); }
inline std::wstring ExportM3U8() { return Key("export_m3u8"); }

// ─── Player bar ─────────────────────────────────────────────────────

inline std::wstring NotPlaying() { return Key("not_playing"); }
inline std::wstring UrlPlaceholder() { return Key("url_placeholder"); }
inline std::wstring PlayUrl() { return Key("url_play"); }
inline std::wstring Resolving() { return Key("url_resolving"); }
inline std::wstring Buffering() { return Key("buffering"); }
inline std::wstring UrlErrorTitle() { return Key("url_error_title"); }
inline std::wstring Ok() { return Key("ok"); }

// ─── Import feedback (WA-23, mirrors the macOS import alert) ─────────

inline std::wstring ImportedTracks(int32_t count) {
    return Fill(Key("imported_tracks"),
                {{L"count", std::to_wstring(count)}, {L"s", count == 1 ? L"" : L"s"}});
}
inline std::wstring ImportNoFiles() { return Key("import_dir_empty"); }
inline std::wstring ImportFailed() { return Key("import_dir_failed"); }
inline std::wstring ImportSomeFailed(int32_t imported, int32_t failed) {
    return Fill(Key("import_some_failed"),
                {{L"imported", std::to_wstring(imported)}, {L"failed", std::to_wstring(failed)}});
}

// ─── Source tags ────────────────────────────────────────────────────

inline std::wstring SourceTag(const std::wstring& sourceType) {
    if (sourceType == L"local")      return Key("tag_local");
    if (sourceType == L"youtube")    return Key("tag_youtube");
    if (sourceType == L"bilibili")   return Key("tag_bilibili");
    if (sourceType == L"direct_url") return Key("tag_link");
    return L"";
}

// ─── Resolver provisioning status ───────────────────────────────────

inline std::wstring ResolverStatusText(const std::wstring& phase, int64_t received, int64_t total) {
    auto mb = [](int64_t bytes) { return static_cast<double>(bytes) / 1048576.0; };
    if (phase == L"checking")   return Key("resolver_status_checking");
    if (phase == L"verifying")  return Key("resolver_status_verifying");
    if (phase == L"updating")   return Key("resolver_status_updating");
    if (phase == L"failed")     return Key("resolver_status_failed");
    if (phase == L"downloading") {
        std::wstring progress;
        if (total > 0) {
            wchar_t buf[64];
            swprintf_s(buf, L"%.1f / %.1f MB", mb(received), mb(total));
            progress = buf;
        } else {
            wchar_t buf[64];
            swprintf_s(buf, L"%.1f MB", mb(received));
            progress = buf;
        }
        return Fill(Key("resolver_status_downloading"), {{L"progress", progress}});
    }
    return L"";
}

// ─── URL resolution failure ─────────────────────────────────────────

/// Chinese users get a translated headline plus the engine detail; English
/// users get the engine message verbatim (mirrors macOS `urlResolveError`).
/// The yt-dlp install copy comes from the platform-diff key (winget /
/// RHYTHM_YTDLP_PATH — #167 platform field).
inline std::wstring UrlResolveError(const std::wstring& kind, const std::wstring& detail) {
    if (!IsChinese()) return detail;

    std::wstring headline;
    if (kind == L"yt_dlp_missing") {
        headline = Key("resolve_error_yt_dlp_missing_windows");
    } else if (kind == L"timeout") {
        headline = Key("resolve_error_timeout");
    } else if (kind == L"network") {
        headline = Key("resolve_error_network");
    } else if (kind == L"unavailable") {
        headline = Key("resolve_error_unavailable");
    } else if (kind == L"no_audio_stream") {
        headline = Key("resolve_error_no_audio_stream");
    } else if (kind == L"yt_dlp_outdated") {
        headline = Key("resolve_error_yt_dlp_outdated_windows");
    } else if (kind == L"invalid_url") {
        headline = Key("resolve_error_invalid_url");
    } else {
        return detail;
    }
    return headline + L"\n\n" + Key("detail_prefix_zh") + L"\n" + detail;
}

// ─── Playback failure (#120 classification) ──────────────────────────

/// Explain a playback failure (as opposed to a resolution failure). The
/// core classifies HTTP failures: a genuinely expired link ("expired")
/// keeps the "re-paste" advice; a CDN rejecting a still-valid URL
/// ("cdn_rejected") gets the truthful "your network is being rejected"
/// copy. Accepts both the raw core kinds and the "playback_"-prefixed
/// codes the UI maps them to (mirrors macOS `playbackFailed`).
inline std::wstring PlaybackFailed(const std::wstring& kind, const std::wstring& detail) {
    std::wstring normalized = kind;
    if (normalized.rfind(L"playback_", 0) == 0) {
        normalized = normalized.substr(9);
    }

    std::wstring headline;
    if (normalized == L"expired") {
        headline = Key("playback_failed_expired");
    } else if (normalized == L"cdn_rejected") {
        headline = Key("playback_failed_cdn_rejected");
    } else {
        headline = Key("playback_failed_headline");
    }
    return detail.empty()
        ? headline
        : IsChinese()
            ? headline + L"\n\n" + Key("detail_prefix_zh") + L"\n" + detail
            : headline + L"\n\n" + detail;
}

/// Dispatch an OnUrlError (kind, message) pair to the matching localizer.
inline std::wstring UrlErrorText(const std::wstring& kind, const std::wstring& message) {
    if (kind.rfind(L"playback_", 0) == 0) {
        return PlaybackFailed(kind, message);
    }
    return UrlResolveError(kind, message);
}

// ─── Tray ───────────────────────────────────────────────────────────

inline std::wstring TrayPlayPause() { return Key("tray_play_pause"); }
inline std::wstring TrayShowWindow() { return Key("tray_show"); }

} // namespace L10n
} // namespace rhythm
