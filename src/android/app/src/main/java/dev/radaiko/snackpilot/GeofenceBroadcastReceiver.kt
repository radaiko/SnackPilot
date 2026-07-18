package dev.radaiko.snackpilot

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import com.google.android.gms.location.Geofence
import com.google.android.gms.location.GeofencingEvent
import uniffi.snackpilot_core.CoreConfig
import uniffi.snackpilot_core.GeofenceEvent
import uniffi.snackpilot_core.SnackPilotCore
import java.io.File

/**
 * Handles company-geofence Enter/Exit (notifications-location §4). Self-contained: the app may be
 * killed when this fires, so it builds its own core over the same storage dir, records
 * `is_at_company`, and delivers whatever the core decides (Enter → cancel any cancel-reminder and,
 * if nothing ordered today, fire the 08:45 reminder; Exit → re-evaluate). No network.
 */
class GeofenceBroadcastReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val event = GeofencingEvent.fromIntent(intent) ?: return
        if (event.hasError()) return
        val geoEvent = when (event.geofenceTransition) {
            Geofence.GEOFENCE_TRANSITION_ENTER -> GeofenceEvent.ENTER
            Geofence.GEOFENCE_TRANSITION_EXIT -> GeofenceEvent.EXIT
            else -> return
        }
        val dir = File(context.filesDir, "snackpilot")
        val core = SnackPilotCore(CoreConfig(storageDir = dir.absolutePath), analytics = AptabaseSink())
        core.loadCachedOrders()
        core.setIsAtCompany(geoEvent == GeofenceEvent.ENTER)
        val notifications = NotificationService(context)
        core.geofenceCommands(geoEvent).forEach { notifications.execute(it) }
    }
}
