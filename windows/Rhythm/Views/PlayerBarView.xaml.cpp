#include "pch.h"
#include "PlayerBarView.xaml.h"
#include "L10n.h"

namespace winrt::Rhythm::Views::implementation {

PlayerBarView::PlayerBarView() {
    InitializeComponent();
    // #141: static copy from the language layer.
    trackTitle().Text(rhythm::L10n::NotPlaying());
    urlBox().PlaceholderText(rhythm::L10n::UrlPlaceholder());
    btnUrlPlay().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::PlayUrl() }));
}

void PlayerBarView::BindState(rhythm::AppState* state) {
    appState_ = state;
    if (!appState_) return;

    appState_->OnUrlError = [this](const std::wstring&, const std::wstring&) {
        // #230: 分派在核心，AppState 已在每个失败处一次性本地化；本层
        // 只渲染，UI 侧不再有第二个错误分派入口（macOS 一直如此）。
        ShowUrlError();
    };
}

void PlayerBarView::ShowUrlError() {
    winrt::Microsoft::UI::Xaml::Controls::ContentDialog dialog;
    dialog.XamlRoot(XamlRoot());
    dialog.Title(winrt::box_value(winrt::hstring{ rhythm::L10n::UrlErrorTitle() }));
    dialog.Content(winrt::box_value(winrt::hstring{ appState_->UrlError }));
    dialog.CloseButtonText(rhythm::L10n::Ok());
    dialog.ShowAsync();
}

void PlayerBarView::Update() {
    if (!appState_) return;

    if (appState_->CurrentTrack) {
        trackTitle().Text(appState_->CurrentTrack->title);
        if (appState_->CurrentTrack->artist) {
            trackArtist().Text(*appState_->CurrentTrack->artist);
        } else {
            trackArtist().Text(L"");
        }
    }

    playIcon().Symbol(
        appState_->IsPlaying ? Symbol::Pause : Symbol::Play);

    if (appState_->Duration > 0) {
        progressBar().Value(appState_->Position / appState_->Duration * 100.0);
    }

    // #137: resolving + connecting + prebuffering can take a while on a
    // link; showing 0:00 / 0:00 for all of it reads as a dead player
    // (mirrors macOS L10n.buffering).
    if (appState_->IsBuffering) {
        timeText().Text(rhythm::L10n::Buffering());
    } else {
        auto pos = appState_->Position;
        auto dur = appState_->Duration;
        auto pm = static_cast<int>(pos) / 60;
        auto ps = static_cast<int>(pos) % 60;
        auto dm = static_cast<int>(dur) / 60;
        auto ds = static_cast<int>(dur) % 60;
        timeText().Text(std::format(L"{}:{:02} / {}:{:02}", pm, ps, dm, ds));
    }

    volumeSlider().Value(appState_->Volume * 100.0);

    if (appState_->IsResolvingUrl) {
        auto status = rhythm::Resolver::Status();
        urlStatus().Text(status.IsQuiet() ? rhythm::L10n::Resolving()
                                          : rhythm::Resolver::StatusText(status));
    } else {
        urlStatus().Text(L"");
    }
}

void PlayerBarView::OnPlayPauseClick(IInspectable const&, RoutedEventArgs const&) {
    if (appState_) appState_->TogglePlayPause();
    Update();
}

void PlayerBarView::OnVolumeChanged(IInspectable const&,
                                    Primitives::RangeBaseValueChangedEventArgs const& args) {
    if (appState_) appState_->SetVolume(args.NewValue() / 100.0);
}

void PlayerBarView::OnUrlPlayClick(IInspectable const&, RoutedEventArgs const&) {
    if (appState_) appState_->ResolveAndPlay(urlBox().Text().c_str());
}

void PlayerBarView::OnUrlKeyDown(IInspectable const&,
                                 winrt::Microsoft::UI::Xaml::Input::KeyRoutedEventArgs const& args) {
    if (args.Key() == winrt::Windows::System::VirtualKey::Enter && appState_) {
        appState_->ResolveAndPlay(urlBox().Text().c_str());
    }
}

} // namespace winrt::Rhythm::Views::implementation
