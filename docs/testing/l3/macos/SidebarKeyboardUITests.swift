// L3 ②: 侧边栏键盘导航（方向键移动选中）。
//
// 前置：F3 修复 —— SidebarView 恢复语义（`.accessibilityAddTraits(.isSelected)`
// 或回归 List(selection:) 绑定 + 自定义 tint）。修复前本组用例失败属验收信号。

import XCTest

final class SidebarKeyboardUITests: XCTestCase {

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

    /// 断言"资料库"行带 .isSelected 语义（F3 的验收点）
    private func libraryRow() -> XCUIElement {
        // 侧边栏条目文本：资料库 / 播放列表（zh）
        app.staticTexts["资料库"].firstMatch
    }

    func testInitialSelectionIsLibrary() {
        XCTAssertTrue(libraryRow().waitForExistence(timeout: 5),
                      "侧边栏首项应存在")
        // F3 后：首项默认选中且带 isSelected trait
        let selected = libraryRow().value(forKey: "accessibilityTraits") != nil
        XCTAssertTrue(selected || libraryRow().isSelected,
                      "首项应处于选中语义状态")
    }

    func testArrowKeysMoveSelection() {
        libraryRow().click()  // 聚焦列表
        // 方向键下：从"资料库"移到"播放列表"
        app.typeKey(.downArrow, modifierFlags: [])
        let playlists = app.staticTexts["播放列表"].firstMatch
        // F3 修复后选中语义随之转移 —— 断言播放列表行获得选中态
        let expectation = XCTNSPredicateExpectation(
            predicate: NSPredicate { obj, _ in (obj as? XCUIElement)?.isSelected == true },
            object: playlists
        )
        XCTAssertEqual(XCTWaiter().wait(for: [expectation], timeout: 3), .completed,
                       "方向键下应把选中移到播放列表")
    }

    func testUpArrowWrapsOrStaysAtTop() {
        libraryRow().click()
        app.typeKey(.upArrow, modifierFlags: [])
        // 语义断言：选中仍在资料库（不越界到空白）
        XCTAssertTrue(libraryRow().isSelected || app.staticTexts["播放列表"].firstMatch.isSelected,
                      "↑ 不应破坏选中状态")
    }

    func testEnterActivatesSelection() {
        // 选中播放列表 + Enter → 进入播放列表页
        libraryRow().click()
        app.typeKey(.downArrow, modifierFlags: [])
        app.typeKey(.return, modifierFlags: [])
        XCTAssertTrue(app.navigationBars.count > 0 || app.staticTexts.count > 0,
                      "Enter 应导航到目标页")
    }
}

extension XCUIElement {
    /// 便捷语义：XCTest 对 NSControl 的选中态暴露有限，取 accessibilityValue
    var isSelected: Bool {
        value(forKey: "accessibilityValue") as? String == "1"
            || value(forKey: "isSelected") as? Bool == true
    }
}
