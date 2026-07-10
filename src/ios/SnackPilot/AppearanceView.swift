import SwiftUI

/// "Darstellung" appearance screen (themes §5), pushed from the Settings "Konto"-adjacent
/// `Darstellung` row. Two cards: a light/dark/system **Design** picker and an **Akzentfarbe**
/// picker of five swatches. Every tap applies and persists immediately — there is no save button.
struct AppearanceView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        Form {
            Section("Design") {
                HStack(spacing: 12) {
                    ForEach(ThemePreference.allCases) { schemeCell($0) }
                }
                .padding(.vertical, 4)
            }

            Section("Akzentfarbe") {
                HStack(alignment: .top, spacing: 8) {
                    ForEach(AccentColor.allCases) { swatch($0) }
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 4)
            }
        }
        .navigationTitle("Darstellung")
        .navigationBarTitleDisplayMode(.inline)
    }

    // MARK: Design (theme preference)

    /// One of the three equal-width scheme options: an icon above a label; the selected option is
    /// highlighted with the current accent (themes §5).
    private func schemeCell(_ preference: ThemePreference) -> some View {
        let selected = model.themePreference == preference
        return Button {
            model.setThemePreference(preference)
        } label: {
            VStack(spacing: 6) {
                Image(systemName: preference.symbol).font(.title2)
                Text(preference.label).font(.caption)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 12)
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(selected ? model.accentColor.opacity(0.15) : Color.clear)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 12)
                    .strokeBorder(selected ? model.accentColor : Color.clear, lineWidth: 1)
            )
            .foregroundStyle(selected ? model.accentColor : Color.secondary)
        }
        .buttonStyle(.plain)
    }

    // MARK: Akzentfarbe (accent color)

    /// A circular swatch filled with the accent's **light-mode** primary (always the light value,
    /// even in dark mode; themes §5), with its German label beneath. The selected swatch gets a
    /// colored border and a white checkmark.
    private func swatch(_ accentColor: AccentColor) -> some View {
        let selected = model.accent == accentColor
        let fill = accentColor.lightPrimary
        return Button {
            model.setAccent(accentColor)
        } label: {
            VStack(spacing: 6) {
                ZStack {
                    Circle()
                        .fill(fill)
                        .frame(width: 40, height: 40)
                        .overlay(
                            Circle().strokeBorder(selected ? fill : Color.clear,
                                                  lineWidth: selected ? 3 : 2)
                        )
                    if selected {
                        Image(systemName: "checkmark")
                            .font(.system(size: 18, weight: .bold))
                            .foregroundStyle(.white)
                    }
                }
                Text(accentColor.label)
                    .font(.caption2)
                    .foregroundStyle(selected ? model.accentColor : Color.secondary)
            }
            .frame(maxWidth: .infinity)
        }
        .buttonStyle(.plain)
    }
}
