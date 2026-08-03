#pragma once

namespace winrt::Rhythm::Views::implementation {
struct SidebarView : winrt::Microsoft::UI::Xaml::Controls::UserControlT<SidebarView> {
    SidebarView();
    void OnSelectionChanged(winrt::Windows::Foundation::IInspectable const&,
                            winrt::Microsoft::UI::Xaml::Controls::SelectionChangedEventArgs const&);
};
} // namespace winrt::Rhythm::Views::implementation
