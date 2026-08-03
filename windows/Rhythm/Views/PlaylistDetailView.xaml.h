#pragma once

#include "../AppState.h"

namespace winrt::Rhythm::Views::implementation {

struct PlaylistDetailView : winrt::Microsoft::UI::Xaml::Controls::UserControlT<PlaylistDetailView> {
    PlaylistDetailView();

    void OnNavigatedTo(winrt::Microsoft::UI::Xaml::Navigation::NavigationEventArgs const& args);
    void OnBackClick(winrt::Windows::Foundation::IInspectable const&,
                     winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnImportClick(winrt::Windows::Foundation::IInspectable const&,
                       winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnExportClick(winrt::Windows::Foundation::IInspectable const&,
                       winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnTrackClick(winrt::Windows::Foundation::IInspectable const&,
                      winrt::Microsoft::UI::Xaml::Controls::ItemClickEventArgs const& args);

private:
    rhythm::Playlist* playlist_ = nullptr;
    rhythm::AppState* appState_ = nullptr;
};

} // namespace winrt::Rhythm::Views::implementation
