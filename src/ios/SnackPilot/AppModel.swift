import Foundation
import SwiftUI

/// Thin SwiftUI shell state over the Rust core (`SnackPilotCore`). The core owns all
/// scraping, caching, demo-mode and domain logic; this object injects a storage directory,
/// forwards credentials, and republishes the records the core returns.
@MainActor
final class AppModel: ObservableObject {
    let core: SnackPilotCore

    @Published var userInfo: GourmetUserInfo?
    @Published var snapshot: MenuSnapshot?
    @Published var errorText: String?
    @Published var busy = false
    /// True while showing offline demo data (magic credentials).
    @Published var demoMode = false
    @Published var selectedTab = 0

    /// Live auth flags mirrored from the core (settings §3.7 no-wall navigation). Per-tab empty
    /// states key off these; the root never gates on them.
    @Published var gourmetAuthenticated = false
    @Published var ventopayAuthenticated = false
    @Published var ventopayError: String?
    @Published var ventopayBusy = false

    // Orders (Bestellungen)
    @Published var ordersSplit: OrdersSplit?
    @Published var ordersError: String?

    // Notification preferences (shell-owned; fed to the core's decision fns)
    @Published var dailyReminderEnabled = UserDefaults.standard.bool(forKey: "daily_reminder_enabled")
    @Published var reminderHour = UserDefaults.standard.object(forKey: "daily_reminder_hour") as? Int ?? 8
    @Published var reminderMinute = UserDefaults.standard.object(forKey: "daily_reminder_minute") as? Int ?? 0

    // Diagnostics (Einstellungen → Diagnose)
    @Published var logActive = false
    @Published var logEntries: [LogEntry] = []

    // Billing (Abrechnung)
    @Published var monthOptions: [MonthOption] = []
    @Published var selectedOffset: UInt8 = 0
    @Published var gourmetMonth: GourmetMonthlyBilling?
    @Published var ventopayMonth: VentopayMonthlyBilling?

    /// Crate version — a cheap end-to-end proof the FFI is wired.
    let coreVersion: String = SnackPilot.coreVersion()

    init() {
        let base = FileManager.default
            .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("snackpilot", isDirectory: true)
        try? FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        do {
            core = try SnackPilotCore(config: CoreConfig(storageDir: base.path), analytics: nil)
        } catch {
            fatalError("SnackPilotCore init failed: \(error)")
        }

        // Cache-first display (caching §4): publish any cached menus/orders/billing before the
        // network fetches so returning users see content instantly.
        loadCached()

        NotificationService.shared.configure()
        Task {
            await NotificationService.shared.requestPermission()
            #if DEBUG
            if ProcessInfo.processInfo.arguments.contains("-uiTestNotify") {
                NotificationService.shared.execute(.fireNow(
                    id: "test-menu", title: "Neues Menü verfügbar",
                    body: "Das Menü für nächste Woche ist online.",
                    channelId: nil, screen: nil))
            }
            #endif
        }

        #if DEBUG
        // UI-test / preview hooks. Never compiled into release builds; only render demo data.
        let args = ProcessInfo.processInfo.arguments
        if args.contains("-uiTestDemo") {
            if let i = args.firstIndex(of: "-uiTestTab"), i + 1 < args.count {
                selectedTab = tabIndex(args[i + 1])
            }
            Task { await login(user: "demo", pass: "demo1234!") }
            return
        }
        #endif
        attemptAutoLogin()
    }

    /// Startup auto-login (settings §3.7): if credentials are saved (this install's, or a v1
    /// install's — same Keychain coordinates), log straight in for both services (Gourmet first,
    /// then Ventopay), fire-and-forget. No login wall for returning users.
    private func attemptAutoLogin() {
        let g = KeychainStore.savedGourmet()
        let v = KeychainStore.savedVentopay()
        guard g != nil || v != nil else { return }
        // Sequence in ONE task: authenticate Ventopay first WITHOUT its own billing fetch, so the
        // Gourmet session-load fetches both sources in a single pass — avoids duplicate concurrent
        // billing scrapes (ban risk from bot-like duplicate traffic).
        Task {
            if let v { await ventopayLogin(user: v.username, pass: v.password, fetchBilling: false) }
            if let g {
                await login(user: g.username, pass: g.password)
            } else if ventopayAuthenticated {
                // Ventopay-only user: no Gourmet session-load, so fetch billing here.
                await loadBilling(offset: selectedOffset)
            }
        }
    }

