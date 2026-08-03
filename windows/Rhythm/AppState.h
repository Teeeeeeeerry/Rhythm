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

    void OpenDatabase(const std::wstring& path);
    void RefreshLibrary();
    void ImportDirectory(const std::wstring& path);
    void DoSearch();
    void PlayTrack(const Track& track);
    void TogglePlayPause();
    void SetVolume(double v);
};

} // namespace rhythm
