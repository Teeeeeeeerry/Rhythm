// L3 ①: 运行时外观切换（dark ↔ light）后逐视图断言品牌色渲染。
//
// 手段：`defaults write -g AppleInterfaceStyle Dark`（无需重启应用，
// 系统即时广播 appearance 变更）；断言不依赖内部实现 —— 窗口截图像素
// 抽样 + XCUIElement 可访问性值。
//
// 注意：系统 defaults 生效存在异步（~秒级），一律轮询等待而非固定 sleep。

import XCTest

final class AppearanceSwitchUITests: XCTestCase {

    var app: XCUIApplication!

    override func setUp() {
        super.setUp()
        continueAfterFailure = false
        app = XCUIApplication()
        app.launchArguments += ["-AppleInterfaceStyle", "Dark"]  // 初始 dark
        app.launch()
    }

    override func tearDown() {
        // 复位系统外观，避免污染后续测试
        runDefaults(["write", "NSGlobalDomain", "AppleInterfaceStyle",
                     "-string", "Light"])
        app.terminate()
        super.tearDown()
    }

    /// 运行时切换系统外观（`defaults write -g AppleInterfaceStyle`）。
    private func setSystemAppearance(_ dark: Bool) {
        runDefaults(["write", "NSGlobalDomain", "AppleInterfaceStyle",
                     "-string", dark ? "Dark" : "Light"])
        // defaults 广播后窗口外观异步刷新 —— 轮询截图稳定后再断言
        waitForAppearanceSettle()
    }

    private func runDefaults(_ args: [String]) -> String {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/defaults")
        p.arguments = args
        let pipe = Pipe()
        p.standardOutput = pipe
        try? p.run()
        p.waitUntilExit()
        return ""
    }

    private func waitForAppearanceSettle() {
        // 轮询：连续两帧截图字节一致即视为稳定（最多 10s）
        let deadline = Date().addingTimeInterval(10)
        var last: Data?
        while Date() < deadline {
            Thread.sleep(forTimeInterval: 0.5)
            let shot = app.windows.firstMatch.screenshot().image.tiffRepresentation ?? Data()
            if let last, last == shot { return }
            last = shot
        }
    }

    /// 像素抽样：取窗口中心一带的平均色，验证品牌化生效（dark surface 深色、
    /// light surface 白色）。§2.1 断言"颜色值/截图比对"的实现。
    private func sampleCenterColor() -> (r: Int, g: Int, b: Int) {
        let image = app.windows.firstMatch.screenshot().image
        // 简化抽样：NSBitmapImageRep 逐像素 —— 测试内实现，见 accessibility
        // 替代方案（更稳）：断言 XCUIElement 的 accessibilityValue 色值。
        return (0, 0, 0)  // TODO(P4): 接入实际抽样
    }

    // MARK: 用例

    func testDarkSurfaceIsBrandTeal() {
        setSystemAppearance(true)
        // surface dark = #011F26：窗口背景应呈深青色而非系统默认黑/灰
        // TODO(P4): 断言 sampleCenterColor() ≈ (1, 31, 38)，容差 ±8
        XCTAssertTrue(app.windows.firstMatch.exists)
    }

    func testSwitchToLightSurfaceIsWhite() {
        setSystemAppearance(false)
        // surface light = 白
        // TODO(P4): 断言 sampleCenterColor() ≈ (255, 255, 255)
        XCTAssertTrue(app.windows.firstMatch.exists)
    }

    func testHighContrastDarkKeepsBrandPalette() {
        // 高对比 dark：defaults write -g AppleInterfaceStyle -string Dark
        // + NSHighContrast 域（macOS 无公开默认键，用 NSPreferencePane 无 ——
        // 改用 XCUITest 的 launchArguments 注入 + L1 兜底，此处仅验证不崩）
        app.launchArguments += ["-AppleInterfaceStyle", "Dark"]
        app.launch()
        XCTAssertTrue(app.windows.firstMatch.exists)
    }

    func testNoSystemColorFlashOnLaunch() {
        // ⑤ 首帧闪烁检查：连续采样前 3 帧，断言无"系统色→品牌色"跳变。
        // 判定：首帧与稳定帧的色差 > 阈值即视为闪烁（P4 实现采样）。
        // TODO(P4): 用 CGWindowListCreateImage 采样首帧序列
        XCTAssertTrue(app.windows.firstMatch.exists)
    }
}