    /// Read the core's cached menus/orders/billing into the published state (caching §4).
    private func loadCached() {
        core.loadCachedMenus()
        core.loadCachedOrders()
        core.loadCachedBillingMonths()
        snapshot = core.menuSnapshot()
        ordersSplit = core.splitOrders()
        monthOptions = core.billingMonthOptions()
        refreshBilling()
        gourmetAuthenticated = core.gourmetIsAuthenticated()
        ventopayAuthenticated = core.ventopayIsAuthenticated()
    }

    func isDemoCredentials(user: String, pass: String) -> Bool {
        SnackPilot.isDemoCredentials(username: user, password: pass)
    }

    /// Log in and load the session. Demo credentials are intercepted by the core (it swaps to
    /// offline data and never touches the live server); everything else performs live scraping.
    func login(user: String, pass: String) async {
        busy = true
        errorText = nil
        let demo = isDemoCredentials(user: user, pass: pass)
        // Persist before the attempt (settings §3.6; v1 saves even if login later fails).
        KeychainStore.saveGourmet(username: user, password: pass)
        do {
            userInfo = try await core.gourmetLogin(creds: Credentials(username: user, password: pass))
            gourmetAuthenticated = core.gourmetIsAuthenticated()
            demoMode = demo
            // In demo mode also "log in" Ventopay so its transactions show; live users log in
            // to the Automaten side separately (Settings → Automaten-Zugangsdaten).
            if demo {
                try? await core.ventopayLogin(creds: Credentials(username: user, password: pass))
                ventopayAuthenticated = core.ventopayIsAuthenticated()
            }
            await loadSession()
        } catch {
            errorText = String(describing: error)
        }
        busy = false
    }

    // MARK: Ventopay (Automaten) session

    /// Persist Automaten credentials first (settings §3.6), then log in against the core.
    /// Publishes `ventopayAuthenticated`; refreshes billing so Automaten transactions appear.
    /// `fetchBilling` is false during startup auto-login (the Gourmet session-load fetches both
    /// sources once); interactive Automaten login leaves it true so transactions appear immediately.
    func ventopayLogin(user: String, pass: String, fetchBilling: Bool = true) async {
        ventopayBusy = true
        ventopayError = nil
        KeychainStore.saveVentopay(username: user, password: pass)
        do {
            try await core.ventopayLogin(creds: Credentials(username: user, password: pass))
            ventopayAuthenticated = core.ventopayIsAuthenticated()
            if fetchBilling {
                monthOptions = core.billingMonthOptions()
                await loadBilling(offset: selectedOffset)
            }
        } catch {
            ventopayError = String(describing: error)
        }
        ventopayBusy = false
    }

    /// End the Automaten session (core.ventopayLogout). Saved credentials are kept (settings §3.4).
    func ventopayLogout() async {
        try? await core.ventopayLogout()
        ventopayAuthenticated = core.ventopayIsAuthenticated()
        ventopayMonth = nil
    }

    /// End the Kantine session (core.gourmetLogout). Saved credentials are kept (settings §3.4);
    /// clears the session-scoped state so the tabs fall back to their empty states.
    func gourmetLogout() async {
        try? await core.gourmetLogout()
        userInfo = nil
        gourmetAuthenticated = core.gourmetIsAuthenticated()
        demoMode = false
        snapshot = nil
        ordersSplit = nil
        ordersError = nil
        gourmetMonth = nil
    }

    /// Fetch menus + billing for the active session.
    private func loadSession() async {
        do { snapshot = try await core.fetchMenus(force: false) } catch {
            errorText = String(describing: error)
        }
        await loadOrders()
        monthOptions = core.billingMonthOptions()
        await loadBilling(offset: selectedOffset)
        applyDailyReminder()
        #if DEBUG
        // Headless order-flow check: order the furthest-out menu and submit, so screenshots
        // can show a placed order in Bestellungen.
        if ProcessInfo.processInfo.arguments.contains("-uiTestOrder"),
           let s = snapshot, let lastDay = s.availableDates.last,
           let item = s.items.first(where: { $0.day == lastDay }) {
            toggle(item: item)
            await submitOrders()
        }
        if ProcessInfo.processInfo.arguments.contains("-uiTestLog") {
            activateLog()
            await runMenuCheck()
        }
        if ProcessInfo.processInfo.arguments.contains("-uiTestReminder") {
            setDailyReminder(enabled: true, hour: 8, minute: 30)
        }
        #endif
    }

