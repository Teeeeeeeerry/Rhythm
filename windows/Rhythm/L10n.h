#pragma once

#include "pch.h"

namespace rhythm {

/// Windows-side runtime localization, mirroring the macOS `L10n` enum
/// (macos/Rhythm/Models/L10n.swift). All user-facing strings live here —
/// views never hardcode literals (CONTEXT.md L10n convention).
///
/// Language resolution (README: 中英文界面, 跟随系统语言、支持手动切换):
/// a manual override persisted at HKCU\Software\Rhythm\AppLanguage wins;
/// otherwise the system UI language decides. The resolved value is cached
/// and invalidated by SetOverrideLanguage.
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

/// Persist a manual language override (empty clears it back to system).
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

// ─── Tab / Sidebar ──────────────────────────────────────────────────

inline std::wstring LibraryTab() { return IsChinese() ? L"资料库" : L"Library"; }
inline std::wstring PlaylistsTab() { return IsChinese() ? L"播放列表" : L"Playlists"; }

// ─── Main window ────────────────────────────────────────────────────

inline std::wstring ImportFolderTooltip() { return IsChinese() ? L"导入文件夹" : L"Import Folder"; }
inline std::wstring SearchPlaceholder() { return IsChinese() ? L"搜索..." : L"Search..."; }
inline std::wstring ByArtistAlbum() { return IsChinese() ? L"按艺人/专辑" : L"By Artist/Album"; }
inline std::wstring ByLetter() { return IsChinese() ? L"按首字母" : L"A–Z"; }
inline std::wstring LibraryEmpty() {
    return IsChinese() ? L"资料库为空 — 点击 + 导入音乐文件夹"
                       : L"Library is empty — tap + to import a music folder";
}

// ─── Playlist ───────────────────────────────────────────────────────

inline std::wstring NewPlaylist() { return IsChinese() ? L"新建播放列表" : L"New Playlist"; }
inline std::wstring PlaylistEmpty() { return IsChinese() ? L"暂无播放列表" : L"No Playlists Yet"; }
inline std::wstring PlaylistNamePlaceholder() { return IsChinese() ? L"播放列表名称" : L"Playlist Name"; }
inline std::wstring Create() { return IsChinese() ? L"创建" : L"Create"; }
inline std::wstring Cancel() { return IsChinese() ? L"取消" : L"Cancel"; }
inline std::wstring ImportM3U8() { return IsChinese() ? L"导入 m3u8" : L"Import M3U8"; }
inline std::wstring ExportM3U8() { return IsChinese() ? L"导出 m3u8" : L"Export M3U8"; }

// ─── Player bar ─────────────────────────────────────────────────────

inline std::wstring NotPlaying() { return IsChinese() ? L"未在播放" : L"Not Playing"; }
inline std::wstring UrlPlaceholder() {
    return IsChinese() ? L"粘贴 YouTube / Bilibili 链接播放"
                       : L"Paste a YouTube / Bilibili URL to play";
}
inline std::wstring PlayUrl() { return IsChinese() ? L"播放" : L"Play"; }
inline std::wstring Resolving() { return IsChinese() ? L"解析中…" : L"Resolving…"; }
inline std::wstring Buffering() { return IsChinese() ? L"缓冲中…" : L"Buffering…"; }
inline std::wstring UrlErrorTitle() { return IsChinese() ? L"无法播放链接" : L"Cannot Play URL"; }
inline std::wstring Ok() { return IsChinese() ? L"确定" : L"OK"; }

// ─── Import feedback (WA-23, mirrors the macOS import alert) ─────────

inline std::wstring ImportedTracks(int32_t count) {
    return IsChinese()
        ? std::format(L"已导入 {} 首歌曲", count)
        : std::format(L"Imported {} track{}.", count, count == 1 ? L"" : L"s");
}
inline std::wstring ImportNoFiles() {
    return IsChinese() ? L"该目录中未找到支持的音频文件"
                       : L"No supported audio files found in that folder";
}
inline std::wstring ImportFailed() {
    return IsChinese() ? L"导入失败，请检查目录是否可访问"
                       : L"Import failed — check that the folder is accessible";
}

// ─── Source tags ────────────────────────────────────────────────────

inline std::wstring SourceTag(const std::wstring& sourceType) {
    if (sourceType == L"local")      return IsChinese() ? L"本地" : L"Local";
    if (sourceType == L"youtube")    return L"YT";
    if (sourceType == L"bilibili")   return IsChinese() ? L"B站" : L"Bili";
    if (sourceType == L"direct_url") return IsChinese() ? L"链接" : L"Link";
    return L"";
}

// ─── Resolver provisioning status ───────────────────────────────────

inline std::wstring ResolverStatusText(const std::wstring& phase, int64_t received, int64_t total) {
    auto mb = [](int64_t bytes) { return static_cast<double>(bytes) / 1048576.0; };
    if (phase == L"checking")   return IsChinese() ? L"正在准备解析组件…" : L"Preparing resolver…";
    if (phase == L"verifying")  return IsChinese() ? L"正在校验解析组件…" : L"Verifying resolver…";
    if (phase == L"updating")   return IsChinese() ? L"正在更新解析组件…" : L"Updating resolver…";
    if (phase == L"failed")     return IsChinese() ? L"解析组件安装失败" : L"Resolver install failed";
    if (phase == L"downloading") {
        if (total > 0) {
            return IsChinese()
                ? std::format(L"正在下载解析组件 {:.1f} / {:.1f} MB", mb(received), mb(total))
                : std::format(L"Downloading resolver {:.1f} / {:.1f} MB", mb(received), mb(total));
        }
        return IsChinese()
            ? std::format(L"正在下载解析组件 {:.1f} MB", mb(received))
            : std::format(L"Downloading resolver {:.1f} MB", mb(received));
    }
    return L"";
}

// ─── URL resolution failure ─────────────────────────────────────────

/// Chinese users get a translated headline plus the engine detail; English
/// users get the engine message verbatim (mirrors macOS `urlResolveError`).
inline std::wstring UrlResolveError(const std::wstring& kind, const std::wstring& detail) {
    if (!IsChinese()) return detail;

    std::wstring headline;
    if (kind == L"yt_dlp_missing") {
        headline =
            L"未找到 yt-dlp。播放 YouTube / Bilibili 链接需要先安装它：\n"
            L"  winget install yt-dlp   或   pip install yt-dlp\n\n"
            L"如果已经安装：应用不会继承你在终端里的 PATH，"
            L"请把 RHYTHM_YTDLP_PATH 设为 yt-dlp.exe 的完整路径。";
    } else if (kind == L"timeout") {
        headline = L"解析超时。请检查网络连接后重试。";
    } else if (kind == L"network") {
        headline = L"网络错误，无法访问该链接。请检查网络、代理或 VPN 设置。";
    } else if (kind == L"unavailable") {
        headline = L"该视频无法访问：可能是私享、已删除、年龄限制、会员专属或所在地区不可用。";
    } else if (kind == L"no_audio_stream") {
        headline = L"该链接没有可播放的音频流。";
    } else if (kind == L"yt_dlp_outdated") {
        headline = L"yt-dlp 版本过旧，无法解析该站点。请升级后重试：\n  pip install -U yt-dlp";
    } else if (kind == L"invalid_url") {
        headline = L"链接无效，请输入以 http:// 或 https:// 开头的地址。";
    } else {
        return detail;
    }
    return headline + L"\n\n详细信息：\n" + detail;
}

// ─── Playback failure (#120 classification) ──────────────────────────

/// Explain a playback failure (as opposed to a resolution failure). The
/// core classifies HTTP failures: a genuinely expired link ("expired")
/// keeps the "re-paste" advice; a CDN rejecting a still-valid URL
/// ("cdn_rejected") gets the truthful "your network is being rejected"
/// copy. Accepts both the raw core kinds and the "playback_"-prefixed
/// codes the Windows timer maps them to (mirrors macOS `playbackFailed`).
inline std::wstring PlaybackFailed(const std::wstring& kind, const std::wstring& detail) {
    std::wstring normalized = kind;
    if (normalized.rfind(L"playback_", 0) == 0) {
        normalized = normalized.substr(9);
    }

    std::wstring headline;
    if (normalized == L"expired") {
        headline = IsChinese()
            ? L"播放失败。链接可能已过期，重新粘贴一次试试。"
            : L"Playback failed. The link may have expired — try pasting it again.";
    } else if (normalized == L"cdn_rejected") {
        headline = IsChinese()
            ? L"播放失败。YouTube 拒绝了当前网络的请求（可能与 ISP 或 VPN 有关），换网络或稍后再试。"
            : L"Playback failed. YouTube rejected this network's request (possibly your ISP or VPN) — switch networks or try again later.";
    } else {
        headline = IsChinese() ? L"播放失败。" : L"Playback failed.";
    }
    return detail.empty()
        ? headline
        : IsChinese()
            ? headline + L"\n\n详细信息：\n" + detail
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

inline std::wstring TrayPlayPause() { return IsChinese() ? L"播放 / 暂停" : L"Play / Pause"; }
inline std::wstring TrayShowWindow() { return IsChinese() ? L"显示主窗口" : L"Show Window"; }
inline std::wstring TrayQuit() { return IsChinese() ? L"退出 Rhythm" : L"Quit Rhythm"; }

} // namespace L10n

} // namespace rhythm
