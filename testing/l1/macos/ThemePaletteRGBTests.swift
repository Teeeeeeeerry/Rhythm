import AppKit
import SwiftUI
import XCTest
@testable import RhythmTheme

/// L1: 每 token × 每外观，强制 NSAppearance 后 sRGB 解析逐通道断言。
///
/// 期望值来自 PaletteSeed.swift（由 gen-palette.py --emit-swift-seed 从
/// palette.json 生成）—— 新增 token 自动获得本组测试。
/// alpha 逐字节相等：透明度由生成器一次算出并写入双端，容差随 #250 取消。
final class ThemePaletteRGBTests: XCTestCase {


    // token 名 → 访问器（数据驱动：循环 PaletteSeed.tokens 生成用例）
    private func color(for token: String) -> Color? {
        switch token {
        case "rhythmAccent": return .rhythmAccent
        case "rhythmSurface": return .rhythmSurface
        case "rhythmElevated": return .rhythmElevated
        case "rhythmTextPrimary": return .rhythmTextPrimary
        case "rhythmTextSecondary": return .rhythmTextSecondary
        case "rhythmTextTertiary": return .rhythmTextTertiary
        case "rhythmBorder": return .rhythmBorder
        case "rhythmSourceLocal": return .rhythmSourceLocal
        case "rhythmSourceYoutube": return .rhythmSourceYoutube
        case "rhythmSourceBilibili": return .rhythmSourceBilibili
        case "rhythmSourceUrl": return .rhythmSourceUrl
        default: return nil
        }
    }

    private func resolvedRGB(_ color: Color, appearanceName: NSAppearance.Name)
        -> PaletteSeed.RGB? {
        guard let appearance = NSAppearance(named: appearanceName) else { return nil }
        // performAsCurrentDrawingAppearance 闭包返回 Void（macOS 26 SDK 签名），
        // 结果经可变变量取回。
        var result: PaletteSeed.RGB?
        appearance.performAsCurrentDrawingAppearance {
            guard let ns = NSColor(color).usingColorSpace(.sRGB) else {
                result = nil
                return
            }
            result = PaletteSeed.RGB(
                r: Int((ns.redComponent * 255).rounded()),
                g: Int((ns.greenComponent * 255).rounded()),
                b: Int((ns.blueComponent * 255).rounded()),
                a: Int((ns.alphaComponent * 255).rounded())
            )
        }
        return result
    }

    func testAllTokensResolveToSeedValues() {
        for (token, variants) in PaletteSeed.tokens {
            guard let color = color(for: token) else {
                XCTFail("PaletteSeed 包含未知 token: \(token)（访问器未登记）")
                continue
            }
            for (appearanceName, expected) in variants {
                let appearance: NSAppearance.Name =
                    appearanceName == "dark" ? .darkAqua : .aqua
                guard let actual = resolvedRGB(color, appearanceName: appearance) else {
                    XCTFail("\(token).\(appearanceName) 无法解析为 sRGB")
                    continue
                }
                XCTAssertEqual(actual.r, expected.r,
                               "\(token).\(appearanceName) R 通道")
                XCTAssertEqual(actual.g, expected.g,
                               "\(token).\(appearanceName) G 通道")
                XCTAssertEqual(actual.b, expected.b,
                               "\(token).\(appearanceName) B 通道")
                XCTAssertEqual(actual.a, expected.a,
                               "\(token).\(appearanceName) alpha 通道")
            }
        }
    }

    func testSeedCoversEveryDeclaredToken() {
        // 反向校验：访问器登记的所有 token 都必须在 seed 中（防漏测）
        let declared = Set([
            "rhythmAccent", "rhythmSurface", "rhythmElevated",
            "rhythmTextPrimary", "rhythmTextSecondary", "rhythmTextTertiary",
            "rhythmBorder",
            "rhythmSourceLocal", "rhythmSourceYoutube",
            "rhythmSourceBilibili", "rhythmSourceUrl",
        ])
        XCTAssertEqual(Set(PaletteSeed.tokens.keys), declared,
                       "token 集合漂移：seed 与声明不一致")
    }
}
