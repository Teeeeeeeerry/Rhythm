#include "pch.h"
#include "Views/LibraryTemplateSelector.h"
#include <winrt/Rhythm.Models.h>
#if __has_include("Views/LibraryTemplateSelector.g.cpp")
#include "Views/LibraryTemplateSelector.g.cpp"
#endif

using winrt::Microsoft::UI::Xaml::DataTemplate;
using winrt::Microsoft::UI::Xaml::DependencyObject;
using winrt::Windows::Foundation::IInspectable;

namespace winrt::Rhythm::Views::implementation {

DataTemplate LibraryTemplateSelector::SelectTemplateCore(IInspectable const& item) {
    if (item.try_as<Rhythm::Models::LibraryHeaderItem>()) return header_;
    if (item.try_as<Rhythm::Models::AlbumGroupItem>()) return album_;
    return track_;
}

DataTemplate LibraryTemplateSelector::SelectTemplateCore(IInspectable const& item,
                                                          DependencyObject const&) {
    return SelectTemplateCore(item);
}

} // namespace winrt::Rhythm::Views::implementation
