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
            OrdersView()
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
                Section("Benachrichtigungen") {
                    Toggle("Tägliche Bestell-Erinnerung", isOn: Binding(
                        get: { model.dailyReminderEnabled },
                        set: { model.setDailyReminder(enabled: $0, hour: model.reminderHour, minute: model.reminderMinute) }
                    ))
                    if model.dailyReminderEnabled {
                        DatePicker("Uhrzeit", selection: Binding(
                            get: {
                                Calendar.current.date(from: DateComponents(
                                    hour: model.reminderHour, minute: model.reminderMinute)) ?? Date()
                            },
                            set: {
                                let c = Calendar.current.dateComponents([.hour, .minute], from: $0)
                                model.setDailyReminder(enabled: true, hour: c.hour ?? 8, minute: c.minute ?? 0)
                            }
                        ), displayedComponents: .hourAndMinute)
                    }
                }

                Section {
                    LabeledContent("Core-Version", value: model.coreVersion)
                }

                Section("Diagnose") {
                    LabeledContent("Protokoll",
                                   value: model.logActive ? "Aktiv" : "Inaktiv")
                    Button("Protokoll aktivieren (24 h)") { model.activateLog() }
                    Button("Menü-Check ausführen") { Task { await model.runMenuCheck() } }
                    if !model.logEntries.isEmpty {
                        Button("Protokoll leeren", role: .destructive) { model.clearLog() }
                    }
                }

                if !model.logEntries.isEmpty {
                    Section("Protokoll-Einträge (\(model.logEntries.count))") {
                        ForEach(Array(model.logEntries.enumerated()), id: \.offset) { _, entry in
                            VStack(alignment: .leading, spacing: 2) {
                                Text("\(Self.subsystem(entry.subsystem)) · \(entry.event)")
                                    .font(.footnote)
                                if let detail = entry.detail, !detail.isEmpty {
                                    Text(detail).font(.caption2).foregroundStyle(.secondary)
                                }
                                Text(entry.ts).font(.caption2).foregroundStyle(.tertiary)
                            }
                        }
                    }
                }

                Section {
                    Button("Abmelden", role: .destructive) { model.logout() }
                }
            }
            .navigationTitle("Einstellungen")
            .onAppear { model.refreshLog() }
        }
    }

    static func subsystem(_ s: LogSubsystem) -> String {
        switch s {
        case .geofence: return "geofence"
        case .orderSync: return "order-sync"
        case .dailyReminder: return "daily-reminder"
        case .menuCheck: return "menu-check"
        }
    }
}
