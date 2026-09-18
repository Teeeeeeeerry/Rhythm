#pragma once

// Win32 interop the XAML shell needs (#428): a WinUI Window's HWND, and
// parenting the WinRT pickers to it (unpackaged desktop apps must do this
// before showing a picker).

#include <microsoft.ui.xaml.window.h>
#include <shobjidl_core.h>

namespace rhythm::shell {

inline HWND WindowHandle(winrt::Microsoft::UI::Xaml::Window const& window) {
    HWND hwnd{};
    winrt::check_hresult(window.as<::IWindowNative>()->get_WindowHandle(&hwnd));
    return hwnd;
}

template <typename Picker>
void ParentPicker(Picker const& picker, HWND owner) {
    winrt::check_hresult(picker.template as<::IInitializeWithWindow>()->Initialize(owner));
}

} // namespace rhythm::shell
