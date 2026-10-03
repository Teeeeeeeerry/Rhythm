#pragma once

// The effective app theme, resolved by the XAML shell (#342). The behaviour
// library takes the theme as a flag and never reads system settings itself.

#include <winrt/Windows.UI.ViewManagement.h>

namespace rhythm::shell {

/// The theme a view renders in, given the view's `RequestedTheme`. A theme
/// pinned on the view itself wins (the L2 capture host pins Dark and Light,
/// #495); otherwise the UI follows the system, since the app never pins `Application.RequestedTheme` (the same
/// resolution ThemeDictionaries use for `ActualTheme`). Light foreground text
/// means a dark system theme.
inline bool IsDarkTheme(winrt::Microsoft::UI::Xaml::ElementTheme pinned) {
    using winrt::Microsoft::UI::Xaml::ElementTheme;
    switch (pinned) {
        case ElementTheme::Dark:  return true;
        case ElementTheme::Light: return false;
        default:                  break;
    }
    auto fg = winrt::Windows::UI::ViewManagement::UISettings()
                  .GetColorValue(winrt::Windows::UI::ViewManagement::UIColorType::Foreground);
    return (fg.R + fg.G + fg.B) / 3 >= 128;
}

} // namespace rhythm::shell
