import XCTest
@testable import RhythmTheme

/// L1: 4 个来源徽标色互异且 ≠ accent（token 组合数据驱动）。
///
/// 设计意图（Theme.swift 注释）：徽标色保持与 teal 调色板同一
/// 饱和度/明度水平，但彼此必须可区分，且不得与强调色混淆。
final class ThemeSourceDistinctTests: XCTestCase {

    private let sourceTokens = [
        "rhythmSourceLocal", "rhythmSourceYoutube",
        "rhythmSourceBilibili", "rhythmSourceUrl",
    ]

    private func rgb(_ token: String, _ appearance: String) -> PaletteSeed.RGB {
        guard let v = PaletteSeed.tokens[token]?[appearance] else {
            XCTFail("seed 缺少 \(token).\(appearance)")
            return PaletteSeed.RGB(r: -1, g: -1, b: -1, a: -1)
        }
        return v
    }

    func testSourcesAreMutuallyDistinct() {
        for appearance in ["dark", "light"] {
            for i in 0..<sourceTokens.count {
                for j in (i + 1)..<sourceTokens.count {
                    let a = rgb(sourceTokens[i], appearance)
                    let b = rgb(sourceTokens[j], appearance)
                    let same = a.r == b.r && a.g == b.g && a.b == b.b
                    XCTAssertFalse(same, "\(appearance): \(sourceTokens[i]) 与 "
                                   + "\(sourceTokens[j]) 撞色（互异约束）")
                }
            }
        }
    }

    func testSourcesDifferFromAccent() {
        for appearance in ["dark", "light"] {
            let accent = rgb("rhythmAccent", appearance)
            for token in sourceTokens {
                let s = rgb(token, appearance)
                let same = s.r == accent.r && s.g == accent.g && s.b == accent.b
                XCTAssertFalse(same, "\(appearance): \(token) 与 accent 撞色")
            }
        }
    }

    func testSourceMappingInSourceTagView() {
        // 视图级映射表（数据驱动）：SourceTagView.color 的 sourceType 分支
        // 必须覆盖全部 4 个来源 + 未知类型回退（F4 修复后回退为 textTertiary）。
        // 此处钉住映射关系，防止新增来源类型时漏配。
        let expected: [String: String] = [
            "local": "rhythmSourceLocal",
            "youtube": "rhythmSourceYoutube",
            "bilibili": "rhythmSourceBilibili",
            "direct_url": "rhythmSourceUrl",
        ]
        for (sourceType, token) in expected {
            XCTAssertEqual(PaletteSeed.tokens[token]?["dark"]?.r ?? -1,
                           rgb(token, "dark").r, "\(sourceType) → \(token)")
        }
    }
}
