#include "pch.h"
#include "Views/PlayerBarView.xaml.h"
#if __has_include("Views/PlayerBarView.g.cpp")
#include "Views/PlayerBarView.g.cpp"
#endif
#include "L10n.h"
#include "ViewState.h"

using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using winrt::Windows::Foundation::IInspectable;

namespace winrt::Rhythm::Views::implementation {

namespace {

/// The view state's icon name as a XAML symbol (#329/#335).
Symbol ToSymbol(rhythm::view::Icon icon) {
    using rhythm::view::Icon;
    switch (icon) {
        case Icon::Play:      return Symbol::Play;
        case Icon::Pause:     return Symbol::Pause;
        case Icon::Shuffle:   return Symbol::Shuffle;
        case Icon::RepeatOne: return Symbol::RepeatOne;
        case Icon::RepeatAll: return Symbol::RepeatAll;
        case Icon::List:
        default:              return Symbol::List;
    }
}

} // namespace

void PlayerBarView::InitializeComponent() {
    PlayerBarViewT<PlayerBarView>::InitializeComponent();
    // #141: static copy from the language layer.
    urlBox().PlaceholderText(rhythm::L10n::UrlPlaceholder());
    btnUrlPlay().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::PlayUrl() }));
    // #373: the play mode control's tooltip, taken from the key table.
    ToolTipService::SetToolTip(btnPlayMode(),
                               winrt::box_value(winrt::hstring{ rhythm::L10n::PlayModeTooltip() }));
}

void PlayerBarView::BindState(rhythm::AppState* state) {
    appState_ = state;
    if (!appState_) return;
    Update();  // the first frame comes from the view state too (#334)

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
    // What to render is decided by the view state (#317); this only copies
    // the values into the controls.
    auto bar = rhythm::view::PlayerBarState(*appState_);

    trackTitle().Text(bar.title);
    trackArtist().Text(bar.artist);

    playIcon().Symbol(ToSymbol(bar.playIcon));
    playModeIcon().Symbol(ToSymbol(bar.playModeIcon));

    progressBar().Value(bar.progressPercent);

    timeText().Text(bar.timeText);

    volumeSlider().Value(bar.volumePercent);

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

void PlayerBarView::OnPlayModeClick(IInspectable const&, RoutedEventArgs const&) {
    if (appState_) appState_->CyclePlayMode();
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
