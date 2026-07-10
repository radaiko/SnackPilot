import Foundation
import SwiftUI
import UIKit

/// Thin SwiftUI shell state over the Rust core (`SnackPilotCore`). The core owns all
/// scraping, caching, demo-mode and domain logic; this object injects a storage directory,
/// forwards credentials, and republishes the records the core returns.
@MainActor
final class AppModel: ObservableObject {
    let core: SnackPilotCore

    @Published var userInfo: GourmetUserInfo?
    @Published var snapshot: MenuSnapshot?
    /// The day currently shown in the Menüs tab (menus §4). Mirrors `core.selectedDate()`;
    /// falls back to the first available date when the core has none. May be a day that is
    /// not in `availableDates` (menus §4: navigator then shows index 0).
    @Published var selectedDay: String?
    @Published var errorText: String?
    @Published var busy = false
    /// Live submit-pipeline phase while an order submission is in flight (menus §6.6).
    @Published var orderProgress: OrderProgress?
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

    // Appearance (themes §1). Two independent, persisted settings; changing one never changes the
    // other. Both drive an immediate, restart-free re-render (applied at RootView, §2/§4).
    @Published var themePreference: ThemePreference =
        ThemePreference(rawValue: UserDefaults.standard.string(forKey: "theme_preference") ?? "") ?? .system
    @Published var accent: AccentColor =
        AccentColor(rawValue: UserDefaults.standard.string(forKey: "accent_color") ?? "") ?? .orange

    // Notification preferences (shell-owned; fed to the core's decision fns)
    @Published var dailyReminderEnabled = UserDefaults.standard.bool(forKey: "daily_reminder_enabled")
    @Published var reminderHour = UserDefaults.standard.object(forKey: "daily_reminder_hour") as? Int ?? 8
    @Published var reminderMinute = UserDefaults.standard.object(forKey: "daily_reminder_minute") as? Int ?? 0

    // Location notifications (notifications-location). `companyLocation != nil` is the on/off state
    // (§1); the geofence itself lives in `LocationService`. `locationBusy` drives the "wird
    // ermittelt" button state (§8); `locationAlert` surfaces the §7 permission/result alerts.
    @Published var companyLocation: CompanyLocation?
    @Published var locationBusy = false
    @Published var locationAlert: LocationAlert?

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

        // Restore the company geofence on every launch (§3/§9): install the region handler and,
        // when a location is saved, re-register monitoring (idempotent — the skip-if-active guard
        // makes it safe to call each launch, including background relaunches for region crossings).
        companyLocation = core.companyLocation()
        LocationService.shared.onRegionEvent = { [weak self] event in
            Task { @MainActor in self?.handleGeofence(event) }
        }
        startGeofencingIfSaved()

