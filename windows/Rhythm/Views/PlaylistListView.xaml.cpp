#include "pch.h"
#include "Views/PlaylistListView.xaml.h"
#if __has_include("Views/PlaylistListView.g.cpp")
#include "Views/PlaylistListView.g.cpp"
#endif
#include "Models/PlaylistItem.h"
#include "L10n.h"

using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using winrt::Windows::Foundation::IInspectable;

namespace winrt::Rhythm::Views::implementation {

void PlaylistListView::InitializeComponent() {
    PlaylistListViewT<PlaylistListView>::InitializeComponent();
    // #141: copy from the language layer.
    newPlaylistText().Text(rhythm::L10n::NewPlaylist());
    emptyMessage().Text(rhythm::L10n::PlaylistEmpty());
}

void PlaylistListView::BindState(rhythm::AppState* state) {
    appState_ = state;
    Refresh();
}

winrt::fire_and_forget PlaylistListView::OnNewPlaylistClick(IInspectable const&, RoutedEventArgs const&) {
    if (!appState_) co_return;
    auto lifetime = get_strong();

    // Simple name input dialog
    TextBox tb;
    tb.PlaceholderText(rhythm::L10n::PlaylistNamePlaceholder());
    tb.Width(200);

    ContentDialog dialog;
    dialog.XamlRoot(XamlRoot());
    dialog.Title(winrt::box_value(winrt::hstring{ rhythm::L10n::NewPlaylist() }));
    dialog.Content(tb);
    dialog.PrimaryButtonText(rhythm::L10n::Create());
    dialog.CloseButtonText(rhythm::L10n::Cancel());
    dialog.DefaultButton(ContentDialogButton::Primary);

    try {
        if (co_await dialog.ShowAsync() != ContentDialogResult::Primary) co_return;
    } catch (winrt::hresult_error const& e) {
        // An exception leaving a fire_and_forget coroutine ends the process.
        OutputDebugStringW((L"New playlist dialog failed: " + e.message() + L"\n").c_str());
        co_return;
    }
    if (!appState_) co_return;
    // #348: creating is the app state's job; the view holds no library handle.
    appState_->CreatePlaylist(std::wstring{ tb.Text() });
    Refresh();
}

void PlaylistListView::OnPlaylistClick(IInspectable const&, ItemClickEventArgs const& args) {
    auto item = args.ClickedItem().as<Rhythm::Models::PlaylistItem>();
    auto id = get_self<Models::implementation::PlaylistItem>(item)->Model().id;
    if (!id || !appState_) return;
    // #357: selecting is the state's job and navigation carries no value of
    // its own -- the detail page reads the current playlist from AppState,
    // so nothing derived from this list's storage outlives it.
    appState_->SelectPlaylist(*id);
    // MainWindow binds the detail page's state when it lands in the frame.
    Frame().Navigate(winrt::xaml_typename<Rhythm::Views::PlaylistDetailView>());
}

void PlaylistListView::Refresh() {
    if (!appState_) return;
    auto const& playlists = appState_->Playlists;
    emptyMessage().Visibility(playlists.empty() ? Visibility::Visible : Visibility::Collapsed);
    playlistList().Visibility(playlists.empty() ? Visibility::Collapsed : Visibility::Visible);

    auto items = winrt::single_threaded_observable_vector<IInspectable>();
    for (const auto& pl : playlists) {
        items.Append(winrt::make<Models::implementation::PlaylistItem>(pl));
    }
    playlistList().ItemsSource(items);
}

} // namespace winrt::Rhythm::Views::implementation
