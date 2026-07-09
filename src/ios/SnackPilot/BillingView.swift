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
                if model.monthOptions.isEmpty {
                    ContentUnavailableView("Keine Abrechnung",
                                           systemImage: "eurosign.circle",
                                           description: Text("Nach der Anmeldung verfügbar."))
                }
            }
        }
    }

    @ViewBuilder private var gourmetSection: some View {
        if let g = model.gourmetMonth, !g.bills.isEmpty {
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
        if let v = model.ventopayMonth, !v.transactions.isEmpty {
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
