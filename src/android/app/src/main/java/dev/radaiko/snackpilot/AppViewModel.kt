package dev.radaiko.snackpilot

import android.app.Application
import android.content.ComponentName
import android.content.pm.PackageManager
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.launch
import uniffi.snackpilot_core.CompanyLocation
import uniffi.snackpilot_core.CoreConfig
import uniffi.snackpilot_core.Credentials
import uniffi.snackpilot_core.DailyReminderSettings
import uniffi.snackpilot_core.NotificationCommand
import uniffi.snackpilot_core.GourmetMonthlyBilling
import uniffi.snackpilot_core.GourmetUserInfo
import uniffi.snackpilot_core.LogEntry
import uniffi.snackpilot_core.MenuItem
import uniffi.snackpilot_core.MenuSnapshot
import uniffi.snackpilot_core.MonthOption
import uniffi.snackpilot_core.OrderProgress
import uniffi.snackpilot_core.OrdersSplit
import uniffi.snackpilot_core.ProgressListener
import uniffi.snackpilot_core.SnackPilotCore
import uniffi.snackpilot_core.VentopayMonthlyBilling
import uniffi.snackpilot_core.coreVersion
import uniffi.snackpilot_core.isDemoCredentials
import java.io.File

/** Source filter for the unified Abrechnung list (billing §6.1). */
enum class BillingSource { ALL, GOURMET, VENTOPAY }

/** A one-off dialog (title + message) for the location setup flow (notifications-location §7). */
data class LocationDialog(val title: String, val message: String)

/** Color-scheme preference (themes §1). Independent of [AccentColor]. */
enum class ThemePreference { SYSTEM, LIGHT, DARK }

/**
 * Selectable accent theme (themes §3). Each carries its German label and the exact light/dark
 * `primary` hex from the spec. The effective tint is [lightPrimary] in light mode, [darkPrimary]
 * in dark mode; swatches in the picker always render [lightPrimary] regardless of scheme (§5).
 */
enum class AccentColor(val label: String, val lightPrimary: Long, val darkPrimary: Long) {
    ORANGE("Orange", 0xFFD4501A, 0xFFFF6B35),
    EMERALD("Smaragd", 0xFF2E7D4F, 0xFF4CAF7D),
    BERRY("Beere", 0xFFA62547, 0xFFE04868),
    GOLDEN("Gold", 0xFFC08B1A, 0xFFE8B03E),
    OCEAN("Ozean", 0xFF2563A8, 0xFF4A90D9);

    /** The effective primary ARGB for the resolved scheme (themes §3). */
    fun primary(isDark: Boolean): Long = if (isDark) darkPrimary else lightPrimary
}

/** Accent → launcher activity-alias name (themes §6). Orange is the default alias. */
private val ALIAS_FOR = mapOf(
    AccentColor.ORANGE to "MainActivityOrange",
    AccentColor.EMERALD to "MainActivityEmerald",
    AccentColor.BERRY to "MainActivityBerry",
    AccentColor.GOLDEN to "MainActivityGolden",
    AccentColor.OCEAN to "MainActivityOcean",
)

/**
 * Thin Compose shell state over the Rust core (`SnackPilotCore`). The core owns all scraping,
 * caching, demo-mode and domain logic; this holds a storage dir, forwards credentials, and
 * republishes the records the core returns.
 */
class AppViewModel(app: Application) : AndroidViewModel(app) {
    private val core: SnackPilotCore
    private val creds = SecureCredentialStore(app)
    private val notifications = NotificationService(app)
    private val locationService = LocationService(app)
    private val prefs = app.getSharedPreferences("settings", Application.MODE_PRIVATE)

    var userInfo by mutableStateOf<GourmetUserInfo?>(null)
        private set
    var snapshot by mutableStateOf<MenuSnapshot?>(null)
        private set

    /** Currently displayed menu day (menus §4). "YYYY-MM-DD" key; the core tracks the same
     *  value via selectedDate()/setSelectedDate(). Null only when there are no available days. */
    var selectedDay by mutableStateOf<String?>(null)
        private set
    var errorText by mutableStateOf<String?>(null)
        private set
    var busy by mutableStateOf(false)
        private set
    var demoMode by mutableStateOf(false)
        private set
    var selectedTab by mutableIntStateOf(0)

