package dev.radaiko.snackpilot

import android.app.Application
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.launch
import uniffi.snackpilot_core.CoreConfig
import uniffi.snackpilot_core.Credentials
import uniffi.snackpilot_core.GourmetUserInfo
import uniffi.snackpilot_core.MenuSnapshot
import uniffi.snackpilot_core.SnackPilotCore
import uniffi.snackpilot_core.coreVersion
import uniffi.snackpilot_core.isDemoCredentials
import java.io.File

/**
 * Thin Compose shell state over the Rust core (`SnackPilotCore`). The core owns all scraping,
 * caching and domain logic; this holds a storage dir, forwards credentials, and republishes
 * the records the core returns.
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
    /** True while showing offline demo data (no live session). */
    var demoMode by mutableStateOf(false)
        private set

    /** Crate version — a cheap end-to-end proof the FFI is wired. */
    val coreVersion: String = coreVersion()

    init {
        val dir = File(app.filesDir, "snackpilot").apply { mkdirs() }
        core = SnackPilotCore(CoreConfig(storageDir = dir.absolutePath), analytics = null)
    }

    fun isDemoCreds(user: String, pass: String): Boolean =
        isDemoCredentials(username = user, password = pass)

    /**
     * Real Gourmet login + first menu fetch. Demo credentials are short-circuited to offline
     * data and never reach the live server.
     */
    fun login(user: String, pass: String) {
        if (isDemoCreds(user, pass)) {
            loadDemo()
            return
        }
        viewModelScope.launch {
            busy = true
            errorText = null
            try {
                userInfo = core.gourmetLogin(Credentials(username = user, password = pass))
                snapshot = core.fetchMenus(force = false)
                demoMode = false
            } catch (e: Exception) {
                errorText = e.toString()
            }
            busy = false
        }
    }

    /** Render the canned demo menus offline — no network. */
    fun loadDemo() {
        snapshot = core.demoMenuSnapshot()
        userInfo = GourmetUserInfo(username = "Demo", shopModelId = "", eaterId = "", staffGroupId = "")
        demoMode = true
        errorText = null
    }

    fun logout() {
        userInfo = null
        snapshot = null
        demoMode = false
    }
}
