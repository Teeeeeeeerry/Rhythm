#pragma once

#include "pch.h"
#include "AppState.h"

namespace winrt::Rhythm::implementation {

struct MainWindow : winrt::Microsoft::UI::Xaml::WindowT<MainWindow> {
    MainWindow();

    void OnNavSelectionChanged(
        winrt::Microsoft::UI::Xaml::Controls::NavigationView const& sender,
        winrt::Microsoft::UI::Xaml::Controls::NavigationViewSelectionChangedEventArgs const& args);
    void OnImportClick(winrt::Windows::Foundation::IInspectable const&,
                       winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnSearchSubmitted(winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBox const& sender,
                           winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBoxQuerySubmittedEventArgs const& args);
    void OnSearchTextChanged(winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBox const& sender,
                             winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBoxTextChangedEventArgs const& args);
    void OnViewModeChanged(winrt::Windows::Foundation::IInspectable const&,
                           winrt::Microsoft::UI::Xaml::Controls::SelectionChangedEventArgs const& args);

private:
    void LoadLibraryView();
    void LoadPlaylistListView();

    rhythm::AppState appState_;
};

} // namespace winrt::Rhythm::implementation
