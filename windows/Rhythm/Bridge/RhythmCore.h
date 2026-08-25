#pragma once

#include "pch.h"
#include "L10n.h"
#include <rhythm_core.h>

namespace rhythm {

/// UTF-8 → wide-string conversion shared by the bridge and the UI layer
/// (coordinator event payloads).
std::wstring Utf8ToWide(const std::string& s);

/// Parse a Track from the core's snake_case JSON (used for coordinator
/// event payloads like track_changed).
Track ParseTrackJson(const std::string& json);

/// Effective app theme: the app never pins `Application.RequestedTheme`, so
/// the UI follows the system (the same resolution ThemeDictionaries use for
/// `ActualTheme`). Light foreground text ⇒ dark system theme.
inline bool IsDarkTheme() {
    auto fg = winrt::Windows::UI::ViewManagement::UISettings()
                  .GetColorValue(winrt::Windows::UI::ViewManagement::UIColorType::Foreground);
    return (fg.R + fg.G + fg.B) / 3 >= 128;
}

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

    std::wstring DurationFormatted() const {
        int m = static_cast<int>(duration) / 60;
        int s = static_cast<int>(duration) % 60;
        return std::format(L"{}:{:02}", m, s);
    }

    std::wstring SourceTag() const {
        return L10n::SourceTag(sourceType);
    }

    /// Single source-type → colour mapping, shared by the badge foreground
    /// and its capsule background (#147). Dark/light values mirror macOS
    /// Theme.swift `rhythmSource*`; unknown sources return nullopt so
    /// callers pick their own fallback.
    struct SourceRGB {
        uint8_t r, g, b;
    };

    static std::optional<SourceRGB> SourceColorRGB(std::wstring_view sourceType, bool isDarkTheme) {
        struct Entry {
            std::wstring_view name;
            SourceRGB dark, light;
        };
        static constexpr Entry kTable[] = {
            {L"local",      {0x8A, 0xBC, 0xD0}, {0x3A, 0x7A, 0x8C}},
            {L"youtube",    {0xD4, 0x95, 0x73}, {0x8B, 0x4A, 0x28}},
            {L"bilibili",   {0xC8, 0x8D, 0xA8}, {0x8C, 0x4D, 0x68}},
            {L"direct_url", {0x8C, 0xB8, 0x9A}, {0x4C, 0x78, 0x5A}},
        };
        for (const auto& e : kTable) {
            if (e.name == sourceType) {
                return isDarkTheme ? e.dark : e.light;
            }
        }
        return std::nullopt;
    }

    /// Badge foreground colour for a source type, theme-aware (F1, #121).
    /// Unknown sources fall back to the teal text colour — never system Gray (F4).
    std::wstring SourceColor(std::wstring_view sourceType, bool isDarkTheme) const {
        if (auto rgb = SourceColorRGB(sourceType, isDarkTheme)) {
            return std::format(L"#{:02X}{:02X}{:02X}", rgb->r, rgb->g, rgb->b);
        }
        return isDarkTheme ? L"#ABC8D4" : L"#0D464D";  // teal textPrimary
    }

    /// Binding surface: resolves the effective theme (see `IsDarkTheme`).
    std::wstring SourceColor() const {
        return SourceColor(sourceType, IsDarkTheme());
    }

    /// Capsule badge background brush — foreground colour at 15 % opacity,
    /// matching the macOS `.background(color.opacity(0.15))` treatment.
    winrt::Microsoft::UI::Xaml::Media::SolidColorBrush SourceBackgroundBrush() const {
        const SourceRGB fallback{0x80, 0x80, 0x80};  // unknown: grey
        auto rgb = SourceColorRGB(sourceType, IsDarkTheme()).value_or(fallback);
        return winrt::Microsoft::UI::Xaml::Media::SolidColorBrush(
            winrt::Windows::UI::Color{38, rgb.r, rgb.g, rgb.b});  // A=38 ≈ 15 %
    }
};

struct Playlist {
    std::optional<int64_t> id;
    std::wstring name;
    std::optional<std::wstring> description;
    std::optional<std::wstring> dateCreated;
    std::optional<std::wstring> dateModified;
    std::vector<Track> tracks;
};

// Wrapper around the Rust rhythm-core library
class Library {
public:
    explicit Library(const std::wstring& dbPath);
    ~Library();

    int32_t ImportDirectory(const std::wstring& path);
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
    /// Parse an M3U8 file into entries (parsing only — the caller persists
    /// each entry and counts failures; ticket #173 fixes the old no-op).
    std::vector<M3u8Entry> ImportM3U8(const std::wstring& path);

private:
    RhythmLibrary* ptr_ = nullptr;
};

/// Structured result of a coordinator call (mirror of the core's
/// `CoordinatorResult` JSON): success payload + classified error in one
/// return. `errorKind` is one of: no_playable_location, playback_failed,
/// invalid_input.
struct CoordinatorResult {
    bool ok = false;
    std::wstring errorKind;
    std::wstring errorMessage;
    std::optional<Track> currentTrack;
    /// Whether playback is active (engine Playing/Buffering) after the
    /// operation — what the UI should render for `IsPlaying`.
    bool playbackActive = false;
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
