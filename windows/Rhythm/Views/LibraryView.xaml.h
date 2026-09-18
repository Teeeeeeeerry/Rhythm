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

    void OnPivotChanged(winrt::Windows::Foundation::IInspectable const&,
                        winrt::Microsoft::UI::Xaml::Controls::SelectionChangedEventArgs const&);
    void OnTrackClick(winrt::Windows::Foundation::IInspectable const&,
                      winrt::Microsoft::UI::Xaml::Controls::ItemClickEventArgs const& args);

private:
    /// Render the library in the order the pivot selects (#336).
    void Populate();
    void ShowRows(std::vector<rhythm::view::TrackRow> const& rows, bool isDark);
    void ShowEmptyMessage(bool show);

    rhythm::AppState* appState_ = nullptr;
};

} // namespace winrt::Rhythm::Views::implementation

namespace winrt::Rhythm::Views::factory_implementation {

struct LibraryView : LibraryViewT<LibraryView, implementation::LibraryView> {};

} // namespace winrt::Rhythm::Views::factory_implementation
