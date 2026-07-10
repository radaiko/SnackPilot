import SwiftUI

/// Top-level shell: always the four-tab app (settings §3.7 — no login wall). Unauthenticated
/// users see per-tab "not signed in" empty states and log in from the Settings tab.
struct RootView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        MainTabView()
            .tint(.brand)
    }
}
