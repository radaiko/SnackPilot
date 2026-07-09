import SwiftUI

/// App theming. A single warm brand accent drives every control tint; primary actions adopt
/// iOS 26 Liquid Glass where available and fall back to the bordered-prominent style on iOS 17–18.
extension Color {
    /// SnackPilot brand accent — a warm, appetizing amber.
    static let brand = Color(red: 0.96, green: 0.45, blue: 0.13)
}

/// Primary call-to-action button: Liquid Glass on iOS 26+, bordered-prominent below.
struct PrimaryActionStyle: ViewModifier {
    func body(content: Content) -> some View {
        if #available(iOS 26.0, *) {
            content.buttonStyle(.glassProminent).tint(.brand)
        } else {
            content.buttonStyle(.borderedProminent).tint(.brand)
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
