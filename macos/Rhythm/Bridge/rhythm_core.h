#ifndef RHYTHM_CORE_H
#define RHYTHM_CORE_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

// ─── Opaque Types ──────────────────────────────────────────────────

typedef struct RhythmLibrary RhythmLibrary;
typedef struct RhythmQueue RhythmQueue;
typedef struct RhythmCoordinator RhythmCoordinator;

// ─── Library API ───────────────────────────────────────────────────

RhythmLibrary* rhythm_library_open(const char* db_path);
void rhythm_library_close(RhythmLibrary* lib);

// Named import outcome as JSON: {"imported":N,"unsupported":N,"failed":N} (#239).
// Null only for a null handle. Free with rhythm_free_string.
// rhythm_library_import_paths takes a JSON array of path strings; the
// partial-success aggregation happens in the core.
char* rhythm_library_import_directory(RhythmLibrary* lib, const char* dir);
char* rhythm_library_import_single_file(RhythmLibrary* lib, const char* file_path);
char* rhythm_library_import_paths(RhythmLibrary* lib, const char* paths_json);
char* rhythm_library_get_all_tracks(RhythmLibrary* lib);
char* rhythm_library_search(RhythmLibrary* lib, const char* query);
char* rhythm_library_add_track(RhythmLibrary* lib, const char* track_json);
int32_t rhythm_library_remove_track(RhythmLibrary* lib, int64_t track_id);
char* rhythm_library_verify_files(RhythmLibrary* lib);

char* rhythm_library_get_playlists(RhythmLibrary* lib);
int64_t rhythm_library_create_playlist(RhythmLibrary* lib, const char* name, const char* description);
int32_t rhythm_library_playlist_add(RhythmLibrary* lib, int64_t playlist_id, int64_t track_id);
int32_t rhythm_library_playlist_remove(RhythmLibrary* lib, int64_t playlist_id, int64_t track_id);
int32_t rhythm_library_delete_playlist(RhythmLibrary* lib, int64_t playlist_id);
int32_t rhythm_library_record_play(RhythmLibrary* lib, int64_t track_id);

// ─── Metadata API ──────────────────────────────────────────────────

char* rhythm_metadata_extract(const char* file_path);
char* rhythm_metadata_scan(const char* directory);
char* rhythm_metadata_extract_artwork(const char* file_path, const char* cache_dir);

// ─── Playback Coordinator API ─────────────────────────────────────
//
// The coordinator owns the orchestration rules (stop old playback, dispatch
// by source type, record plays, queue build + positioning, bounded skip of
// unplayable tracks). Every call returns a structured result JSON:
// {"ok":true,"current_track":{...},"playback_active":true} or
// {"ok":false,"error_kind":"...","error_message":"...","playback_active":false}.
// Error kinds: no_playable_location, playback_failed, invalid_input.

RhythmCoordinator* rhythm_coordinator_create(void);
void rhythm_coordinator_destroy(RhythmCoordinator* coordinator);

char* rhythm_coordinator_start(RhythmCoordinator* coordinator, RhythmLibrary* library,
                               const char* track_json, const char* queue_tracks_json,
                               int32_t mode);
// Register the library handle the coordinator uses for play recording
// (transport moves and auto-advance).
void rhythm_coordinator_set_library(RhythmCoordinator* coordinator, RhythmLibrary* library);

// Event callback type: receives a JSON string
//   {"type":"finished"} |
//   {"type":"error","kind":"expired"|"cdn_rejected"|"other"|null,"message":"..."} |
//   {"type":"progress","position":12.3,"duration":180.0} |
//   {"type":"state","state":"stopped"|"playing"|"paused"|"buffering"|"finished"} |
//   {"type":"track_changed","track":{...}}
// Free the string with rhythm_free_string. Invoked from the playback thread.
typedef void (*RhythmEventCallback)(void* userdata, char* event_json);
// Subscribe to coordinator events. On "finished" the coordinator
// auto-advances to the next playable track (core-driven auto-advance).
void rhythm_coordinator_set_event_callback(RhythmCoordinator* coordinator,
                                           RhythmEventCallback callback, void* userdata);