        #if DEBUG
        // UI-test / preview hooks. Never compiled into release builds; only render demo data.
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "-uiTestAccent"), i + 1 < args.count,
           let accent = AccentColor(rawValue: args[i + 1]) {
            setAccent(accent, switchIcon: false)
        }
        if args.contains("-uiTestSavedLocation") {
            // Persist a location without the GPS/permission flow so the saved-state UI (§8) is
            // screenshot-verifiable in the Simulator (real geofencing needs a device).
            core.setCompanyLocation(latitude: 48.21, longitude: 16.37)
            companyLocation = core.companyLocation()
        }
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
        syncSelectedDay()
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
        do { snapshot = try await core.fetchMenus(force: false); syncSelectedDay() } catch {
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

    // MARK: Day navigation (menus §4)

    /// Resolve the day to display: the core's tracked selection, or — when it has none — the
    /// first available date (also written back so the core tracks it). Called after every fetch
    /// that can change `availableDates`.
    private func syncSelectedDay() {
        guard let dates = snapshot?.availableDates, !dates.isEmpty else {
            selectedDay = nil
            return
        }
        // Prefer the core's tracked day when it's valid (after a fetch this is the §3.2 nearest
        // selection, or the user's pick). Otherwise fall back to the NEAREST day (on-or-after
        // today, else last) — NOT the oldest — and do NOT write it back, so a later fetch_menus
        // can still run its own nearest-selection.
        if let sel = core.selectedDate(), dates.contains(sel) {
            selectedDay = sel
        } else {
            selectedDay = Self.nearestDay(in: dates)
        }
    }

    /// Nearest available day to today: first date on-or-after today, else the last date. Mirrors
    /// the core's `find_nearest_date` (menus §3.2 / §4.1). `dates` are sorted ascending "YYYY-MM-DD".
    static func nearestDay(in dates: [String]) -> String {
        let today = todayKey()
        return dates.first(where: { $0 >= today }) ?? dates[dates.count - 1]
    }

    /// Select a specific day (writes through to the core so it survives refreshes).
    func selectDay(_ dateKey: String) {
        core.setSelectedDate(dateKey: dateKey)
        selectedDay = dateKey
    }

    /// Step to the previous available day. No-op at the first day (or when the current day is
    /// not in the list — menus §4: back arrow is disabled there).
    func prevDay() {
        guard let dates = snapshot?.availableDates, let cur = selectedDay,
              let idx = dates.firstIndex(of: cur), idx > 0 else { return }
        selectDay(dates[idx - 1])
    }

    /// Step to the next available day. When the current day is absent from the list
    /// (menus §4), the forward arrow selects the first date.
    func nextDay() {
        guard let dates = snapshot?.availableDates, !dates.isEmpty else { return }
        guard let cur = selectedDay, let idx = dates.firstIndex(of: cur) else {
            selectDay(dates[0]); return
        }
        guard idx < dates.count - 1 else { return }
        selectDay(dates[idx + 1])
    }

    /// Jump to the nearest menu day — today, or the nearest upcoming day when today has no menu
    /// (menus §4.1 center-tap → findNearestDate).
    func goToToday() {
        guard let dates = snapshot?.availableDates, !dates.isEmpty else { return }
        selectDay(Self.nearestDay(in: dates))
    }

    /// Whether ordering is closed for a day (menus §6.2 — 09:00 Europe/Vienna cutoff, computed
    /// in the core).
    func isCutoff(_ dateKey: String) -> Bool {
        core.isOrderingCutoff(dateKey: dateKey)
    }

    /// Today's local date as the core's `YYYY-MM-DD` day key.
    static func todayKey() -> String {
        let f = DateFormatter()
        f.locale = Locale(identifier: "en_US_POSIX")
        f.dateFormat = "yyyy-MM-dd"
        return f.string(from: Date())
    }

    var hasPendingChanges: Bool {
        guard let s = snapshot else { return false }
        return !s.pendingOrders.isEmpty || !s.pendingCancellations.isEmpty
    }

    func submitOrders() async {
        busy = true
        errorText = nil
        // Pass a progress listener so the submit bar can show the live pipeline phase (§6.6).
        let bridge = ProgressBridge { [weak self] phase in self?.orderProgress = phase }
        do {
            snapshot = try await core.submitOrders(progress: bridge)
            syncSelectedDay()
            await loadOrders()
        } catch {
            errorText = String(describing: error)
        }
        orderProgress = nil
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

    /// Pull-to-refresh on Abrechnung: re-fetch the selected month's billing (Kantine + Automaten).
    /// The current month always re-hits the server; new/late-posted bills appear on refresh.
    func reloadBilling() async {
        await loadBilling(offset: selectedOffset)
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

    // MARK: Appearance (themes §1–§4)

    /// The scheme to force at the root via `.preferredColorScheme` (themes §4): `nil` follows the
    /// OS (SYSTEM), otherwise the explicit preference.
    var preferredColorScheme: ColorScheme? {
        switch themePreference {
        case .system: return nil
        case .light: return .light
        case .dark: return .dark
        }
    }

    /// Whether the *resolved* scheme is dark (themes §1.1). For SYSTEM this reads the current OS
    /// trait; if the OS reports unspecified it falls back to light.
    private var isDark: Bool {
        switch themePreference {
        case .light: return false
        case .dark: return true
        case .system: return UITraitCollection.current.userInterfaceStyle == .dark
        }
    }

    /// The effective accent tint: the selected accent's primary for the resolved scheme (themes §3).
    var accentColor: Color { accent.primary(isDark: isDark) }

    /// Effective accent when the caller already knows the OS scheme (RootView reads it from the
    /// environment so the tint re-resolves the instant the system flips light/dark; themes §1.1).
    func accentColor(systemDark: Bool) -> Color {
        let dark: Bool
        switch themePreference {
        case .light: dark = false
        case .dark: dark = true
        case .system: dark = systemDark
        }
        return accent.primary(isDark: dark)
    }

    /// Change the light/dark/system preference and persist it (themes §1.2). Independent of the
    /// accent. Republishing re-renders the whole UI immediately (no restart).
    func setThemePreference(_ preference: ThemePreference) {
        themePreference = preference
        UserDefaults.standard.set(preference.rawValue, forKey: "theme_preference")
    }

    /// Change the accent color and persist it (themes §1.2). Independent of the preference.
    /// Also switches the home-screen app icon to match (themes §6) — this is the only call site,
    /// mirroring v1 (no startup reconciliation). `switchIcon: false` skips it for the DEBUG
    /// screenshot hook so tests don't trip iOS's icon-change system alert.
    func setAccent(_ newAccent: AccentColor, switchIcon: Bool = true) {
        accent = newAccent
        UserDefaults.standard.set(newAccent.rawValue, forKey: "accent_color")
        if switchIcon { AppIconService.setIcon(for: newAccent) }
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

    // MARK: Location notifications (notifications-location §7–§9)

    /// Setup flow (§7): request "Always" location, then notifications, then capture a one-shot GPS
    /// fix → persist it in the core → start the geofence. Each failure aborts with the exact v1
    /// German alert. The saved location itself is the on/off switch (§1) — there is no toggle.
    func setCompanyLocationFromCurrentPosition() async {
        locationBusy = true
        defer { locationBusy = false }

        // 1. Location — region monitoring needs "Always". WhenInUse-only (or denied) → tell the
        //    user to set "Immer" in Settings, and abort (§7.1).
        let status = await LocationService.shared.requestAlwaysAuthorization()
        guard status == .authorizedAlways else {
            locationAlert = LocationAlert(
                title: "Standort „Immer\" erforderlich",
                message: "Für Standort-Benachrichtigungen muss der Standortzugriff auf „Immer\" "
                    + "gesetzt werden.\n\nBitte öffne die Einstellungen und wähle unter Standort "
                    + "„Immer\" aus.")
            return
        }

        // 2. Notifications (§7.2).
        guard await NotificationService.shared.requestPermissionGranted() else {
            locationAlert = LocationAlert(
                title: "Berechtigung fehlt",
                message: "Benachrichtigungen werden für diese Funktion benötigt. "
                    + "Bitte in den Einstellungen aktivieren.")
            return
        }

        // 3. One-shot fix → persist → geofence (§7.3).
        do {
            let loc = try await LocationService.shared.requestOneShotLocation()
            core.setCompanyLocation(
                latitude: loc.coordinate.latitude, longitude: loc.coordinate.longitude)
            companyLocation = core.companyLocation()
            LocationService.shared.startMonitoring(
                latitude: loc.coordinate.latitude, longitude: loc.coordinate.longitude,
                requestInitialState: true)
            locationAlert = LocationAlert(
                title: "Gespeichert",
                message: "Firmenstandort gesetzt. Du wirst um 8:45 benachrichtigt, wenn du im Büro "
                    + "bist und nicht bestellt hast.")
        } catch {
            locationAlert = LocationAlert(
                title: "Fehler", message: "Standort konnte nicht ermittelt werden.")
        }
    }

    /// "Standort entfernen" (§8): stop the geofence and clear the saved location (which also resets
    /// `is_at_company` in the core, §1). No confirmation dialog, matching v1.
    func clearCompanyLocation() {
        LocationService.shared.stopMonitoring()
        core.clearCompanyLocation()
        companyLocation = nil
    }

    /// Region Enter/Exit handler (§4): record the flag, then deliver whatever the core decides
    /// (Enter cancels any pending cancel-reminder and — if nothing is ordered today — fires the
    /// 08:45 "im Büro, nicht bestellt" reminder; Exit re-evaluates the cancel-reminder).
    func handleGeofence(_ event: GeofenceEvent) {
        core.setIsAtCompany(value: event == .enter)
        for command in core.geofenceCommands(event: event) {
            NotificationService.shared.execute(command)
        }
        refreshLog()
    }

    /// Re-register the geofence at launch if a location is saved (§9). Idempotent via the
    /// skip-if-active guard, so re-running it never re-fires the initial Enter.
    private func startGeofencingIfSaved() {
        guard let loc = core.companyLocation() else { return }
        LocationService.shared.startMonitoring(latitude: loc.latitude, longitude: loc.longitude)
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

/// A one-off alert (title + message) for the location setup flow (notifications-location §7).
struct LocationAlert: Identifiable {
    let id = UUID()
    let title: String
    let message: String
}

/// Bridges the core's `ProgressListener` callback (invoked off the main thread by the tokio
/// runtime) onto the main actor so it can update `@Published` state (menus §6.6).
final class ProgressBridge: ProgressListener {
    private let onPhase: (OrderProgress?) -> Void
    init(_ onPhase: @escaping (OrderProgress?) -> Void) { self.onPhase = onPhase }
    func onProgress(phase: OrderProgress?) {
        Task { @MainActor in onPhase(phase) }
    }
}
