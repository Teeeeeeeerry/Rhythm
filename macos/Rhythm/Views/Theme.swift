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
}