    // Appearance (themes §1) — two independent, persisted settings. Held as observable state so a
    // change recomposes the whole UI (theme is rebuilt from these in MainActivity's setContent).
    var themePreference by mutableStateOf(ThemePreference.SYSTEM)
        private set
    var accentColor by mutableStateOf(AccentColor.ORANGE)
        private set

    // Auth flags (settings §3.7 / §3.1) — drive the per-tab empty states and Settings hints.
    var gourmetAuthenticated by mutableStateOf(false)
        private set
    var ventopayAuthenticated by mutableStateOf(false)
        private set
    var ventopayBusy by mutableStateOf(false)
        private set
    var ventopayError by mutableStateOf<String?>(null)
        private set

    // Orders (Bestellungen)
    var ordersSplit by mutableStateOf<OrdersSplit?>(null)
        private set
    var ordersError by mutableStateOf<String?>(null)
        private set
    /** Live submit-pipeline phase while a submission is in flight (menus §6.6). */
    var orderProgress by mutableStateOf<OrderProgress?>(null)
        private set

    /** DEBUG headless hooks (set before loadDemo). */
    var debugAutoOrder = false
    var debugAutoLog = false
    var debugAutoReminder = false

    // Notification preferences (shell-owned; fed to the core's decision fns)
    var dailyReminderEnabled by mutableStateOf(false)
        private set
    var reminderHour by mutableStateOf(8)
        private set
    var reminderMinute by mutableStateOf(0)
        private set

    // Location notifications (notifications-location). `companyLocation != null` is the on/off
    // state (§1); the geofence lives in [LocationService]. `locationBusy` drives the "wird
    // ermittelt" button (§8); `locationDialog` surfaces the §7 permission/result messages.
    var companyLocation by mutableStateOf<CompanyLocation?>(null)
        private set
    var locationBusy by mutableStateOf(false)
        private set
    var locationDialog by mutableStateOf<LocationDialog?>(null)
        private set

    // Diagnostics (Einstellungen → Diagnose)
    var logActive by mutableStateOf(false)
        private set
    var logEntries by mutableStateOf<List<LogEntry>>(emptyList())
        private set

    // Billing (Abrechnung)
    var monthOptions by mutableStateOf<List<MonthOption>>(emptyList())
        private set
    var selectedOffset by mutableStateOf<UByte>(0u)
        private set
    var gourmetMonth by mutableStateOf<GourmetMonthlyBilling?>(null)
        private set
    var ventopayMonth by mutableStateOf<VentopayMonthlyBilling?>(null)
        private set

    /** Source filter for the unified billing list (billing §6.1). Presentation-only; never
     *  suppresses fetching/caching. Not persisted — resets to ALL on restart. */
    var billingSourceFilter by mutableStateOf(BillingSource.ALL)
        private set

    val coreVersion: String = coreVersion()

    init {
        val dir = File(app.filesDir, "snackpilot").apply { mkdirs() }
        core = SnackPilotCore(CoreConfig(storageDir = dir.absolutePath), analytics = null)
        dailyReminderEnabled = prefs.getBoolean("daily_reminder_enabled", false)
        reminderHour = prefs.getInt("daily_reminder_hour", 8)
        reminderMinute = prefs.getInt("daily_reminder_minute", 0)
        themePreference = prefs.getString("theme_preference", null)
            ?.let { runCatching { ThemePreference.valueOf(it) }.getOrNull() } ?: ThemePreference.SYSTEM
        accentColor = prefs.getString("accent_color", null)
            ?.let { runCatching { AccentColor.valueOf(it) }.getOrNull() } ?: AccentColor.ORANGE
        loadCachedData()
        // Restore the company geofence on every launch (§9). Idempotent-ish: re-adds without an
        // initial Enter trigger so a restart never re-fires a spurious in-zone notification (§3.2).
        companyLocation = runCatching { core.companyLocation() }.getOrNull()
        startGeofencingIfSaved()
    }

