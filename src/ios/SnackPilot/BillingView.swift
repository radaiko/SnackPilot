import SwiftUI

/// Abrechnung tab (billing §6). A month picker over the two billing sources — Gourmet (Kantine)
/// monthly bills and Ventopay (Automaten) transactions — presented as ONE unified, date-descending
/// list with a source filter (Alle / Kantine / Automaten) and a summary bar (Gesamt / Belege /
/// Zuschuss).
struct BillingView: View {
    @EnvironmentObject var model: AppModel

    /// Source filter (billing §6.1). UI-only, not persisted; resets to `.all` on restart.
    @State private var sourceFilter: SourceFilter = .all

    enum SourceFilter: Hashable { case all, gourmet, ventopay }
    enum Source { case gourmet, ventopay }

    /// One row in the unified list (billing §6.2). A Gourmet bill carries all its line items
    /// (count × description → total), like v1's BillCard; Ventopay carries none and uses `description`.
    struct Entry: Identifiable {
        let id: String
        let epochMs: Int64
        let source: Source
        let description: String
        let amount: Double
        var items: [ItemLine] = []
    }

    struct ItemLine: Identifiable {
        let id = UUID()
        let count: Int64
        let description: String
        let total: Double
    }

    var body: some View {
        NavigationStack {
            Group {
                if !anyAuth {
                    // Neither source authenticated and nothing cached (settings §3.7).
                    ContentUnavailableView("Anmeldung erforderlich",
                                           systemImage: "person.crop.circle.badge.xmark",
                                           description: Text("Melde dich in den Einstellungen an."))
                } else {
                    List {
                        monthSection
                        filterSection
                        if !entries.isEmpty {
                            summarySection
                        }
                        entriesSection
                    }
                    .refreshable { await model.reloadBilling() }
                }
            }
            .toolbar(.hidden, for: .navigationBar)
        }
    }

    // MARK: Sections

    @ViewBuilder private var monthSection: some View {
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
    }

    private var filterSection: some View {
        Section {
            Picker("Quelle", selection: $sourceFilter) {
                Text("Alle").tag(SourceFilter.all)
                Text("Kantine").tag(SourceFilter.gourmet)
                Text("Automaten").tag(SourceFilter.ventopay)
            }
            .pickerStyle(.segmented)
        }
    }

    @ViewBuilder private var summarySection: some View {
        Section {
            HStack(alignment: .top) {
                metric("Gesamt", Self.euro(total))
                Spacer()
                metric("Belege", "\(count)")
                if subsidy > 0 {
                    Spacer()
                    metric("Zuschuss", Self.euro(subsidy), color: .green)
                }
            }
        }
    }

