#pragma once

#include "BehaviorPch.h"
#include "L10n.h"
#include <map>
#include <rhythm_core.h>

namespace rhythm {

/// UTF-8 → wide-string conversion shared by the bridge and the UI layer
/// (coordinator event payloads).
std::wstring Utf8ToWide(const std::string& s);

struct Track {
    int64_t id = 0;
    std::optional<std::wstring> filePath;
    std::wstring sourceType;
    std::optional<std::wstring> sourceUrl;
    std::wstring title;
    std::optional<std::wstring> artist;
    std::optional<std::wstring> album;
    std::optional<std::wstring> albumArtist;
    std::optional<int32_t> trackNumber;
    std::optional<int32_t> discNumber;
    std::optional<std::wstring> genre;
    std::optional<int32_t> year;
    double duration = 0.0;
    std::optional<std::wstring> format;
    std::optional<int32_t> bitrate;
    std::optional<int32_t> sampleRate;
    std::optional<int32_t> channels;
    std::optional<int64_t> fileSize;
    std::optional<std::wstring> dateAdded;
    std::optional<std::wstring> lastPlayed;
    int32_t playCount = 0;
    std::optional<std::wstring> artworkPath;
    bool isAvailable = true;

    /// Field-by-field equality, as for every contract model: results decoded
    /// two ways must agree (#363), a round trip must give the object back (#365).
    bool operator==(const Track&) const = default;
};

/// Parse a Track from the core's snake_case JSON (used for coordinator
/// event payloads like track_changed). Declared after `Track` (#408).
Track ParseTrackJson(const std::string& json);

/// Audio extensions the file picker offers, mirroring the core's
/// SUPPORTED_EXTENSIONS (rust-core/src/metadata/mod.rs) -- the core is still
/// the gate, this list only shapes the dialog (#242). Plain strings: the
/// shell hands them to its picker, the bridge carries no WinRT type (#327).
inline const std::vector<std::wstring> kAudioFileTypes = {
    L".mp3", L".m4a", L".aac", L".flac", L".wav", L".ogg", L".oga", L".opus",
    L".alac", L".ape", L".wma", L".mp4", L".m4b", L".m4p", L".m4r", L".aiff",
    L".aif", L".aifc", L".wv",
};

/// One parsed M3U8 entry — named fields across the seam (#177). Field list
/// and codec come from contracts/ffi-contract.json (#180).
struct M3u8Entry {
    std::wstring title;
    std::optional<std::wstring> artist;
    std::wstring location;

    bool operator==(const M3u8Entry&) const = default;
};

/// How an M3U8 export ended (#351). The two failures are kept apart the way
/// the import outcomes are: bad track data is a bug, an unwritable target is
/// something the user can act on.
enum class M3u8ExportStatus { Exported, InvalidTracks, WriteFailed };

/// Named outcome of an M3U8 export (#351), in place of the core's bare
/// integer. `code` is the core's return code, kept for the failure copy.
struct M3u8ExportOutcome {
    M3u8ExportStatus status = M3u8ExportStatus::Exported;
    int32_t exported = 0;
    int32_t code = 0;

    bool operator==(const M3u8ExportOutcome&) const = default;
};

/// Map the core's export return code onto the named outcome (#351). An
/// unknown code is a write failure, never a silent success.
M3u8ExportOutcome M3u8ExportOutcomeFromCode(int32_t code, int32_t trackCount);

/// Write tracks to an M3U8 file through the core (#428: the view used to
/// hand-roll its own track encoder; the generated codec is the one encoder).
M3u8ExportOutcome ExportM3U8(const std::wstring& path, const std::vector<Track>& tracks);

/// Named outcome of an M3U8 import (#234): how many entries the core stored
/// and how many it could not. Field list and codec come from
/// contracts/ffi-contract.json.
struct M3u8ImportOutcome {
    int32_t imported = 0;
    int32_t failed = 0;

    bool operator==(const M3u8ImportOutcome&) const = default;
};

/// Named outcome of a library import (#237): stored / skipped as an
/// unsupported format / failed to read. The three counts stay separate --
/// folding "unsupported" into "failed" loses the only detail the user can
/// act on. Field list and codec come from contracts/ffi-contract.json.
struct ImportOutcome {
    int32_t imported = 0;
    int32_t unsupported = 0;
    int32_t failed = 0;