    /** Cache-first display (caching §4): publish any cached menus/orders/billing + auth state
     *  synchronously at startup so returning users see content instantly, before network fetches. */
    private fun loadCachedData() {
        runCatching {
            core.loadCachedMenus()
            core.loadCachedOrders()
            core.loadCachedBillingMonths()
            snapshot = core.menuSnapshot()
            syncSelectedDay()
            ordersSplit = core.splitOrders()
            monthOptions = core.billingMonthOptions()
            userInfo = core.gourmetUserInfo()
            gourmetAuthenticated = core.gourmetIsAuthenticated()
            ventopayAuthenticated = core.ventopayIsAuthenticated()
            val key = monthOptions.firstOrNull { it.offset == selectedOffset }?.key
            if (key != null) {
                gourmetMonth = core.gourmetBillingMonth(key)
                ventopayMonth = core.ventopayBillingMonth(key)
            }
        }
    }

    fun isDemoCreds(user: String, pass: String): Boolean =
        isDemoCredentials(username = user, password = pass)

    /** Log in and load the session. Demo credentials are intercepted by the core (offline data,
     *  never reaches the live server); everything else performs live scraping. */
    fun login(user: String, pass: String) {
        viewModelScope.launch { loginSuspend(user, pass) }
    }

    private suspend fun loginSuspend(user: String, pass: String) {
        val demo = isDemoCreds(user, pass)
        // Persist before the attempt (settings §3.6; v1 saves even if login later fails).
        creds.saveGourmet(user, pass)
        busy = true
        errorText = null
        try {
            userInfo = core.gourmetLogin(Credentials(username = user, password = pass))
            gourmetAuthenticated = core.gourmetIsAuthenticated()
            demoMode = demo
            if (demo) {
                runCatching {
                    core.ventopayLogin(Credentials(username = user, password = pass))
                    ventopayAuthenticated = core.ventopayIsAuthenticated()
                }
            }
            loadSession()
        } catch (e: Exception) {
            errorText = e.toString()
        }
        busy = false
    }

    private suspend fun loadSession() {
        try {
            snapshot = core.fetchMenus(force = false)
        } catch (e: Exception) {
            errorText = e.toString()
        }
        syncSelectedDay()
        loadOrders()
        monthOptions = core.billingMonthOptions()
        loadBilling(selectedOffset)
        applyDailyReminder()
        if (BuildConfig.DEBUG && debugAutoOrder) {
            val s = snapshot
            val lastDay = s?.availableDates?.lastOrNull()
            val item = s?.items?.firstOrNull { it.day == lastDay }
            if (item != null) {
                toggle(item)
                submitOrders()
            }
        }
        if (BuildConfig.DEBUG && debugAutoLog) {
            activateLog()
            runMenuCheck()
        }
        if (BuildConfig.DEBUG && debugAutoReminder) {
            setDailyReminder(enabled = true, hour = 8, minute = 30)
        }
    }

    // Appearance (themes §1) — persist each setting independently; changing one never touches the
    // other. Both are observable state, so the change re-renders the whole UI immediately.

    fun setPreference(pref: ThemePreference) {
        themePreference = pref
        prefs.edit().putString("theme_preference", pref.name).apply()
    }

    fun setAccent(accent: AccentColor) {
        accentColor = accent
        prefs.edit().putString("accent_color", accent.name).apply()
        applyAppIcon(accent)
    }

    /** Switch the home-screen icon to match the accent (themes §6) by enabling the accent's
     *  launcher activity-alias and disabling the others. Only here — no startup reconciliation,
     *  mirroring v1. DONT_KILL_APP keeps the app alive across the swap. */
    private fun applyAppIcon(accent: AccentColor) {
        val ctx = getApplication<Application>()
        val pm = ctx.packageManager
        val classPkg = MainActivity::class.java.`package`?.name ?: "dev.radaiko.snackpilot"
        val target = ALIAS_FOR[accent] ?: return
        for (alias in ALIAS_FOR.values) {
            val state = if (alias == target) {
                PackageManager.COMPONENT_ENABLED_STATE_ENABLED
            } else {
                PackageManager.COMPONENT_ENABLED_STATE_DISABLED
            }
            runCatching {
                pm.setComponentEnabledSetting(
                    ComponentName(ctx.packageName, "$classPkg.$alias"),
                    state, PackageManager.DONT_KILL_APP
                )
            }
        }
    }

