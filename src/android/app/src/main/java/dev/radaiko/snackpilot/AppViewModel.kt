package dev.radaiko.snackpilot

import android.app.Application
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.launch
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

/**
 * Thin Compose shell state over the Rust core (`SnackPilotCore`). The core owns all scraping,
 * caching, demo-mode and domain logic; this holds a storage dir, forwards credentials, and
 * republishes the records the core returns.
 */
class AppViewModel(app: Application) : AndroidViewModel(app) {
    private val core: SnackPilotCore
    private val creds = SecureCredentialStore(app)
    private val notifications = NotificationService(app)
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
        loadCachedData()
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

    /** Set the unified-billing source filter (billing §6.1). Presentation-only. */
    fun setBillingSource(source: BillingSource) {
        billingSourceFilter = source
    }

    private suspend fun loadBilling(offset: UByte) {
        runCatching { core.fetchBilling(offset) }
        runCatching { core.fetchVentopayBilling(offset) }
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
