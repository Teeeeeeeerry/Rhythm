#pragma once

#include "Views/PlayerBarView.g.h"
#include "AppState.h"

namespace winrt::Rhythm::Views::implementation {

struct PlayerBarView : PlayerBarViewT<PlayerBarView> {
    PlayerBarView() = default;

    void InitializeComponent();

    /// Called by MainWindow (not projected).
    void BindState(rhythm::AppState* state);

    void OnPlayPauseClick(winrt::Windows::Foundation::IInspectable const&,
                          winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnPlayModeClick(winrt::Windows::Foundation::IInspectable const&,
                         winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnVolumeChanged(winrt::Windows::Foundation::IInspectable const&,
                         winrt::Microsoft::UI::Xaml::Controls::Primitives::RangeBaseValueChangedEventArgs const& args);
    void OnUrlPlayClick(winrt::Windows::Foundation::IInspectable const&,
                        winrt::Microsoft::UI::Xaml::RoutedEventArgs const&);
    void OnUrlKeyDown(winrt::Windows::Foundation::IInspectable const&,
                      winrt::Microsoft::UI::Xaml::Input::KeyRoutedEventArgs const& args);

    void Update();

private:
    /// 弹出错误对话框，内容取 AppState 已本地化的 `UrlError`（#230）。
    void ShowUrlError();

    rhythm::AppState* appState_ = nullptr;
};

} // namespace winrt::Rhythm::Views::implementation

namespace winrt::Rhythm::Views::factory_implementation {

struct PlayerBarView : PlayerBarViewT<PlayerBarView, implementation::PlayerBarView> {};

} // namespace winrt::Rhythm::Views::factory_implementation
