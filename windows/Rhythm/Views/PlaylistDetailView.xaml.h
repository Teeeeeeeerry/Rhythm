#pragma once

#include "Views/PlaylistDetailView.g.h"
#include "AppState.h"

namespace winrt::Rhythm::Views::implementation {

struct PlaylistDetailView : PlaylistDetailViewT<PlaylistDetailView> {
    PlaylistDetailView() = default;

    void InitializeComponent();

    /// The navigation parameter is the playlist id.
    void OnNavigatedTo(winrt::Microsoft::UI::Xaml::Navigation::NavigationEventArgs const& args);
    /// Called by MainWindow once the page is in the frame (not projected).
    void BindState(rhythm::AppState* state, HWND owner);

    void OnBackClick(winrt::Windows::Foundation::IInspectable const&,
                     winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    winrt::fire_and_forget OnImportClick(winrt::Windows::Foundation::IInspectable const&,
                                         winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    winrt::fire_and_forget OnExportClick(winrt::Windows::Foundation::IInspectable const&,
                                         winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnTrackClick(winrt::Windows::Foundation::IInspectable const&,
                      winrt::Microsoft::UI::Xaml::Controls::ItemClickEventArgs const& args);

private:
    /// Render once both the id (navigation) and the state (MainWindow) are in.
    void Refresh();
    rhythm::Playlist const* CurrentPlaylist() const;
    /// Show the import/export feedback the state asks for, if any (#352).
    winrt::Windows::Foundation::IAsyncAction ShowPendingAlert();

    std::optional<int64_t> playlistId_;
    rhythm::AppState* appState_ = nullptr;
    HWND owner_{};
};

} // namespace winrt::Rhythm::Views::implementation

namespace winrt::Rhythm::Views::factory_implementation {

struct PlaylistDetailView : PlaylistDetailViewT<PlaylistDetailView, implementation::PlaylistDetailView> {};

} // namespace winrt::Rhythm::Views::factory_implementation
