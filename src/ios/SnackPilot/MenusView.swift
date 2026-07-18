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
                    // Swipe left/right to step between days (menus §4). Runs alongside the list's
                    // vertical scroll; only a predominantly-horizontal swipe changes the day.
                    .simultaneousGesture(
                        DragGesture(minimumDistance: 30, coordinateSpace: .local)
                            .onEnded { value in
                                let dx = value.translation.width
                                let dy = value.translation.height
                                guard abs(dx) > abs(dy), abs(dx) > 50 else { return }
                                if dx < 0 { model.nextDay() } else { model.prevDay() }
                            }
                    )
                } else if model.busy {
                    ProgressView("Menüs werden geladen …")
                } else if !model.gourmetAuthenticated {
                    // No-wall navigation (settings §3.7): unauthenticated + no cached data.
                    ContentUnavailableView("Nicht angemeldet",
                                           systemImage: "person.crop.circle.badge.xmark",
                                           description: Text("Melde dich in den Einstellungen an."))
                } else if let message = model.snapshot?.error ?? model.errorText {
                    // A fetch/parse failure — distinct from a genuinely empty menu (menus retry).
                    // Offer a retry so a transient network hiccup isn't a dead end.
                    ContentUnavailableView {
                        Label("Menüs nicht geladen", systemImage: "exclamationmark.triangle")
                    } description: {
                        Text(message)
                    } actions: {
                        Button("Erneut versuchen") { Task { await model.refreshMenus() } }
                    }
                } else {
                    ContentUnavailableView("Keine Menüs",
                                           systemImage: "fork.knife",
                                           description: Text("Für diesen Zeitraum liegen keine Menüs vor."))
                }
            }
            .toolbar(.hidden, for: .navigationBar)
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
        // "Heute" jumps to today's menu (menus §4.1). Only meaningful when today is actually a menu
        // day: on weekends/holidays today has no menu, the view opens on the nearest upcoming day,
        // and labeling a jump to it "Heute" would be wrong — so hide the button then.
        let todayKey = AppModel.todayKey()
        let todayIsAvailable = dates.contains(todayKey)
        let onToday = model.selectedDay == todayKey

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

            if todayIsAvailable && !onToday {
                Button("Heute") { model.goToToday() }
                    .font(.caption)
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
        let ordered = model.isOrdered(item) // display state (includes the orders cross-reference)
        // menus §6.1: a menu-marked ordered item stays tappable (to cancel) even past cutoff; an
        // un-ordered item is tappable only while available and before cutoff. A cross-ref-only
        // ordered item (present in the orders list but not flagged by the menu HTML) is NOT tappable
        // — cancel it in Bestellungen — so a tap can't create a duplicate order.
        let canInteract = item.ordered || (item.available && !cutoff && !ordered)

        Button {
            model.toggle(item: item)
        } label: {
            MenuRow(item: item,
                    isPendingOrder: isPendingOrder,
                    isPendingCancel: isPendingCancel,
                    cutoff: cutoff,
                    isOrdered: ordered)
        }
        .buttonStyle(.plain)
        .disabled(!canInteract)
    }

    private var submitBar: some View {
        VStack(spacing: 8) {
            if model.busy {
                // While submitting: ONE progress indicator (phase spinner + label). The buttons are
                // hidden rather than shown greyed/unreadable — there's nothing to do mid-submit.
                HStack(spacing: 8) {
                    ProgressView()
                    Text(model.orderProgress.map(Self.progressLabel) ?? "Wird verarbeitet …")
                        .font(.footnote).foregroundStyle(.secondary)
                    Spacer()
                }
            } else {
                // Surface a failed submit inline — without this the user sees the pending changes
                // silently persist and wrongly believes the order went through (menus §6.6).
                if let message = model.errorText {
                    HStack(spacing: 8) {
                        Image(systemName: "exclamationmark.triangle.fill")
                        Text(message).font(.footnote)
                        Spacer()
                    }
                    .foregroundStyle(.red)
                }
                HStack(spacing: 12) {
                    Button("Verwerfen") { model.clearPending() }
                        .buttonStyle(.bordered)
                    Button {
                        Task { await model.submitOrders() }
                    } label: {
                        Text("Bestellen").bold().frame(maxWidth: .infinity)
                    }
                    .primaryAction()
                }
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
    /// Ordered state resolved against the orders list too (the menu HTML doesn't always mark it).
    let isOrdered: Bool

    /// Card state badge (menus §6.1), evaluated in priority order.
    private var badge: (text: String, color: Color)? {
        if isPendingCancel { return ("Wird storniert", .orange) }
        if isOrdered { return ("Bestellt", .green) }
        if !item.available { return ("Ausverkauft", .secondary) }
        if cutoff { return ("Geschlossen", .secondary) }
        return nil
    }

    /// Dim non-orderable / to-be-cancelled cards (menus §6.1).
    private var dim: Double {
        if isPendingCancel { return 0.55 }
        if !isOrdered && (!item.available || cutoff) { return 0.5 }
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
            Image(systemName: "checkmark.circle.fill").foregroundStyle(.tint)
        } else if isOrdered {
            Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
        } else {
            Image(systemName: "circle").foregroundStyle(.secondary)
        }
    }
}
