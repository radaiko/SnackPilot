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
import uniffi.snackpilot_core.OrdersSplit
import uniffi.snackpilot_core.SnackPilotCore
import uniffi.snackpilot_core.VentopayMonthlyBilling
import uniffi.snackpilot_core.coreVersion
import uniffi.snackpilot_core.isDemoCredentials
import java.io.File

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
    var errorText by mutableStateOf<String?>(null)
        private set
    var busy by mutableStateOf(false)
        private set
    var demoMode by mutableStateOf(false)
        private set
    var selectedTab by mutableIntStateOf(0)

    // Orders (Bestellungen)
    var ordersSplit by mutableStateOf<OrdersSplit?>(null)
        private set
    var ordersError by mutableStateOf<String?>(null)
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

    val coreVersion: String = coreVersion()

    init {
        val dir = File(app.filesDir, "snackpilot").apply { mkdirs() }
        core = SnackPilotCore(CoreConfig(storageDir = dir.absolutePath), analytics = null)
        dailyReminderEnabled = prefs.getBoolean("daily_reminder_enabled", false)
        reminderHour = prefs.getInt("daily_reminder_hour", 8)
        reminderMinute = prefs.getInt("daily_reminder_minute", 0)
    }

    fun isDemoCreds(user: String, pass: String): Boolean =
        isDemoCredentials(username = user, password = pass)

    /** Log in and load the session. Demo credentials are intercepted by the core (offline data,
     *  never reaches the live server); everything else performs live scraping. */
    fun login(user: String, pass: String) {
        val demo = isDemoCreds(user, pass)
        // Persist before the attempt (settings §3.6; v1 saves even if login later fails).
        creds.saveGourmet(user, pass)
        viewModelScope.launch {
            busy = true
            errorText = null
            try {
                userInfo = core.gourmetLogin(Credentials(username = user, password = pass))
                demoMode = demo
                if (demo) {
                    runCatching { core.ventopayLogin(Credentials(username = user, password = pass)) }
                }
                loadSession()
            } catch (e: Exception) {
                errorText = e.toString()
            }
            busy = false
        }
    }

    private suspend fun loadSession() {
        try {
            snapshot = core.fetchMenus(force = false)
        } catch (e: Exception) {
            errorText = e.toString()
        }
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

    val hasPendingChanges: Boolean
        get() = snapshot?.let { it.pendingOrders.isNotEmpty() || it.pendingCancellations.isNotEmpty() } ?: false

    suspend fun submitOrders() {
        busy = true
        errorText = null
        try {
            snapshot = core.submitOrders(progress = null)
            loadOrders()
        } catch (e: Exception) {
            errorText = e.toString()
        }
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

    fun selectMonth(offset: UByte) {
        selectedOffset = offset
        viewModelScope.launch { loadBilling(offset) }
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

    /** Startup auto-login (settings §3.7): log in from saved credentials (this install's, or a
     *  v1 install's — same store format). No login wall for returning users. */
    fun attemptAutoLogin() {
        val saved = creds.savedGourmet() ?: return
        login(saved.first, saved.second)
    }

    fun logout() {
        userInfo = null
        snapshot = null
        demoMode = false
        ordersSplit = null
        ordersError = null
        monthOptions = emptyList()
        gourmetMonth = null
        ventopayMonth = null
        selectedOffset = 0u
        selectedTab = 0
    }
}
