// 本文件由 scripts/gen-l10n.py 从 contracts/l10n-keys.json 生成（#167 组）。
// 请勿手改——新增文案只改键表，再重新生成。
#pragma once

namespace rhythm {

/// 键表生成的文案取值层（单一事实来源）。Windows 的语言检测（系统 UI
/// 语言 + 注册表覆盖）留在 L10n.h 的 IsChinese()，本层只做键→文案映射。
struct L10nKeys {
    static const wchar_t* Zh(const char* key);
    static const wchar_t* En(const char* key);
};

const wchar_t* L10nKeysZh_add_to_playlist() { return L"添加到播放列表"; }
const wchar_t* L10nKeysEn_add_to_playlist() { return L"Add to Playlist"; }
const wchar_t* L10nKeysZh_back() { return L"返回"; }
const wchar_t* L10nKeysEn_back() { return L"Back"; }
const wchar_t* L10nKeysZh_buffering() { return L"缓冲中…"; }
const wchar_t* L10nKeysEn_buffering() { return L"Buffering…"; }
const wchar_t* L10nKeysZh_by_artist_album() { return L"按艺人/专辑"; }
const wchar_t* L10nKeysEn_by_artist_album() { return L"By Artist/Album"; }
const wchar_t* L10nKeysZh_by_letter() { return L"按首字母"; }
const wchar_t* L10nKeysEn_by_letter() { return L"A–Z"; }
const wchar_t* L10nKeysZh_cancel() { return L"取消"; }
const wchar_t* L10nKeysEn_cancel() { return L"Cancel"; }
const wchar_t* L10nKeysZh_create() { return L"创建"; }
const wchar_t* L10nKeysEn_create() { return L"Create"; }
const wchar_t* L10nKeysZh_delete_button() { return L"删除"; }
const wchar_t* L10nKeysEn_delete_button() { return L"Delete"; }
const wchar_t* L10nKeysZh_delete_confirm_message() { return L"确定要从资料库中删除「{title}」吗？\n此操作不可撤销。"; }
const wchar_t* L10nKeysEn_delete_confirm_message() { return L"Are you sure you want to delete \"{title}\" from the library?\nThis action cannot be undone."; }
const wchar_t* L10nKeysZh_delete_confirm_title() { return L"确认删除"; }
const wchar_t* L10nKeysEn_delete_confirm_title() { return L"Confirm Deletion"; }
const wchar_t* L10nKeysZh_delete_from_library() { return L"从资料库删除"; }
const wchar_t* L10nKeysEn_delete_from_library() { return L"Delete from Library"; }
const wchar_t* L10nKeysZh_detail_prefix_zh() { return L"详细信息："; }
const wchar_t* L10nKeysEn_detail_prefix_zh() { return L""; }
const wchar_t* L10nKeysZh_export_failed() { return L"导出失败（错误码: {code}），请重试。"; }
const wchar_t* L10nKeysEn_export_failed() { return L"Export failed (code: {code}). Please try again."; }
const wchar_t* L10nKeysZh_export_failed_title() { return L"导出失败"; }
const wchar_t* L10nKeysEn_export_failed_title() { return L"Export Failed"; }
const wchar_t* L10nKeysZh_export_m3u8() { return L"导出 M3U8"; }
const wchar_t* L10nKeysEn_export_m3u8() { return L"Export M3U8"; }
const wchar_t* L10nKeysZh_import_all_failed() { return L"全部导入失败，请检查文件是否支持"; }
const wchar_t* L10nKeysEn_import_all_failed() { return L"All imports failed. Check that the files are supported."; }
const wchar_t* L10nKeysZh_import_button() { return L"导入"; }
const wchar_t* L10nKeysEn_import_button() { return L"Import"; }
const wchar_t* L10nKeysZh_import_dir_empty() { return L"该目录中未找到支持的音频文件"; }
const wchar_t* L10nKeysEn_import_dir_empty() { return L"No supported audio files found in this directory."; }
const wchar_t* L10nKeysZh_import_dir_failed() { return L"导入失败，请检查目录是否可访问"; }
const wchar_t* L10nKeysEn_import_dir_failed() { return L"Import failed. Please check that the directory is accessible."; }
const wchar_t* L10nKeysZh_import_file_failed() { return L"导入失败，文件可能已损坏或无法读取"; }
const wchar_t* L10nKeysEn_import_file_failed() { return L"Import failed. The file may be corrupted or unreadable."; }
const wchar_t* L10nKeysZh_import_file_unsupported() { return L"不支持的音频格式"; }
const wchar_t* L10nKeysEn_import_file_unsupported() { return L"Unsupported audio format."; }
const wchar_t* L10nKeysZh_import_hint() { return L"点击工具栏 + 按钮导入音乐文件或文件夹"; }
const wchar_t* L10nKeysEn_import_hint() { return L"Click the + button in the toolbar to import music files or folders"; }
const wchar_t* L10nKeysZh_import_m3u8() { return L"导入 M3U8"; }
const wchar_t* L10nKeysEn_import_m3u8() { return L"Import M3U8"; }
const wchar_t* L10nKeysZh_import_none_found() { return L"未找到支持的音频文件"; }
const wchar_t* L10nKeysEn_import_none_found() { return L"No supported audio files found."; }
const wchar_t* L10nKeysZh_import_result_title() { return L"导入结果"; }
const wchar_t* L10nKeysEn_import_result_title() { return L"Import Result"; }
const wchar_t* L10nKeysZh_import_some_failed() { return L"已导入 {imported} 首，{failed} 个失败"; }
const wchar_t* L10nKeysEn_import_some_failed() { return L"Imported {imported} tracks, {failed} failed."; }
const wchar_t* L10nKeysZh_import_tooltip() { return L"导入音乐"; }
const wchar_t* L10nKeysEn_import_tooltip() { return L"Import Music"; }
const wchar_t* L10nKeysZh_imported_tracks() { return L"已导入 {count} 首歌曲"; }
const wchar_t* L10nKeysEn_imported_tracks() { return L"Imported {count} track{s}."; }
const wchar_t* L10nKeysZh_importing() { return L"正在导入…"; }
const wchar_t* L10nKeysEn_importing() { return L"Importing…"; }
const wchar_t* L10nKeysZh_library_empty() { return L"无内容"; }
const wchar_t* L10nKeysEn_library_empty() { return L"No Content"; }
const wchar_t* L10nKeysZh_library_tab() { return L"资料库"; }
const wchar_t* L10nKeysEn_library_tab() { return L"Library"; }
const wchar_t* L10nKeysZh_menu_next() { return L"下一首"; }
const wchar_t* L10nKeysEn_menu_next() { return L"Next Track"; }
const wchar_t* L10nKeysZh_menu_play_pause() { return L"播放/暂停"; }
const wchar_t* L10nKeysEn_menu_play_pause() { return L"Play/Pause"; }
const wchar_t* L10nKeysZh_menu_playback() { return L"播放"; }
const wchar_t* L10nKeysEn_menu_playback() { return L"Playback"; }
const wchar_t* L10nKeysZh_menu_prev() { return L"上一首"; }
const wchar_t* L10nKeysEn_menu_prev() { return L"Previous Track"; }
const wchar_t* L10nKeysZh_menu_stop() { return L"停止"; }
const wchar_t* L10nKeysEn_menu_stop() { return L"Stop"; }
const wchar_t* L10nKeysZh_menu_toggle_mode() { return L"切换播放模式"; }
const wchar_t* L10nKeysEn_menu_toggle_mode() { return L"Cycle Play Mode"; }
const wchar_t* L10nKeysZh_mode_list_loop() { return L"列表循环"; }
const wchar_t* L10nKeysEn_mode_list_loop() { return L"Repeat All"; }
const wchar_t* L10nKeysZh_mode_sequential() { return L"顺序"; }
const wchar_t* L10nKeysEn_mode_sequential() { return L"Sequential"; }
const wchar_t* L10nKeysZh_mode_shuffle() { return L"随机"; }
const wchar_t* L10nKeysEn_mode_shuffle() { return L"Shuffle"; }
const wchar_t* L10nKeysZh_mode_single_loop() { return L"单曲循环"; }
const wchar_t* L10nKeysEn_mode_single_loop() { return L"Repeat One"; }
const wchar_t* L10nKeysZh_new_playlist() { return L"新建播放列表"; }
const wchar_t* L10nKeysEn_new_playlist() { return L"New Playlist"; }
const wchar_t* L10nKeysZh_no_playlists() { return L"暂无播放列表"; }
const wchar_t* L10nKeysEn_no_playlists() { return L"No playlists yet"; }
const wchar_t* L10nKeysZh_not_playing() { return L"未在播放"; }
const wchar_t* L10nKeysEn_not_playing() { return L"Not Playing"; }
const wchar_t* L10nKeysZh_ok() { return L"确定"; }
const wchar_t* L10nKeysEn_ok() { return L"OK"; }
const wchar_t* L10nKeysZh_play() { return L"播放"; }
const wchar_t* L10nKeysEn_play() { return L"Play"; }
const wchar_t* L10nKeysZh_play_mode_tooltip() { return L"播放模式"; }
const wchar_t* L10nKeysEn_play_mode_tooltip() { return L"Play Mode"; }
const wchar_t* L10nKeysZh_playback_failed_cdn_rejected() { return L"播放失败。YouTube 拒绝了当前网络的请求（可能与 ISP 或 VPN 有关），换网络或稍后再试。"; }
const wchar_t* L10nKeysEn_playback_failed_cdn_rejected() { return L"Playback failed. YouTube rejected this network's request (possibly your ISP or VPN) — switch networks or try again later."; }
const wchar_t* L10nKeysZh_playback_failed_expired() { return L"播放失败。链接可能已过期，重新粘贴一次试试。"; }
const wchar_t* L10nKeysEn_playback_failed_expired() { return L"Playback failed. The link may have expired — try pasting it again."; }
const wchar_t* L10nKeysZh_playback_failed_headline() { return L"播放失败。"; }
const wchar_t* L10nKeysEn_playback_failed_headline() { return L"Playback failed."; }
const wchar_t* L10nKeysZh_playlist_empty() { return L"列表为空"; }
const wchar_t* L10nKeysEn_playlist_empty() { return L"Playlist is empty"; }
const wchar_t* L10nKeysZh_playlist_empty_hint() { return L"从资料库右键添加歌曲"; }
const wchar_t* L10nKeysEn_playlist_empty_hint() { return L"Right-click a track in Library to add it here"; }
const wchar_t* L10nKeysZh_playlist_name() { return L"播放列表名称"; }
const wchar_t* L10nKeysEn_playlist_name() { return L"Playlist Name"; }
const wchar_t* L10nKeysZh_playlists_tab() { return L"播放列表"; }
const wchar_t* L10nKeysEn_playlists_tab() { return L"Playlists"; }
const wchar_t* L10nKeysZh_remove_from_playlist() { return L"从列表移除"; }
const wchar_t* L10nKeysEn_remove_from_playlist() { return L"Remove from Playlist"; }
const wchar_t* L10nKeysZh_resolve_error_invalid_url() { return L"链接无效，请输入以 http:// 或 https:// 开头的地址。"; }
const wchar_t* L10nKeysEn_resolve_error_invalid_url() { return L""; }
const wchar_t* L10nKeysZh_resolve_error_network() { return L"网络错误，无法访问该链接。请检查网络、代理或 VPN 设置。"; }
const wchar_t* L10nKeysEn_resolve_error_network() { return L""; }
const wchar_t* L10nKeysZh_resolve_error_no_audio_stream() { return L"该链接没有可播放的音频流。"; }
const wchar_t* L10nKeysEn_resolve_error_no_audio_stream() { return L""; }
const wchar_t* L10nKeysZh_resolve_error_timeout() { return L"解析超时。请检查网络连接后重试。"; }
const wchar_t* L10nKeysEn_resolve_error_timeout() { return L""; }
const wchar_t* L10nKeysZh_resolve_error_unavailable() { return L"该视频无法访问：可能是私享、已删除、年龄限制、会员专属或所在地区不可用。"; }
const wchar_t* L10nKeysEn_resolve_error_unavailable() { return L""; }
const wchar_t* L10nKeysZh_resolve_error_yt_dlp_missing() { return L"未找到 yt-dlp。播放 YouTube / Bilibili 链接需要先安装它：\n  brew install yt-dlp\n\n如果已经安装：从访达启动的应用不会继承终端的 PATH，请把 RHYTHM_YTDLP_PATH 设为 yt-dlp 的完整路径（用 which yt-dlp 查看）。"; }
const wchar_t* L10nKeysEn_resolve_error_yt_dlp_missing() { return L""; }
const wchar_t* L10nKeysZh_resolve_error_yt_dlp_outdated() { return L"yt-dlp 版本过旧，无法解析该站点。请升级后重试：\n  brew upgrade yt-dlp"; }
const wchar_t* L10nKeysEn_resolve_error_yt_dlp_outdated() { return L""; }
const wchar_t* L10nKeysZh_resolver_status_checking() { return L"正在准备解析组件…"; }
const wchar_t* L10nKeysEn_resolver_status_checking() { return L"Preparing resolver…"; }
const wchar_t* L10nKeysZh_resolver_status_downloading() { return L"正在下载解析组件 {progress}"; }
const wchar_t* L10nKeysEn_resolver_status_downloading() { return L"Downloading resolver {progress}"; }
const wchar_t* L10nKeysZh_resolver_status_failed() { return L"解析组件安装失败"; }
const wchar_t* L10nKeysEn_resolver_status_failed() { return L"Resolver install failed"; }
const wchar_t* L10nKeysZh_resolver_status_updating() { return L"正在更新解析组件…"; }
const wchar_t* L10nKeysEn_resolver_status_updating() { return L"Updating resolver…"; }
const wchar_t* L10nKeysZh_resolver_status_verifying() { return L"正在校验解析组件…"; }
const wchar_t* L10nKeysEn_resolver_status_verifying() { return L"Verifying resolver…"; }
const wchar_t* L10nKeysZh_search_placeholder() { return L"搜索..."; }
const wchar_t* L10nKeysEn_search_placeholder() { return L"Search..."; }
const wchar_t* L10nKeysZh_tag_bilibili() { return L"B站"; }
const wchar_t* L10nKeysEn_tag_bilibili() { return L"Bili"; }
const wchar_t* L10nKeysZh_tag_link() { return L"链接"; }
const wchar_t* L10nKeysEn_tag_link() { return L"Link"; }
const wchar_t* L10nKeysZh_tag_local() { return L"本地"; }
const wchar_t* L10nKeysEn_tag_local() { return L"Local"; }
const wchar_t* L10nKeysZh_tag_youtube() { return L"YT"; }
const wchar_t* L10nKeysEn_tag_youtube() { return L"YT"; }
const wchar_t* L10nKeysZh_track_count() { return L"{count} 首"; }
const wchar_t* L10nKeysEn_track_count() { return L"{count} track{s}"; }
const wchar_t* L10nKeysZh_tray_next() { return L"下一首"; }
const wchar_t* L10nKeysEn_tray_next() { return L"Next Track"; }
const wchar_t* L10nKeysZh_tray_pause() { return L"暂停"; }
const wchar_t* L10nKeysEn_tray_pause() { return L"Pause"; }
const wchar_t* L10nKeysZh_tray_play() { return L"播放"; }
const wchar_t* L10nKeysEn_tray_play() { return L"Play"; }
const wchar_t* L10nKeysZh_tray_play_pause() { return L"播放 / 暂停"; }
const wchar_t* L10nKeysEn_tray_play_pause() { return L"Play / Pause"; }
const wchar_t* L10nKeysZh_tray_prev() { return L"上一首"; }
const wchar_t* L10nKeysEn_tray_prev() { return L"Previous Track"; }
const wchar_t* L10nKeysZh_tray_quit() { return L"退出 Rhythm"; }
const wchar_t* L10nKeysEn_tray_quit() { return L"Quit Rhythm"; }
const wchar_t* L10nKeysZh_tray_show() { return L"显示主窗口"; }
const wchar_t* L10nKeysEn_tray_show() { return L"Show Window"; }
const wchar_t* L10nKeysZh_tray_stop() { return L"停止"; }
const wchar_t* L10nKeysEn_tray_stop() { return L"Stop"; }
const wchar_t* L10nKeysZh_url_error_title() { return L"无法播放链接"; }
const wchar_t* L10nKeysEn_url_error_title() { return L"Cannot Play URL"; }
const wchar_t* L10nKeysZh_url_placeholder() { return L"粘贴 YouTube / Bilibili 链接播放"; }
const wchar_t* L10nKeysEn_url_placeholder() { return L"Paste a YouTube / Bilibili URL to play"; }
const wchar_t* L10nKeysZh_url_play() { return L"播放链接"; }
const wchar_t* L10nKeysEn_url_play() { return L"Play URL"; }
const wchar_t* L10nKeysZh_url_resolve_failed() { return L"链接解析失败，请检查链接是否有效"; }
const wchar_t* L10nKeysEn_url_resolve_failed() { return L"Failed to resolve the URL. Please check it is valid."; }
const wchar_t* L10nKeysZh_url_resolving() { return L"解析中…"; }
const wchar_t* L10nKeysEn_url_resolving() { return L"Resolving…"; }
const wchar_t* L10nKeysZh_view() { return L"视图"; }
const wchar_t* L10nKeysEn_view() { return L"View"; }
const wchar_t* L10nKeysZh_yt_dlp_install_command_windows() { return L"winget install yt-dlp"; }
const wchar_t* L10nKeysEn_yt_dlp_install_command_windows() { return L"winget install yt-dlp"; }

} // namespace rhythm
