#pragma once

#include "pch.h"
#include <rhythm_core.h>

namespace rhythm {

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
        if (sourceType == L"local") return L"本地";
        if (sourceType == L"youtube") return L"YT";
        if (sourceType == L"bilibili") return L"B站";
        if (sourceType == L"direct_url") return L"链接";
        return L"";
    }

    std::wstring SourceColor() const {
        if (sourceType == L"local") return L"Blue";
        if (sourceType == L"youtube") return L"Red";
        if (sourceType == L"bilibili") return L"HotPink";
        if (sourceType == L"direct_url") return L"Green";
        return L"Gray";
    }

    /// Capsule badge background brush — foreground colour at 15 % opacity,
    /// matching the macOS `.background(color.opacity(0.15))` treatment.
    winrt::Microsoft::UI::Xaml::Media::SolidColorBrush SourceBackgroundBrush() const {
        uint8_t r = 128, g = 128, b = 128;
        if (sourceType == L"local")      { r = 0;   g = 0;   b = 255; }
        else if (sourceType == L"youtube")  { r = 255; g = 0;   b = 0; }
        else if (sourceType == L"bilibili") { r = 255; g = 105; b = 180; }
        else if (sourceType == L"direct_url") { r = 0;   g = 128; b = 0; }

        return winrt::Microsoft::UI::Xaml::Media::SolidColorBrush(
            winrt::Windows::UI::Color{38, r, g, b});  // A=38 ≈ 15 %
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

private:
    RhythmLibrary* ptr_ = nullptr;
};

class Player {
public:
    Player();
    ~Player();

    void PlayFile(const std::wstring& path);
    void PlayURL(const std::wstring& url);
    void Pause();
    void Resume();
    void Stop();
    void SetVolume(float volume);
    float Volume() const;
    int32_t State() const;
    /// Why playback failed, when State() is 4 (Error); empty otherwise.
    std::wstring ErrorMessage() const;
    double Position() const;
    double Duration() const;

private:
    RhythmPlayer* ptr_ = nullptr;
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