    @ViewBuilder private var entriesSection: some View {
        if entries.isEmpty {
            Section {
                Text("Keine Abrechnungsdaten für diesen Monat")
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.vertical, 24)
                    .listRowBackground(Color.clear)
            }
        } else {
            Section {
                ForEach(entries) { entry in
                    entryRow(entry)
                }
            }
        }
    }

    private func entryRow(_ entry: Entry) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(Self.dateLabel(entry.epochMs)).font(.subheadline)
                    Text(Self.timeLabel(entry.epochMs)).font(.caption).foregroundStyle(.secondary)
                    if entry.items.isEmpty && !entry.description.isEmpty {
                        Text(entry.description).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                    }
                }
                Spacer()
                VStack(alignment: .trailing, spacing: 4) {
                    sourceBadge(entry.source)
                    Text(Self.euro(entry.amount)).bold().monospacedDigit()
                }
            }
            // Kantine bill line items (billing §6): count × description → item total, like v1.
            if !entry.items.isEmpty {
                VStack(alignment: .leading, spacing: 3) {
                    ForEach(entry.items) { item in
                        HStack(spacing: 6) {
                            Text("\(item.count)×").font(.caption2).monospacedDigit()
                                .foregroundStyle(.tertiary)
                            Text(item.description).font(.caption).foregroundStyle(.secondary)
                                .lineLimit(1)
                            Spacer()
                            Text(Self.euro(item.total)).font(.caption2).monospacedDigit()
                                .foregroundStyle(.tertiary)
                        }
                    }
                }
                .padding(.top, 2)
            }
        }
    }

    private func sourceBadge(_ source: Source) -> some View {
        let (text, color): (String, Color)
        switch source {
        case .gourmet: (text, color) = ("Kantine", model.accentColor)
        case .ventopay: (text, color) = ("Automaten", .green)
        }
        return Text(text.uppercased())
            .font(.caption2).bold()
            .padding(.horizontal, 6).padding(.vertical, 2)
            .background(color.opacity(0.18), in: Capsule())
            .foregroundStyle(color)
    }

    private func metric(_ label: String, _ value: String, color: Color = .primary) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label).font(.caption).foregroundStyle(.secondary)
            Text(value).font(.headline).monospacedDigit().foregroundStyle(color)
        }
    }

    // MARK: Derived data (billing §6.2 / §6.3)

    private var anyAuth: Bool { model.gourmetAuthenticated || model.ventopayAuthenticated }

    /// Merged, date-descending entry list for the selected month, gated on auth + source filter.
    private var entries: [Entry] {
        var list: [Entry] = []
        if sourceFilter != .ventopay, model.gourmetAuthenticated, let g = model.gourmetMonth {
            for bill in g.bills {
                list.append(Entry(id: "g-\(bill.billNr)",
                                  epochMs: bill.billDateEpochMs,
                                  source: .gourmet,
                                  description: "",
                                  amount: bill.billing,
                                  items: bill.items.map {
                                      ItemLine(count: $0.count, description: $0.description, total: $0.total)
                                  }))
            }
        }
        if sourceFilter != .gourmet, model.ventopayAuthenticated, let v = model.ventopayMonth {
            for tx in v.transactions {
                list.append(Entry(id: "v-\(tx.id)",
                                  epochMs: tx.dateEpochMs,
                                  source: .ventopay,
                                  description: tx.restaurant,
                                  amount: tx.amount))
            }
        }
        return list.sorted { $0.epochMs > $1.epochMs }
    }

    private var gourmetTotal: Double {
        (sourceFilter != .ventopay && model.gourmetAuthenticated) ? (model.gourmetMonth?.totalBilling ?? 0) : 0
    }
    private var ventopayTotal: Double {
        (sourceFilter != .gourmet && model.ventopayAuthenticated) ? (model.ventopayMonth?.total ?? 0) : 0
    }
    private var total: Double { gourmetTotal + ventopayTotal }
    private var subsidy: Double {
        (sourceFilter != .ventopay && model.gourmetAuthenticated) ? (model.gourmetMonth?.totalSubsidy ?? 0) : 0
    }
    private var count: Int { entries.count }

    // MARK: Formatting (billing §8.3 — native de_AT locale)

    static func euro(_ v: Double) -> String {
        let f = NumberFormatter()
        f.locale = Locale(identifier: "de_AT")
        f.numberStyle = .currency
        f.currencyCode = "EUR"
        return f.string(from: NSNumber(value: v)) ?? String(format: "%.2f €", v)
    }

    static func dateLabel(_ epochMs: Int64) -> String {
        let d = Date(timeIntervalSince1970: Double(epochMs) / 1000)
        let f = DateFormatter()
        f.locale = Locale(identifier: "de_AT")
        f.dateFormat = "EEE, d. MMM yyyy"
        return f.string(from: d)
    }

    static func timeLabel(_ epochMs: Int64) -> String {
        let d = Date(timeIntervalSince1970: Double(epochMs) / 1000)
        let f = DateFormatter()
        f.locale = Locale(identifier: "de_AT")
        f.dateFormat = "HH:mm"
        return f.string(from: d)
    }
}
