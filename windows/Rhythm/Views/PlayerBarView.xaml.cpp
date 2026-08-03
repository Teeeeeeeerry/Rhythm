#include "pch.h"
#include "PlayerBarView.xaml.h"

namespace winrt::Rhythm::Views::implementation {

PlayerBarView::PlayerBarView() {
    InitializeComponent();
}

void PlayerBarView::BindState(rhythm::AppState* state) {
    appState_ = state;
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

    auto pos = appState_->Position;
    auto dur = appState_->Duration;
    auto pm = static_cast<int>(pos) / 60;
    auto ps = static_cast<int>(pos) % 60;
    auto dm = static_cast<int>(dur) / 60;
    auto ds = static_cast<int>(dur) % 60;
    timeText().Text(std::format(L"{}:{:02} / {}:{:02}", pm, ps, dm, ds));

    volumeSlider().Value(appState_->Volume * 100.0);
}

void PlayerBarView::OnPlayPauseClick(IInspectable const&, RoutedEventArgs const&) {
    if (appState_) appState_->TogglePlayPause();
    Update();
}

void PlayerBarView::OnVolumeChanged(IInspectable const&,
                                    Primitives::RangeBaseValueChangedEventArgs const& args) {
    if (appState_) appState_->SetVolume(args.NewValue() / 100.0);
}

} // namespace winrt::Rhythm::Views::implementation
