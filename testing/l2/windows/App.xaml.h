#pragma once

#include "App.xaml.g.h"

namespace winrt::Rhythm::implementation {

/// The L2 capture host's application (#495): instead of opening the main
/// window for a user, it renders every managed view in both themes to PNG
/// files and exits. Usage: RhythmCapture.exe <output directory>.
struct App : AppT<App> {
    App();
    void OnLaunched(winrt::Microsoft::UI::Xaml::LaunchActivatedEventArgs const& args);
};

} // namespace winrt::Rhythm::implementation
