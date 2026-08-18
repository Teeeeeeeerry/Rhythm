import Foundation

/// Simple runtime localization — uses UserDefaults to persist language preference.
/// Falls back to system locale on first launch.
///
/// All entries are computed properties (not cached `static let`): the language
/// preference is read on every access so a runtime override (AppLanguage) takes
/// effect immediately, and tests can flip the locale deterministically (#145).
enum L10n {
    private static var locale: Locale {
        if let code = UserDefaults.standard.string(forKey: "AppLanguage") {
            return Locale(identifier: code)
        }
        return Locale.current
    }

    static var isChinese: Bool {
        locale.identifier.hasPrefix("zh")
    }

    // ─── Tab / Sidebar ────────────────────────────────────

    static var libraryTab: String { isChinese ? "资料库" : "Library" }
    static var playlistsTab: String { isChinese ? "播放列表" : "Playlists" }

    // ─── Player Bar ───────────────────────────────────────

    static var notPlaying: String { isChinese ? "未在播放" : "Not Playing" }
    static var playModeTooltip: String { isChinese ? "播放模式" : "Play Mode" }
    static var urlPlaceholder: String { isChinese ? "粘贴 YouTube / Bilibili 链接播放" : "Paste a YouTube / Bilibili URL to play" }
    static var urlPlay: String { isChinese ? "播放链接" : "Play URL" }
    static var urlResolving: String { isChinese ? "解析中…" : "Resolving…" }
    static var urlErrorTitle: String { isChinese ? "无法播放链接" : "Cannot Play URL" }
    static var urlResolveFailed: String { isChinese ? "链接解析失败，请检查链接是否有效" : "Failed to resolve the URL. Please check it is valid." }
    static var ok: String { isChinese ? "确定" : "OK" }

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
            headline = isChinese
                ? "播放失败。链接可能已过期，重新粘贴一次试试。"
                : "Playback failed. The link may have expired — try pasting it again."
        case "cdn_rejected":
            headline = isChinese
                ? "播放失败。YouTube 拒绝了当前网络的请求（可能与 ISP 或 VPN 有关），换网络或稍后再试。"
                : "Playback failed. YouTube rejected this network's request (possibly your ISP or VPN) — switch networks or try again later."
        default:
            headline = isChinese ? "播放失败。" : "Playback failed."
        }
        return detail.isEmpty
            ? headline
            : isChinese
                ? "\(headline)\n\n详细信息：\n\(detail)"
                : "\(headline)\n\n\(detail)"
    }

    /// Describe what the resolver is doing while the user waits.
    static func resolverStatusText(phase: String, received: Int64?, total: Int64?) -> String {
        switch phase {
        case "checking":
            return isChinese ? "正在准备解析组件…" : "Preparing resolver…"
        case "downloading":
            let progress = downloadProgress(received: received, total: total)
            return isChinese ? "正在下载解析组件 \(progress)" : "Downloading resolver \(progress)"
        case "verifying":
            return isChinese ? "正在校验解析组件…" : "Verifying resolver…"
        case "updating":
            return isChinese ? "正在更新解析组件…" : "Updating resolver…"
        case "failed":
            return isChinese ? "解析组件安装失败" : "Resolver install failed"
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
            headline = """
                未找到 yt-dlp。播放 YouTube / Bilibili 链接需要先安装它：
                  brew install yt-dlp

                如果已经安装：从访达启动的应用不会继承终端的 PATH，\
                请把 RHYTHM_YTDLP_PATH 设为 yt-dlp 的完整路径（用 which yt-dlp 查看）。
                """
        case "timeout":
            headline = "解析超时。请检查网络连接后重试。"
        case "network":
            headline = "网络错误，无法访问该链接。请检查网络、代理或 VPN 设置。"
        case "unavailable":
            headline = "该视频无法访问：可能是私享、已删除、年龄限制、会员专属或所在地区不可用。"
        case "no_audio_stream":
            headline = "该链接没有可播放的音频流。"
        case "yt_dlp_outdated":
            headline = """
                yt-dlp 版本过旧，无法解析该站点。请升级后重试：
                  brew upgrade yt-dlp
                """
        case "invalid_url":
            headline = "链接无效，请输入以 http:// 或 https:// 开头的地址。"
        default:
            return detail
        }

        return "\(headline)\n\n详细信息：\n\(detail)"
    }

    // ─── Play Mode Labels ─────────────────────────────────

    static var modeSequential: String { isChinese ? "顺序" : "Sequential" }
    static var modeShuffle: String { isChinese ? "随机" : "Shuffle" }
    static var modeSingleLoop: String { isChinese ? "单曲循环" : "Repeat One" }
    static var modeListLoop: String { isChinese ? "列表循环" : "Repeat All" }

    // ─── Library ──────────────────────────────────────────

    static var libraryEmpty: String { isChinese ? "无内容" : "No Content" }
    static var byArtistAlbum: String { isChinese ? "按艺人/专辑" : "By Artist/Album" }
    static var byLetter: String { isChinese ? "按首字母" : "A–Z" }
    static var searchPlaceholder: String { isChinese ? "搜索..." : "Search..." }
    static var importTooltip: String { isChinese ? "导入音乐" : "Import Music" }
    static var view: String { isChinese ? "视图" : "View" }

    static func importedTracks(_ count: Int) -> String {
        isChinese ? "已导入 \(count) 首歌曲" : "Imported \(count) track\(count == 1 ? "" : "s")."
    }

    static func trackCount(_ count: Int) -> String {
        isChinese ? "\(count) 首" : "\(count) track\(count == 1 ? "" : "s")"
    }

    // ─── Import ───────────────────────────────────────────

    static var importResultTitle: String { isChinese ? "导入结果" : "Import Result" }
    static var importing: String { isChinese ? "正在导入…" : "Importing…" }
    static var importButton: String { isChinese ? "导入" : "Import" }
    static var importDirEmpty: String { isChinese ? "该目录中未找到支持的音频文件" : "No supported audio files found in this directory." }
    static var importDirFailed: String { isChinese ? "导入失败，请检查目录是否可访问" : "Import failed. Please check that the directory is accessible." }
    static var importFileUnsupported: String { isChinese ? "不支持的音频格式" : "Unsupported audio format." }
    static var importFileFailed: String { isChinese ? "导入失败，文件可能已损坏或无法读取" : "Import failed. The file may be corrupted or unreadable." }
    static var importAllFailed: String { isChinese ? "全部导入失败，请检查文件是否支持" : "All imports failed. Check that the files are supported." }
    static var importNoneFound: String { isChinese ? "未找到支持的音频文件" : "No supported audio files found." }
    static var importHint: String { isChinese ? "点击工具栏 + 按钮导入音乐文件或文件夹" : "Click the + button in the toolbar to import music files or folders" }

    static func importSomeFailed(_ imported: Int, _ failed: Int) -> String {
        isChinese ? "已导入 \(imported) 首，\(failed) 个失败" : "Imported \(imported) tracks, \(failed) failed."
    }

    // ─── Playlist ─────────────────────────────────────────

    static var newPlaylist: String { isChinese ? "新建播放列表" : "New Playlist" }
    static var playlistName: String { isChinese ? "播放列表名称" : "Playlist Name" }
    static var create: String { isChinese ? "创建" : "Create" }
    static var cancel: String { isChinese ? "取消" : "Cancel" }
    static var importM3U8: String { isChinese ? "导入 M3U8" : "Import M3U8" }
    static var exportM3U8: String { isChinese ? "导出 M3U8" : "Export M3U8" }
    static var noPlaylists: String { isChinese ? "暂无播放列表" : "No playlists yet" }
    static var removeFromPlaylist: String { isChinese ? "从列表移除" : "Remove from Playlist" }
    static var exportFailedTitle: String { isChinese ? "导出失败" : "Export Failed" }
    static var back: String { isChinese ? "返回" : "Back" }
    static var playlistEmpty: String { isChinese ? "列表为空" : "Playlist is empty" }
    static var playlistEmptyHint: String { isChinese ? "从资料库右键添加歌曲" : "Right-click a track in Library to add it here" }

    static func exportFailed(_ code: Int) -> String {
        isChinese ? "导出失败（错误码: \(code)），请重试。" : "Export failed (code: \(code)). Please try again."
    }

    // ─── Context Menu / Actions ───────────────────────────

    static var play: String { isChinese ? "播放" : "Play" }
    static var addToPlaylist: String { isChinese ? "添加到播放列表" : "Add to Playlist" }
    static var deleteFromLibrary: String { isChinese ? "从资料库删除" : "Delete from Library" }

    static var deleteConfirmTitle: String { isChinese ? "确认删除" : "Confirm Deletion" }
    static func deleteConfirmMessage(_ title: String) -> String {
        isChinese ? "确定要从资料库中删除「\(title)」吗？\n此操作不可撤销。" : "Are you sure you want to delete \"\(title)\" from the library?\nThis action cannot be undone."
    }
    static var deleteButton: String { isChinese ? "删除" : "Delete" }

    static var buffering: String { isChinese ? "缓冲中…" : "Buffering…" }

    // ─── Tray ─────────────────────────────────────────────

    /// Initial title only; the tray menu swaps in `trayPlay` / `trayPause`
    /// on each validation pass.
    static var trayPlayPause: String { isChinese ? "播放 / 暂停" : "Play / Pause" }
    static var trayPlay: String { isChinese ? "播放" : "Play" }
    static var trayPause: String { isChinese ? "暂停" : "Pause" }
    static var trayStop: String { isChinese ? "停止" : "Stop" }
    static var trayNext: String { isChinese ? "下一首" : "Next Track" }
    static var trayPrev: String { isChinese ? "上一首" : "Previous Track" }
    static var trayShow: String { isChinese ? "显示主窗口" : "Show Window" }
    static var trayQuit: String { isChinese ? "退出 Rhythm" : "Quit Rhythm" }

    // ─── Source Tags ──────────────────────────────────────

    static var tagLocal: String { isChinese ? "本地" : "Local" }
    static var tagYoutube: String { "YT" }
    static var tagBilibili: String { isChinese ? "B站" : "Bili" }
    static var tagLink: String { isChinese ? "链接" : "Link" }

    // ─── Menu / Commands ──────────────────────────────────

    static var menuPlayback: String { isChinese ? "播放" : "Playback" }
    static var menuPlayPause: String { isChinese ? "播放/暂停" : "Play/Pause" }
    static var menuStop: String { isChinese ? "停止" : "Stop" }
    static var menuNext: String { isChinese ? "下一首" : "Next Track" }
    static var menuPrev: String { isChinese ? "上一首" : "Previous Track" }
    static var menuToggleMode: String { isChinese ? "切换播放模式" : "Cycle Play Mode" }
}
