import SwiftUI

/// Menüs tab (menus §4–§6). Shows exactly one day at a time behind a day navigator; the day's
/// items are grouped by category (menus §5) and rendered as cards whose interactivity respects
/// the 09:00 ordering cutoff and availability (menus §6.1). Tapping an orderable card toggles a
/// pending order/cancellation; the submit bar commits all pending changes through the core.
struct MenusView: View {
    @EnvironmentObject var model: AppModel

    /// Fixed category display order (menus §5).
    private static let categoryOrder: [MenuCategory] = [.menu1, .menu2, .menu3, .soupAndSalad, .unknown]

    var body: some View {
        NavigationStack {
            Group {
                if let snapshot = model.snapshot, !snapshot.items.isEmpty {
                    VStack(spacing: 0) {
                        dayNavigator(snapshot)
                        dayList(snapshot)
                    }
                } else if model.busy {
                    ProgressView("Menüs werden geladen …")
                } else if !model.gourmetAuthenticated {
                    // No-wall navigation (settings §3.7): unauthenticated + no cached data.
                    ContentUnavailableView("Nicht angemeldet",
                                           systemImage: "person.crop.circle.badge.xmark",
                                           description: Text("Melde dich in den Einstellungen an."))
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

    // MARK: Day navigator (menus §4)

    @ViewBuilder private func dayNavigator(_ s: MenuSnapshot) -> some View {
        let dates = s.availableDates
        let idx = model.selectedDay.flatMap { dates.firstIndex(of: $0) }
        let position = idx.map { $0 + 1 } ?? 0
        let prevDisabled = (idx ?? 0) <= 0
        let nextDisabled = idx.map { $0 >= dates.count - 1 } ?? false
        // "Heute" (center-tap → nearest day, menus §4.1) is reachable whenever we're not already
        // on the nearest day — including weekends when today itself has no menu.
        let onNearest = model.selectedDay == AppModel.nearestDay(in: dates)

        VStack(spacing: 4) {
            HStack {
                Button { model.prevDay() } label: {
                    Image(systemName: "chevron.left").font(.title3.weight(.semibold))
                }
                .disabled(prevDisabled)
                .opacity(prevDisabled ? 0.3 : 1)

                Spacer()

                // Tapping the center jumps to the nearest menu day (menus §4.1).
                Button { model.goToToday() } label: {
                    VStack(spacing: 2) {
                        Text(Self.dayLabel(model.selectedDay ?? "")).font(.headline)
                        Text("\(position) / \(dates.count)")
                            .font(.caption).foregroundStyle(.secondary).monospacedDigit()
                    }
                }
                .buttonStyle(.plain)

                Spacer()

                Button { model.nextDay() } label: {
                    Image(systemName: "chevron.right").font(.title3.weight(.semibold))
                }
                .disabled(nextDisabled)
                .opacity(nextDisabled ? 0.3 : 1)
            }
            .tint(.brand)

            if !onNearest {
                Button("Heute") { model.goToToday() }
                    .font(.caption).tint(.brand)
            }
        }
        .padding(.horizontal)
        .padding(.vertical, 8)
        .glassBar()
    }

    // MARK: Day list (menus §5 grouping)

    @ViewBuilder private func dayList(_ s: MenuSnapshot) -> some View {
        let day = model.selectedDay
        let dayItems = day.map { d in s.items.filter { $0.day == d } } ?? []
        List {
            ForEach(Self.categoryOrder, id: \.self) { cat in
                let group = dayItems.filter { $0.category == cat }
                if !group.isEmpty {
                    Section {
                        ForEach(group, id: \.id) { item in
                            menuButton(item, s)
                        }
                    } header: {
                        if let heading = Self.categoryHeading(cat) {
                            Text(heading)
                        }
                    }
                }
            }
        }
    }

    @ViewBuilder private func menuButton(_ item: MenuItem, _ s: MenuSnapshot) -> some View {
        let key = "\(item.id)|\(item.day)"
        let isPendingOrder = s.pendingOrders.contains(key)
        let isPendingCancel = s.pendingCancellations.contains(key)
        let cutoff = model.isCutoff(item.day)
        // menus §6.1: an ordered item stays tappable (to cancel) even past cutoff; a
        // not-yet-ordered item is tappable only while available and before cutoff.
        let canInteract = item.ordered || (item.available && !cutoff)

        Button {
            model.toggle(item: item)
        } label: {
            MenuRow(item: item,
                    isPendingOrder: isPendingOrder,
                    isPendingCancel: isPendingCancel,
                    cutoff: cutoff)
        }
        .buttonStyle(.plain)
        .disabled(!canInteract)
    }

    private var submitBar: some View {
        VStack(spacing: 8) {
            if model.busy, let phase = model.orderProgress {
                HStack(spacing: 8) {
                    ProgressView()
                    Text(Self.progressLabel(phase)).font(.footnote).foregroundStyle(.secondary)
                    Spacer()
                }
            }
            HStack(spacing: 12) {
                Button("Verwerfen") { model.clearPending() }
                    .buttonStyle(.bordered)
                    .disabled(model.busy)
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
        }
        .padding()
        .glassBar()
    }

    /// Live submit-pipeline phase labels (menus §6.6).
    static func progressLabel(_ phase: OrderProgress) -> String {
        switch phase {
        case .adding: return "Wird in den Warenkorb gelegt …"
        case .confirming: return "Wird bestätigt …"
        case .cancelling: return "Wird storniert …"
        case .refreshing: return "Wird aktualisiert …"
        }
    }

    /// Category heading (menus §5): uppercase display strings, with SUPPE & SALAT suppressed.
    static func categoryHeading(_ cat: MenuCategory) -> String? {
        switch cat {
        case .menu1: return "MENÜ I"
        case .menu2: return "MENÜ II"
        case .menu3: return "MENÜ III"
        case .soupAndSalad: return nil // heading suppressed (menus §5)
        case .unknown: return "UNKNOWN"
        }
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
    let isPendingOrder: Bool
    let isPendingCancel: Bool
    let cutoff: Bool

    /// Card state badge (menus §6.1), evaluated in priority order.
    private var badge: (text: String, color: Color)? {
        if isPendingCancel { return ("Wird storniert", .orange) }
        if item.ordered { return ("Bestellt", .green) }
        if !item.available { return ("Ausverkauft", .secondary) }
        if cutoff { return ("Geschlossen", .secondary) }
        return nil
    }

    /// Dim non-orderable / to-be-cancelled cards (menus §6.1).
    private var dim: Double {
        if isPendingCancel { return 0.55 }
        if !item.ordered && (!item.available || cutoff) { return 0.5 }
        return 1
    }

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            icon
            VStack(alignment: .leading, spacing: 2) {
                HStack {
                    Text(item.title).font(.body)
                    if let badge {
                        Text(badge.text)
                            .font(.caption2).bold()
                            .padding(.horizontal, 6).padding(.vertical, 2)
                            .background(badge.color.opacity(0.18), in: Capsule())
                            .foregroundStyle(badge.color)
                    }
                }
                if !item.subtitle.isEmpty {
                    Text(item.subtitle).font(.footnote).foregroundStyle(.secondary)
                        .lineLimit(4)
                }
                if !item.allergens.isEmpty {
                    Text("Allergene: \(item.allergens.joined(separator: ", "))")
                        .font(.caption2).foregroundStyle(.tertiary).lineLimit(1)
                }
            }
            Spacer()
            if !item.price.isEmpty {
                Text(item.price).font(.callout).monospacedDigit()
            }
        }
        .padding(.vertical, 2)
        .opacity(dim)
        .strikethrough(isPendingCancel)
    }

    @ViewBuilder private var icon: some View {
        if isPendingCancel {
            Image(systemName: "minus.circle.fill").foregroundStyle(.orange)
        } else if isPendingOrder {
            Image(systemName: "checkmark.circle.fill").foregroundStyle(Color.brand)
        } else if item.ordered {
            Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
        } else {
            Image(systemName: "circle").foregroundStyle(.secondary)
        }
    }
}
