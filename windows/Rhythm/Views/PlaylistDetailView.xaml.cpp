#include "pch.h"
#include "PlaylistDetailView.xaml.h"
#include "L10n.h"

#include <nlohmann/json.hpp>

namespace winrt::Rhythm::Views::implementation {

using json = nlohmann::json;

namespace {

std::string WideToUtf8(const std::wstring& ws) {
    if (ws.empty()) return {};
    int len = WideCharToMultiByte(CP_UTF8, 0, ws.data(), (int)ws.size(), nullptr, 0, nullptr, nullptr);
    std::string result(len, '\0');
    WideCharToMultiByte(CP_UTF8, 0, ws.data(), (int)ws.size(), result.data(), len, nullptr, nullptr);
    return result;
}

json TrackToJson(const rhythm::Track& t) {
    json j;
    j["id"] = t.id;
    if (t.filePath) j["file_path"] = WideToUtf8(*t.filePath); else j["file_path"] = nullptr;
    j["source_type"] = WideToUtf8(t.sourceType);
    if (t.sourceUrl) j["source_url"] = WideToUtf8(*t.sourceUrl); else j["source_url"] = nullptr;
    j["title"] = WideToUtf8(t.title);
    if (t.artist) j["artist"] = WideToUtf8(*t.artist); else j["artist"] = nullptr;
    if (t.album) j["album"] = WideToUtf8(*t.album); else j["album"] = nullptr;
    if (t.albumArtist) j["album_artist"] = WideToUtf8(*t.albumArtist); else j["album_artist"] = nullptr;
    if (t.trackNumber) j["track_number"] = *t.trackNumber; else j["track_number"] = nullptr;
    if (t.discNumber) j["disc_number"] = *t.discNumber; else j["disc_number"] = nullptr;
    if (t.genre) j["genre"] = WideToUtf8(*t.genre); else j["genre"] = nullptr;
    if (t.year) j["year"] = *t.year; else j["year"] = nullptr;
    j["duration"] = t.duration;
    if (t.format) j["format"] = WideToUtf8(*t.format); else j["format"] = nullptr;
    if (t.bitrate) j["bitrate"] = *t.bitrate; else j["bitrate"] = nullptr;
    if (t.sampleRate) j["sample_rate"] = *t.sampleRate; else j["sample_rate"] = nullptr;
    if (t.channels) j["channels"] = *t.channels; else j["channels"] = nullptr;
    if (t.fileSize) j["file_size"] = *t.fileSize; else j["file_size"] = nullptr;
    if (t.dateAdded) j["date_added"] = WideToUtf8(*t.dateAdded); else j["date_added"] = nullptr;
    if (t.lastPlayed) j["last_played"] = WideToUtf8(*t.lastPlayed); else j["last_played"] = nullptr;
    j["play_count"] = t.playCount;
    if (t.artworkPath) j["artwork_path"] = WideToUtf8(*t.artworkPath); else j["artwork_path"] = nullptr;
    j["is_available"] = t.isAvailable;
    return j;
}

std::string SerializeTracks(const std::vector<rhythm::Track>& tracks) {
    json arr = json::array();
    for (const auto& t : tracks) {
        arr.push_back(TrackToJson(t));
    }
    return arr.dump();
}

} // anonymous namespace

PlaylistDetailView::PlaylistDetailView() {
    InitializeComponent();
    // #141: copy from the language layer.
    btnImport().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::ImportM3U8() }));
    btnExport().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::ExportM3U8() }));
}

void PlaylistDetailView::OnNavigatedTo(Navigation::NavigationEventArgs const& args) {
    if (!args.Parameter()) return;
    playlist_ = winrt::unbox_value<rhythm::Playlist*>(args.Parameter());
    if (!playlist_) return;

    playlistTitle().Text(playlist_->name);

    auto items = winrt::single_threaded_observable_vector<IInspectable>();
    for (auto& track : playlist_->tracks) {
        items.Append(box_value(track));
    }
    trackList().ItemsSource(items);
}

void PlaylistDetailView::OnBackClick(IInspectable const&, RoutedEventArgs const&) {
    if (auto frame = this->Parent().try_as<Frame>()) {
        frame.GoBack();
    }
}

void PlaylistDetailView::OnImportClick(IInspectable const&, RoutedEventArgs const&) {
    auto picker = WinRT::Windows::Storage::Pickers::FileOpenPicker();
    picker.FileTypeFilter().Append(L".m3u8");
    picker.FileTypeFilter().Append(L".m3u");

    auto hwnd = winrt::Microsoft::UI::GetWindowFromElement(*this).GetWindowHandle();
    picker.as<IInitializeWithWindow>()->Initialize(hwnd);

    picker.PickSingleFileAsync().Completed([this](auto const& op, auto) {
        if (auto file = op.GetResults()) {
            char* raw = rhythm_import_m3u8(winrt::to_string(file.Path()).c_str());
            if (raw) rhythm_free_string(raw);
            if (appState_) appState_->RefreshLibrary();
        }
    });
}

void PlaylistDetailView::OnExportClick(IInspectable const&, RoutedEventArgs const&) {
    if (!playlist_) return;

    auto picker = WinRT::Windows::Storage::Pickers::FileSavePicker();
    picker.SuggestedFileName(playlist_->name);
    picker.FileTypeChoices().Insert(L"M3U8", { L".m3u8" });

    auto hwnd = winrt::Microsoft::UI::GetWindowFromElement(*this).GetWindowHandle();
    picker.as<IInitializeWithWindow>()->Initialize(hwnd);

    picker.PickSaveFileAsync().Completed([this](auto const& op, auto) {
        if (auto file = op.GetResults()) {
            std::string json = SerializeTracks(playlist_->tracks);
            int32_t result = rhythm_export_m3u8(winrt::to_string(file.Path()).c_str(), json.c_str());
            if (result != 0) {
                OutputDebugStringA("M3U8 export failed\n");
            }
        }
    });
}

void PlaylistDetailView::OnTrackClick(IInspectable const&, ItemClickEventArgs const& args) {
    if (!appState_) return;
    auto track = winrt::unbox_value<rhythm::Track>(args.ClickedItem());
    appState_->PlayTrack(track);
}

} // namespace winrt::Rhythm::Views::implementation
