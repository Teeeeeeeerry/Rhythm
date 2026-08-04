#pragma once

#include "pch.h"
#include "Bridge/RhythmCore.h"

namespace rhythm {

enum class SidebarItem { Library, Playlists };

class AppState : public winrt::implements<AppState, winrt::Windows::Foundation::IInspectable> {
public:
    AppState();

    std::unique_ptr<Library> Library;
    std::unique_ptr<Player> Player;
    SidebarItem SelectedView = SidebarItem::Library;
    std::wstring SearchQuery;
    std::vector<Track> Tracks;
    std::vector<Playlist> Playlists;
    std::optional<Track> CurrentTrack;
    bool IsPlaying = false;
    double Volume = 1.0;
    double Position = 0.0;
    double Duration = 0.0;
    /// Last URL resolution failure, empty when the last attempt succeeded.
    std::wstring UrlError;
    /// Raised on the UI thread when a URL fails to resolve: (kind, message).
    std::function<void(const std::wstring&, const std::wstring&)> OnUrlError;

    void OpenDatabase(const std::wstring& path);
    void RefreshLibrary();
    void ImportDirectory(const std::wstring& path);
    void DoSearch();
    void PlayTrack(const Track& track);
    void TogglePlayPause();
    void SetVolume(double v);
    void ResolveAndPlay(const std::wstring& url);

    /// UI-thread dispatcher for marshalling async resolver results.
    void SetDispatcherQueue(winrt::Microsoft::UI::Dispatching::DispatcherQueue dq) {
        dispatcher_ = dq;
    }

private:
    winrt::Microsoft::UI::Dispatching::DispatcherQueue dispatcher_{ nullptr };
};

} // namespace rhythm
