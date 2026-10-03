// L2: Windows view capture (#495, the gap registered by #387).
//
// The managed views are rendered inside this process with RenderTargetBitmap
// -- never by screen capture, which comes back black from an agent session --
// in the dark and the light theme, and written as <View>_<Dark|Light>.png to
// the output directory. compare_screenshots.py compares them with the golden
// images committed next to this file.
//
// Determinism: the UI language is pinned to English for this process only,
// the main window opens an empty scratch library, and the library page and
// player bar are fed fixed data (CaptureFixture.h). The user's library, tray
// and language preference are never touched.

#include "pch.h"
#include "App.xaml.h"
#include "MainWindow.xaml.h"
#include "Views/LibraryView.xaml.h"
#include "Views/PlayerBarView.xaml.h"
#include "CaptureFixture.h"
#include "L10n.h"

#include <winrt/Microsoft.UI.Xaml.Markup.h>
#include <winrt/Microsoft.UI.Xaml.Media.Imaging.h>
#include <winrt/Windows.Graphics.Imaging.h>
#include <winrt/Windows.Storage.Streams.h>

#include <chrono>
#include <cmath>
#include <fstream>
#include <shellapi.h>

using namespace winrt;
using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using winrt::Microsoft::UI::Xaml::Media::Imaging::RenderTargetBitmap;
using winrt::Windows::Foundation::IAsyncAction;
using winrt::Windows::Graphics::Imaging::BitmapAlphaMode;
using winrt::Windows::Graphics::Imaging::BitmapEncoder;
using winrt::Windows::Graphics::Imaging::BitmapPixelFormat;
using winrt::Windows::Storage::Streams::DataReader;
using winrt::Windows::Storage::Streams::InMemoryRandomAccessStream;

