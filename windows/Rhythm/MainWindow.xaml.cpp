#include "pch.h"
#include "MainWindow.xaml.h"
#if __has_include("MainWindow.g.cpp")
#include "MainWindow.g.cpp"
#endif
#include "Views/LibraryView.xaml.h"
#include "Views/PlaylistListView.xaml.h"
#include "Views/PlaylistDetailView.xaml.h"
#include "Views/PlayerBarView.xaml.h"
#include "Views/Win32Interop.h"
#include "ShellHost.h"
#include "L10n.h"
#include "ViewState.h"

#include <winrt/Microsoft.UI.Xaml.Automation.h>

using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using winrt::Windows::Foundation::IInspectable;

namespace winrt::Rhythm::implementation {

namespace {

/// The view state's sidebar icon as a XAML symbol (#501).
Symbol SidebarSymbol(rhythm::view::Icon icon) {
    return icon == rhythm::view::Icon::Playlists ? Symbol::List : Symbol::MusicInfo;
}

} // namespace

void MainWindow::InitializeComponent() {
    MainWindowT<MainWindow>::InitializeComponent();
    hwnd_ = rhythm::shell::WindowHandle(*this);
    AppWindow().Resize({960, 600});

    // #141: all static copy comes from the language layer (system UI
    // language, manual override in L10n::SetOverrideLanguage).
    ToolTipService::SetToolTip(btnImport(), winrt::box_value(winrt::hstring{ rhythm::L10n::ImportFolderTooltip() }));
    ToolTipService::SetToolTip(btnImportFile(), winrt::box_value(winrt::hstring{ rhythm::L10n::ImportTooltip() }));
    searchBox().PlaceholderText(rhythm::L10n::SearchPlaceholder());

    // The hosting process decides where the library lives (#495).
    appState_.OpenDatabase(rhythm::shell::LibraryDatabasePath());

    // Wire the player bar to the shared state. The UI thread is this
    // window's DispatcherQueue -- a WinUI type, so the shell adapts it (#326).
    if (auto dq = DispatcherQueue()) {
        appState_.SetUiPost([dq](std::function<void()> work) {
            dq.TryEnqueue([work = std::move(work)] { work(); });
        });
    }
    get_self<Views::implementation::PlayerBarView>(playerBar())->BindState(&appState_);

    // #172/#173: playback state, progress, auto-advance, and failure
    // reporting arrive as coordinator events — the old 500 ms polling timer
    // is gone. The player bar re-renders after every applied event.
    appState_.OnStateChanged = [this] {
        get_self<Views::implementation::PlayerBarView>(playerBar())->Update();
    };

    // #494: the library page follows the data. Every reload of the lists --
    // any import path, including the online one that finishes on a resolver
    // thread -- arrives here on the UI thread; no import entry refreshes the
    // page by hand any more.
    appState_.OnLibraryChanged = [this] { RefreshLibraryIfShown(); };

    // #516: an import runs off the UI thread; its start and end swap the
    // import buttons for the progress indicator and back.
    appState_.OnImportingChanged = [this] { RenderImportControls(); };

    rhythm::shell::AttachTray(hwnd_, &appState_);
    Closed([](auto&&, auto&&) { rhythm::shell::DetachTray(); });

    contentFrame().Navigated({ this, &MainWindow::OnFrameNavigated });
    RenderSidebar();
    RenderImportControls();
    ready_ = true;
    LoadLibraryView();
}

void MainWindow::OnFrameNavigated(IInspectable const&,
                                  Navigation::NavigationEventArgs const& args) {
    auto content = args.Content();
    if (auto page = content.try_as<Rhythm::Views::LibraryView>()) {
        get_self<Views::implementation::LibraryView>(page)->BindState(&appState_);
    } else if (auto page = content.try_as<Rhythm::Views::PlaylistListView>()) {
        get_self<Views::implementation::PlaylistListView>(page)->BindState(&appState_);
    } else if (auto page = content.try_as<Rhythm::Views::PlaylistDetailView>()) {
        get_self<Views::implementation::PlaylistDetailView>(page)->BindState(&appState_, hwnd_);
    }
}

void MainWindow::OnSidebarClick(IInspectable const& sender, RoutedEventArgs const&) {
    if (!ready_) return;
    // The clicked slot's page is whatever the view state put in that slot.
    const RadioButton buttons[] = {sideLibrary(), sidePlaylists()};
    const auto entries = rhythm::view::SidebarState(appState_);
    for (size_t i = 0; i < entries.size() && i < std::size(buttons); ++i) {
        if (sender != buttons[i]) continue;
        appState_.SelectedView = entries[i].item;
        if (entries[i].item == rhythm::SidebarItem::Library) {
            LoadLibraryView();
        } else {
            LoadPlaylistListView();
        }
        RenderSidebar();
        RenderImportControls();
        return;
    }
}

void MainWindow::RenderSidebar() {
    struct Slot {
        RadioButton button;
        Border highlight;
        SymbolIcon icon;
        TextBlock label;
    };
    const Slot slots[] = {
        {sideLibrary(), sideLibraryHighlight(), sideLibraryIcon(), sideLibraryLabel()},
        {sidePlaylists(), sidePlaylistsHighlight(), sidePlaylistsIcon(), sidePlaylistsLabel()},
    };
    auto style = [this](const wchar_t* key) {
        return rootGrid().Resources().Lookup(winrt::box_value(key)).as<Style>();
    };
    const auto entries = rhythm::view::SidebarState(appState_);
    for (size_t i = 0; i < entries.size() && i < std::size(slots); ++i) {
        const auto& entry = entries[i];
        const auto& slot = slots[i];
        slot.icon.Symbol(SidebarSymbol(entry.icon));
        slot.label.Text(entry.label);
        // The button's content is a layout, so screen readers need the name.
        Automation::AutomationProperties::SetName(slot.button, entry.label);
        // #516: checked mirrors the view state, so UI Automation reads the
        // selected entry through the radio button's selection item.
        slot.button.IsChecked(entry.selected);
        slot.highlight.Style(style(entry.selected ? L"SidebarHighlightSelectedStyle"
                                                  : L"SidebarHighlightStyle"));
        slot.icon.Style(style(entry.selected ? L"SidebarIconSelectedStyle"
                                             : L"SidebarIconStyle"));
        slot.label.Style(style(entry.selected ? L"SidebarLabelSelectedStyle"
                                              : L"SidebarLabelStyle"));
    }
}

/// The toolbar's import controls (#516): which of the buttons and the
/// progress indicator show comes from the view state.
void MainWindow::RenderImportControls() {
    const auto controls = rhythm::view::ImportControlsState(appState_);
    const auto buttons = controls.showsButtons ? Visibility::Visible : Visibility::Collapsed;
    btnImport().Visibility(buttons);
    btnImportFile().Visibility(buttons);
    importProgress().Visibility(controls.showsProgress ? Visibility::Visible : Visibility::Collapsed);
    importProgress().IsActive(controls.showsProgress);
    ToolTipService::SetToolTip(importProgress(), winrt::box_value(winrt::hstring{ controls.progressLabel }));
    Automation::AutomationProperties::SetName(importProgress(), controls.progressLabel);
}

winrt::fire_and_forget MainWindow::OnImportClick(IInspectable const&, RoutedEventArgs const&) {
    auto lifetime = get_strong();
    winrt::Windows::Storage::Pickers::FolderPicker picker;
    picker.SuggestedStartLocation(
        winrt::Windows::Storage::Pickers::PickerLocationId::MusicLibrary);
    picker.FileTypeFilter().Append(L"*");
    rhythm::shell::ParentPicker(picker, hwnd_);

    try {
        // Awaited from the UI thread, so the continuation is back on it.
        auto folder = co_await picker.PickSingleFolderAsync();
        if (!folder) co_return;
        appState_.ImportDirectory(folder.Path().c_str());
    } catch (winrt::hresult_error const& e) {
        // An exception leaving a fire_and_forget coroutine ends the process.
        OutputDebugStringW((L"Folder import failed: " + e.message() + L"\n").c_str());
    }
}

/// #242/#243: Windows can import audio files, not only a folder, and more
/// than one at a time -- the capability macOS has always had. One file goes
/// through the single-file path, several through the core's batch import.
winrt::fire_and_forget MainWindow::OnImportFileClick(IInspectable const&, RoutedEventArgs const&) {
    auto lifetime = get_strong();
    winrt::Windows::Storage::Pickers::FileOpenPicker picker;
    picker.SuggestedStartLocation(
        winrt::Windows::Storage::Pickers::PickerLocationId::MusicLibrary);
    for (const auto& ext : rhythm::kAudioFileTypes) {
        picker.FileTypeFilter().Append(winrt::hstring{ ext });
    }
    rhythm::shell::ParentPicker(picker, hwnd_);

    try {
        auto files = co_await picker.PickMultipleFilesAsync();
        if (!files || files.Size() == 0) co_return;
        if (files.Size() == 1) {
            appState_.ImportFile(files.GetAt(0).Path().c_str());
        } else {
            std::vector<std::wstring> paths;
            for (const auto& file : files) {
                paths.emplace_back(file.Path().c_str());
            }
            appState_.ImportPaths(paths);
        }
    } catch (winrt::hresult_error const& e) {
        OutputDebugStringW((L"File import failed: " + e.message() + L"\n").c_str());
    }
}

void MainWindow::OnSearchSubmitted(AutoSuggestBox const& sender,
                                   AutoSuggestBoxQuerySubmittedEventArgs const&) {
    appState_.SearchQuery = sender.Text().c_str();
    appState_.DoSearch();
    RefreshLibraryIfShown();
}

void MainWindow::OnSearchTextChanged(AutoSuggestBox const& sender,
                                     AutoSuggestBoxTextChangedEventArgs const&) {
    if (!ready_) return;
    if (sender.Text().empty()) {
        appState_.SearchQuery = L"";
        appState_.DoSearch();
        RefreshLibraryIfShown();
    }
}

/// Library content changed: re-render only when that page is showing, so the
/// frame never switches away from Playlists behind the sidebar.
void MainWindow::RefreshLibraryIfShown() {
    if (appState_.SelectedView == rhythm::SidebarItem::Library) LoadLibraryView();
}

// Top-level pages start a fresh history: only the playlist detail page goes
// "back" (to the list), never across tabs.
void MainWindow::LoadLibraryView() {
    contentFrame().Navigate(winrt::xaml_typename<Rhythm::Views::LibraryView>());
    contentFrame().BackStack().Clear();
}

void MainWindow::LoadPlaylistListView() {
    contentFrame().Navigate(winrt::xaml_typename<Rhythm::Views::PlaylistListView>());
    contentFrame().BackStack().Clear();
}

} // namespace winrt::Rhythm::implementation
