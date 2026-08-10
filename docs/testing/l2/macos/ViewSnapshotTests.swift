// L2: macOS 视图快照测试模板（swift-snapshot-testing）。
//
// 前置：
//   - P3：用 XcodeGen 从 Package.swift 生成工程（见 docs/testing/ci/visual.yml），
//     测试 target 以 Rhythm app 为 host（快照需 import app module 的视图）。
//   - 依赖 pointfreeco/swift-snapshot-testing（.package(url:...from: "1.17.0")）。
//
// 用例矩阵 = §2.2 使用点清单 × 选中/未选中 × dark/light × zh/en。
// 快照内强制 NSAppearance.current —— 与真实渲染路径一致。
//
// Golden 维护约定（必须遵守）：
//   - 外观/布局改动必附快照更新，review 看 diff（git diff --no-index 或 CI 贴图）。
//   - 改色后 CI 快照必红 —— 这是 L2 的核心价值，禁止用 --record 静默覆盖后不评审。
//   - golden 绑定固定字体（macOS 13+ SF Pro），CI 跑同一系统版本。

import AppKit
import SnapshotTesting
import SwiftUI
import XCTest

/// 快照用例描述：视图构造 + 状态 + 外观 + 语言
struct SnapshotCase {
    let name: String                       // 用例名（snapshot 文件名的一部分）
    let appearance: NSAppearance.Name
    let isChinese: Bool
    let makeView: () -> AnyView            // 视图工厂（返回前设置好选中态/数据）
}

final class ViewSnapshotTests: XCTestCase {

    override func setUp() {
        super.setUp()
        // 中文/英文由 appState 的 L10n.isChinese 驱动 —— 测试夹具在工厂内设置
    }

    /// 逐用例渲染并快照：窗口尺寸固定，避免字体度量漂移
    private func assertSnapshot(_ case_: SnapshotCase, file: StaticString = #filePath,
                                testName: String = #function, line: UInt = #line) {
        let appearance = NSAppearance(named: case_.appearance)!
        let view = case_.makeView()
            .frame(width: 800, height: 600)
            .environment(\.colorScheme, case_.appearance == .darkAqua ? .dark : .light)
        let hosting = NSHostingView(rootView: view)
        hosting.frame = NSRect(x: 0, y: 0, width: 800, height: 600)
        hosting.appearance = appearance

        // 强制当前 appearance 后渲染（与 L1 一致；NSHostingView 渲染依赖
        // current 语义，performAsCurrent 防止静默退化为系统外观）
        appearance.performAsCurrentDrawingAppearance {
            hosting.layoutSubtreeIfNeeded()
            SwiftSnapshotTesting.assertSnapshot(
                matching: hosting,
                as: .image(precision: 0.99),   // 抗锯齿容差，防止无意义 CI 抖动
                named: "\(case_.name)-\(case_.isChinese ? "zh" : "en")",
                record: false,                 // 禁止 --record 静默入库
                file: file, testName: testName, line: line
            )
        }
    }

    // MARK: §2.2 使用点清单 × 状态 × 外观 × 语言（占位实现，接入真实视图后填充）

    func testSidebar() {
        for appearance: NSAppearance.Name in [.darkAqua, .aqua] {
            for selected in [false, true] {
                for isChinese in [true, false] {
                    let case_ = SnapshotCase(
                        name: "sidebar-\(selected ? "selected" : "idle")",
                        appearance: appearance,
                        isChinese: isChinese,
                        makeView: {
                            // TODO(P3): 用真实 AppState 夹具构造 SidebarView，
                            // selected 时选中"资料库"项
                            AnyView(Text("SidebarView"))  // 占位
                        }
                    )
                    assertSnapshot(case_)
                }
            }
        }
    }

    func testArtistAlbumRow() {
        // 占位：AlbumRow 用例（含/不含封面、来源徽标 4 色混排、长标题截断）
    }

    func testLibraryEmptyState() {
        // 占位：空库状态（textSecondary + textTertiary 双级文字）
    }

    func testPlayerBar() {
        // 占位：播放/暂停/缓冲状态 + 时长文本
    }

    func testNewPlaylistSheet() {
        // 占位：新建播放列表弹窗（surface 背景）
    }

    func testPlaylistDetailEmpty() {
        // 占位：空列表 + 返回头
    }

    func testAlphabeticalIndex() {
        // 占位：首字母索引条
    }
}
