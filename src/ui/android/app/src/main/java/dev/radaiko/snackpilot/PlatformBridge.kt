package dev.radaiko.snackpilot

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.Settings
import android.util.Log
import com.aptabase.Aptabase
import dev.radaiko.gourmetclient.SnackPilotActivity
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger

/**
 * Wire form of the core's NotificationCommand at the JNI boundary (task 3.4). Plain data
 * class: the Rust side marshals the uniffi enum into these fields, so no
 * `uniffi.snackpilot_core` type crosses into the shell (design D4).
 * kind: 0=FireNow, 1=ScheduleAt, 2=CancelPending (mirrors wire_command in android.rs).
 */
data class CommandData(
    val kind: Int,
    val id: String,
    val title: String,
    val body: String,
    val channelId: String,
    val fireAtEpochMs: Long,
)

/** Recipient details for a support email (model SupportEmail). */
data class SupportEmailData(
    val to: String,
    val subject: String,
    val body: String,
    val attachmentFilename: String,
    val attachmentMime: String,
)

/** Application context, set by [SnackPilotApp.onCreate] before Rust loads. */
object AppHolder {
    @Volatile lateinit var context: Context
}

/**
 * Static entry points called from Rust over JNI (android.rs `JniBridge`). Every method
 * returns null on success or an error message on failure; Rust turns a non-null return
 * into a typed PlatformError (thrown Java exceptions are converted the same way).
 * Async results (permission grants, one-shot location) return through the
 * `external fun` resolvers below, matched to the request id the Rust side generated.
 */
object PlatformBridge {
    private const val TAG = "SnackPilotBridge"

    @Volatile
    private var activity: SnackPilotActivity? = null

    /** request id -> (Activity requestCode, insert time), for permission-result
     *  routing. Entries whose dialog was never answered are dropped as stale on
     *  the next request (the Rust side times out on its own), so the map cannot
     *  grow without bound. */
    private val permissionRequests = ConcurrentHashMap<String, Pair<Int, Long>>()
    private val requestCode = AtomicInteger(1000)
    private const val STALE_REQUEST_MS = 5 * 60 * 1000L

    fun setActivity(a: SnackPilotActivity?) {
        activity = a
    }

    fun getActivity(): SnackPilotActivity? = activity

    // Rust caches its JNI class lookups; must run on the Java main thread at
    // startup before any bridge call (SnackPilotActivity.onCreate).
    @JvmStatic external fun nativeCacheClass()

    // ---- async results back into Rust (android.rs nativeResolve* symbols) ----

    external fun nativeResolvePermission(requestId: String, granted: Boolean)
    external fun nativeResolveLocation(requestId: String, latitude: Double, longitude: Double, found: Boolean)

    // ---- credentials (REQ-PLT-01) ----

    @JvmStatic
    fun saveSecret(key: String, value: String): String? = guard("saveSecret") {
        SecureCredentialStore(AppHolder.context).set(key, value)
    }

    /** Returns the stored value directly (null = nothing stored); failures propagate as
     *  Java exceptions which Rust converts via its pending-exception check. */
    @JvmStatic
    fun getSecret(key: String): String? = SecureCredentialStore(AppHolder.context).get(key)

    @JvmStatic
    fun deleteSecret(key: String): String? = guard("deleteSecret") {
        SecureCredentialStore(AppHolder.context).delete(key)
    }

    // ---- permissions (REQ-PLT-03) ----

    @JvmStatic
    fun requestPermission(permission: String, requestId: String): String? = guard("requestPermission") {
        val a = activity ?: error("no activity in foreground")
        val manifest = when (permission) {
            "notifications" -> android.Manifest.permission.POST_NOTIFICATIONS
            "location" -> android.Manifest.permission.ACCESS_FINE_LOCATION
            else -> error("unknown permission: $permission")
        }
        if (a.checkSelfPermission(manifest) == PackageManager.PERMISSION_GRANTED) {
            resolvePermissionSafely(requestId, true)
        } else {
            val now = android.os.SystemClock.elapsedRealtime()
            permissionRequests.entries.removeIf { now - it.value.second > STALE_REQUEST_MS }
            val code = requestCode.incrementAndGet()
            permissionRequests[requestId] = code to now
            a.requestPermissions(arrayOf(manifest), code)
        }
    }

    /** Called from SnackPilotActivity.onRequestPermissionsResult. */
    fun onPermissionsResult(code: Int, grantResults: IntArray) {
        val requestId = permissionRequests.entries.firstOrNull { it.value.first == code }?.key ?: return
        permissionRequests.remove(requestId)
        val granted = grantResults.isNotEmpty() && grantResults[0] == PackageManager.PERMISSION_GRANTED
        resolvePermissionSafely(requestId, granted)
    }

