#pragma once

#include "Views/PlaylistListView.g.h"
#include "AppState.h"

namespace winrt::Rhythm::Views::implementation {

struct PlaylistListView : PlaylistListViewT<PlaylistListView> {
    PlaylistListView() = default;

    void InitializeComponent();

    /// Called by MainWindow once the page is in the frame (not projected).
    void BindState(rhythm::AppState* state);

    winrt::fire_and_forget OnNewPlaylistClick(winrt::Windows::Foundation::IInspectable const&,
                                              winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnPlaylistClick(winrt::Windows::Foundation::IInspectable const&,
                         winrt::Microsoft::UI::Xaml::Controls::ItemClickEventArgs const& args);

private:
    void Refresh();
    rhythm::AppState* appState_ = nullptr;
};

} // namespace winrt::Rhythm::Views::implementation

namespace winrt::Rhythm::Views::factory_implementation {

struct PlaylistListView : PlaylistListViewT<PlaylistListView, implementation::PlaylistListView> {};

} // namespace winrt::Rhythm::Views::factory_implementation
