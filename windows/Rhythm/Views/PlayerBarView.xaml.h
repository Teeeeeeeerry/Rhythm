#pragma once

#include "../AppState.h"

namespace winrt::Rhythm::Views::implementation {

struct PlayerBarView : winrt::Microsoft::UI::Xaml::Controls::UserControlT<PlayerBarView> {
    PlayerBarView();

    void BindState(rhythm::AppState* state);

    void OnPlayPauseClick(winrt::Windows::Foundation::IInspectable const&,
                          winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnVolumeChanged(winrt::Windows::Foundation::IInspectable const&,
                         winrt::Microsoft::UI::Xaml::Controls::Primitives::RangeBaseValueChangedEventArgs const& args);
    void OnUrlPlayClick(winrt::Windows::Foundation::IInspectable const&,
                        winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnUrlKeyDown(winrt::Windows::Foundation::IInspectable const&,
                      winrt::Microsoft::UI::Xaml::Input::KeyRoutedEventArgs const& args);

    void Update();

private:
    rhythm::AppState* appState_ = nullptr;
};

} // namespace winrt::Rhythm::Views::implementation
