#pragma once

#include "Views/LibraryTemplateSelector.g.h"

namespace winrt::Rhythm::Views::implementation {

/// The library list's template by line kind (#503). Only matches the item's
/// model type to its template; what the lines are comes from the view state.
struct LibraryTemplateSelector : LibraryTemplateSelectorT<LibraryTemplateSelector> {
    LibraryTemplateSelector() = default;

    winrt::Microsoft::UI::Xaml::DataTemplate HeaderTemplate() const { return header_; }
    void HeaderTemplate(winrt::Microsoft::UI::Xaml::DataTemplate const& value) { header_ = value; }
    winrt::Microsoft::UI::Xaml::DataTemplate AlbumTemplate() const { return album_; }
    void AlbumTemplate(winrt::Microsoft::UI::Xaml::DataTemplate const& value) { album_ = value; }
    winrt::Microsoft::UI::Xaml::DataTemplate TrackTemplate() const { return track_; }
    void TrackTemplate(winrt::Microsoft::UI::Xaml::DataTemplate const& value) { track_ = value; }

    winrt::Microsoft::UI::Xaml::DataTemplate SelectTemplateCore(
        winrt::Windows::Foundation::IInspectable const& item);
    winrt::Microsoft::UI::Xaml::DataTemplate SelectTemplateCore(
        winrt::Windows::Foundation::IInspectable const& item,
        winrt::Microsoft::UI::Xaml::DependencyObject const& container);

private:
    winrt::Microsoft::UI::Xaml::DataTemplate header_{nullptr};
    winrt::Microsoft::UI::Xaml::DataTemplate album_{nullptr};
    winrt::Microsoft::UI::Xaml::DataTemplate track_{nullptr};
};

} // namespace winrt::Rhythm::Views::implementation

namespace winrt::Rhythm::Views::factory_implementation {

struct LibraryTemplateSelector
    : LibraryTemplateSelectorT<LibraryTemplateSelector, implementation::LibraryTemplateSelector> {};

} // namespace winrt::Rhythm::Views::factory_implementation