namespace winrt::Rhythm::implementation {

namespace {

namespace fs = std::filesystem;

struct Theme {
    ElementTheme value;
    const wchar_t* name;
};

constexpr Theme kThemes[] = {
    {ElementTheme::Dark, L"Dark"},
    {ElementTheme::Light, L"Light"},
};

/// Windows start far off-screen: the capture does not need them on screen,
/// and the user's desktop does not flash.
constexpr Windows::Graphics::PointInt32 kOffScreen{-20000, -20000};

/// One line to stderr (the task runner logs it). A GUI-subsystem process
/// still inherits the runner's pipes.
void Report(std::wstring const& line) {
    std::string utf8 = winrt::to_string(line + L"\n");
    DWORD written = 0;
    ::WriteFile(::GetStdHandle(STD_ERROR_HANDLE), utf8.data(),
                static_cast<DWORD>(utf8.size()), &written, nullptr);
}

/// Lets layout, template application and entrance transitions finish.
/// Awaited from the UI thread, so the caller resumes there.
IAsyncAction Settle() {
    co_await winrt::resume_after(std::chrono::milliseconds(800));
}

/// Renders an element at its layout size (device-independent pixels, so the
/// golden does not depend on the display scale) and writes it as PNG.
IAsyncAction SavePng(FrameworkElement element, fs::path file) {
    RenderTargetBitmap bitmap;
    co_await bitmap.RenderAsync(element,
                                static_cast<int32_t>(std::lround(element.ActualWidth())),
                                static_cast<int32_t>(std::lround(element.ActualHeight())));
    auto pixels = co_await bitmap.GetPixelsAsync();
    if (bitmap.PixelWidth() == 0 || bitmap.PixelHeight() == 0) {
        throw hresult_error(E_FAIL, L"nothing rendered for " + hstring{file.filename().wstring()});
    }

    std::vector<uint8_t> bgra(pixels.Length());
    DataReader::FromBuffer(pixels).ReadBytes(bgra);

    InMemoryRandomAccessStream stream;
    auto encoder = co_await BitmapEncoder::CreateAsync(BitmapEncoder::PngEncoderId(), stream);
    encoder.SetPixelData(BitmapPixelFormat::Bgra8, BitmapAlphaMode::Premultiplied,
                         bitmap.PixelWidth(), bitmap.PixelHeight(), 96.0, 96.0, bgra);
    co_await encoder.FlushAsync();

    std::vector<uint8_t> png(static_cast<size_t>(stream.Size()));
    auto reader = DataReader(stream.GetInputStreamAt(0));
    co_await reader.LoadAsync(static_cast<uint32_t>(png.size()));
    reader.ReadBytes(png);

    std::ofstream out(file, std::ios::binary | std::ios::trunc);
    out.write(reinterpret_cast<const char*>(png.data()), static_cast<std::streamsize>(png.size()));
    if (!out) throw hresult_error(E_FAIL, L"cannot write " + hstring{file.wstring()});
    Report(L"captured " + file.filename().wstring());
}

/// What the window paints behind its content, in the captured theme. Views
/// that leave their background transparent would otherwise come out as
/// transparent pixels, which no reviewer can judge.
Grid Backdrop(Theme theme) {
    auto grid = Markup::XamlReader::Load(
        L"<Grid xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation' "
        L"Background='{ThemeResource ApplicationPageBackgroundThemeBrush}' />").as<Grid>();
    grid.RequestedTheme(theme.value);
    return grid;
}

/// Shows the window off-screen, lets it settle and captures `root`.
/// Activation gives the first control keyboard focus, and its focus
/// rectangle is not part of any view's look: focus moves to an invisible
/// sink added to the backdrop first.
IAsyncAction Shoot(Window window, Grid root, fs::path file) {
    Button sink;
    sink.Width(1);
    sink.Height(1);
    sink.Opacity(0);
    sink.HorizontalAlignment(HorizontalAlignment::Left);
    sink.VerticalAlignment(VerticalAlignment::Top);
    root.Children().Append(sink);

    window.AppWindow().Move(kOffScreen);
    window.Activate();
    co_await Settle();
    sink.Focus(FocusState::Programmatic);
    co_await Settle();
    co_await SavePng(root, file);
    window.Close();
}

/// The whole main window: sidebar, toolbar, the library's empty state (the
/// scratch library is empty) and the idle player bar.
IAsyncAction CaptureMainWindow(Theme theme, fs::path dir) {
    auto window = make<MainWindow>();
    auto content = window.Content();
    auto backdrop = Backdrop(theme);
    window.Content(backdrop);
    backdrop.Children().Append(content.as<UIElement>());
    co_await Shoot(window, backdrop, dir / (std::wstring(L"MainWindow_") + theme.name + L".png"));
}

/// One view on its own, in a fixed-size frame of a bare window.
IAsyncAction CaptureView(Theme theme, FrameworkElement view, double width, double height,
                         fs::path file) {
    auto frame = Backdrop(theme);
    frame.Width(width);
    frame.Height(height);
    frame.HorizontalAlignment(HorizontalAlignment::Left);
    frame.VerticalAlignment(VerticalAlignment::Top);
    frame.Children().Append(view);

    Window window;
    window.Content(frame);
    window.AppWindow().ResizeClient({static_cast<int32_t>(width) + 200,
                                     static_cast<int32_t>(height) + 200});
    co_await Shoot(window, frame, file);
}

/// The library page with fixture tracks, in both of its sort modes.
IAsyncAction CaptureLibrary(Theme theme, fs::path dir) {
    for (int sort : {0, 1}) {
        ::rhythm::AppState state;
        state.Tracks = ::rhythm::capture::FixtureTracks();

        Rhythm::Views::LibraryView page;
        page.RequestedTheme(theme.value);
        auto impl = get_self<Views::implementation::LibraryView>(page);
        impl->viewPivot().SelectedIndex(sort);
        impl->BindState(&state);

        const wchar_t* name = sort == 0 ? L"LibraryView_ArtistAlbum_" : L"LibraryView_Letter_";
        co_await CaptureView(theme, page, 760, 440, dir / (std::wstring(name) + theme.name + L".png"));
    }
}

/// The player bar idle and while a track plays.
IAsyncAction CapturePlayerBar(Theme theme, fs::path dir) {
    for (bool playing : {false, true}) {
        ::rhythm::AppState state;
        if (playing) {
            state.CurrentTrack = ::rhythm::capture::FixtureTracks()[3];
            state.IsPlaying = true;
            state.Position = 86;
            state.Duration = 2175;
        }

        Rhythm::Views::PlayerBarView bar;
        bar.RequestedTheme(theme.value);
        get_self<Views::implementation::PlayerBarView>(bar)->BindState(&state);

        const wchar_t* name = playing ? L"PlayerBarView_Playing_" : L"PlayerBarView_Idle_";
        co_await CaptureView(theme, bar, 900, 100, dir / (std::wstring(name) + theme.name + L".png"));
    }
}

fire_and_forget CaptureAll(fs::path dir) {
    int code = 0;
    fs::path scratch = fs::temp_directory_path() /
                       (L"RhythmCapture-" + std::to_wstring(::GetCurrentProcessId()));
    try {
        fs::create_directories(dir);
        fs::create_directories(scratch);
        ::rhythm::capture::FixtureLibraryPath() = scratch / L"library.db";
        for (const auto& theme : kThemes) {
            co_await CaptureMainWindow(theme, dir);
            co_await CaptureLibrary(theme, dir);
            co_await CapturePlayerBar(theme, dir);
        }
    } catch (hresult_error const& e) {
        Report(L"capture failed: " + std::wstring(e.message()));
        code = 1;
    } catch (std::exception const& e) {
        Report(L"capture failed: " + std::wstring(winrt::to_hstring(e.what())));
        code = 1;
    }
    std::error_code ignored;
    fs::remove_all(scratch, ignored);
    ::ExitProcess(static_cast<UINT>(code));
}

} // namespace

App::App() = default;

void App::OnLaunched(LaunchActivatedEventArgs const&) {
    int argc = 0;
    LPWSTR* argv = ::CommandLineToArgvW(::GetCommandLineW(), &argc);
    std::wstring outDir = argc >= 2 ? argv[1] : L"";
    ::LocalFree(argv);
    if (outDir.empty()) {
        Report(L"usage: RhythmCapture.exe <output directory>");
        ::ExitProcess(2);
    }

    // Each capture closes its window before the next one opens.
    DispatcherShutdownMode(DispatcherShutdownMode::OnExplicitShutdown);
    ::rhythm::L10n::PinLanguageForProcess(L"en");
    CaptureAll(fs::absolute(outDir));
}

} // namespace winrt::Rhythm::implementation
