import SwiftUI

/// Bestellungen (04-ui-ux §2): the fetched orders split into upcoming and past. Upcoming
/// orders can be confirmed or cancelled.
struct OrdersView: View {
    @EnvironmentObject var model: AppModel

    /// Upcoming orders still awaiting approval (orders §5.3 confirm banner).
    private var unconfirmedCount: Int {
        model.ordersSplit?.upcoming.filter { !$0.approved }.count ?? 0
    }

    /// Drives the batch-cancel confirmation dialog.
    @State private var confirmingBatch = false

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
                            Section {
                                ForEach(split.upcoming, id: \.positionId) { order in
                                    OrderRow(order: order, cancellable: true)
                                }
                            } header: {
                                HStack {
                                    Text("Anstehend")
                                    Spacer()
                                    // Multi-select is offered when at least one upcoming order is
                                    // still cancellable — the "week of holiday" case (orders §6.1).
                                    if split.upcoming.contains(where: { !model.isCancellationCutoff($0) }) {
                                        Button(model.orderSelectionMode ? "Fertig" : "Auswählen") {
                                            if model.orderSelectionMode { model.endOrderSelection() }
                                            else { model.orderSelectionMode = true }
                                        }
                                        .font(.caption).textCase(nil)
                                    }
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
                    .safeAreaInset(edge: .bottom) {
                        if model.orderSelectionMode { selectionBar }
                    }
                } else if !model.gourmetAuthenticated {
                    // No-wall navigation (settings §3.7): unauthenticated + no cached orders.
                    ContentUnavailableView("Nicht angemeldet",
                                           systemImage: "person.crop.circle.badge.xmark",
                                           description: Text("Melde dich in den Einstellungen an."))
                } else if let message = model.ordersError {
                    // A fetch/parse failure — distinct from having no orders. Offer a retry.
                    ContentUnavailableView {
                        Label("Bestellungen nicht geladen", systemImage: "exclamationmark.triangle")
                    } description: {
                        Text(message)
                    } actions: {
                        Button("Erneut versuchen") { Task { await model.loadOrders() } }
                    }
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
        VStack(alignment: .leading, spacing: 8) {
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
            // Surface a failed confirm inline — otherwise the banner just stays and the user has
            // no idea the confirm didn't go through (orders §5.3).
            if !model.busy, let message = model.ordersError {
                HStack(spacing: 8) {
                    Image(systemName: "exclamationmark.triangle.fill")
                    Text(message).font(.footnote)
                }
                .foregroundStyle(.red)
            }
        }
    }

    /// Bottom bar in multi-select mode: batch "N stornieren" + Abbrechen, or a progress line.
    private var selectionBar: some View {
        VStack(spacing: 8) {
            if model.busy {
                HStack(spacing: 8) {
                    ProgressView()
                    Text("Wird storniert …").font(.footnote).foregroundStyle(.secondary)
                    Spacer()
                }
            } else {
                HStack(spacing: 12) {
                    Button("Abbrechen") { model.endOrderSelection() }
                        .buttonStyle(.bordered)
                    Button(role: .destructive) {
                        confirmingBatch = true
                    } label: {
                        Text("\(model.selectedOrderIds.count) stornieren")
                            .bold().frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(model.selectedOrderIds.isEmpty)
                }
            }
        }
        .padding()
        .background(.ultraThinMaterial)
        .confirmationDialog("Bestellungen stornieren", isPresented: $confirmingBatch, titleVisibility: .visible) {
            Button("\(model.selectedOrderIds.count) stornieren", role: .destructive) {
                Task { await model.cancelSelectedOrders() }
            }
            Button("Behalten", role: .cancel) {}
        } message: {
            Text("\(model.selectedOrderIds.count) \(model.selectedOrderIds.count == 1 ? "Bestellung" : "Bestellungen") stornieren?")
        }
    }
}

private struct OrderRow: View {
    @EnvironmentObject var model: AppModel
    let order: OrderedMenu
    let cancellable: Bool

    /// Drives the destructive confirmation dialog (orders §6.2).
    @State private var confirming = false

    /// This row's order is the one currently being cancelled (shows the delete spinner).
    private var cancelling: Bool { model.cancellingId == order.positionId }
    /// Past the 09:00 cancellation cutoff → cancel is unavailable (orders §6.4).
    private var cutoff: Bool { cancellable && model.isCancellationCutoff(order) }

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
                // Explain the greyed-out cancel button: the 09:00 cutoff has passed (orders §6.4).
                if cutoff {
                    Text("Stornofrist abgelaufen (nach 9:00)")
                        .font(.caption2).foregroundStyle(.orange)
                }
            }
            Spacer()
            if model.orderSelectionMode {
                // Multi-select: only cancellable, non-cutoff upcoming rows are selectable.
                if cancellable && !cutoff {
                    let selected = model.selectedOrderIds.contains(order.positionId)
                    Image(systemName: selected ? "checkmark.circle.fill" : "circle")
                        .imageScale(.large)
                        .foregroundStyle(selected ? model.accentColor : .secondary)
                } else if order.approved {
                    Image(systemName: "checkmark.seal.fill").foregroundStyle(.green)
                }
            } else {
                // Approval is a status indicator, NOT a substitute for the cancel button: an approved
                // upcoming order is still cancellable (orders §6.2 — cancel shows for all upcoming rows).
                if order.approved {
                    Image(systemName: "checkmark.seal.fill").foregroundStyle(.green)
                }
                if cancelling {
                    ProgressView()   // this order is being cancelled
                } else if cancellable {
                    Button(role: .destructive) {
                        confirming = true
                    } label: {
                        Image(systemName: "trash")
                    }
                    .buttonStyle(.borderless)
                    // Grey when past the cutoff (reason shown at left) or while another cancel runs.
                    .disabled(cutoff || model.cancellingId != nil)
                }
            }
        }
        .contentShape(Rectangle())
        .onTapGesture {
            if model.orderSelectionMode, cancellable, !cutoff {
                model.toggleOrderSelection(order.positionId)
            }
        }
        .confirmationDialog("Bestellung stornieren", isPresented: $confirming, titleVisibility: .visible) {
            Button("Stornieren", role: .destructive) {
                Task { await model.cancelOrder(order.positionId) }
            }
            Button("Behalten", role: .cancel) {}
        } message: {
            Text("\"\(order.title)\" stornieren?")
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
