package dev.radaiko.snackpilot

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import com.google.android.gms.location.Geofence
import com.google.android.gms.location.GeofencingEvent
import dev.radaiko.gourmetclient.SnackPilotActivity

/**
 * Handles company-geofence Enter/Exit (notifications-location §4). Self-contained: the app may be
 * killed when this fires. Thin pump only: the transition maps to a boolean and goes to the Rust
 * model via [SnackPilotActivity.getGeofencePump] (nativeOnGeofenceEvent → AppModel::on_geofence_event),
 * which sets is_at_company and delivers core's geofence commands (ordered-today check, Exit
 * re-evaluation, cancel reminder) through Platform::notify → NotificationService. The receiver
 * holds NO notification decision logic: v1 built its own core here, but at the JNI boundary that
 * would mean duplicating the uniffi core into the shell, which design D4 forbids.
 *
 * The native symbols exist only after [SnackPilotActivity] loaded the Rust library in this
 * process; a process spawned just for the broadcast has no pump, and the event is logged and
 * dropped (the next session load re-evaluates state through the normal flows).
 */
class GeofenceBroadcastReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val event = GeofencingEvent.fromIntent(intent) ?: return
        if (event.hasError()) return
        val entered = when (event.geofenceTransition) {
            Geofence.GEOFENCE_TRANSITION_ENTER -> true
            Geofence.GEOFENCE_TRANSITION_EXIT -> false
            else -> return
        }
        val pump = SnackPilotActivity.getGeofencePump()
        if (pump == null) {
            Log.w(TAG, "geofence ${if (entered) "ENTER" else "EXIT"}: native library not loaded in this process")
            return
        }
        Log.i(TAG, "geofence ${if (entered) "ENTER" else "EXIT"}: delegating to Rust model")
        try {
            pump.onTransition(entered)
        } catch (e: UnsatisfiedLinkError) {
            Log.w(TAG, "nativeOnGeofenceEvent unavailable: ${e.message}")
        }
    }

    companion object {
        private const val TAG = "GeofenceReceiver"
    }
}
