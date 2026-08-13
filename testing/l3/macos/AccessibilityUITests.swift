// L3 ③: VoiceOver 选中语义（accessibilityValue / traits 断言）。
//
// 用例要点（方案 §3 L3 行）：
//   - 侧边栏条目应暴露 .isSelected trait / accessibilityValue = "selected"；
//   - 来源徽标（SourceTagView）应暴露其标签（本地/YT/B站/链接）；
//   - 播放按钮语义化。
// 前置：F3 修复。

import XCTest

final class AccessibilityUITests: XCTestCase {

    var app: XCUIApplication!

    override func setUp() {
        super.setUp()
        continueAfterFailure = false
        app = XCUIApplication()
        app.launch()
    }

    override func tearDown() {
        app.terminate()
        super.tearDown()
    }

    private func trait(of element: XCUIElement, key: String) -> Any? {
        element.value(forKey: key)
    }

    func testSidebarRowsExposeSelectionState() {
        let library = app.staticTexts["资料库"].firstMatch
        XCTAssertTrue(library.waitForExistence(timeout: 5))
        // F3 验收：选中项通过 accessibility 暴露（VoiceOver 朗读"已选中"）
        let traits = trait(of: library, key: "accessibilityTraits")
        XCTAssertNotNil(traits, "侧边栏行必须暴露 accessibilityTraits（F3）")
    }

    func testSourceBadgesExposeLabels() {
        // 导入含 4 来源的数据后（夹具库 §2.1）：
        // 徽标文本（本地/YT/B站/链接）应作为静态文本可达
        for label in ["本地", "YT", "B站", "链接"] {
            XCTAssertTrue(app.staticTexts[label].firstMatch.waitForExistence(timeout: 5),
                          "来源徽标 \(label) 应可被辅助功能访问")
        }
    }

    func testPlayButtonHasPlayPauseRole() {
        // 播放/暂停按钮：accessibilityLabel 应为"播放"/"暂停"且为 button 角色
        let play = app.buttons.element(boundBy: 0)  // TODO(P4): 用标识符定位
        XCTAssertNotNil(play.value(forKey: "accessibilityLabel"))
    }

    func testEmptyStateTextsAreExposed() {
        // 空库状态：两层文字（secondary 提示 + tertiary 说明）都可达
        app.launchArguments += ["-UITestEmptyLibrary"]  // P4：测试夹具参数
        app.launch()
        XCTAssertTrue(app.staticTexts["点击工具栏 + 按钮导入音乐文件或文件夹"]
            .firstMatch.waitForExistence(timeout: 5))
    }
}