    bool operator==(const ImportOutcome&) const = default;
};

/// A user-built playlist. Field list and codec come from
/// contracts/ffi-contract.json (#367) -- the tracks it holds go through the
/// contract's own track codec, so a field added to a track reaches a playlist
/// without a second decoder to update.
struct Playlist {
    std::optional<int64_t> id;
    std::wstring name;
    std::optional<std::wstring> description;
    std::optional<std::wstring> dateCreated;
    std::optional<std::wstring> dateModified;
    std::vector<Track> tracks;

    bool operator==(const Playlist&) const = default;
};

// Wrapper around the Rust rhythm-core library
class Library {
public:
    explicit Library(const std::wstring& dbPath);
    ~Library();

    /// Import every audio file under a directory, reporting the core's
    /// named outcome (#241). nullopt only when the library handle is gone.
    std::optional<ImportOutcome> ImportDirectory(const std::wstring& path);
    /// Import one audio file (#242). Same result shape as the directory
    /// path; an unsupported format and a read failure keep their own counts.
    std::optional<ImportOutcome> ImportFile(const std::wstring& path);
    /// Import a mixed batch of directories and files (#243). The directory
    /// dispatch and the "partial success" aggregation happen in the core --
    /// this side never sums counts.
    std::optional<ImportOutcome> ImportPaths(const std::vector<std::wstring>& paths);
    std::vector<Track> AllTracks();
    std::vector<Track> Search(const std::wstring& query);
    void VerifyFiles();
    std::vector<Playlist> AllPlaylists();
    int64_t CreatePlaylist(const std::wstring& name);
    void AddToPlaylist(int64_t playlistId, int64_t trackId);
    void RemoveFromPlaylist(int64_t playlistId, int64_t trackId);
    void DeletePlaylist(int64_t id);
    void RecordPlay(int64_t trackId);
    /// Persist a track to the library. Returns the track with its
    /// database id, or the original on failure (#39).
    Track AddTrack(const Track& track);
    /// Delete a track from the library. Returns true if a row was deleted.
    bool RemoveTrack(int64_t id);
    /// Parse an M3U8 file and import every entry (#236). Parsing, location
    /// mapping, title fallback and the success test all live in the core
    /// (#233); nullopt means the playlist could not be read.
    std::optional<M3u8ImportOutcome> ImportM3U8(const std::wstring& path);

    /// The underlying core handle, for the coordinator (#416). Ownership stays
    /// here; null when the database failed to open.
    RhythmLibrary* Handle() const { return ptr_; }

private:
    RhythmLibrary* ptr_ = nullptr;
};

/// Structured result of a coordinator call (mirror of the core's
/// `CoordinatorResult` JSON): success payload + classified error in one
/// return. `errorKind` is one of: no_playable_location, playback_failed,
/// invalid_input. The fields mirror the contract's `coordinator_result`
/// (the error pair is absent on success); the codec is generated (#363).
struct CoordinatorResult {
    bool ok = false;
    std::optional<std::wstring> errorKind;
    std::optional<std::wstring> errorMessage;
    std::optional<Track> currentTrack;
    /// Whether playback is active (engine Playing/Buffering) after the
    /// operation — what the UI should render for `IsPlaying`.
    bool playbackActive = false;

    bool operator==(const CoordinatorResult&) const = default;
};

/// The playback surface `AppState` orchestrates against (parent issue #165):
/// owns the engine, the queue, the current track, and the play mode inside
/// the core. Ticket #173 migrates the Windows AppState onto this seam, and
/// tests inject a spy (no audio device required).
class ICoordinator {
public:
    virtual ~ICoordinator() = default;

    /// Start playback of `track` with `queueTracks` as the queue. The
    /// no-playable-location guard lives in the core (#81).
    virtual CoordinatorResult Start(const Track& track,
                                    const std::vector<Track>& queueTracks,
                                    int32_t mode) = 0;
    virtual CoordinatorResult Next() = 0;
    virtual CoordinatorResult Previous() = 0;
    virtual CoordinatorResult TogglePlayPause() = 0;
    /// Sync the queue after a library refresh (#69).
    virtual void SyncQueue(const std::vector<Track>& tracks) = 0;
    virtual void Stop() = 0;
    virtual void SetVolume(float volume) = 0;
    virtual void SetPlayMode(int32_t mode) = 0;
    virtual bool HasNext() const = 0;
    virtual bool HasPrevious() const = 0;
    virtual bool CanTogglePlayback() const = 0;
    virtual bool CanStop() const = 0;
    virtual double Position() const = 0;
    virtual double Duration() const = 0;
    virtual int32_t State() const = 0;
    virtual std::wstring ErrorMessage() const = 0;
    virtual std::wstring ErrorKind() const = 0;
    virtual std::optional<Track> CurrentTrack() const = 0;
    /// Register the library handle for play recording (auto-advance).
    virtual void SetLibrary(Library* library) = 0;
    /// Event subscription (ticket #172): receives event JSON
    /// (`{"type":"finished"|"error"|"progress"|"state"|"track_changed",...}`).
    /// Invoked from the playback thread — marshal to the UI thread yourself.
    virtual void SetEventHandler(std::function<void(const std::wstring&)> handler) = 0;
};

class Coordinator final : public ICoordinator {
public:
    Coordinator();
    ~Coordinator() override;
    Coordinator(const Coordinator&) = delete;
    Coordinator& operator=(const Coordinator&) = delete;

