import Foundation

/// Simple runtime localization — uses UserDefaults to persist language preference.
/// Falls back to system locale on first launch.
///
/// All entries are computed properties (not cached `static let`): the language
/// preference is read on every access so a runtime override (AppLanguage) takes
/// effect immediately, and tests can flip the locale deterministically (#145).
///
/// 文案单一事实来源（#167 组）：静态文案全部来自键表
/// `contracts/l10n-keys.json`（经 `scripts/gen-l10n.py` 生成 `L10nKeys`）；
/// 本层只保留带参数/逻辑的文案函数。新增文案只改键表。
enum L10n {
    static var isChinese: Bool {
        L10nKeys.isChinese
    }

    /// 替换 `{name}` 占位符（键表文案的格式化入口）。
    private static func fill(_ template: String, _ values: [String: String]) -> String {
        var result = template
        for (name, value) in values {
            result = result.replacingOccurrences(of: "{\(name)}", with: value)
        }
        return result
    }

    // ─── Tab / Sidebar ────────────────────────────────────

    static var libraryTab: String { L10nKeys.value("library_tab") }
    static var playlistsTab: String { L10nKeys.value("playlists_tab") }

    // ─── Player Bar ───────────────────────────────────────

    static var notPlaying: String { L10nKeys.value("not_playing") }
    static var playModeTooltip: String { L10nKeys.value("play_mode_tooltip") }
    static var urlPlaceholder: String { L10nKeys.value("url_placeholder") }
    static var urlPlay: String { L10nKeys.value("url_play") }
    static var urlResolving: String { L10nKeys.value("url_resolving") }
    static var urlErrorTitle: String { L10nKeys.value("url_error_title") }
    static var urlResolveFailed: String { L10nKeys.value("url_resolve_failed") }
    static var ok: String { L10nKeys.value("ok") }
    static var buffering: String { L10nKeys.value("buffering") }

    /// Explain a playback failure (as opposed to a resolution failure).
    ///
    /// `kind` is the core's classification of HTTP failures (#120): a link
    /// that genuinely expired ("expired") keeps the old "re-paste it" advice;
    /// a CDN refusing a still-valid URL ("cdn_rejected") gets the truthful
    /// "your network is being rejected" copy — re-pasting cannot help there.
    static func playbackFailed(kind: String?, detail: String) -> String {
        // #135: the classification switch must not be inside the Chinese
        // branch — English users need the same expired / cdn_rejected advice.
        let headline: String
        switch kind {
        case "expired":
            headline = L10nKeys.value("playback_failed_expired")
        case "cdn_rejected":
            headline = L10nKeys.value("playback_failed_cdn_rejected")
        default:
            headline = L10nKeys.value("playback_failed_headline")
        }
        return detail.isEmpty
            ? headline
            : isChinese
                ? "\(headline)\n\n\(L10nKeys.value("detail_prefix_zh"))\n\(detail)"
                : "\(headline)\n\n\(detail)"
    }

    /// Describe what the resolver is doing while the user waits.
    static func resolverStatusText(phase: String, received: Int64?, total: Int64?) -> String {
        switch phase {
        case "checking":
            return L10nKeys.value("resolver_status_checking")
        case "downloading":
            let progress = downloadProgress(received: received, total: total)
            return fill(L10nKeys.value("resolver_status_downloading"), ["progress": progress])
        case "verifying":
            return L10nKeys.value("resolver_status_verifying")
        case "updating":
            return L10nKeys.value("resolver_status_updating")
        case "failed":
            return L10nKeys.value("resolver_status_failed")
        default:
            return ""
        }
    }

    /// "12.3 / 40.1 MB" — or just the received size when the server sent no
    /// content length.
    private static func downloadProgress(received: Int64?, total: Int64?) -> String {
        let mb = { (bytes: Int64) in String(format: "%.1f", Double(bytes) / 1_048_576) }
        guard let received else { return "" }
        guard let total, total > 0 else { return "\(mb(received)) MB" }
        return "\(mb(received)) / \(mb(total)) MB"
    }

    /// Explain a resolution failure.
    ///
    /// The core's `kind` drives a localized headline; its English `message`
    /// carries the actionable detail (install commands, yt-dlp's own output)
    /// and is appended so nothing is lost on an unrecognised kind.
    static func urlResolveError(kind: String, detail: String) -> String {
        guard isChinese else { return detail }

        let headline: String
        switch kind {
        case "yt_dlp_missing":
            headline = L10nKeys.value("resolve_error_yt_dlp_missing")
        case "timeout":
            headline = L10nKeys.value("resolve_error_timeout")
        case "network":
            headline = L10nKeys.value("resolve_error_network")
        case "unavailable":
            headline = L10nKeys.value("resolve_error_unavailable")
        case "no_audio_stream":
            headline = L10nKeys.value("resolve_error_no_audio_stream")
        case "yt_dlp_outdated":
            headline = L10nKeys.value("resolve_error_yt_dlp_outdated")
        case "invalid_url":
            headline = L10nKeys.value("resolve_error_invalid_url")
        default:
            return detail
        }

        return "\(headline)\n\n\(L10nKeys.value("detail_prefix_zh"))\n\(detail)"
    }

    // ─── Play Mode Labels ─────────────────────────────────

