import SwiftUI

// MARK: - Appearance helper

/// Resolves the effective appearance so high-contrast variants don't fall
/// back to the static `Color` defaults and miss the brand palette.
private func isDark(_ appearance: NSAppearance) -> Bool {
    let match = appearance.bestMatch(from: [
        .darkAqua, .aqua,
        .accessibilityHighContrastDarkAqua, .accessibilityHighContrastAqua,
    ])
    return match == .darkAqua || match == .accessibilityHighContrastDarkAqua
}

// MARK: - Brand colour palette (#011F26 / #0D464D / #ABC8D4)

/// Dark-first palette exposed as `ShapeStyle` statics so SwiftUI modifiers
/// (`.foregroundStyle`, `.fill`, `.background`, `.tint`) resolve them
/// through the standard `ShapeStyle` member-lookup path.
extension ShapeStyle where Self == Color {

    // MARK: Accent

    /// Accent / interactive elements.
    /// Dark: #ABC8D4   Light: #0D464D (10.50:1 on white)
    static var rhythmAccent: Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            isDark(appearance)
                ? NSColor(red: 0xAB / 255.0, green: 0xC8 / 255.0, blue: 0xD4 / 255.0, alpha: 1.0)
                : NSColor(red: 0x0D / 255.0, green: 0x46 / 255.0, blue: 0x4D / 255.0, alpha: 1.0)
        })
    }

    // MARK: Surfaces

    /// Deepest background.
    /// Dark: #011F26   Light: white
    static var rhythmSurface: Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            isDark(appearance)
                ? NSColor(red: 0x01 / 255.0, green: 0x1F / 255.0, blue: 0x26 / 255.0, alpha: 1.0)
                : NSColor.white
        })
    }

    /// Elevated surface (cards, artwork placeholders).
    /// Dark: #0D464D   Light: white
    ///
    /// In light mode this is intentionally the same as `rhythmSurface` —
    /// the brand palette is dark-first and light-mode layering relies on
    /// shadows and borders rather than colour difference (see `rhythmBorder`).
    static var rhythmElevated: Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            isDark(appearance)
                ? NSColor(red: 0x0D / 255.0, green: 0x46 / 255.0, blue: 0x4D / 255.0, alpha: 1.0)
                : NSColor.white
        })
    }

    // MARK: Text

    /// Primary text.
    /// Dark: #ABC8D4   Light: #0D464D
    static var rhythmTextPrimary: Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            isDark(appearance)
                ? NSColor(red: 0xAB / 255.0, green: 0xC8 / 255.0, blue: 0xD4 / 255.0, alpha: 1.0)
                : NSColor(red: 0x0D / 255.0, green: 0x46 / 255.0, blue: 0x4D / 255.0, alpha: 1.0)
        })
    }

    /// Secondary / muted text.
    /// Dark: #ABC8D4 @ 0.7   Light: #0D464D @ 0.6
    static var rhythmTextSecondary: Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            isDark(appearance)
                ? NSColor(red: 0xAB / 255.0, green: 0xC8 / 255.0, blue: 0xD4 / 255.0, alpha: 0.7)
                : NSColor(red: 0x0D / 255.0, green: 0x46 / 255.0, blue: 0x4D / 255.0, alpha: 0.6)
        })
    }

    /// Tertiary / hint text.
    /// Dark: #ABC8D4 @ 0.55   Light: #0D464D @ 0.4
    static var rhythmTextTertiary: Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            isDark(appearance)
                ? NSColor(red: 0xAB / 255.0, green: 0xC8 / 255.0, blue: 0xD4 / 255.0, alpha: 0.55)
                : NSColor(red: 0x0D / 255.0, green: 0x46 / 255.0, blue: 0x4D / 255.0, alpha: 0.4)
        })
    }

    // MARK: Strokes

    /// Panel-separating stroke.
    ///
    /// #0D464D / #011F26 is only 1.63:1, so adjacent panels need a border
    /// to stay legible.
    /// Dark: #ABC8D4 @ 0.15   Light: #ABC8D4 @ 0.30
    static var rhythmBorder: Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            isDark(appearance)
                ? NSColor(red: 0xAB / 255.0, green: 0xC8 / 255.0, blue: 0xD4 / 255.0, alpha: 0.15)
                : NSColor(red: 0xAB / 255.0, green: 0xC8 / 255.0, blue: 0xD4 / 255.0, alpha: 0.30)
        })
    }

    // MARK: Source badge accents

    /// Harmonised accent colours for source-type badges.
    ///
    /// All four are held at the same muted-saturation / mid-lightness level as
    /// the core teal palette so they blend rather than jump.
    /// Dark mode  — foreground ~L* 75 (matches `#ABC8D4`)
    /// Light mode — foreground ~L* 16-20 (matches `#0D464D`, ≥4.5:1 on white)

    /// 本地 — blue-slate, stays closest to the brand teal family.
    /// Dark: #8ABCD0   Light: #3A7A8C
    static var rhythmSourceLocal: Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            isDark(appearance)
                ? NSColor(red: 0x8A / 255.0, green: 0xBC / 255.0, blue: 0xD0 / 255.0, alpha: 1.0)
                : NSColor(red: 0x3A / 255.0, green: 0x7A / 255.0, blue: 0x8C / 255.0, alpha: 1.0)
        })
    }

    /// YouTube — muted terracotta (warm, complementary to teal).
    /// Dark: #D49573   Light: #8B4A28
    static var rhythmSourceYoutube: Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            isDark(appearance)
                ? NSColor(red: 0xD4 / 255.0, green: 0x95 / 255.0, blue: 0x73 / 255.0, alpha: 1.0)
                : NSColor(red: 0x8B / 255.0, green: 0x4A / 255.0, blue: 0x28 / 255.0, alpha: 1.0)
        })
    }

    /// Bilibili — dusty rose (restrained pink, nod to B站 brand without the
    /// saturation clash).
    /// Dark: #C88DA8   Light: #8C4D68
    static var rhythmSourceBilibili: Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            isDark(appearance)
                ? NSColor(red: 0xC8 / 255.0, green: 0x8D / 255.0, blue: 0xA8 / 255.0, alpha: 1.0)
                : NSColor(red: 0x8C / 255.0, green: 0x4D / 255.0, blue: 0x68 / 255.0, alpha: 1.0)
        })
    }

    /// 链接 — sage green (teal-adjacent, natural extension of the palette).
    /// Dark: #8CB89A   Light: #4C785A
    static var rhythmSourceUrl: Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            isDark(appearance)
                ? NSColor(red: 0x8C / 255.0, green: 0xB8 / 255.0, blue: 0x9A / 255.0, alpha: 1.0)
                : NSColor(red: 0x4C / 255.0, green: 0x78 / 255.0, blue: 0x5A / 255.0, alpha: 1.0)
        })
    }
}
