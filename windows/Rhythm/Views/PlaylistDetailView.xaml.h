#pragma once

#include "Views/PlaylistDetailView.g.h"
#include "AppState.h"

namespace winrt::Rhythm::Views::implementation {

struct PlaylistDetailView : PlaylistDetailViewT<PlaylistDetailView> {
    PlaylistDetailView() = default;

    void InitializeComponent();

    /// Navigation carries no parameter (#357): the playlist is the one the
    /// state has selected.
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
    /// Render from the state's current playlist (#358): the page holds no
    /// playlist of its own, so every render takes the values fresh.
    void Refresh();
    /// Show the import/export feedback the state asks for, if any (#352).
    winrt::Windows::Foundation::IAsyncAction ShowPendingAlert();

    rhythm::AppState* appState_ = nullptr;
    HWND owner_{};
};

} // namespace winrt::Rhythm::Views::implementation

namespace winrt::Rhythm::Views::factory_implementation {

struct PlaylistDetailView : PlaylistDetailViewT<PlaylistDetailView, implementation::PlaylistDetailView> {};

} // namespace winrt::Rhythm::Views::factory_implementation
