import AppKit
import XCTest
@testable import RhythmTheme

/// L1: isDark() 判定矩阵钉住（§2.1 外观维度）。
///
/// 前置：P2 重构后 Theme.swift 拆入 RhythmTheme library target，`isDark`
/// 由 private 改为 internal（见 macos-Package-patch.md）。
///
/// 注意：对未知 appearance，`NSAppearance.bestMatch(from:)` 返回数组首个元素
/// （.darkAqua）→ 当前实现落 dark。F6 决策（改为落 light 或显式报错）落地后
/// 本测试相应更新，不得静默。
final class ThemeIsDarkMatrixTests: XCTestCase {

    private func resolvedIsDark(_ name: NSAppearance.Name) -> Bool {
        guard let appearance = NSAppearance(named: name) else {
            XCTFail("无法构造 appearance: \(name.rawValue)")
            return false
        }
        // isDark 依赖 NSAppearance.current 解析 —— 以 performAsCurrent 包裹，
        // 与真实渲染路径一致。注意：performAsCurrentDrawingAppearance 闭包
        // 返回 Void（macOS 26 SDK 签名），结果经可变变量取回。
        // 命名 resolvedIsDark 避开模块级 isDark，防止遮蔽（两者签名不同）。
        var result = false
        appearance.performAsCurrentDrawingAppearance {
            result = isDark(appearance)
        }
        return result
    }

    func testStandardAppearances() {
        XCTAssertTrue(resolvedIsDark(.darkAqua), "darkAqua 必须判为 dark")
        XCTAssertFalse(resolvedIsDark(.aqua), "aqua 必须判为 light")
        XCTAssertTrue(resolvedIsDark(.accessibilityHighContrastDarkAqua),
                      "高对比 dark 必须判为 dark")
        XCTAssertFalse(resolvedIsDark(.accessibilityHighContrastAqua),
                       "高对比 light 必须判为 light")
    }

    func testVibrantAppearances() {
        // vibrant 系列无直接 bestMatch 成员 —— bestMatch 退化为最近匹配族
        XCTAssertTrue(resolvedIsDark(.vibrantDark), "vibrantDark 归入 dark 族")
        XCTAssertFalse(resolvedIsDark(.vibrantLight), "vibrantLight 归入 light 族")
    }

    func testUnknownAppearanceFallsBackToFirstMatch() {
        // F6：bestMatch 对候选列表外 appearance 的行为钉住。
        // macOS 26 SDK 已移除 NSAppearance(name:) 自定义名构造（仅剩
        // init?(named:) 标准名）—— "完全未知 appearance"不可再构造；
        // 改用不在候选数组中的标准 vibrant 系列作为候选外输入：
        //   vibrantDark → 最近匹配族 dark（darkAqua），isDark 判 dark；
        //   vibrantLight → 最近匹配族 light（aqua），isDark 判 light。
        // F6 决策（未知 appearance 落 dark 或 light）变更时须同步更新。
        let candidates: [NSAppearance.Name] = [
            .darkAqua, .aqua,
            .accessibilityHighContrastDarkAqua, .accessibilityHighContrastAqua,
        ]
        for (name, expectedMatch, expectedDark) in [
            (NSAppearance.Name.vibrantDark, NSAppearance.Name.darkAqua, true),
            (NSAppearance.Name.vibrantLight, NSAppearance.Name.aqua, false),
        ] {
            guard let appearance = NSAppearance(named: name) else {
                XCTFail("无法构造 appearance: \(name.rawValue)")
                continue
            }
            let match = appearance.bestMatch(from: candidates)
            XCTAssertEqual(match, expectedMatch,
                           "\(name.rawValue) 的 bestMatch 行为变更需复核")
            var result = false
            appearance.performAsCurrentDrawingAppearance {
                result = isDark(appearance)
            }
            XCTAssertEqual(result, expectedDark,
                           "\(name.rawValue) 的 dark 判定变更需复核（F6 待决策）")
        }
    }
}
