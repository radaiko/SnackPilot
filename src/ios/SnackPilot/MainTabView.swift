import SwiftUI

/// The four-tab shell (docs/requirements 04-ui-ux §1). Only Menüs is wired in this vertical
/// slice; the other three are placeholders until their stores get their screens.
struct MainTabView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        TabView(selection: $model.selectedTab) {
            MenusView()
                .tabItem { Label("Menüs", systemImage: "fork.knife") }
                .tag(0)
            PlaceholderView(title: "Bestellungen", symbol: "checklist")
                .tabItem { Label("Bestellungen", systemImage: "checklist") }
                .tag(1)
            BillingView()
                .tabItem { Label("Abrechnung", systemImage: "eurosign.circle") }
                .tag(2)
            SettingsView()
                .tabItem { Label("Einstellungen", systemImage: "gearshape") }
                .tag(3)
        }
    }
}

/// Stand-in for the not-yet-built tabs.
struct PlaceholderView: View {
    let title: String
    let symbol: String

    var body: some View {
        NavigationStack {
            ContentUnavailableView(title, systemImage: symbol,
                                   description: Text("Kommt in einer späteren Iteration."))
                .navigationTitle(title)
        }
    }
}

/// Minimal settings — enough to end the session in the vertical slice.
struct SettingsView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        NavigationStack {
            Form {
                if let user = model.userInfo {
                    Section("Konto") {
                        LabeledContent("Benutzer", value: user.username)
                        if model.demoMode {
                            LabeledContent("Modus", value: "Demo")
                        }
                    }
                }
                Section {
                    LabeledContent("Core-Version", value: model.coreVersion)
                }
                Section {
                    Button("Abmelden", role: .destructive) { model.logout() }
                }
            }
            .navigationTitle("Einstellungen")
        }
    }
}
