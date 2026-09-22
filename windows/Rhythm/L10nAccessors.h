// 本文件由 scripts/gen-l10n.py 从 contracts/l10n-keys.json 生成（#371）。
// 请勿手改——新增文案只改键表，再重新生成。
//
// Windows L10n 模块的具名访问器：每个 windows 键一个，调用面与检验面是同一份生成物。
// 命名规则：键名转 PascalCase；带 {占位符} 的模板加 Template 后缀（由 L10n.h 的
// 手写函数填充）；键表的 accessor 字段可固定历史名称。
// 只由 L10n.h 在 Key() 定义之后包含。
#pragma once

#include <string>

namespace rhythm {
namespace L10n {

inline std::wstring AddToPlaylist() { return Key("add_to_playlist"); }
inline std::wstring Back() { return Key("back"); }
inline std::wstring Buffering() { return Key("buffering"); }
inline std::wstring ByArtistAlbum() { return Key("by_artist_album"); }
inline std::wstring ByLetter() { return Key("by_letter"); }
inline std::wstring Cancel() { return Key("cancel"); }
inline std::wstring Create() { return Key("create"); }
inline std::wstring DeleteButton() { return Key("delete_button"); }
inline std::wstring DeleteConfirmMessageTemplate() { return Key("delete_confirm_message"); }
inline std::wstring DeleteConfirmTitle() { return Key("delete_confirm_title"); }
inline std::wstring DeleteFromLibrary() { return Key("delete_from_library"); }
inline std::wstring DetailPrefixZh() { return Key("detail_prefix_zh"); }
inline std::wstring ExportFailedTemplate() { return Key("export_failed"); }
inline std::wstring ExportFailedTitle() { return Key("export_failed_title"); }
inline std::wstring ExportInvalidTracksTemplate() { return Key("export_invalid_tracks"); }
inline std::wstring ExportM3U8() { return Key("export_m3u8"); }
inline std::wstring ExportResultTitle() { return Key("export_result_title"); }
inline std::wstring ExportedTracksTemplate() { return Key("exported_tracks"); }
inline std::wstring ImportAllFailed() { return Key("import_all_failed"); }
inline std::wstring ImportButton() { return Key("import_button"); }
inline std::wstring ImportNoFiles() { return Key("import_dir_empty"); }
inline std::wstring ImportFailed() { return Key("import_dir_failed"); }
inline std::wstring ImportFileFailed() { return Key("import_file_failed"); }
inline std::wstring ImportFileUnsupported() { return Key("import_file_unsupported"); }
inline std::wstring ImportFolderTooltip() { return Key("import_folder_tooltip"); }
inline std::wstring ImportHint() { return Key("import_hint"); }
inline std::wstring ImportM3U8() { return Key("import_m3u8"); }
inline std::wstring ImportNoneFound() { return Key("import_none_found"); }
inline std::wstring ImportResultTitle() { return Key("import_result_title"); }
inline std::wstring ImportSomeFailedTemplate() { return Key("import_some_failed"); }
inline std::wstring ImportTooltip() { return Key("import_tooltip"); }
inline std::wstring ImportedTracksTemplate() { return Key("imported_tracks"); }
inline std::wstring Importing() { return Key("importing"); }
inline std::wstring LibraryEmpty() { return Key("library_empty"); }
inline std::wstring LibraryTab() { return Key("library_tab"); }
inline std::wstring MenuNext() { return Key("menu_next"); }
inline std::wstring MenuPlayPause() { return Key("menu_play_pause"); }
inline std::wstring MenuPlayback() { return Key("menu_playback"); }
inline std::wstring MenuPrev() { return Key("menu_prev"); }
inline std::wstring MenuStop() { return Key("menu_stop"); }
inline std::wstring MenuToggleMode() { return Key("menu_toggle_mode"); }
inline std::wstring ModeListLoop() { return Key("mode_list_loop"); }
inline std::wstring ModeSequential() { return Key("mode_sequential"); }
inline std::wstring ModeShuffle() { return Key("mode_shuffle"); }
inline std::wstring ModeSingleLoop() { return Key("mode_single_loop"); }
inline std::wstring NewPlaylist() { return Key("new_playlist"); }
inline std::wstring PlaylistEmpty() { return Key("no_playlists"); }
inline std::wstring NotPlaying() { return Key("not_playing"); }
inline std::wstring Ok() { return Key("ok"); }
inline std::wstring Play() { return Key("play"); }
inline std::wstring PlayModeTooltip() { return Key("play_mode_tooltip"); }
inline std::wstring PlaybackFailedCdnRejected() { return Key("playback_failed_cdn_rejected"); }
inline std::wstring PlaybackFailedExpired() { return Key("playback_failed_expired"); }
inline std::wstring PlaybackFailedHeadline() { return Key("playback_failed_headline"); }
inline std::wstring PlaylistDetailEmpty() { return Key("playlist_empty"); }
inline std::wstring PlaylistEmptyHint() { return Key("playlist_empty_hint"); }
inline std::wstring PlaylistNamePlaceholder() { return Key("playlist_name"); }
inline std::wstring PlaylistsTab() { return Key("playlists_tab"); }
inline std::wstring RemoveFromPlaylist() { return Key("remove_from_playlist"); }
inline std::wstring ResolveErrorInvalidUrl() { return Key("resolve_error_invalid_url"); }
inline std::wstring ResolveErrorNetwork() { return Key("resolve_error_network"); }
inline std::wstring ResolveErrorNoAudioStream() { return Key("resolve_error_no_audio_stream"); }
inline std::wstring ResolveErrorTimeout() { return Key("resolve_error_timeout"); }
inline std::wstring ResolveErrorUnavailable() { return Key("resolve_error_unavailable"); }
inline std::wstring ResolveErrorYtDlpMissingWindows() { return Key("resolve_error_yt_dlp_missing_windows"); }
inline std::wstring ResolveErrorYtDlpOutdatedWindows() { return Key("resolve_error_yt_dlp_outdated_windows"); }
inline std::wstring ResolverStatusChecking() { return Key("resolver_status_checking"); }
inline std::wstring ResolverStatusDownloadingTemplate() { return Key("resolver_status_downloading"); }
inline std::wstring ResolverStatusDownloadingUnknownTotalTemplate() { return Key("resolver_status_downloading_unknown_total"); }
inline std::wstring ResolverStatusFailed() { return Key("resolver_status_failed"); }
inline std::wstring ResolverStatusUpdating() { return Key("resolver_status_updating"); }
inline std::wstring ResolverStatusVerifying() { return Key("resolver_status_verifying"); }
inline std::wstring SearchPlaceholder() { return Key("search_placeholder"); }
inline std::wstring TagBilibili() { return Key("tag_bilibili"); }
inline std::wstring TagLink() { return Key("tag_link"); }
inline std::wstring TagLocal() { return Key("tag_local"); }
inline std::wstring TagYoutube() { return Key("tag_youtube"); }
inline std::wstring TrackCountTemplate() { return Key("track_count"); }
inline std::wstring TrayNext() { return Key("tray_next"); }
inline std::wstring TrayPause() { return Key("tray_pause"); }
inline std::wstring TrayPlay() { return Key("tray_play"); }
inline std::wstring TrayPlayPause() { return Key("tray_play_pause"); }
inline std::wstring TrayPrev() { return Key("tray_prev"); }
inline std::wstring TrayQuit() { return Key("tray_quit"); }
inline std::wstring TrayShowWindow() { return Key("tray_show"); }
inline std::wstring TrayStop() { return Key("tray_stop"); }
inline std::wstring UrlErrorTitle() { return Key("url_error_title"); }
inline std::wstring UrlPlaceholder() { return Key("url_placeholder"); }
inline std::wstring PlayUrl() { return Key("url_play"); }
inline std::wstring UrlResolveFailed() { return Key("url_resolve_failed"); }
inline std::wstring Resolving() { return Key("url_resolving"); }
inline std::wstring View() { return Key("view"); }
inline std::wstring YtDlpInstallCommandWindows() { return Key("yt_dlp_install_command_windows"); }

} // namespace L10n
} // namespace rhythm