    static var modeSequential: String { L10nKeys.value("mode_sequential") }
    static var modeShuffle: String { L10nKeys.value("mode_shuffle") }
    static var modeSingleLoop: String { L10nKeys.value("mode_single_loop") }
    static var modeListLoop: String { L10nKeys.value("mode_list_loop") }

    // ─── Library ──────────────────────────────────────────

    static var libraryEmpty: String { L10nKeys.value("library_empty") }
    static var byArtistAlbum: String { L10nKeys.value("by_artist_album") }
    static var byLetter: String { L10nKeys.value("by_letter") }
    static var searchPlaceholder: String { L10nKeys.value("search_placeholder") }
    static var importTooltip: String { L10nKeys.value("import_tooltip") }
    static var view: String { L10nKeys.value("view") }

    static func importedTracks(_ count: Int) -> String {
        fill(L10nKeys.value("imported_tracks"), ["count": "\(count)", "s": count == 1 ? "" : "s"])
    }

    static func trackCount(_ count: Int) -> String {
        fill(L10nKeys.value("track_count"), ["count": "\(count)", "s": count == 1 ? "" : "s"])
    }

    // ─── Import ───────────────────────────────────────────

    static var importResultTitle: String { L10nKeys.value("import_result_title") }
    static var importing: String { L10nKeys.value("importing") }
    static var importButton: String { L10nKeys.value("import_button") }
    static var importDirEmpty: String { L10nKeys.value("import_dir_empty") }
    static var importDirFailed: String { L10nKeys.value("import_dir_failed") }
    static var importFileUnsupported: String { L10nKeys.value("import_file_unsupported") }
    static var importFileFailed: String { L10nKeys.value("import_file_failed") }
    static var importAllFailed: String { L10nKeys.value("import_all_failed") }
    static var importNoneFound: String { L10nKeys.value("import_none_found") }
    static var importHint: String { L10nKeys.value("import_hint") }

    static func importSomeFailed(_ imported: Int, _ failed: Int) -> String {
        fill(L10nKeys.value("import_some_failed"), ["imported": "\(imported)", "failed": "\(failed)"])
    }

    // ─── Playlist ─────────────────────────────────────────

    static var newPlaylist: String { L10nKeys.value("new_playlist") }
    static var playlistName: String { L10nKeys.value("playlist_name") }
    static var create: String { L10nKeys.value("create") }
    static var cancel: String { L10nKeys.value("cancel") }
    static var importM3U8: String { L10nKeys.value("import_m3u8") }
    static var exportM3U8: String { L10nKeys.value("export_m3u8") }
    static var noPlaylists: String { L10nKeys.value("no_playlists") }
    static var removeFromPlaylist: String { L10nKeys.value("remove_from_playlist") }
    static var exportFailedTitle: String { L10nKeys.value("export_failed_title") }
    static var back: String { L10nKeys.value("back") }
    static var playlistEmpty: String { L10nKeys.value("playlist_empty") }
    static var playlistEmptyHint: String { L10nKeys.value("playlist_empty_hint") }

    static func exportFailed(_ code: Int) -> String {
        fill(L10nKeys.value("export_failed"), ["code": "\(code)"])
    }

    // ─── Context Menu / Actions ───────────────────────────

    static var play: String { L10nKeys.value("play") }
    static var addToPlaylist: String { L10nKeys.value("add_to_playlist") }
    static var deleteFromLibrary: String { L10nKeys.value("delete_from_library") }

    static var deleteConfirmTitle: String { L10nKeys.value("delete_confirm_title") }
    static func deleteConfirmMessage(_ title: String) -> String {
        fill(L10nKeys.value("delete_confirm_message"), ["title": title])
    }
    static var deleteButton: String { L10nKeys.value("delete_button") }

    // ─── Tray ─────────────────────────────────────────────

    /// Initial title only; the tray menu swaps in `trayPlay` / `trayPause`
    /// on each validation pass.
    static var trayPlayPause: String { L10nKeys.value("tray_play_pause") }
    static var trayPlay: String { L10nKeys.value("tray_play") }
    static var trayPause: String { L10nKeys.value("tray_pause") }
    static var trayStop: String { L10nKeys.value("tray_stop") }
    static var trayNext: String { L10nKeys.value("tray_next") }
    static var trayPrev: String { L10nKeys.value("tray_prev") }
    static var trayShow: String { L10nKeys.value("tray_show") }
    static var trayQuit: String { L10nKeys.value("tray_quit") }

    // ─── Source Tags ──────────────────────────────────────

    static var tagLocal: String { L10nKeys.value("tag_local") }
    static var tagYoutube: String { L10nKeys.value("tag_youtube") }
    static var tagBilibili: String { L10nKeys.value("tag_bilibili") }
    static var tagLink: String { L10nKeys.value("tag_link") }

    // ─── Menu / Commands ──────────────────────────────────

    static var menuPlayback: String { L10nKeys.value("menu_playback") }
    static var menuPlayPause: String { L10nKeys.value("menu_play_pause") }
    static var menuStop: String { L10nKeys.value("menu_stop") }
    static var menuNext: String { L10nKeys.value("menu_next") }
    static var menuPrev: String { L10nKeys.value("menu_prev") }
    static var menuToggleMode: String { L10nKeys.value("menu_toggle_mode") }
}
