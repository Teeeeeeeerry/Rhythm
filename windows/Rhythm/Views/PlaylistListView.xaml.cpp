#include "pch.h"
#include "Views/PlaylistListView.xaml.h"
#if __has_include("Views/PlaylistListView.g.cpp")
#include "Views/PlaylistListView.g.cpp"
#endif
#include "Models/PlaylistItem.h"
#include "L10n.h"
#include "ViewState.h"

using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using winrt::Windows::Foundation::IInspectable;

namespace winrt::Rhythm::Views::implementation {

void PlaylistListView::InitializeComponent() {
    PlaylistListViewT<PlaylistListView>::InitializeComponent();
    // #141: copy from the language layer.
    newPlaylistText().Text(rhythm::L10n::NewPlaylist());
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
    if (!appState_) return;
    auto item = args.ClickedItem().as<Rhythm::Models::PlaylistItem>();
    // #357: selecting is the state's job and navigation carries no value of
    // its own -- the detail page reads the current playlist from AppState,
    // so nothing derived from this list's storage outlives it.
    appState_->SelectPlaylist(get_self<Models::implementation::PlaylistItem>(item)->Id());
    // MainWindow binds the detail page's state when it lands in the frame.
    Frame().Navigate(winrt::xaml_typename<Rhythm::Views::PlaylistDetailView>());
}

void PlaylistListView::Refresh() {
    if (!appState_) return;
    // #320: the rows come from the view state, so the list and the detail
    // correspond by identity, not by position in the state's storage.
    auto page = rhythm::view::PlaylistListState(*appState_);
    const bool empty = page.rows.empty();
    emptyMessage().Text(page.emptyMessage);
    emptyMessage().Visibility(empty ? Visibility::Visible : Visibility::Collapsed);
    playlistList().Visibility(empty ? Visibility::Collapsed : Visibility::Visible);

    auto items = winrt::single_threaded_observable_vector<IInspectable>();
    for (auto& row : page.rows) {
        items.Append(winrt::make<Models::implementation::PlaylistItem>(std::move(row)));
    }
    playlistList().ItemsSource(items);
}

} // namespace winrt::Rhythm::Views::implementation
