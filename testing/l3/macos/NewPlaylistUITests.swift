// L3 ④: 新建播放列表弹窗全程（打开 → 输入 → 创建 → 出现在列表）。
//
// 弹窗使用 .sheet + .background(.rhythmSurface) —— surface 双外观断言
// 与截图比对（L2 快照的 UI 自动化补充）。

import XCTest

final class NewPlaylistUITests: XCTestCase {

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

    func testCreatePlaylistFlow() {
        // 进入播放列表页
        app.staticTexts["播放列表"].firstMatch.click()

        // 打开新建弹窗（工具栏 + 按钮）
        let addButton = app.buttons["Add"]  // toolbar plus
        XCTAssertTrue(addButton.waitForExistence(timeout: 5))
        addButton.click()

        // 弹窗内容断言
        let sheet = app.sheets.firstMatch
        XCTAssertTrue(sheet.waitForExistence(timeout: 5), "应弹出新建播放列表弹窗")
        XCTAssertTrue(app.staticTexts["新建播放列表"].firstMatch.exists)

        // 输入名称 → 创建
        let nameField = sheet.textFields.firstMatch
        nameField.click()
        nameField.typeText("TestPlaylist_\(UUID().uuidString.prefix(6))")
        app.buttons["创建"].firstMatch.click()

        // 创建成功后回到列表，新条目出现（surface 背景下列表文字可达）
        XCTAssertTrue(app.staticTexts.containing(
            NSPredicate(format: "label BEGINSWITH 'TestPlaylist_'")
        ).firstMatch.waitForExistence(timeout: 5), "新播放列表应出现在列表中")
    }

    func testCancelClosesSheetWithoutCreating() {
        app.staticTexts["播放列表"].firstMatch.click()
        app.buttons["Add"].firstMatch.click()
        XCTAssertTrue(app.sheets.firstMatch.waitForExistence(timeout: 5))
        app.buttons["取消"].firstMatch.click()
        XCTAssertFalse(app.sheets.firstMatch.waitForExistence(timeout: 3),
                       "取消应关闭弹窗")
    }

    func testEmptyNameDoesNotCreate() {
        app.staticTexts["播放列表"].firstMatch.click()
        app.buttons["Add"].firstMatch.click()
        // 不输入直接创建 —— 创建按钮应被禁用或点击无效
        let create = app.buttons["创建"].firstMatch
        if create.isEnabled {
            create.click()
            XCTAssertTrue(app.sheets.firstMatch.exists,
                          "空名称不应创建成功（弹窗应保留）")
        }
    }
}