    // ---- location (REQ-PLT-04) ----

    @JvmStatic
    fun requestLocation(requestId: String): String? = guard("requestLocation") {
        val granted = AppHolder.context.checkSelfPermission(android.Manifest.permission.ACCESS_FINE_LOCATION) ==
            PackageManager.PERMISSION_GRANTED
        if (!granted) {
            resolveLocationSafely(requestId, 0.0, 0.0, false)
        } else {
            LocationService(AppHolder.context).currentLocation { loc ->
                resolveLocationSafely(requestId, loc?.latitude ?: 0.0, loc?.longitude ?: 0.0, loc != null)
            }
        }
    }

    // ---- notifications (REQ-PLT-02) ----

    @JvmStatic
    fun executeCommand(data: CommandData): String? = guard("executeCommand") {
        NotificationService(AppHolder.context).execute(data)
    }

    // ---- geofence (REQ-PLT-04) ----

    /** Last known registration state (Play services has no query API; v1 tracked it the same way). */
    @Volatile
    private var geofenceActive = false

    @JvmStatic
    fun startGeofence(latitude: Double, longitude: Double, triggerOnEntry: Boolean): String? = guard("startGeofence") {
        LocationService(AppHolder.context).startGeofence(latitude, longitude, triggerOnEntry) { ok ->
            geofenceActive = ok
            if (!ok) Log.w(TAG, "startGeofence rejected by Play services (background permission?)")
        }
    }

    @JvmStatic
    fun stopGeofence(): String? = guard("stopGeofence") {
        LocationService(AppHolder.context).stopGeofence()
        geofenceActive = false
    }

    /** "true" while a geofence is registered, null otherwise (value channel). */
    @JvmStatic
    fun isGeofenceMonitoring(): String? = if (geofenceActive) "true" else null

    // ---- system (REQ-PLT-05) ----

    @JvmStatic
    fun track(event: String, props: HashMap<String, String>): String? = guard("track") {
        Aptabase.instance.trackEvent(event, props)
    }

    @JvmStatic
    fun setAppIcon(accent: String): String? = guard("setAppIcon") {
        val ctx = AppHolder.context
        val pkg = ctx.packageName
        val suffixes = listOf("Orange", "Emerald", "Berry", "Golden", "Ocean")
        val wanted = "Orange" + accent.replaceFirstChar { it.uppercase() }
        val target = if (suffixes.contains(wanted)) wanted else "Orange"
        for (suffix in suffixes) {
            val component = ComponentName(ctx, "$pkg.SnackPilotActivity$suffix")
            val state = if (suffix == target) {
                PackageManager.COMPONENT_ENABLED_STATE_ENABLED
            } else {
                PackageManager.COMPONENT_ENABLED_STATE_DISABLED
            }
            ctx.packageManager.setComponentEnabledSetting(component, state, PackageManager.DONT_KILL_APP)
        }
    }

    @JvmStatic
    fun openAppSettings(): String? = guard("openAppSettings") {
        val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
            data = Uri.fromParts("package", AppHolder.context.packageName, null)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        AppHolder.context.startActivity(intent)
    }

    @JvmStatic
    fun composeEmail(email: SupportEmailData): String? = guard("composeEmail") {
        val intent = Intent(Intent.ACTION_SENDTO).apply {
            data = Uri.parse("mailto:")
            putExtra(Intent.EXTRA_EMAIL, arrayOf(email.to))
            putExtra(Intent.EXTRA_SUBJECT, email.subject)
            putExtra(Intent.EXTRA_TEXT, email.body)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        AppHolder.context.startActivity(intent)
    }

    private fun resolvePermissionSafely(requestId: String, granted: Boolean) {
        try {
            nativeResolvePermission(requestId, granted)
        } catch (e: UnsatisfiedLinkError) {
            Log.w(TAG, "nativeResolvePermission unavailable: ${e.message}")
        }
    }

    private fun resolveLocationSafely(requestId: String, lat: Double, lon: Double, found: Boolean) {
        try {
            nativeResolveLocation(requestId, lat, lon, found)
        } catch (e: UnsatisfiedLinkError) {
            Log.w(TAG, "nativeResolveLocation unavailable: ${e.message}")
        }
    }

    private inline fun guard(label: String, block: () -> Unit): String? =
        try {
            block()
            null
        } catch (e: Exception) {
            Log.e(TAG, "$label failed", e)
            e.message ?: e.javaClass.simpleName
        }
}