    // MARK: Orders

    func loadOrders() async {
        try? await core.fetchOrders()
        ordersSplit = core.splitOrders()
        ordersError = core.ordersError()
    }

    /// Toggle a pending order/cancellation for a menu item (composite key handled in the core).
    func toggle(item: MenuItem) {
        snapshot = core.togglePending(menuId: item.id, dateKey: item.day)
    }

    var hasPendingChanges: Bool {
        guard let s = snapshot else { return false }
        return !s.pendingOrders.isEmpty || !s.pendingCancellations.isEmpty
    }

    func submitOrders() async {
        busy = true
        errorText = nil
        do {
            snapshot = try await core.submitOrders(progress: nil)
            await loadOrders()
        } catch {
            errorText = String(describing: error)
        }
        busy = false
    }

    func clearPending() {
        snapshot = core.clearPendingChanges()
    }

    func confirmOrders() async {
        busy = true // guards the banner button against double-taps → duplicate confirm POSTs
        do {
            try await core.confirmOrders()
        } catch {
            ordersError = String(describing: error)
        }
        await loadOrders()
        busy = false
    }

    func cancelOrder(_ positionId: String) async {
        try? await core.cancelOrder(positionId: positionId)
        await loadOrders()
    }

    func selectMonth(offset: UInt8) async {
        selectedOffset = offset
        await loadBilling(offset: offset)
    }

    private func loadBilling(offset: UInt8) async {
        try? await core.fetchBilling(offset: offset)
        try? await core.fetchVentopayBilling(offset: offset)
        refreshBilling()
    }

    private func refreshBilling() {
        guard let key = monthOptions.first(where: { $0.offset == selectedOffset })?.key else { return }
        gourmetMonth = core.gourmetBillingMonth(monthKey: key)
        ventopayMonth = core.ventopayBillingMonth(monthKey: key)
    }

    /// Offline demo session (button / debug hook) — routes through the same login path.
    func loadDemo() {
        Task { await login(user: "demo", pass: "demo1234!") }
    }

    // MARK: Notification preferences

    /// Persist the daily-reminder preference and (re)apply it through the core decision fn.
    func setDailyReminder(enabled: Bool, hour: Int, minute: Int) {
        dailyReminderEnabled = enabled
        reminderHour = hour
        reminderMinute = minute
        let d = UserDefaults.standard
        d.set(enabled, forKey: "daily_reminder_enabled")
        d.set(hour, forKey: "daily_reminder_hour")
        d.set(minute, forKey: "daily_reminder_minute")
        applyDailyReminder()
    }

    /// Ask the core for the daily-reminder command given the current orders/time and deliver it.
    /// When disabled, cancel any previously scheduled reminder (the core returns nil when off).
    func applyDailyReminder() {
        if dailyReminderEnabled {
            let settings = DailyReminderSettings(
                enabled: true, hour: UInt8(reminderHour), minute: UInt8(reminderMinute))
            if let command = core.dailyReminderCommand(settings: settings) {
                NotificationService.shared.execute(command)
            }
        } else {
            NotificationService.shared.execute(.cancelPending(id: "daily-order-reminder"))
        }
    }

    // MARK: Diagnostics

    func refreshLog() {
        logActive = core.logIsActive()
        logEntries = core.logEntries()
    }

    func activateLog() {
        core.logActivate(hours: 24)
        refreshLog()
    }

    func clearLog() {
        core.logClear()
        refreshLog()
    }

    /// Run the background new-menu check on demand (records diagnostic-log entries). Uses the
    /// current credentials; in demo mode the core short-circuits it (logs `demo_credentials_skip`).
    func runMenuCheck() async {
        let creds = KeychainStore.savedGourmet().map {
            Credentials(username: $0.username, password: $0.password)
        }
        _ = try? await core.runMenuCheck(creds: creds)
        refreshLog()
    }

    private func tabIndex(_ name: String) -> Int {
        switch name {
        case "orders": return 1
        case "billing": return 2
        case "settings": return 3
        default: return 0
        }
    }
}
