import SwiftUI

/// Menu list grouped by day (docs/requirements 04-ui-ux §2). Renders whatever `MenuSnapshot`
/// the core produced — live-fetched after login, or the offline demo set.
struct MenusView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        NavigationStack {
            Group {
                if let snapshot = model.snapshot, !snapshot.items.isEmpty {
                    List {
                        ForEach(snapshot.availableDates, id: \.self) { day in
                            Section(Self.dayLabel(day)) {
                                ForEach(items(for: day, in: snapshot), id: \.id) { item in
                                    MenuRow(item: item)
                                }
                            }
                        }
                    }
                } else if model.busy {
                    ProgressView("Menüs werden geladen …")
                } else {
                    ContentUnavailableView("Keine Menüs",
                                           systemImage: "fork.knife",
                                           description: Text("Für diesen Zeitraum liegen keine Menüs vor."))
                }
            }
            .navigationTitle("Menüs")
            .toolbar {
                if model.demoMode {
                    ToolbarItem(placement: .topBarTrailing) {
                        Text("DEMO")
                            .font(.caption2).bold()
                            .padding(.horizontal, 6).padding(.vertical, 2)
                            .background(.yellow.opacity(0.3), in: Capsule())
                    }
                }
            }
        }
    }

    private func items(for day: String, in snapshot: MenuSnapshot) -> [MenuItem] {
        snapshot.items.filter { $0.day == day }
    }

    /// `YYYY-MM-DD` (the core's normalized day key) → localized weekday + date, falling back
    /// to the raw key.
    static func dayLabel(_ key: String) -> String {
        let parser = DateFormatter()
        parser.dateFormat = "yyyy-MM-dd"
        parser.locale = Locale(identifier: "en_US_POSIX")
        guard let date = parser.date(from: key) else { return key }
        let out = DateFormatter()
        out.locale = Locale(identifier: "de_AT")
        out.dateFormat = "EEEE, d. MMMM"
        return out.string(from: date)
    }
}

private struct MenuRow: View {
    let item: MenuItem

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(categoryLabel).font(.caption2).foregroundStyle(.secondary)
                Text(item.title).font(.body)
                if !item.subtitle.isEmpty {
                    Text(item.subtitle).font(.footnote).foregroundStyle(.secondary)
                }
                if !item.allergens.isEmpty {
                    Text("Allergene: \(item.allergens.joined(separator: ", "))")
                        .font(.caption2).foregroundStyle(.tertiary)
                }
            }
            Spacer()
            VStack(alignment: .trailing, spacing: 4) {
                if !item.price.isEmpty {
                    Text(item.price).font(.callout).monospacedDigit()
                }
                if item.ordered {
                    Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
                } else if !item.available {
                    Image(systemName: "lock.fill").foregroundStyle(.secondary)
                }
            }
        }
        .padding(.vertical, 2)
    }

    private var categoryLabel: String {
        switch item.category {
        case .menu1: return "Menü I"
        case .menu2: return "Menü II"
        case .menu3: return "Menü III"
        case .soupAndSalad: return "Suppe & Salat"
        case .unknown: return ""
        }
    }
}
