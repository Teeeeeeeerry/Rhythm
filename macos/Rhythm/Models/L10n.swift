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
    static let importTooltip: String = isChinese ? "导入文件夹" : "Import Folder"

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

    // ─── Tray ─────────────────────────────────────────────

    static let trayPlayPause: String = isChinese ? "播放 / 暂停" : "Play / Pause"
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
    static let menuNext: String = isChinese ? "下一首" : "Next Track"
    static let menuPrev: String = isChinese ? "上一首" : "Previous Track"
    static let menuToggleMode: String = isChinese ? "切换播放模式" : "Cycle Play Mode"
}
