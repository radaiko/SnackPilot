import SwiftUI

/// App theming (themes §1–§4). Two independent, persisted settings drive appearance:
/// a light/dark/system `ThemePreference` and one of five `AccentColor`s. The resolved accent
/// tints every control app-wide (applied once at `RootView` via `.tint`); primary actions adopt
/// iOS 26 Liquid Glass where available and fall back to the bordered-prominent style on iOS 17–18.

// MARK: - Appearance model (themes §1)

/// Light/dark/system color-scheme preference (themes §1). Persisted under `theme_preference`.
enum ThemePreference: String, CaseIterable, Identifiable {
    case system, light, dark

    var id: String { rawValue }

    /// German label shown in the Settings hint and the "Design" picker (themes §5).
    var label: String {
        switch self {
        case .system: return "System"
        case .light: return "Hell"
        case .dark: return "Dunkel"
        }
    }

    /// SF Symbol for the "Design" picker (themes §5: phone/portrait, sun, moon).
    var symbol: String {
        switch self {
        case .system: return "iphone"
        case .light: return "sun.max.fill"
        case .dark: return "moon.fill"
        }
    }
}

/// Selectable accent color (themes §3). Persisted under `accent_color`. Each accent carries an
/// exact light + dark `primary` hex; the effective tint is the primary for the *resolved* scheme.
enum AccentColor: String, CaseIterable, Identifiable {
    case orange, emerald, berry, golden, ocean

    var id: String { rawValue }

    /// German label shown beneath the swatch (themes §3).
    var label: String {
        switch self {
        case .orange: return "Orange"
        case .emerald: return "Smaragd"
        case .berry: return "Beere"
        case .golden: return "Gold"
        case .ocean: return "Ozean"
        }
    }

    /// The accent's `primary` for light mode (themes §3). Also used as the swatch fill —
    /// always the light value, even in dark mode (themes §5).
    var lightPrimary: Color {
        switch self {
        case .orange: return Color(hex: 0xD4501A)
        case .emerald: return Color(hex: 0x2E7D4F)
        case .berry: return Color(hex: 0xA62547)
        case .golden: return Color(hex: 0xC08B1A)
        case .ocean: return Color(hex: 0x2563A8)
        }
    }

    /// The accent's `primary` for dark mode (themes §3).
    var darkPrimary: Color {
        switch self {
        case .orange: return Color(hex: 0xFF6B35)
        case .emerald: return Color(hex: 0x4CAF7D)
        case .berry: return Color(hex: 0xE04868)
        case .golden: return Color(hex: 0xE8B03E)
        case .ocean: return Color(hex: 0x4A90D9)
        }
    }

    /// Effective tint: the accent's primary for the resolved scheme (themes §3).
    func primary(isDark: Bool) -> Color { isDark ? darkPrimary : lightPrimary }
}

extension Color {
    /// Build a `Color` from a 24-bit `0xRRGGBB` literal (sRGB, opaque).
    init(hex: UInt32) {
        self.init(
            .sRGB,
            red: Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8) & 0xFF) / 255,
            blue: Double(hex & 0xFF) / 255,
            opacity: 1
        )
    }
}

// MARK: - Surface / control styles

/// Primary call-to-action button: Liquid Glass on iOS 26+, bordered-prominent below. No explicit
/// tint — it inherits the app-wide accent set at `RootView` (themes §4), so the selected accent
/// drives it rather than a fixed brand color.
struct PrimaryActionStyle: ViewModifier {
    func body(content: Content) -> some View {
        if #available(iOS 26.0, *) {
            content.buttonStyle(.glassProminent)
        } else {
            content.buttonStyle(.borderedProminent)
        }
    }
}

/// A surface that reads as Liquid Glass on iOS 26+ and as a material below.
struct GlassBar: ViewModifier {
    func body(content: Content) -> some View {
        if #available(iOS 26.0, *) {
            content.background(.clear).glassEffect(in: .rect(cornerRadius: 0))
        } else {
            content.background(.regularMaterial)
        }
    }
}

extension View {
    func primaryAction() -> some View { modifier(PrimaryActionStyle()) }
    func glassBar() -> some View { modifier(GlassBar()) }
}
