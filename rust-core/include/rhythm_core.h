#ifndef RHYTHM_CORE_H
#define RHYTHM_CORE_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

// ─── Opaque Types ──────────────────────────────────────────────────

typedef struct RhythmLibrary RhythmLibrary;
typedef struct RhythmPlayer RhythmPlayer;
typedef struct RhythmQueue RhythmQueue;

// ─── Library API ───────────────────────────────────────────────────

RhythmLibrary* rhythm_library_open(const char* db_path);
void rhythm_library_close(RhythmLibrary* lib);

int32_t rhythm_library_import(RhythmLibrary* lib, const char* dir);
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

// ─── Player API ────────────────────────────────────────────────────

RhythmPlayer* rhythm_player_create(void);
void rhythm_player_destroy(RhythmPlayer* player);

int32_t rhythm_player_play_file(RhythmPlayer* player, const char* file_path);
int32_t rhythm_player_play_url(RhythmPlayer* player, const char* url);
void rhythm_player_pause(RhythmPlayer* player);
void rhythm_player_resume(RhythmPlayer* player);
void rhythm_player_stop(RhythmPlayer* player);
void rhythm_player_set_volume(RhythmPlayer* player, float volume);
float rhythm_player_get_volume(RhythmPlayer* player);
int32_t rhythm_player_seek(RhythmPlayer* player, double seconds);
double rhythm_player_get_position(RhythmPlayer* player);
double rhythm_player_get_duration(RhythmPlayer* player);
int32_t rhythm_player_get_state(RhythmPlayer* player);
// State values:
//   0 = Stopped
//   1 = Playing
//   2 = Paused
//   3 = Buffering
//   4 = Error
//   5 = Finished (track ended naturally)

// ─── Play Queue API ────────────────────────────────────────────────

RhythmQueue* rhythm_queue_create(const char* tracks_json);
void rhythm_queue_destroy(RhythmQueue* queue);

char* rhythm_queue_current(RhythmQueue* queue);
char* rhythm_queue_next(RhythmQueue* queue);
char* rhythm_queue_previous(RhythmQueue* queue);
void rhythm_queue_set_mode(RhythmQueue* queue, int32_t mode);
int32_t rhythm_queue_jump_to(RhythmQueue* queue, int64_t track_id);
void rhythm_queue_replace(RhythmQueue* queue, const char* tracks_json);
int32_t rhythm_queue_has_next(RhythmQueue* queue);
int32_t rhythm_queue_has_previous(RhythmQueue* queue);
// Play modes: 0=Sequential, 1=Shuffle, 2=SingleLoop, 3=ListLoop

// ─── URL Resolver API ──────────────────────────────────────────────

char* rhythm_resolve_url(const char* url);
char* rhythm_classify_url(const char* url);

// ─── M3U8 Import/Export ────────────────────────────────────────────

int32_t rhythm_export_m3u8(const char* file_path, const char* tracks_json);
char* rhythm_import_m3u8(const char* file_path);

// ─── Memory Management ────────────────────────────────────────────

void rhythm_free_string(char* ptr);

#ifdef __cplusplus
}
#endif

#endif // RHYTHM_CORE_H
