#pragma once

#include "../AppState.h"

namespace winrt::Rhythm::Views::implementation {

struct PlaylistListView : winrt::Microsoft::UI::Xaml::Controls::UserControlT<PlaylistListView> {
    PlaylistListView();

    void OnNavigatedTo(winrt::Microsoft::UI::Xaml::Navigation::NavigationEventArgs const& args);
    void OnNewPlaylistClick(winrt::Windows::Foundation::IInspectable const&,
                            winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnSelectionChanged(winrt::Windows::Foundation::IInspectable const&,
                            winrt::Microsoft::UI::Xaml::Controls::SelectionChangedEventArgs const&);

private:
    void Refresh();
    rhythm::AppState* appState_ = nullptr;
};

} // namespace winrt::Rhythm::Views::implementation
