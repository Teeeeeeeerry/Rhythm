import Foundation

/// Simple runtime localization — uses UserDefaults to persist language preference.
/// Falls back to system locale on first launch.
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

    static let libraryTab: String = isChinese ? "资料库" : "Library"
    static let playlistsTab: String = isChinese ? "播放列表" : "Playlists"

    // ─── Player Bar ───────────────────────────────────────

    static let notPlaying: String = isChinese ? "未在播放" : "Not Playing"
    static let playModeTooltip: String = isChinese ? "播放模式" : "Play Mode"
    static let urlPlaceholder: String = isChinese ? "粘贴 YouTube / Bilibili 链接播放" : "Paste a YouTube / Bilibili URL to play"
    static let urlPlay: String = isChinese ? "播放链接" : "Play URL"
    static let urlResolving: String = isChinese ? "解析中…" : "Resolving…"
    static let urlErrorTitle: String = isChinese ? "无法播放链接" : "Cannot Play URL"
    static let urlResolveFailed: String = isChinese ? "链接解析失败，请检查链接是否有效" : "Failed to resolve the URL. Please check it is valid."
    static let ok: String = isChinese ? "确定" : "OK"

    /// Explain a playback failure (as opposed to a resolution failure).
    static func playbackFailed(detail: String) -> String {
        guard isChinese else {
            return detail.isEmpty ? "Playback failed." : "Playback failed.\n\n\(detail)"
        }
        let headline = "播放失败。链接可能已过期，重新粘贴一次试试。"
        return detail.isEmpty ? headline : "\(headline)\n\n详细信息：\n\(detail)"
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

    static let modeSequential: String = isChinese ? "顺序" : "Sequential"
    static let modeShuffle: String = isChinese ? "随机" : "Shuffle"
    static let modeSingleLoop: String = isChinese ? "单曲循环" : "Repeat One"
    static let modeListLoop: String = isChinese ? "列表循环" : "Repeat All"

    // ─── Library ──────────────────────────────────────────

    static let libraryEmpty: String = isChinese ? "无内容" : "No Content"
    static let byArtistAlbum: String = isChinese ? "按艺人/专辑" : "By Artist/Album"
    static let byLetter: String = isChinese ? "按首字母" : "A–Z"
    static let searchPlaceholder: String = isChinese ? "搜索..." : "Search..."
    static let importTooltip: String = isChinese ? "导入音乐" : "Import Music"

    static func importedTracks(_ count: Int) -> String {
        isChinese ? "已导入 \(count) 首歌曲" : "Imported \(count) track\(count == 1 ? "" : "s")."
    }

    // ─── Playlist ─────────────────────────────────────────

    static let newPlaylist: String = isChinese ? "新建播放列表" : "New Playlist"
    static let playlistName: String = isChinese ? "播放列表名称" : "Playlist Name"
    static let create: String = isChinese ? "创建" : "Create"
    static let cancel: String = isChinese ? "取消" : "Cancel"
    static let importM3U8: String = isChinese ? "导入 M3U8" : "Import M3U8"
    static let exportM3U8: String = isChinese ? "导出 M3U8" : "Export M3U8"

    // ─── Context Menu / Actions ───────────────────────────

    static let play: String = isChinese ? "播放" : "Play"
    static let addToPlaylist: String = isChinese ? "添加到播放列表" : "Add to Playlist"
    static let deleteFromLibrary: String = isChinese ? "从资料库删除" : "Delete from Library"

    static let deleteConfirmTitle: String = isChinese ? "确认删除" : "Confirm Deletion"
    static func deleteConfirmMessage(_ title: String) -> String {
        isChinese ? "确定要从资料库中删除「\(title)」吗？\n此操作不可撤销。" : "Are you sure you want to delete \"\(title)\" from the library?\nThis action cannot be undone."
    }
    static let deleteButton: String = isChinese ? "删除" : "Delete"

    static let buffering: String = isChinese ? "缓冲中…" : "Buffering…"

    // ─── Tray ─────────────────────────────────────────────

    /// Initial title only; the tray menu swaps in `trayPlay` / `trayPause`
    /// on each validation pass.
    static let trayPlayPause: String = isChinese ? "播放 / 暂停" : "Play / Pause"
    static let trayPlay: String = isChinese ? "播放" : "Play"
    static let trayPause: String = isChinese ? "暂停" : "Pause"
    static let trayStop: String = isChinese ? "停止" : "Stop"
    static let trayNext: String = isChinese ? "下一首" : "Next Track"
    static let trayPrev: String = isChinese ? "上一首" : "Previous Track"
    static let trayShow: String = isChinese ? "显示主窗口" : "Show Window"
    static let trayQuit: String = isChinese ? "退出 Rhythm" : "Quit Rhythm"

    // ─── Source Tags ──────────────────────────────────────

    static let tagLocal: String = isChinese ? "本地" : "Local"
    static let tagLink: String = isChinese ? "链接" : "Link"

    // ─── Menu / Commands ──────────────────────────────────

    static let menuPlayback: String = isChinese ? "播放" : "Playback"
    static let menuPlayPause: String = isChinese ? "播放/暂停" : "Play/Pause"
    static let menuStop: String = isChinese ? "停止" : "Stop"
    static let menuNext: String = isChinese ? "下一首" : "Next Track"
    static let menuPrev: String = isChinese ? "上一首" : "Previous Track"
    static let menuToggleMode: String = isChinese ? "切换播放模式" : "Cycle Play Mode"
}
