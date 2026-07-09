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
import uniffi.snackpilot_core.GourmetMonthlyBilling
import uniffi.snackpilot_core.GourmetUserInfo
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

    /** DEBUG headless hook: place a demo order during loadSession (set before loadDemo). */
    var debugAutoOrder = false

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
    }

    fun isDemoCreds(user: String, pass: String): Boolean =
        isDemoCredentials(username = user, password = pass)

    /** Log in and load the session. Demo credentials are intercepted by the core (offline data,
     *  never reaches the live server); everything else performs live scraping. */
    fun login(user: String, pass: String) {
        val demo = isDemoCreds(user, pass)
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
        if (BuildConfig.DEBUG && debugAutoOrder) {
            val s = snapshot
            val lastDay = s?.availableDates?.lastOrNull()
            val item = s?.items?.firstOrNull { it.day == lastDay }
            if (item != null) {
                toggle(item)
                submitOrders()
            }
        }
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