    // Notification preferences

    fun setDailyReminder(enabled: Boolean, hour: Int, minute: Int) {
        dailyReminderEnabled = enabled
        reminderHour = hour
        reminderMinute = minute
        prefs.edit()
            .putBoolean("daily_reminder_enabled", enabled)
            .putInt("daily_reminder_hour", hour)
            .putInt("daily_reminder_minute", minute)
            .apply()
        applyDailyReminder()
    }

    /** Ask the core for the daily-reminder command and deliver it; cancel when disabled. */
    fun applyDailyReminder() {
        if (dailyReminderEnabled) {
            val settings = DailyReminderSettings(
                enabled = true, hour = reminderHour.toUByte(), minute = reminderMinute.toUByte()
            )
            core.dailyReminderCommand(settings)?.let { notifications.execute(it) }
        } else {
            notifications.execute(NotificationCommand.CancelPending("daily-order-reminder"))
        }
    }

    // MARK: Location notifications (notifications-location §7–§9)

    /** Setup flow (§7.3): capture a one-shot fix → persist it in the core → start the geofence.
     *  The Compose layer has already secured location + notification permission. GPS failure aborts
     *  with the exact v1 error. The saved location is the on/off switch (§1) — there is no toggle. */
    fun captureAndSaveCompanyLocation() {
        locationBusy = true
        locationService.currentLocation { loc ->
            if (loc == null) {
                locationDialog = LocationDialog("Fehler", "Standort konnte nicht ermittelt werden.")
                locationBusy = false
                return@currentLocation
            }
            // Persist + report success ONLY once the geofence actually registers. addGeofences
            // fails (SecurityException) without ACCESS_BACKGROUND_LOCATION, so a swallowed failure
            // must not masquerade as success — keep companyLocation null and tell the user.
            locationService.startGeofence(loc.latitude, loc.longitude, triggerOnEntry = true) { ok ->
                if (ok) {
                    core.setCompanyLocation(loc.latitude, loc.longitude)
                    companyLocation = core.companyLocation()
                    locationDialog = LocationDialog(
                        "Gespeichert",
                        "Firmenstandort gesetzt. Du wirst um 8:45 benachrichtigt, wenn du im Büro " +
                            "bist und nicht bestellt hast."
                    )
                } else {
                    locationDialog = LocationDialog(
                        "Fehler",
                        "Standort-Benachrichtigungen konnten nicht aktiviert werden. Bitte erlaube " +
                            "den Standortzugriff „Immer erlauben\"."
                    )
                }
                locationBusy = false
            }
        }
    }

    /** Location permission denied (§7.1). Region monitoring needs background ("Immer") location. */
    fun onLocationPermissionDenied() {
        locationDialog = LocationDialog(
            "Berechtigung fehlt",
            "Für Standort-Benachrichtigungen wird der Standortzugriff „Immer erlauben\" benötigt. " +
                "Bitte in den Einstellungen aktivieren."
        )
    }

    /** "Standort entfernen" (§8): stop the geofence and clear the saved location (also resets
     *  `is_at_company` in the core, §1). No confirmation dialog, matching v1. */
    fun clearCompanyLocation() {
        locationService.stopGeofence()
        core.clearCompanyLocation()
        companyLocation = null
    }

    fun dismissLocationDialog() {
        locationDialog = null
    }

    /** Re-register the geofence at launch if a location is saved (§9), without an initial Enter. */
    private fun startGeofencingIfSaved() {
        val loc = runCatching { core.companyLocation() }.getOrNull() ?: return
        locationService.startGeofence(loc.latitude, loc.longitude, triggerOnEntry = false)
    }

    // MARK: Diagnostics

    fun refreshLog() {
        logActive = core.logIsActive()
        logEntries = core.logEntries()
    }