char* rhythm_coordinator_next(RhythmCoordinator* coordinator, RhythmLibrary* library);
char* rhythm_coordinator_previous(RhythmCoordinator* coordinator, RhythmLibrary* library);
char* rhythm_coordinator_toggle_play_pause(RhythmCoordinator* coordinator, RhythmLibrary* library);
int32_t rhythm_coordinator_can_toggle_playback(RhythmCoordinator* coordinator);
int32_t rhythm_coordinator_can_stop(RhythmCoordinator* coordinator);
void rhythm_coordinator_sync_queue(RhythmCoordinator* coordinator, const char* tracks_json);
void rhythm_coordinator_stop(RhythmCoordinator* coordinator);
void rhythm_coordinator_pause(RhythmCoordinator* coordinator);
void rhythm_coordinator_resume(RhythmCoordinator* coordinator);
int32_t rhythm_coordinator_seek(RhythmCoordinator* coordinator, double seconds);
void rhythm_coordinator_set_volume(RhythmCoordinator* coordinator, float volume);
float rhythm_coordinator_get_volume(RhythmCoordinator* coordinator);
double rhythm_coordinator_get_position(RhythmCoordinator* coordinator);
double rhythm_coordinator_get_duration(RhythmCoordinator* coordinator);
int32_t rhythm_coordinator_get_state(RhythmCoordinator* coordinator);
// State values: 0=Stopped, 1=Playing, 2=Paused, 3=Buffering, 4=Error,
// 5=Finished (track ended naturally).
char* rhythm_coordinator_error(RhythmCoordinator* coordinator);
char* rhythm_coordinator_error_kind(RhythmCoordinator* coordinator);
int32_t rhythm_coordinator_has_next(RhythmCoordinator* coordinator);
int32_t rhythm_coordinator_has_previous(RhythmCoordinator* coordinator);
char* rhythm_coordinator_current_track(RhythmCoordinator* coordinator);
void rhythm_coordinator_set_play_mode(RhythmCoordinator* coordinator, int32_t mode);
int32_t rhythm_coordinator_get_play_mode(RhythmCoordinator* coordinator);

// ─── URL Resolver API ──────────────────────────────────────────────

// Structured result (#176/#181): {"ok":true,"resolved":{...}} or
// {"ok":false,"error_kind":"...","error_message":"..."}.
char* rhythm_resolve_url(const char* url);
// Structured result (#181): {"ok":true,"source_type":"..."} or error.
char* rhythm_classify_url(const char* url);

// Resolver environment as JSON (yt-dlp path/version, PATH, log file), for
// bug reports.
char* rhythm_resolver_diagnostics(void);

// Progress of yt-dlp provisioning as JSON, e.g.
// {"phase":"downloading","received":1048576,"total":41943040}
// Phases: idle, checking, downloading, verifying, updating, ready, failed.
char* rhythm_resolver_status(void);

// Install or update Rhythm's own yt-dlp copy. Structured result (#181):
// {"ok":true,"path":"..."} or {"ok":false,"error_kind":"...",...}.
char* rhythm_install_ytdlp(void);

// ─── M3U8 Import/Export ────────────────────────────────────────────

int32_t rhythm_export_m3u8(const char* file_path, const char* tracks_json);
char* rhythm_import_m3u8(const char* file_path);

// Parse an M3U8 file and import every entry into the library (#234).
// Returns the named outcome as JSON: {"imported":N,"failed":M}, or null
// when the playlist cannot be read. Free with rhythm_free_string.
char* rhythm_import_m3u8_into_library(RhythmLibrary* lib, const char* file_path);

// ─── Memory Management ────────────────────────────────────────────

void rhythm_free_string(char* ptr);

#ifdef __cplusplus
}
#endif

#endif // RHYTHM_CORE_H
