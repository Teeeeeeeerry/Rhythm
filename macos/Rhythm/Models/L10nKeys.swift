// 本文件由 scripts/gen-l10n.py 从 contracts/l10n-keys.json 生成（#167 组）。
// 请勿手改——新增文案只改键表，再重新生成。

import Foundation

/// 键表生成的文案取值层（单一事实来源）。
enum L10nKeys {
    /// 语言解析：手动覆盖（AppLanguage）优先，否则跟随系统。
    static var isChinese: Bool {
        if let code = UserDefaults.standard.string(forKey: "AppLanguage") {
            return Locale(identifier: code).identifier.hasPrefix("zh")
        }
        return Locale.current.identifier.hasPrefix("zh")
    }

    /// 键表（zh/en，生成自 contracts/l10n-keys.json）。
    private static let table: [String: (zh: String, en: String)] = [
        "add_to_playlist": (zh: "添加到播放列表", en: "Add to Playlist"),
        "back": (zh: "返回", en: "Back"),
        "buffering": (zh: "缓冲中…", en: "Buffering…"),
        "by_artist_album": (zh: "按艺人/专辑", en: "By Artist/Album"),
        "by_letter": (zh: "按首字母", en: "A–Z"),
        "cancel": (zh: "取消", en: "Cancel"),
        "create": (zh: "创建", en: "Create"),
        "delete_button": (zh: "删除", en: "Delete"),
        "delete_confirm_message": (zh: "确定要从资料库中删除「{title}」吗？\n此操作不可撤销。", en: "Are you sure you want to delete \"{title}\" from the library?\nThis action cannot be undone."),
        "delete_confirm_title": (zh: "确认删除", en: "Confirm Deletion"),
        "delete_from_library": (zh: "从资料库删除", en: "Delete from Library"),
        "detail_prefix_zh": (zh: "详细信息：", en: ""),
        "export_failed": (zh: "导出失败（错误码: {code}），请重试。", en: "Export failed (code: {code}). Please try again."),
        "export_failed_title": (zh: "导出失败", en: "Export Failed"),
        "export_m3u8": (zh: "导出 M3U8", en: "Export M3U8"),
        "import_all_failed": (zh: "全部导入失败，请检查文件是否支持", en: "All imports failed. Check that the files are supported."),
        "import_button": (zh: "导入", en: "Import"),
        "import_dir_empty": (zh: "该目录中未找到支持的音频文件", en: "No supported audio files found in this directory."),
        "import_dir_failed": (zh: "导入失败，请检查目录是否可访问", en: "Import failed. Please check that the directory is accessible."),
        "import_file_failed": (zh: "导入失败，文件可能已损坏或无法读取", en: "Import failed. The file may be corrupted or unreadable."),
        "import_file_unsupported": (zh: "不支持的音频格式", en: "Unsupported audio format."),
        "import_folder_tooltip": (zh: "导入文件夹", en: "Import Folder"),
        "import_hint": (zh: "点击工具栏 + 按钮导入音乐文件或文件夹", en: "Click the + button in the toolbar to import music files or folders"),
        "import_m3u8": (zh: "导入 M3U8", en: "Import M3U8"),
        "import_none_found": (zh: "未找到支持的音频文件", en: "No supported audio files found."),
        "import_result_title": (zh: "导入结果", en: "Import Result"),
        "import_some_failed": (zh: "已导入 {imported} 首，{failed} 个失败", en: "Imported {imported} tracks, {failed} failed."),
        "import_tooltip": (zh: "导入音乐", en: "Import Music"),
        "imported_tracks": (zh: "已导入 {count} 首歌曲", en: "Imported {count} track{s}."),
        "importing": (zh: "正在导入…", en: "Importing…"),
        "library_empty": (zh: "无内容", en: "No Content"),
        "library_tab": (zh: "资料库", en: "Library"),
        "menu_next": (zh: "下一首", en: "Next Track"),
        "menu_play_pause": (zh: "播放/暂停", en: "Play/Pause"),
        "menu_playback": (zh: "播放", en: "Playback"),
        "menu_prev": (zh: "上一首", en: "Previous Track"),
        "menu_stop": (zh: "停止", en: "Stop"),
        "menu_toggle_mode": (zh: "切换播放模式", en: "Cycle Play Mode"),
        "mode_list_loop": (zh: "列表循环", en: "Repeat All"),
        "mode_sequential": (zh: "顺序", en: "Sequential"),
        "mode_shuffle": (zh: "随机", en: "Shuffle"),
        "mode_single_loop": (zh: "单曲循环", en: "Repeat One"),
        "new_playlist": (zh: "新建播放列表", en: "New Playlist"),
        "no_playlists": (zh: "暂无播放列表", en: "No playlists yet"),
        "not_playing": (zh: "未在播放", en: "Not Playing"),
        "ok": (zh: "确定", en: "OK"),
        "play": (zh: "播放", en: "Play"),
        "play_mode_tooltip": (zh: "播放模式", en: "Play Mode"),
        "playback_failed_cdn_rejected": (zh: "播放失败。YouTube 拒绝了当前网络的请求（可能与 ISP 或 VPN 有关），换网络或稍后再试。", en: "Playback failed. YouTube rejected this network's request (possibly your ISP or VPN) — switch networks or try again later."),
        "playback_failed_expired": (zh: "播放失败。链接可能已过期，重新粘贴一次试试。", en: "Playback failed. The link may have expired — try pasting it again."),
        "playback_failed_headline": (zh: "播放失败。", en: "Playback failed."),
        "playlist_empty": (zh: "列表为空", en: "Playlist is empty"),
        "playlist_empty_hint": (zh: "从资料库右键添加歌曲", en: "Right-click a track in Library to add it here"),
        "playlist_name": (zh: "播放列表名称", en: "Playlist Name"),
        "playlists_tab": (zh: "播放列表", en: "Playlists"),
        "remove_from_playlist": (zh: "从列表移除", en: "Remove from Playlist"),
        "resolve_error_invalid_url": (zh: "链接无效，请输入以 http:// 或 https:// 开头的地址。", en: ""),
        "resolve_error_network": (zh: "网络错误，无法访问该链接。请检查网络、代理或 VPN 设置。", en: ""),
        "resolve_error_no_audio_stream": (zh: "该链接没有可播放的音频流。", en: ""),
        "resolve_error_timeout": (zh: "解析超时。请检查网络连接后重试。", en: ""),
        "resolve_error_unavailable": (zh: "该视频无法访问：可能是私享、已删除、年龄限制、会员专属或所在地区不可用。", en: ""),
        "resolve_error_yt_dlp_missing": (zh: "未找到 yt-dlp。播放 YouTube / Bilibili 链接需要先安装它：\n  brew install yt-dlp\n\n如果已经安装：从访达启动的应用不会继承终端的 PATH，请把 RHYTHM_YTDLP_PATH 设为 yt-dlp 的完整路径（用 which yt-dlp 查看）。", en: ""),
        "resolve_error_yt_dlp_outdated": (zh: "yt-dlp 版本过旧，无法解析该站点。请升级后重试：\n  brew upgrade yt-dlp", en: ""),
        "resolver_status_checking": (zh: "正在准备解析组件…", en: "Preparing resolver…"),
        "resolver_status_downloading": (zh: "正在下载解析组件 {received} / {total} MB", en: "Downloading resolver {received} / {total} MB"),
        "resolver_status_downloading_unknown_total": (zh: "正在下载解析组件 {received} MB", en: "Downloading resolver {received} MB"),
        "resolver_status_failed": (zh: "解析组件安装失败", en: "Resolver install failed"),
        "resolver_status_updating": (zh: "正在更新解析组件…", en: "Updating resolver…"),
        "resolver_status_verifying": (zh: "正在校验解析组件…", en: "Verifying resolver…"),
        "search_placeholder": (zh: "搜索...", en: "Search..."),
        "tag_bilibili": (zh: "B站", en: "Bili"),
        "tag_link": (zh: "链接", en: "Link"),
        "tag_local": (zh: "本地", en: "Local"),
        "tag_youtube": (zh: "YT", en: "YT"),
        "track_count": (zh: "{count} 首", en: "{count} track{s}"),
        "tray_next": (zh: "下一首", en: "Next Track"),
        "tray_pause": (zh: "暂停", en: "Pause"),
        "tray_play": (zh: "播放", en: "Play"),
        "tray_play_pause": (zh: "播放 / 暂停", en: "Play / Pause"),
        "tray_prev": (zh: "上一首", en: "Previous Track"),
        "tray_quit": (zh: "退出 Rhythm", en: "Quit Rhythm"),
        "tray_show": (zh: "显示主窗口", en: "Show Window"),
        "tray_stop": (zh: "停止", en: "Stop"),
        "url_error_title": (zh: "无法播放链接", en: "Cannot Play URL"),
        "url_placeholder": (zh: "粘贴 YouTube / Bilibili 链接播放", en: "Paste a YouTube / Bilibili URL to play"),
        "url_play": (zh: "播放链接", en: "Play URL"),
        "url_resolve_failed": (zh: "链接解析失败，请检查链接是否有效", en: "Failed to resolve the URL. Please check it is valid."),
        "url_resolving": (zh: "解析中…", en: "Resolving…"),
        "view": (zh: "视图", en: "View"),
        "yt_dlp_install_command": (zh: "brew install yt-dlp", en: "brew install yt-dlp"),
    ]

    /// 取当前语言的文案；未知键回退键名（键表缺失会被校验脚本拦截）。
    static func value(_ key: String) -> String {
        guard let entry = table[key] else { return key }
        return isChinese ? entry.zh : entry.en
    }
}