    fun activateLog() {
        core.logActivate(24u)
        refreshLog()
    }

    fun clearLog() {
        core.logClear()
        refreshLog()
    }

    /** Run the background new-menu check on demand (records diagnostic-log entries). In demo the
     *  core short-circuits it, logging `demo_credentials_skip`. */
    suspend fun runMenuCheck() {
        val saved = creds.savedGourmet()
        val c = saved?.let { Credentials(username = it.first, password = it.second) }
        runCatching { core.runMenuCheck(c) }
        refreshLog()
    }

    fun runMenuCheckAsync() {
        viewModelScope.launch { runMenuCheck() }
    }

    // MARK: Orders

    suspend fun loadOrders() {
        runCatching { core.fetchOrders() }
        ordersSplit = core.splitOrders()
        ordersError = core.ordersError()
    }

    fun toggle(item: MenuItem) {
        snapshot = core.togglePending(menuId = item.id, dateKey = item.day)
    }

    /** Reconcile the displayed day with the current snapshot (menus §4): keep a still-valid
     *  selection across refreshes, otherwise seed from the core's selectedDate() or the first day. */
    private fun syncSelectedDay() {
        val dates = snapshot?.availableDates.orEmpty()
        if (dates.isEmpty()) {
            selectedDay = null
            return
        }
        // Prefer the core's tracked day when valid (post-fetch = §3.2 nearest, or the user's pick);
        // otherwise fall back to the NEAREST day (on-or-after today, else last) — NOT the oldest —
        // without writing it back, so a later fetch_menus can still run its own nearest-selection.
        val coreSel = core.selectedDate()
        selectedDay = if (coreSel != null && dates.contains(coreSel)) coreSel else nearestDay(dates)
    }

    /** Nearest available day to today: first date on-or-after today, else the last (menus §3.2/§4.1).
     *  `dates` are ascending "YYYY-MM-DD" keys. */
    private fun nearestDay(dates: List<String>): String {
        val today = todayKey()
        return dates.firstOrNull { it >= today } ?: dates.last()
    }

    private fun todayKey(): String = java.time.LocalDate.now().toString() // ISO yyyy-MM-dd

    /** Navigate to a specific menu day (arrow/Heute affordance); mirrors into the core. */
    fun selectDay(dateKey: String) {
        core.setSelectedDate(dateKey)
        selectedDay = dateKey
    }

    /** Ordering-cutoff flag for a day (menus §6.1): past cutoff → rows non-tappable. */
    fun isOrderingCutoff(dateKey: String): Boolean = core.isOrderingCutoff(dateKey)

    val hasPendingChanges: Boolean
        get() = snapshot?.let { it.pendingOrders.isNotEmpty() || it.pendingCancellations.isNotEmpty() } ?: false

    /** Bridges the core's ProgressListener callback (invoked off the main thread) onto the main
     *  dispatcher so it can update Compose state (menus §6.6). */
    private inner class ProgressBridge : ProgressListener {
        override fun onProgress(phase: OrderProgress?) {
            viewModelScope.launch { orderProgress = phase }
        }
    }

    suspend fun submitOrders() {
        busy = true
        errorText = null
        try {
            snapshot = core.submitOrders(progress = ProgressBridge())
            syncSelectedDay()
            loadOrders()
        } catch (e: Exception) {
            errorText = e.toString()
        }
        orderProgress = null
        busy = false
    }

    fun submitOrdersAsync() {
        viewModelScope.launch { submitOrders() }
    }

    fun clearPending() {
        snapshot = core.clearPendingChanges()
    }

    fun cancelOrder(positionId: String) {
        viewModelScope.launch {
            runCatching { core.cancelOrder(positionId) }
            loadOrders()
        }
    }

    /** Confirm all unconfirmed upcoming orders (orders §5.3), then refresh. */
    suspend fun confirmOrders() {
        busy = true
        try {
            core.confirmOrders()
            loadOrders()
        } catch (e: Exception) {
            ordersError = e.toString()
        }
        busy = false
    }

    fun confirmOrdersAsync() {
        viewModelScope.launch { confirmOrders() }
    }

