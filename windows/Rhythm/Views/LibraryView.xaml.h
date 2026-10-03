#pragma once

#include "Views/LibraryView.g.h"
#include "AppState.h"
#include "ViewState.h"

namespace winrt::Rhythm::Views::implementation {

struct LibraryView : LibraryViewT<LibraryView> {
    LibraryView() = default;

    void InitializeComponent();

    /// Called by MainWindow once the page is in the frame (not projected).
    void BindState(rhythm::AppState* state);

    void OnSegmentClick(winrt::Windows::Foundation::IInspectable const& sender,
                        winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnTrackClick(winrt::Windows::Foundation::IInspectable const&,
                      winrt::Microsoft::UI::Xaml::Controls::ItemClickEventArgs const& args);
    void OnContainerContentChanging(
        winrt::Microsoft::UI::Xaml::Controls::ListViewBase const&,
        winrt::Microsoft::UI::Xaml::Controls::ContainerContentChangingEventArgs const& args);
    void OnSelectionChanged(winrt::Windows::Foundation::IInspectable const&,
                            winrt::Microsoft::UI::Xaml::Controls::SelectionChangedEventArgs const& args);

private:
    /// Render the view switch and the library in the state's order (#502).
    void Populate();
    void RenderViewSwitch();
    void ShowEmptyMessage(bool show);

    rhythm::AppState* appState_ = nullptr;
};

} // namespace winrt::Rhythm::Views::implementation

namespace winrt::Rhythm::Views::factory_implementation {

struct LibraryView : LibraryViewT<LibraryView, implementation::LibraryView> {};

} // namespace winrt::Rhythm::Views::factory_implementation
