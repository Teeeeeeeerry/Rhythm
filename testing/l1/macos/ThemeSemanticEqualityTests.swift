import XCTest
@testable import RhythmTheme

/// L1: 语义相等性钉住（数据驱动于 PaletteSeed）。
///
/// 设计意图（Theme.swift 注释）：品牌调色为 dark-first，light 下层级
/// 靠阴影/边框而非色差 —— 因此 light 下 elevated == surface。
final class ThemeSemanticEqualityTests: XCTestCase {

    func testLightElevatedEqualsSurface() {
        let elevated = PaletteSeed.tokens["rhythmElevated"]!["light"]!
        let surface = PaletteSeed.tokens["rhythmSurface"]!["light"]!
        XCTAssertEqual(elevated.r, surface.r, "light elevated/surface R 必须同值")
        XCTAssertEqual(elevated.g, surface.g, "light elevated/surface G 必须同值")
        XCTAssertEqual(elevated.b, surface.b, "light elevated/surface B 必须同值")
        XCTAssertEqual(elevated.a, surface.a, "light elevated/surface alpha 必须同值")
    }

    func testDarkElevatedDiffersFromSurface() {
        let elevated = PaletteSeed.tokens["rhythmElevated"]!["dark"]!
        let surface = PaletteSeed.tokens["rhythmSurface"]!["dark"]!
        let same = elevated.r == surface.r && elevated.g == surface.g
            && elevated.b == surface.b
        XCTAssertFalse(same, "dark 下 elevated(\(elevated.r),\(elevated.g),\(elevated.b)) "
                       + "必须与 surface 不同（层级区分依赖色差）")
    }

    func testAccentEqualsTextPrimaryInBothAppearances() {
        // 附加约束：accent 与正文主色同值（accent 即强调文字色）
        for appearance in ["dark", "light"] {
            let accent = PaletteSeed.tokens["rhythmAccent"]![appearance]!
            let primary = PaletteSeed.tokens["rhythmTextPrimary"]![appearance]!
            XCTAssertEqual(accent.r, primary.r, "accent/textPrimary \(appearance) R")
            XCTAssertEqual(accent.g, primary.g, "accent/textPrimary \(appearance) G")
            XCTAssertEqual(accent.b, primary.b, "accent/textPrimary \(appearance) B")
        }
    }
}
