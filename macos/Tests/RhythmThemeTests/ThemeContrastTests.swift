import XCTest
@testable import RhythmTheme

/// L1: 对比度矩阵（复刻 L0 check-contrast.py 的 WCAG 2.1 + alpha 合成数学）。
///
/// 背景映射（与 palette.json backgrounds 段同步，§2.3）：
///   macOS light 下 List .inset 行背景为系统浅灰 #F5F5F7（非纯白）；
///   dark 下行背景 = surface；窗口背景 light=白 / dark=surface。
/// 阈值（与 palette.json usage 段同步）：正文/强调/徽标 4.5，次要/提示 3.0。
/// 例外（exceptions 段登记项）由下方例外集声明 —— 未登记的组合低于阈值即失败。
final class ThemeContrastTests: XCTestCase {

    // MARK: 背景映射（palette.json backgrounds）
    private static let backgrounds: [String: [String: (Int, Int, Int)]] = [
        "dark":  ["window": (0x01, 0x1F, 0x26), "row": (0x01, 0x1F, 0x26)],
        "light": ["window": (0xFF, 0xFF, 0xFF), "row": (0xF5, 0xF5, 0xF7)],
    ]

    // MARK: 阈值（palette.json usage）
    private static let thresholds: [String: Double] = [
        "rhythmAccent": 4.5, "rhythmTextPrimary": 4.5,
        "rhythmTextSecondary": 3.0, "rhythmTextTertiary": 3.0,
        "rhythmBorder": 3.0,
        "rhythmSourceLocal": 4.5, "rhythmSourceYoutube": 4.5,
        "rhythmSourceBilibili": 4.5, "rhythmSourceUrl": 4.5,
    ]

    // MARK: 已登记例外（palette.json exceptions — 决策留痕，修改须经评审）
    private static let registeredExceptions: Set<String> = [
        "rhythmTextSecondary/light/window",   // F8：3.45:1 < 4.5，按大文本线 3.0 批准
        "rhythmTextTertiary/light/window",    // F8：2.15:1，提示文本暂低于 3.0
        "rhythmBorder/dark/window",           // 装饰性分隔线 1.37:1
        "rhythmBorder/light/window",          // 装饰性分隔线 1.17:1
        // row 背景（#F5F5F7，macOS List .inset 行）—— 与 window 登记同源
        "rhythmTextTertiary/light/row",       // F8：2.12:1，提示文本暂低于 3.0
        "rhythmBorder/dark/row",              // 装饰性分隔线 1.37:1（dark row 同 window 色）
        "rhythmBorder/light/row",             // 装饰性分隔线 1.14:1
        "rhythmSourceLocal/light/row",        // 徽标前景 4.44:1 贴近 4.5，待复核
    ]

    // MARK: WCAG 2.1（与 palette_lib.py 同一数学）
    private func linearize(_ c: Double) -> Double {
        c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4)
    }

    private func luminance(_ rgb: (Int, Int, Int)) -> Double {
        0.2126 * linearize(Double(rgb.0) / 255.0)
            + 0.7152 * linearize(Double(rgb.1) / 255.0)
            + 0.0722 * linearize(Double(rgb.2) / 255.0)
    }

    private func blend(_ fg: (Int, Int, Int), alpha: Double,
                       onto bg: (Int, Int, Int)) -> (Int, Int, Int) {
        (Int((Double(fg.0) * alpha + Double(bg.0) * (1 - alpha)).rounded()),
         Int((Double(fg.1) * alpha + Double(bg.1) * (1 - alpha)).rounded()),
         Int((Double(fg.2) * alpha + Double(bg.2) * (1 - alpha)).rounded()))
    }

    private func ratio(_ fg: (Int, Int, Int), alpha: Double,
                       onto bg: (Int, Int, Int)) -> Double {
        let a = blend(fg, alpha: alpha, onto: bg)
        let l1 = luminance(a), l2 = luminance(bg)
        return (max(l1, l2) + 0.05) / (min(l1, l2) + 0.05)
    }

    func testFullMatrix() {
        var checked = 0
        for (appearance, bgMap) in Self.backgrounds {
            for (token, variants) in PaletteSeed.tokens {
                guard let threshold = Self.thresholds[token] else { continue }
                let rgb = variants[appearance]!
                let fg = (rgb.r, rgb.g, rgb.b)
                let alpha = Double(rgb.a) / 255.0

                // 文本类 token 对 window 与 row 两种背景都断言（§2.3）
                for (bgName, bg) in bgMap {
                    let r = ratio(fg, alpha: alpha, onto: bg)
                    let key = "\(token)/\(appearance)/\(bgName)"
                    checked += 1
                    if r >= threshold {
                        continue
                    }
                    XCTAssertTrue(Self.registeredExceptions.contains(key),
                                  "未登记低对比度: \(key) = \(String(format: "%.2f", r)):1 "
                                  + "< \(threshold):1（修复或登记例外）")
                }
            }
        }
        // 覆盖守卫：断言数量 = token × 外观 × 背景（防止矩阵静默缩小）
        let expected = Self.thresholds.count * 2 * 2
        XCTAssertEqual(checked, expected, "对比度矩阵覆盖不完整")
    }
}
