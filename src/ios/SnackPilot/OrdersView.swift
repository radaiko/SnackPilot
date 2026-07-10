import SwiftUI

/// Bestellungen (04-ui-ux §2): the fetched orders split into upcoming and past. Upcoming
/// orders can be confirmed or cancelled.
struct OrdersView: View {
    @EnvironmentObject var model: AppModel

    /// Upcoming orders still awaiting approval (orders §5.3 confirm banner).
    private var unconfirmedCount: Int {
        model.ordersSplit?.upcoming.filter { !$0.approved }.count ?? 0
    }

    var body: some View {
        NavigationStack {
            Group {
                if let split = model.ordersSplit, !(split.upcoming.isEmpty && split.past.isEmpty) {
                    List {
                        if unconfirmedCount > 0 {
                            Section {
                                confirmBanner(count: unconfirmedCount)
                            }
                        }
                        if !split.upcoming.isEmpty {
                            Section("Anstehend") {
                                ForEach(split.upcoming, id: \.positionId) { order in
                                    OrderRow(order: order, cancellable: true)
                                }
                            }
                        }
                        if !split.past.isEmpty {
                            Section("Vergangen") {
                                ForEach(split.past, id: \.positionId) { order in
                                    OrderRow(order: order, cancellable: false)
                                }
                            }
                        }
                    }
                } else if !model.gourmetAuthenticated {
                    // No-wall navigation (settings §3.7): unauthenticated + no cached orders.
                    ContentUnavailableView("Nicht angemeldet",
                                           systemImage: "person.crop.circle.badge.xmark",
                                           description: Text("Melde dich in den Einstellungen an."))
                } else {
                    ContentUnavailableView("Keine Bestellungen",
                                           systemImage: "checklist",
                                           description: Text("Bestelle ein Menü im Menüs-Tab."))
                }
            }
            .toolbar(.hidden, for: .navigationBar)
        }
    }

    private func confirmBanner(count: Int) -> some View {
        HStack(spacing: 12) {
            Image(systemName: "exclamationmark.circle.fill")
                .foregroundStyle(.orange)
            Text("\(count) unbestätigte \(count == 1 ? "Bestellung" : "Bestellungen")")
                .font(.subheadline)
            Spacer()
            Button("Bestätigen") {
                Task { await model.confirmOrders() }
            }
            .buttonStyle(.borderedProminent)
            .disabled(model.busy)
        }
    }
}

private struct OrderRow: View {
    @EnvironmentObject var model: AppModel
    let order: OrderedMenu
    let cancellable: Bool

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(order.title).font(.body)
                // Show the actual dish (looked up from the menu); fall back to the order's own
                // subtitle (the weekday) when the menu for that day isn't loaded.
                if let dish = model.dish(for: order) {
                    Text(dish).font(.footnote).foregroundStyle(.secondary)
                } else if !order.subtitle.isEmpty {
                    Text(order.subtitle).font(.footnote).foregroundStyle(.secondary)
                }
                Text(Self.dateLabel(order.dateEpochMs)).font(.caption2).foregroundStyle(.secondary)
            }
            Spacer()
            if order.approved {
                Image(systemName: "checkmark.seal.fill").foregroundStyle(.green)
            } else if cancellable {
                Button(role: .destructive) {
                    Task { await model.cancelOrder(order.positionId) }
                } label: {
                    Image(systemName: "trash")
                }
                .buttonStyle(.borderless)
            }
        }
    }

    static func dateLabel(_ epochMs: Int64) -> String {
        let d = Date(timeIntervalSince1970: Double(epochMs) / 1000)
        let f = DateFormatter()
        f.locale = Locale(identifier: "de_AT")
        f.dateFormat = "EEE, d. MMM"
        return f.string(from: d)
    }
}
