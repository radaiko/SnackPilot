import SwiftUI

/// Bestellungen (04-ui-ux §2): the fetched orders split into upcoming and past. Upcoming
/// orders can be confirmed or cancelled.
struct OrdersView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        NavigationStack {
            Group {
                if let split = model.ordersSplit, !(split.upcoming.isEmpty && split.past.isEmpty) {
                    List {
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
                } else {
                    ContentUnavailableView("Keine Bestellungen",
                                           systemImage: "checklist",
                                           description: Text("Bestelle ein Menü im Menüs-Tab."))
                }
            }
            .navigationTitle("Bestellungen")
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
                if !order.subtitle.isEmpty {
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
