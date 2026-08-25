#pragma once

#include "pch.h"
#include "Bridge/RhythmCore.h"

namespace rhythm {

enum class SidebarItem { Library, Playlists };

/// Playback mode — the FFI contract values (0-3) are locked below at compile
/// time; the canonical declaration lives in rust-core `queue::PlayMode`
/// (#179).
enum class PlayMode { Sequential = 0, Shuffle = 1, SingleLoop = 2, ListLoop = 3 };

// Contract lock (#179): the seam values must never drift from the core.
static_assert(static_cast<int32_t>(PlayMode::Sequential) == 0);
static_assert(static_cast<int32_t>(PlayMode::Shuffle) == 1);
static_assert(static_cast<int32_t>(PlayMode::SingleLoop) == 2);
static_assert(static_cast<int32_t>(PlayMode::ListLoop) == 3);

class AppState : public winrt::implements<AppState, winrt::Windows::Foundation::IInspectable> {
public:
    AppState();

    std::unique_ptr<Library> Library;
    /// Playback orchestration seam (ticket #173): owns the engine, queue,
    /// current track, and play mode in the core. Tests inject a spy so
    /// playback paths run with no audio device.
    std::unique_ptr<ICoordinator> Coordinator;
    SidebarItem SelectedView = SidebarItem::Library;
    std::wstring SearchQuery;
    std::vector<Track> Tracks;
    std::vector<Playlist> Playlists;
    std::optional<Track> CurrentTrack;
    bool IsPlaying = false;
    /// True while the engine is buffering; driven by state events (ticket
    /// #172/#173) and shown in the player bar (mirrors macOS `isBuffering`).
    bool IsBuffering = false;
    double Volume = 1.0;
    double Position = 0.0;
    double Duration = 0.0;
    /// Last URL resolution failure, empty when the last attempt succeeded.
    std::wstring UrlError;
    /// True while a URL resolution is in flight. Atomic: cleared from the
    /// resolver thread when no dispatcher is available.
    std::atomic<bool> IsResolvingUrl{ false };
    /// Raised on the UI thread when a URL fails to resolve: (kind, message).
    std::function<void(const std::wstring&, const std::wstring&)> OnUrlError;

    // Import feedback (WA-23, mirrors the macOS import alert).
    std::wstring ImportAlertMessage;
    bool ShowImportAlert = false;

    PlayMode CurrentMode = PlayMode::Sequential;

    /// Raised after every applied coordinator event, so the UI can re-render
    /// without polling (ticket #172/#173). Set by the main window.
    std::function<void()> OnStateChanged;

    void OpenDatabase(const std::wstring& path);
    void RefreshLibrary();
    void ImportDirectory(const std::wstring& path);
    void DoSearch();
    void PlayTrack(const Track& track);
    void TogglePlayPause();
    void SetVolume(double v);
    void ResolveAndPlay(const std::wstring& url);
    /// Import an M3U8 file: parse entries, persist each one, count failures,
    /// refresh, and surface the import alert (ticket #173 — the old no-op).
    void ImportM3U8(const std::wstring& path);

    /// Transport availability (WA-22, mirrors the macOS tray-menu gates).
    bool CanTogglePlayback() const;
    bool CanPlayNext() const;
    bool CanPlayPrevious() const;
    bool CanStop() const;

    /// Play the next/previous track in the queue (WA-19). No-ops when the
    /// queue is absent or exhausted.
    void PlayNext();
    void PlayPrevious();

    /// Cycle to the next play mode (WA-21).
    void CyclePlayMode();

    /// UI-thread dispatcher for marshalling async resolver results and
    /// coordinator events.
    void SetDispatcherQueue(winrt::Microsoft::UI::Dispatching::DispatcherQueue dq) {
        dispatcher_ = dq;
    }

    /// Apply a coordinator event JSON to the state (the seam the tests
    /// drive; events arrive via the coordinator's C callback and are
    /// marshalled to the UI thread when a dispatcher is set).
    void ApplyCoordinatorEvent(const std::wstring& json);

private:
    /// Coordinator event entry point: marshal to the UI thread when a
    /// dispatcher is available, otherwise apply directly (tests).
    void OnCoordinatorEvent(const std::wstring& json);

    winrt::Microsoft::UI::Dispatching::DispatcherQueue dispatcher_{ nullptr };
};

} // namespace rhythm
