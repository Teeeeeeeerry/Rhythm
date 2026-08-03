#pragma once

#include "pch.h"

namespace winrt::Rhythm::implementation {

struct App : winrt::Microsoft::UI::Xaml::ApplicationT<App> {
    App();
    void OnLaunched(winrt::Microsoft::UI::Xaml::LaunchActivatedEventArgs const& args);
};

} // namespace winrt::Rhythm::implementation
