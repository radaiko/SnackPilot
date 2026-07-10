import SwiftUI

/// Top-level shell: always the four-tab app (settings §3.7 — no login wall). Unauthenticated
/// users see per-tab "not signed in" empty states and log in from the Settings tab.
///
/// Also the single place the appearance settings are applied (themes §4): `.preferredColorScheme`
/// forces the app's light/dark mode from the preference, and `.tint` propagates the selected
/// accent to every control below. Reading the environment scheme here re-resolves the accent the
/// instant the OS flips light/dark while the preference is SYSTEM (themes §1.1).
struct RootView: View {
    @EnvironmentObject var model: AppModel
    @Environment(\.colorScheme) private var systemScheme

    var body: some View {
        MainTabView()
            .tint(model.accentColor(systemDark: systemScheme == .dark))
            .preferredColorScheme(model.preferredColorScheme)
    }
}
