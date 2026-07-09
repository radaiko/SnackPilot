import SwiftUI

/// Menu list grouped by day (docs/requirements 04-ui-ux §2). Tapping a menu toggles a pending
/// order/cancellation; a submit bar commits them through the core.
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
                                    Button {
                                        model.toggle(item: item)
                                    } label: {
                                        MenuRow(item: item, state: state(item, snapshot))
                                    }
                                    .buttonStyle(.plain)
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
            .safeAreaInset(edge: .bottom) {
                if model.hasPendingChanges {
                    submitBar
                }
            }
        }
    }

    private var submitBar: some View {
        HStack(spacing: 12) {
            Button("Verwerfen") { model.clearPending() }
                .buttonStyle(.bordered)
            Button {
                Task { await model.submitOrders() }
            } label: {
                if model.busy {
                    ProgressView()
                } else {
                    Text("Bestellen").bold().frame(maxWidth: .infinity)
                }
            }
            .primaryAction()
            .disabled(model.busy)
        }
        .padding()
        .glassBar()
    }

    private func items(for day: String, in snapshot: MenuSnapshot) -> [MenuItem] {
        snapshot.items.filter { $0.day == day }
    }

    /// Effective order state for a row, combining the fetched `ordered` flag with pending edits.
    private func state(_ item: MenuItem, _ s: MenuSnapshot) -> OrderRowState {
        let key = "\(item.id)|\(item.day)"
        if s.pendingOrders.contains(key) { return .pendingOrder }
        if s.pendingCancellations.contains(key) { return .pendingCancel }
        if item.ordered { return .ordered }
        return .none
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

enum OrderRowState {
    case none, ordered, pendingOrder, pendingCancel
}

private struct MenuRow: View {
    let item: MenuItem
    let state: OrderRowState

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            icon
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
            if !item.price.isEmpty {
                Text(item.price).font(.callout).monospacedDigit()
            }
        }
        .padding(.vertical, 2)
    }

    @ViewBuilder private var icon: some View {
        switch state {
        case .ordered:
            Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
        case .pendingOrder:
            Image(systemName: "plus.circle.fill").foregroundStyle(.blue)
        case .pendingCancel:
            Image(systemName: "minus.circle.fill").foregroundStyle(.orange)
        case .none:
            Image(systemName: "circle").foregroundStyle(.secondary)
        }
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
