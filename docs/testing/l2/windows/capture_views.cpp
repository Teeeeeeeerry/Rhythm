// L2: Windows 视图离屏渲染截屏（WinUI 3 RenderTargetBitmap → PNG）。
//
// 这是挂接骨架：WinUI 3 的 RenderTargetBitmap 需在 app 上下文（DispatcherQueue）
// 内执行，因此编译为独立的测试 host 可执行文件（或测试工程内的静态方法），
// 由 compare-screenshots.py 驱动并做像素比对。
//
// 集成步骤（P3）：
//   1. 新建测试 host：Console 应用，引用 Microsoft.WindowsAppSDK，
//      初始化 WinRT（RoInitialize）与 DispatcherQueueController。
//   2. 依次加载 5 个视图（LibraryView / PlaylistListView / PlaylistDetailView /
//      PlayerBarView / SidebarView），每个视图 × {Default, Light} 主题字典各截一张。
//   3. 导出 PNG 到 build/artifacts/<view>_<theme>.png（输出路径由 argv[1] 指定）。
//   4. 调用处（示例）：
//        render_view(L"LibraryView", L"Default", outDir);
//        render_view(L"LibraryView", L"Light", outDir);

#include <windows.h>
#include <winrt/Microsoft.UI.Xaml.Media.Imaging.h>
#include <winrt/Microsoft.UI.Xaml.h>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Graphics.Imaging.h>
#include <winrt/Windows.Storage.Streams.h>

using namespace winrt;
using namespace Microsoft::UI::Xaml;
using namespace Microsoft::UI::Xaml::Media::Imaging;
using namespace Windows::Storage::Streams;

/// 核心截屏原语（P3 填充视图加载与 DispatcherQueue 上下文）。
bool render_view_to_png(UIElement const& view, std::wstring const& outPath,
                        std::wstring const& themeKey) {
    // 1) 强制主题字典：遍历 visual tree 或设置 root 的 RequestedTheme
    //    （WinUI 3 无 FrameworkElement.RequestedTheme，需 ThemeResource 重评估，
    //     P3 时验证 —— 若不可行，退化为两套资源字典分别装载）。
    // 2) 放入根容器并 Arrange：
    //    auto root = Grid{}; root.Children().Append(view);
    //    设置 800x600 尺寸并 UpdateLayout()。
    // 3) RenderTargetBitmap 渲染：
    RenderTargetBitmap bmp;
    auto op = bmp.RenderAsync(view);
    op.get();  // 需在 UI 线程
    // 4) 导出 PNG：
    auto pixelBuffer = bmp.GetPixels();
    InMemoryRandomAccessStream stream;
    auto encoder = BitmapEncoder::CreateAsync(
        BitmapEncoder::PngEncoderId(), stream).get();
    encoder.SetPixelData(BitmapPixelFormat::Bgra8, BitmapAlphaMode::Premultiplied,
                         bmp.PixelWidth(), bmp.PixelHeight(), 96.0, 96.0,
                         pixelBuffer);
    encoder.FlushAsync().get();
    // 5) 写文件（stream 倒回后 copy 到 std::ofstream）。
    return true;
}

int wmain(int argc, wchar_t** argv) {
    if (argc < 2) {
        std::wcerr << L"用法: capture_views <输出目录>\n";
        return 1;
    }
    std::wstring outDir = argv[1];
    // P3：加载 5 个视图 × 2 主题，循环调用 render_view_to_png。
    // 输出命名约定（compare-screenshots.py 依赖）：
    //   <ViewName>_<Default|Light>.png
    return 0;
}
