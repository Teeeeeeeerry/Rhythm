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

    /// 本层解析出的语言标识，交给核心决定拼装形状。语言解析（系统语言 +
    /// 手动覆盖）是平台特异的，不下沉。
    private static var languageCode: String { isChinese ? "zh" : "en" }

    /// 渲染核心的消息规格：按键取模板、按参数填占位符，顺序拼接。
    /// 这是适配层剩下的全部职责（#228）。
    private static func render(_ spec: MessageSpec) -> String {
        spec.segments.map { segment in
            switch segment {
            case .key(let key, let params): return fill(L10nKeys.value(key), params)
            case .literal(let text): return text
            }
        }.joined()
    }

    /// Explain a playback failure (as opposed to a resolution failure).
    ///
    /// `kind` 是核心对 HTTP 失败的分类（#120）。分类到文案键的分派、中英
    /// 拼装形状都在核心（#227/#228），本层只填模板；核心不可用时退回引擎
    /// 原文，不再本地重写一套分派。
    static func playbackFailed(kind: String?, detail: String) -> String {
        guard let spec = playbackFailureSpec(kind: kind, detail: detail, language: languageCode)
        else { return detail }
        return render(spec)
    }

    /// Describe what the resolver is doing while the user waits.
    ///
    /// 阶段分派、字节到 MB 的换算与「已收 / 总量」的格式化都在核心
    /// （#231/#232），本层只填模板；静默阶段与核心不可用时都是空串。
    static func resolverStatusText(phase: String, received: Int64?, total: Int64?) -> String {
        guard let spec = resolverStatusSpec(phase: phase, received: received, total: total)
        else { return "" }
        return render(spec)
    }

    /// Explain a resolution failure.
    ///
    /// 分类到文案键的分派、中英拼装形状、平台差异选键都在核心
    /// （#229/#230），本层只填模板；核心不可用时退回引擎原文。
    static func urlResolveError(kind: String, detail: String) -> String {
        guard let spec = resolveFailureSpec(kind: kind, detail: detail, language: languageCode)
        else { return detail }
        return render(spec)
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
