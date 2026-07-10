import SwiftUI

/// Abrechnung (04-ui-ux §2): a month picker over the two billing sources — Gourmet (Kantine)
/// monthly bills and Ventopay (Automaten) transactions — with per-source totals.
struct BillingView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        NavigationStack {
            List {
                if !model.monthOptions.isEmpty {
                    Section {
                        Picker("Monat", selection: Binding(
                            get: { model.selectedOffset },
                            set: { off in Task { await model.selectMonth(offset: off) } }
                        )) {
                            ForEach(model.monthOptions, id: \.offset) { m in
                                Text(m.label).tag(m.offset)
                            }
                        }
                        .pickerStyle(.segmented)
                    }
                }

                gourmetSection
                ventopaySection
            }
            .navigationTitle("Abrechnung")
            .overlay {
                if !hasData {
                    if !model.gourmetAuthenticated && !model.ventopayAuthenticated {
                        // Neither source authenticated and nothing cached (settings §3.7).
                        ContentUnavailableView("Anmeldung erforderlich",
                                               systemImage: "person.crop.circle.badge.xmark",
                                               description: Text("Melde dich in den Einstellungen an."))
                    } else {
                        ContentUnavailableView("Keine Abrechnung",
                                               systemImage: "eurosign.circle",
                                               description: Text("Für diesen Zeitraum liegen keine Buchungen vor."))
                    }
                }
            }
        }
    }

    /// True when an AUTHENTICATED source has rows for the selected month. Gating on the auth
    /// flag (not just a non-empty month) stops a logged-out account's still-cached billing from
    /// reappearing when a month is re-selected (the core keeps its month cache after logout).
    private var hasData: Bool {
        let gourmet = model.gourmetAuthenticated && (model.gourmetMonth.map { !$0.bills.isEmpty } ?? false)
        let ventopay = model.ventopayAuthenticated && (model.ventopayMonth.map { !$0.transactions.isEmpty } ?? false)
        return gourmet || ventopay
    }

    @ViewBuilder private var gourmetSection: some View {
        if model.gourmetAuthenticated, let g = model.gourmetMonth, !g.bills.isEmpty {
            Section("Kantine") {
                ForEach(g.bills, id: \.billNr) { bill in
                    HStack {
                        VStack(alignment: .leading) {
                            Text(Self.dateLabel(bill.billDateEpochMs)).font(.subheadline)
                            if let first = bill.items.first {
                                Text(first.description).font(.caption).foregroundStyle(.secondary)
                            }
                        }
                        Spacer()
                        Text(Self.euro(bill.billing)).monospacedDigit()
                    }
                }
                totalRow("Summe (Kantine)", g.totalBilling)
            }
        }
    }

    @ViewBuilder private var ventopaySection: some View {
        if model.ventopayAuthenticated, let v = model.ventopayMonth, !v.transactions.isEmpty {
            Section("Automaten") {
                ForEach(v.transactions, id: \.id) { tx in
                    HStack {
                        VStack(alignment: .leading) {
                            Text(Self.dateLabel(tx.dateEpochMs)).font(.subheadline)
                            Text(tx.restaurant).font(.caption).foregroundStyle(.secondary)
                        }
                        Spacer()
                        Text(Self.euro(tx.amount)).monospacedDigit()
                    }
                }
                totalRow("Summe (Automaten)", v.total)
            }
        }
    }

    private func totalRow(_ label: String, _ value: Double) -> some View {
        HStack {
            Text(label).bold()
            Spacer()
            Text(Self.euro(value)).bold().monospacedDigit()
        }
    }

    static func euro(_ v: Double) -> String {
        String(format: "%.2f €", v).replacingOccurrences(of: ".", with: ",")
    }

    static func dateLabel(_ epochMs: Int64) -> String {
        let d = Date(timeIntervalSince1970: Double(epochMs) / 1000)
        let f = DateFormatter()
        f.locale = Locale(identifier: "de_AT")
        f.dateFormat = "EEE, d. MMM"
        return f.string(from: d)
    }
}