    fun selectMonth(offset: UByte) {
        selectedOffset = offset
        viewModelScope.launch { loadBilling(offset) }
    }

    /** Pull-to-refresh on Abrechnung: force a re-fetch of the selected month (Kantine + Automaten),
     *  bypassing the past-month cache skip so a refresh always gets fresh data. Suspends so the
     *  pull indicator stays until the fetch completes. */
    suspend fun reloadBilling() {
        loadBilling(selectedOffset, force = true)
    }

    /** Set the unified-billing source filter (billing §6.1). Presentation-only. */
    fun setBillingSource(source: BillingSource) {
        billingSourceFilter = source
    }

    private suspend fun loadBilling(offset: UByte, force: Boolean = false) {
        runCatching { core.fetchBilling(offset, force) }
        runCatching { core.fetchVentopayBilling(offset, force) }
        val key = monthOptions.firstOrNull { it.offset == selectedOffset }?.key ?: return
        gourmetMonth = core.gourmetBillingMonth(key)
        ventopayMonth = core.ventopayBillingMonth(key)
    }

    /** Offline demo session (button / debug hook) — routes through the same login path. */
    fun loadDemo() = login("demo", "demo1234!")

    /** Startup auto-login (settings §3.7): fire-and-forget login from saved credentials for BOTH
     *  services (this install's, or a v1 install's — same store format). No login wall. */
    fun attemptAutoLogin() {
        val g = creds.savedGourmet()
        val v = creds.savedVentopay()
        if (g == null && v == null) return
        // Sequence in ONE coroutine: authenticate Ventopay first WITHOUT its own billing fetch, so
        // the Gourmet session-load fetches both sources once — avoids duplicate concurrent billing
        // scrapes (ban risk).
        viewModelScope.launch {
            if (v != null) ventopayLoginSuspend(v.first, v.second, fetchBilling = false)
            if (g != null) {
                loginSuspend(g.first, g.second)
            } else if (ventopayAuthenticated) {
                loadBilling(selectedOffset)
            }
        }
    }

    /** Kantine (Gourmet) logout: end the session and clear session state; saved credentials are
     *  NOT deleted (settings §3.4). */
    fun gourmetLogout() {
        viewModelScope.launch {
            runCatching { core.gourmetLogout() }
            userInfo = null
            snapshot = null
            selectedDay = null
            demoMode = false
            ordersSplit = null
            ordersError = null
            gourmetMonth = null
            gourmetAuthenticated = false
            // Do NOT clear monthOptions / ventopayMonth / selectedOffset — the Automaten session
            // may still be active and its billing must remain visible.
        }
    }

    // MARK: Ventopay (Automaten) auth

    fun savedGourmetCreds(): Pair<String, String>? = creds.savedGourmet()
    fun savedVentopayCreds(): Pair<String, String>? = creds.savedVentopay()

    /** Persist Ventopay credentials (§3.6: persist-before-validate), then log in. */
    fun ventopayLogin(user: String, pass: String) {
        viewModelScope.launch { ventopayLoginSuspend(user, pass, fetchBilling = true) }
    }

    /** `fetchBilling` is false during startup auto-login (the Gourmet session-load fetches both
     *  sources once); interactive Automaten login leaves it true so transactions appear immediately. */
    private suspend fun ventopayLoginSuspend(user: String, pass: String, fetchBilling: Boolean) {
        creds.saveVentopay(user, pass)
        ventopayBusy = true
        ventopayError = null
        try {
            core.ventopayLogin(Credentials(username = user, password = pass))
            ventopayAuthenticated = core.ventopayIsAuthenticated()
            if (fetchBilling) {
                monthOptions = core.billingMonthOptions()
                loadBilling(selectedOffset)
            }
        } catch (e: Exception) {
            ventopayError = e.toString()
        }
        ventopayBusy = false
    }

    /** Ventopay logout: end the session; saved credentials are NOT deleted (settings §3.4). */
    fun ventopayLogout() {
        viewModelScope.launch {
            runCatching { core.ventopayLogout() }
            ventopayAuthenticated = false
            ventopayError = null
        }
    }
}