    CoordinatorResult Start(const Track& track,
                            const std::vector<Track>& queueTracks,
                            int32_t mode) override;
    CoordinatorResult Next() override;
    CoordinatorResult Previous() override;
    CoordinatorResult TogglePlayPause() override;
    void SyncQueue(const std::vector<Track>& tracks) override;
    void Stop() override;
    void SetVolume(float volume) override;
    void SetPlayMode(int32_t mode) override;
    bool HasNext() const override;
    bool HasPrevious() const override;
    bool CanTogglePlayback() const override;
    bool CanStop() const override;
    double Position() const override;
    double Duration() const override;
    int32_t State() const override;
    std::wstring ErrorMessage() const override;
    std::wstring ErrorKind() const override;
    std::optional<Track> CurrentTrack() const override;
    void SetLibrary(Library* library) override;
    void SetEventHandler(std::function<void(const std::wstring&)> handler) override;
    /// Deliver an event JSON string from the C callback (playback thread).
    void DispatchEvent(const std::string& utf8);

private:
    RhythmCoordinator* ptr_ = nullptr;
    Library* library_ = nullptr;
    std::function<void(const std::wstring&)> handler_;
};

/// What the core resolved a pasted URL to (mirror of the core's
/// `ResolvedUrl`). The fields mirror its declaration in
/// contracts/ffi-contract.json; the codec is generated from it (#362).
struct ResolvedUrl {
    std::wstring title;
    std::optional<std::wstring> artist;
    std::wstring streamUrl;
    double duration = 0.0;
    std::wstring sourceType;
    std::optional<std::wstring> thumbnailUrl;
    /// Headers the CDN requires on every request for `streamUrl`.
    std::map<std::wstring, std::wstring> httpHeaders;

    bool operator==(const ResolvedUrl&) const = default;
};

/// The core's structured resolve result (#176) as it crosses the seam:
/// success payload + classified error in one return. `errorKind` is a
/// `resolve_error_kind` value. The fields mirror its declaration in
/// contracts/ffi-contract.json; the codec is generated from it (#362).
struct ResolveResult {
    bool ok = false;
    std::optional<ResolvedUrl> resolved;
    std::optional<std::wstring> errorKind;
    std::optional<std::wstring> errorMessage;

    bool operator==(const ResolveResult&) const = default;
};

/// Outcome of a URL resolution: either a playable track, or why it failed.
///
/// `errorKind` is one of: invalid_url, yt_dlp_missing, timeout, network,
/// unavailable, no_audio_stream, yt_dlp_outdated, internal.
struct ResolveOutcome {
    bool ok = false;
    Track track;
    std::wstring errorKind;
    std::wstring errorMessage;
};

/// Progress of yt-dlp provisioning. `phase` is one of: idle, checking,
/// downloading, verifying, updating, ready, failed.
struct ResolverStatus {
    std::wstring phase = L"idle";
    int64_t received = 0;
    int64_t total = 0;

    /// Nothing worth telling the user about.
    bool IsQuiet() const { return phase == L"idle" || phase == L"ready"; }
};

class Resolver {
public:
    static ResolveOutcome ResolveURL(const std::wstring& url);
    static std::wstring ClassifyURL(const std::wstring& url);

    /// Resolver environment as JSON (yt-dlp path/version, PATH, log file).
    static std::wstring Diagnostics();

    /// Poll while a resolution runs: a fresh install downloads yt-dlp on the
    /// first link, which should read as progress rather than a stall.
    static ResolverStatus Status();

    /// Localized description of a provisioning status, empty when quiet.
    static std::wstring StatusText(const ResolverStatus& status);
};

} // namespace rhythm
