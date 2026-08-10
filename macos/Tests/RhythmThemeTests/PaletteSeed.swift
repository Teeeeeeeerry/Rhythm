// 自动生成 — 由 docs/testing/sync-palette.py --emit-swift-seed 生成，勿手改。
// 修改 token 后重新生成：python3 docs/testing/sync-palette.py --emit-swift-seed
import Foundation

/// palette.json 的 Swift 种子：L1 测试断言的实际期望值。
enum PaletteSeed {
    struct RGB { let r, g, b, a: Int }

    static let tokens: [String: [String: RGB]] = [
        "rhythmAccent": ["dark": RGB(r: 171, g: 200, b: 212, a: 255) , "light": RGB(r: 13, g: 70, b: 77, a: 255)],
        "rhythmBorder": ["dark": RGB(r: 171, g: 200, b: 212, a: 38) , "light": RGB(r: 171, g: 200, b: 212, a: 76)],
        "rhythmElevated": ["dark": RGB(r: 13, g: 70, b: 77, a: 255) , "light": RGB(r: 255, g: 255, b: 255, a: 255)],
        "rhythmSourceBilibili": ["dark": RGB(r: 200, g: 141, b: 168, a: 255) , "light": RGB(r: 140, g: 77, b: 104, a: 255)],
        "rhythmSourceLocal": ["dark": RGB(r: 138, g: 188, b: 208, a: 255) , "light": RGB(r: 58, g: 122, b: 140, a: 255)],
        "rhythmSourceUrl": ["dark": RGB(r: 140, g: 184, b: 154, a: 255) , "light": RGB(r: 76, g: 120, b: 90, a: 255)],
        "rhythmSourceYoutube": ["dark": RGB(r: 212, g: 149, b: 115, a: 255) , "light": RGB(r: 139, g: 74, b: 40, a: 255)],
        "rhythmSurface": ["dark": RGB(r: 1, g: 31, b: 38, a: 255) , "light": RGB(r: 255, g: 255, b: 255, a: 255)],
        "rhythmTextPrimary": ["dark": RGB(r: 171, g: 200, b: 212, a: 255) , "light": RGB(r: 13, g: 70, b: 77, a: 255)],
        "rhythmTextSecondary": ["dark": RGB(r: 171, g: 200, b: 212, a: 178) , "light": RGB(r: 13, g: 70, b: 77, a: 153)],
        "rhythmTextTertiary": ["dark": RGB(r: 171, g: 200, b: 212, a: 140) , "light": RGB(r: 13, g: 70, b: 77, a: 102)],
    ]

    static let sources: [String: [String: RGB]] = [
        "bilibili": ["dark": RGB(r: 200, g: 141, b: 168, a: 255)],
        "direct_url": ["dark": RGB(r: 140, g: 184, b: 154, a: 255)],
        "local": ["dark": RGB(r: 138, g: 188, b: 208, a: 255)],
        "youtube": ["dark": RGB(r: 212, g: 149, b: 115, a: 255)],
    ]
}
