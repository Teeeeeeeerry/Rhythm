#pragma once

// The generated glue names Rhythm.Views types (the player bar element).
#include <winrt/Rhythm.Views.h>
#include "MainWindow.g.h"
#include "AppState.h"

namespace winrt::Rhythm::implementation {

struct MainWindow : MainWindowT<MainWindow> {
    MainWindow() = default;

    /// Named elements exist only once the generated InitializeComponent has
    /// run (C++/WinRT calls it after construction), so setup lives here.
    void InitializeComponent();

    void OnNavSelectionChanged(
        winrt::Microsoft::UI::Xaml::Controls::NavigationView const& sender,
        winrt::Microsoft::UI::Xaml::Controls::NavigationViewSelectionChangedEventArgs const& args);
    winrt::fire_and_forget OnImportClick(winrt::Windows::Foundation::IInspectable const&,
                                         winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    winrt::fire_and_forget OnImportFileClick(winrt::Windows::Foundation::IInspectable const&,
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
    void RefreshLibraryIfShown();
    /// Every page that lands in the content frame (including back
    /// navigation) gets the shared state here.
    void OnFrameNavigated(winrt::Windows::Foundation::IInspectable const&,
                          winrt::Microsoft::UI::Xaml::Navigation::NavigationEventArgs const& args);

    rhythm::AppState appState_;
    HWND hwnd_{};
    bool ready_ = false;
};

} // namespace winrt::Rhythm::implementation

namespace winrt::Rhythm::factory_implementation {

struct MainWindow : MainWindowT<MainWindow, implementation::MainWindow> {};

} // namespace winrt::Rhythm::factory_implementation
