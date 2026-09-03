// 本文件由 scripts/gen-l10n.py 从 contracts/l10n-keys.json 生成（#167 组）。
// 请勿手改——新增文案只改键表，再重新生成。
#pragma once

namespace rhythm {

// 键表生成的文案取值（单一事实来源）。Windows 的语言检测（系统 UI
// 语言 + 注册表覆盖）留在 L10n.h 的 IsChinese()，本层只做键→文案映射。
// 带 {占位符} 的模板由 L10n.h 的 Fill 填充。

inline const wchar_t* L10nKeys_zh_add_to_playlist() { return L"添加到播放列表"; }
inline const wchar_t* L10nKeys_en_add_to_playlist() { return L"Add to Playlist"; }
inline const wchar_t* L10nKeys_zh_back() { return L"返回"; }
inline const wchar_t* L10nKeys_en_back() { return L"Back"; }
inline const wchar_t* L10nKeys_zh_buffering() { return L"缓冲中…"; }
inline const wchar_t* L10nKeys_en_buffering() { return L"Buffering…"; }
inline const wchar_t* L10nKeys_zh_by_artist_album() { return L"按艺人/专辑"; }
inline const wchar_t* L10nKeys_en_by_artist_album() { return L"By Artist/Album"; }
inline const wchar_t* L10nKeys_zh_by_letter() { return L"按首字母"; }
inline const wchar_t* L10nKeys_en_by_letter() { return L"A–Z"; }
inline const wchar_t* L10nKeys_zh_cancel() { return L"取消"; }
inline const wchar_t* L10nKeys_en_cancel() { return L"Cancel"; }
inline const wchar_t* L10nKeys_zh_create() { return L"创建"; }
inline const wchar_t* L10nKeys_en_create() { return L"Create"; }
inline const wchar_t* L10nKeys_zh_delete_button() { return L"删除"; }
inline const wchar_t* L10nKeys_en_delete_button() { return L"Delete"; }
inline const wchar_t* L10nKeys_zh_delete_confirm_message() { return L"确定要从资料库中删除「{title}」吗？\n此操作不可撤销。"; }
inline const wchar_t* L10nKeys_en_delete_confirm_message() { return L"Are you sure you want to delete \"{title}\" from the library?\nThis action cannot be undone."; }
inline const wchar_t* L10nKeys_zh_delete_confirm_title() { return L"确认删除"; }
inline const wchar_t* L10nKeys_en_delete_confirm_title() { return L"Confirm Deletion"; }
inline const wchar_t* L10nKeys_zh_delete_from_library() { return L"从资料库删除"; }
inline const wchar_t* L10nKeys_en_delete_from_library() { return L"Delete from Library"; }
inline const wchar_t* L10nKeys_zh_detail_prefix_zh() { return L"详细信息："; }
inline const wchar_t* L10nKeys_en_detail_prefix_zh() { return L""; }
inline const wchar_t* L10nKeys_zh_export_failed() { return L"导出失败（错误码: {code}），请重试。"; }
inline const wchar_t* L10nKeys_en_export_failed() { return L"Export failed (code: {code}). Please try again."; }
inline const wchar_t* L10nKeys_zh_export_failed_title() { return L"导出失败"; }
inline const wchar_t* L10nKeys_en_export_failed_title() { return L"Export Failed"; }
inline const wchar_t* L10nKeys_zh_export_m3u8() { return L"导出 M3U8"; }
inline const wchar_t* L10nKeys_en_export_m3u8() { return L"Export M3U8"; }
inline const wchar_t* L10nKeys_zh_import_all_failed() { return L"全部导入失败，请检查文件是否支持"; }
inline const wchar_t* L10nKeys_en_import_all_failed() { return L"All imports failed. Check that the files are supported."; }
inline const wchar_t* L10nKeys_zh_import_button() { return L"导入"; }
inline const wchar_t* L10nKeys_en_import_button() { return L"Import"; }
inline const wchar_t* L10nKeys_zh_import_dir_empty() { return L"该目录中未找到支持的音频文件"; }
inline const wchar_t* L10nKeys_en_import_dir_empty() { return L"No supported audio files found in this directory."; }
inline const wchar_t* L10nKeys_zh_import_dir_failed() { return L"导入失败，请检查目录是否可访问"; }
inline const wchar_t* L10nKeys_en_import_dir_failed() { return L"Import failed. Please check that the directory is accessible."; }
inline const wchar_t* L10nKeys_zh_import_file_failed() { return L"导入失败，文件可能已损坏或无法读取"; }
inline const wchar_t* L10nKeys_en_import_file_failed() { return L"Import failed. The file may be corrupted or unreadable."; }
inline const wchar_t* L10nKeys_zh_import_file_unsupported() { return L"不支持的音频格式"; }
inline const wchar_t* L10nKeys_en_import_file_unsupported() { return L"Unsupported audio format."; }
inline const wchar_t* L10nKeys_zh_import_folder_tooltip() { return L"导入文件夹"; }
inline const wchar_t* L10nKeys_en_import_folder_tooltip() { return L"Import Folder"; }
inline const wchar_t* L10nKeys_zh_import_hint() { return L"点击工具栏 + 按钮导入音乐文件或文件夹"; }
inline const wchar_t* L10nKeys_en_import_hint() { return L"Click the + button in the toolbar to import music files or folders"; }
inline const wchar_t* L10nKeys_zh_import_m3u8() { return L"导入 M3U8"; }
inline const wchar_t* L10nKeys_en_import_m3u8() { return L"Import M3U8"; }
inline const wchar_t* L10nKeys_zh_import_none_found() { return L"未找到支持的音频文件"; }
inline const wchar_t* L10nKeys_en_import_none_found() { return L"No supported audio files found."; }
inline const wchar_t* L10nKeys_zh_import_result_title() { return L"导入结果"; }
inline const wchar_t* L10nKeys_en_import_result_title() { return L"Import Result"; }
inline const wchar_t* L10nKeys_zh_import_some_failed() { return L"已导入 {imported} 首，{failed} 个失败"; }
inline const wchar_t* L10nKeys_en_import_some_failed() { return L"Imported {imported} tracks, {failed} failed."; }
inline const wchar_t* L10nKeys_zh_import_tooltip() { return L"导入音乐"; }
inline const wchar_t* L10nKeys_en_import_tooltip() { return L"Import Music"; }
inline const wchar_t* L10nKeys_zh_imported_tracks() { return L"已导入 {count} 首歌曲"; }
inline const wchar_t* L10nKeys_en_imported_tracks() { return L"Imported {count} track{s}."; }
inline const wchar_t* L10nKeys_zh_importing() { return L"正在导入…"; }
inline const wchar_t* L10nKeys_en_importing() { return L"Importing…"; }
inline const wchar_t* L10nKeys_zh_library_empty() { return L"无内容"; }
inline const wchar_t* L10nKeys_en_library_empty() { return L"No Content"; }
inline const wchar_t* L10nKeys_zh_library_tab() { return L"资料库"; }
inline const wchar_t* L10nKeys_en_library_tab() { return L"Library"; }
inline const wchar_t* L10nKeys_zh_menu_next() { return L"下一首"; }
inline const wchar_t* L10nKeys_en_menu_next() { return L"Next Track"; }
inline const wchar_t* L10nKeys_zh_menu_play_pause() { return L"播放/暂停"; }
inline const wchar_t* L10nKeys_en_menu_play_pause() { return L"Play/Pause"; }
inline const wchar_t* L10nKeys_zh_menu_playback() { return L"播放"; }
inline const wchar_t* L10nKeys_en_menu_playback() { return L"Playback"; }
inline const wchar_t* L10nKeys_zh_menu_prev() { return L"上一首"; }
inline const wchar_t* L10nKeys_en_menu_prev() { return L"Previous Track"; }
inline const wchar_t* L10nKeys_zh_menu_stop() { return L"停止"; }
inline const wchar_t* L10nKeys_en_menu_stop() { return L"Stop"; }
inline const wchar_t* L10nKeys_zh_menu_toggle_mode() { return L"切换播放模式"; }
inline const wchar_t* L10nKeys_en_menu_toggle_mode() { return L"Cycle Play Mode"; }
inline const wchar_t* L10nKeys_zh_mode_list_loop() { return L"列表循环"; }
inline const wchar_t* L10nKeys_en_mode_list_loop() { return L"Repeat All"; }
inline const wchar_t* L10nKeys_zh_mode_sequential() { return L"顺序"; }
inline const wchar_t* L10nKeys_en_mode_sequential() { return L"Sequential"; }
inline const wchar_t* L10nKeys_zh_mode_shuffle() { return L"随机"; }
inline const wchar_t* L10nKeys_en_mode_shuffle() { return L"Shuffle"; }
inline const wchar_t* L10nKeys_zh_mode_single_loop() { return L"单曲循环"; }
inline const wchar_t* L10nKeys_en_mode_single_loop() { return L"Repeat One"; }
inline const wchar_t* L10nKeys_zh_new_playlist() { return L"新建播放列表"; }
inline const wchar_t* L10nKeys_en_new_playlist() { return L"New Playlist"; }
inline const wchar_t* L10nKeys_zh_no_playlists() { return L"暂无播放列表"; }
inline const wchar_t* L10nKeys_en_no_playlists() { return L"No playlists yet"; }
inline const wchar_t* L10nKeys_zh_not_playing() { return L"未在播放"; }
inline const wchar_t* L10nKeys_en_not_playing() { return L"Not Playing"; }
inline const wchar_t* L10nKeys_zh_ok() { return L"确定"; }
inline const wchar_t* L10nKeys_en_ok() { return L"OK"; }
inline const wchar_t* L10nKeys_zh_play() { return L"播放"; }
inline const wchar_t* L10nKeys_en_play() { return L"Play"; }
inline const wchar_t* L10nKeys_zh_play_mode_tooltip() { return L"播放模式"; }
inline const wchar_t* L10nKeys_en_play_mode_tooltip() { return L"Play Mode"; }
inline const wchar_t* L10nKeys_zh_playback_failed_cdn_rejected() { return L"播放失败。YouTube 拒绝了当前网络的请求（可能与 ISP 或 VPN 有关），换网络或稍后再试。"; }
inline const wchar_t* L10nKeys_en_playback_failed_cdn_rejected() { return L"Playback failed. YouTube rejected this network's request (possibly your ISP or VPN) — switch networks or try again later."; }
inline const wchar_t* L10nKeys_zh_playback_failed_expired() { return L"播放失败。链接可能已过期，重新粘贴一次试试。"; }
inline const wchar_t* L10nKeys_en_playback_failed_expired() { return L"Playback failed. The link may have expired — try pasting it again."; }
inline const wchar_t* L10nKeys_zh_playback_failed_headline() { return L"播放失败。"; }
inline const wchar_t* L10nKeys_en_playback_failed_headline() { return L"Playback failed."; }
inline const wchar_t* L10nKeys_zh_playlist_empty() { return L"列表为空"; }
inline const wchar_t* L10nKeys_en_playlist_empty() { return L"Playlist is empty"; }
inline const wchar_t* L10nKeys_zh_playlist_empty_hint() { return L"从资料库右键添加歌曲"; }
inline const wchar_t* L10nKeys_en_playlist_empty_hint() { return L"Right-click a track in Library to add it here"; }
inline const wchar_t* L10nKeys_zh_playlist_name() { return L"播放列表名称"; }
inline const wchar_t* L10nKeys_en_playlist_name() { return L"Playlist Name"; }
inline const wchar_t* L10nKeys_zh_playlists_tab() { return L"播放列表"; }
inline const wchar_t* L10nKeys_en_playlists_tab() { return L"Playlists"; }
inline const wchar_t* L10nKeys_zh_remove_from_playlist() { return L"从列表移除"; }
inline const wchar_t* L10nKeys_en_remove_from_playlist() { return L"Remove from Playlist"; }
inline const wchar_t* L10nKeys_zh_resolve_error_invalid_url() { return L"链接无效，请输入以 http:// 或 https:// 开头的地址。"; }
inline const wchar_t* L10nKeys_en_resolve_error_invalid_url() { return L""; }
inline const wchar_t* L10nKeys_zh_resolve_error_network() { return L"网络错误，无法访问该链接。请检查网络、代理或 VPN 设置。"; }
inline const wchar_t* L10nKeys_en_resolve_error_network() { return L""; }
inline const wchar_t* L10nKeys_zh_resolve_error_no_audio_stream() { return L"该链接没有可播放的音频流。"; }
inline const wchar_t* L10nKeys_en_resolve_error_no_audio_stream() { return L""; }
inline const wchar_t* L10nKeys_zh_resolve_error_timeout() { return L"解析超时。请检查网络连接后重试。"; }
inline const wchar_t* L10nKeys_en_resolve_error_timeout() { return L""; }
inline const wchar_t* L10nKeys_zh_resolve_error_unavailable() { return L"该视频无法访问：可能是私享、已删除、年龄限制、会员专属或所在地区不可用。"; }
inline const wchar_t* L10nKeys_en_resolve_error_unavailable() { return L""; }
inline const wchar_t* L10nKeys_zh_resolve_error_yt_dlp_missing_windows() { return L"未找到 yt-dlp。播放 YouTube / Bilibili 链接需要先安装它：\n  winget install yt-dlp   或   pip install yt-dlp\n\n如果已经安装：应用不会继承你在终端里的 PATH，请把 RHYTHM_YTDLP_PATH 设为 yt-dlp.exe 的完整路径。"; }
inline const wchar_t* L10nKeys_en_resolve_error_yt_dlp_missing_windows() { return L""; }
inline const wchar_t* L10nKeys_zh_resolve_error_yt_dlp_outdated_windows() { return L"yt-dlp 版本过旧，无法解析该站点。请升级后重试：\n  pip install -U yt-dlp"; }
inline const wchar_t* L10nKeys_en_resolve_error_yt_dlp_outdated_windows() { return L""; }
inline const wchar_t* L10nKeys_zh_resolver_status_checking() { return L"正在准备解析组件…"; }
inline const wchar_t* L10nKeys_en_resolver_status_checking() { return L"Preparing resolver…"; }
inline const wchar_t* L10nKeys_zh_resolver_status_downloading() { return L"正在下载解析组件 {received} / {total} MB"; }
inline const wchar_t* L10nKeys_en_resolver_status_downloading() { return L"Downloading resolver {received} / {total} MB"; }
inline const wchar_t* L10nKeys_zh_resolver_status_downloading_unknown_total() { return L"正在下载解析组件 {received} MB"; }
inline const wchar_t* L10nKeys_en_resolver_status_downloading_unknown_total() { return L"Downloading resolver {received} MB"; }
inline const wchar_t* L10nKeys_zh_resolver_status_failed() { return L"解析组件安装失败"; }
inline const wchar_t* L10nKeys_en_resolver_status_failed() { return L"Resolver install failed"; }
inline const wchar_t* L10nKeys_zh_resolver_status_updating() { return L"正在更新解析组件…"; }
inline const wchar_t* L10nKeys_en_resolver_status_updating() { return L"Updating resolver…"; }
inline const wchar_t* L10nKeys_zh_resolver_status_verifying() { return L"正在校验解析组件…"; }
inline const wchar_t* L10nKeys_en_resolver_status_verifying() { return L"Verifying resolver…"; }
inline const wchar_t* L10nKeys_zh_search_placeholder() { return L"搜索..."; }
inline const wchar_t* L10nKeys_en_search_placeholder() { return L"Search..."; }
inline const wchar_t* L10nKeys_zh_tag_bilibili() { return L"B站"; }
inline const wchar_t* L10nKeys_en_tag_bilibili() { return L"Bili"; }
inline const wchar_t* L10nKeys_zh_tag_link() { return L"链接"; }
inline const wchar_t* L10nKeys_en_tag_link() { return L"Link"; }
inline const wchar_t* L10nKeys_zh_tag_local() { return L"本地"; }
inline const wchar_t* L10nKeys_en_tag_local() { return L"Local"; }
inline const wchar_t* L10nKeys_zh_tag_youtube() { return L"YT"; }
inline const wchar_t* L10nKeys_en_tag_youtube() { return L"YT"; }
inline const wchar_t* L10nKeys_zh_track_count() { return L"{count} 首"; }
inline const wchar_t* L10nKeys_en_track_count() { return L"{count} track{s}"; }
inline const wchar_t* L10nKeys_zh_tray_next() { return L"下一首"; }
inline const wchar_t* L10nKeys_en_tray_next() { return L"Next Track"; }
inline const wchar_t* L10nKeys_zh_tray_pause() { return L"暂停"; }
inline const wchar_t* L10nKeys_en_tray_pause() { return L"Pause"; }
inline const wchar_t* L10nKeys_zh_tray_play() { return L"播放"; }
inline const wchar_t* L10nKeys_en_tray_play() { return L"Play"; }
inline const wchar_t* L10nKeys_zh_tray_play_pause() { return L"播放 / 暂停"; }
inline const wchar_t* L10nKeys_en_tray_play_pause() { return L"Play / Pause"; }
inline const wchar_t* L10nKeys_zh_tray_prev() { return L"上一首"; }
inline const wchar_t* L10nKeys_en_tray_prev() { return L"Previous Track"; }
inline const wchar_t* L10nKeys_zh_tray_quit() { return L"退出 Rhythm"; }
inline const wchar_t* L10nKeys_en_tray_quit() { return L"Quit Rhythm"; }
inline const wchar_t* L10nKeys_zh_tray_show() { return L"显示主窗口"; }
inline const wchar_t* L10nKeys_en_tray_show() { return L"Show Window"; }
inline const wchar_t* L10nKeys_zh_tray_stop() { return L"停止"; }
inline const wchar_t* L10nKeys_en_tray_stop() { return L"Stop"; }
inline const wchar_t* L10nKeys_zh_url_error_title() { return L"无法播放链接"; }
inline const wchar_t* L10nKeys_en_url_error_title() { return L"Cannot Play URL"; }
inline const wchar_t* L10nKeys_zh_url_placeholder() { return L"粘贴 YouTube / Bilibili 链接播放"; }
inline const wchar_t* L10nKeys_en_url_placeholder() { return L"Paste a YouTube / Bilibili URL to play"; }
inline const wchar_t* L10nKeys_zh_url_play() { return L"播放链接"; }
inline const wchar_t* L10nKeys_en_url_play() { return L"Play URL"; }
inline const wchar_t* L10nKeys_zh_url_resolve_failed() { return L"链接解析失败，请检查链接是否有效"; }
inline const wchar_t* L10nKeys_en_url_resolve_failed() { return L"Failed to resolve the URL. Please check it is valid."; }
inline const wchar_t* L10nKeys_zh_url_resolving() { return L"解析中…"; }
inline const wchar_t* L10nKeys_en_url_resolving() { return L"Resolving…"; }
inline const wchar_t* L10nKeys_zh_view() { return L"视图"; }
inline const wchar_t* L10nKeys_en_view() { return L"View"; }
inline const wchar_t* L10nKeys_zh_yt_dlp_install_command_windows() { return L"winget install yt-dlp"; }
inline const wchar_t* L10nKeys_en_yt_dlp_install_command_windows() { return L"winget install yt-dlp"; }

} // namespace rhythm
