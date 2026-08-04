#include "pch.h"
#include "PlayerBarView.xaml.h"

namespace winrt::Rhythm::Views::implementation {

PlayerBarView::PlayerBarView() {
    InitializeComponent();
}

void PlayerBarView::BindState(rhythm::AppState* state) {
    appState_ = state;
    if (!appState_) return;

    appState_->OnUrlError = [this](const std::wstring& kind, const std::wstring& message) {
        ShowUrlError(kind, message);
    };
}

std::wstring PlayerBarView::UrlErrorText(const std::wstring& kind, const std::wstring& message) {
    std::wstring headline;
    if (kind == L"yt_dlp_missing") {
        headline =
            L"未找到 yt-dlp。播放 YouTube / Bilibili 链接需要先安装它：\n"
            L"  winget install yt-dlp   或   pip install yt-dlp\n\n"
            L"如果已经安装：应用不会继承你在终端里的 PATH，"
            L"请把 RHYTHM_YTDLP_PATH 设为 yt-dlp.exe 的完整路径。";
    } else if (kind == L"timeout") {
        headline = L"解析超时。请检查网络连接后重试。";
    } else if (kind == L"network") {
        headline = L"网络错误，无法访问该链接。请检查网络、代理或 VPN 设置。";
    } else if (kind == L"unavailable") {
        headline = L"该视频无法访问：可能是私享、已删除、年龄限制、会员专属或所在地区不可用。";
    } else if (kind == L"no_audio_stream") {
        headline = L"该链接没有可播放的音频流。";
    } else if (kind == L"yt_dlp_outdated") {
        headline = L"yt-dlp 版本过旧，无法解析该站点。请升级后重试：\n  pip install -U yt-dlp";
    } else if (kind == L"invalid_url") {
        headline = L"链接无效，请输入以 http:// 或 https:// 开头的地址。";
    } else {
        return message;
    }
    return headline + L"\n\n详细信息：\n" + message;
}

void PlayerBarView::ShowUrlError(const std::wstring& kind, const std::wstring& message) {
    winrt::Microsoft::UI::Xaml::Controls::ContentDialog dialog;
    dialog.XamlRoot(XamlRoot());
    dialog.Title(winrt::box_value(winrt::hstring{ L"无法播放链接" }));
    dialog.Content(winrt::box_value(winrt::hstring{ UrlErrorText(kind, message) }));
    dialog.CloseButtonText(L"确定");
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

    auto pos = appState_->Position;
    auto dur = appState_->Duration;
    auto pm = static_cast<int>(pos) / 60;
    auto ps = static_cast<int>(pos) % 60;
    auto dm = static_cast<int>(dur) / 60;
    auto ds = static_cast<int>(dur) % 60;
    timeText().Text(std::format(L"{}:{:02} / {}:{:02}", pm, ps, dm, ds));

    volumeSlider().Value(appState_->Volume * 100.0);

    if (appState_->IsResolvingUrl) {
        auto status = rhythm::Resolver::Status();
        urlStatus().Text(status.IsQuiet() ? L"解析中…"
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
