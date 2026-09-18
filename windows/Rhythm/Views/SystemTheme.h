#pragma once

// The effective app theme, resolved by the XAML shell (#342). The behaviour
// library takes the theme as a flag and never reads system settings itself.

#include <winrt/Windows.UI.ViewManagement.h>

namespace rhythm::shell {

/// The app never pins `Application.RequestedTheme`, so the UI follows the
/// system (the same resolution ThemeDictionaries use for `ActualTheme`).
/// Light foreground text means a dark system theme.
inline bool IsDarkTheme() {
    auto fg = winrt::Windows::UI::ViewManagement::UISettings()
                  .GetColorValue(winrt::Windows::UI::ViewManagement::UIColorType::Foreground);
    return (fg.R + fg.G + fg.B) / 3 >= 128;
}

} // namespace rhythm::shell
