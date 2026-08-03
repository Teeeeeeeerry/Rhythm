#pragma once

#include "pch.h"
#include "../AppState.h"

namespace winrt::Rhythm::Views::implementation {

struct LibraryView : winrt::Microsoft::UI::Xaml::Controls::UserControlT<LibraryView> {
    LibraryView();

    void OnNavigatedTo(winrt::Microsoft::UI::Xaml::Navigation::NavigationEventArgs const& args);
    void OnPivotChanged(winrt::Windows::Foundation::IInspectable const&,
                        winrt::Microsoft::UI::Xaml::Controls::SelectionChangedEventArgs const&);
    void OnTrackClick(winrt::Windows::Foundation::IInspectable const&,
                      winrt::Microsoft::UI::Xaml::Controls::ItemClickEventArgs const& args);

private:
    void PopulateArtistAlbum();
    void PopulateAlphabetical();
    void ShowEmptyMessage(bool show);

    rhythm::AppState* appState_ = nullptr;
};

} // namespace winrt::Rhythm::Views::implementation
