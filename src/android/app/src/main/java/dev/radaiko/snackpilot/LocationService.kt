package dev.radaiko.snackpilot

import android.annotation.SuppressLint
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.location.Location
import com.google.android.gms.location.Geofence
import com.google.android.gms.location.GeofencingClient
import com.google.android.gms.location.GeofencingRequest
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority

/**
 * Play-services wrapper for the single company geofence (notifications-location §2–§4). The core
 * decides *what* to notify; this registers the 500 m region and routes Enter/Exit to
 * [GeofenceBroadcastReceiver]. Device-only: geofencing does not fire reliably on the emulator.
 */
class LocationService(private val context: Context) {
    private val geofencingClient: GeofencingClient = LocationServices.getGeofencingClient(context)

    private fun geofencePendingIntent(): PendingIntent {
        val intent = Intent(context, GeofenceBroadcastReceiver::class.java)
        return PendingIntent.getBroadcast(
            context, 0, intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE
        )
    }

    /** One-shot high-accuracy fix (§7.3). Caller must hold ACCESS_FINE_LOCATION; `null` on failure. */
    @SuppressLint("MissingPermission")
    fun currentLocation(onResult: (Location?) -> Unit) {
        LocationServices.getFusedLocationProviderClient(context)
            .getCurrentLocation(Priority.PRIORITY_HIGH_ACCURACY, null)
            .addOnSuccessListener { onResult(it) }
            .addOnFailureListener { onResult(null) }
    }

    /**
     * Register the single 500 m region (§2). [triggerOnEntry] fires Enter immediately when the
     * device is already inside — used when the user sets the location while at the office (§3.3).
     * The app-start restore passes false to avoid re-firing a spurious Enter (§3.2).
     */
    @SuppressLint("MissingPermission")
    fun startGeofence(latitude: Double, longitude: Double, triggerOnEntry: Boolean) {
        val geofence = Geofence.Builder()
            .setRequestId(REGION_ID)
            .setCircularRegion(latitude, longitude, RADIUS_M)
            .setExpirationDuration(Geofence.NEVER_EXPIRE)
            .setTransitionTypes(
                Geofence.GEOFENCE_TRANSITION_ENTER or Geofence.GEOFENCE_TRANSITION_EXIT
            )
            .build()
        val request = GeofencingRequest.Builder()
            .setInitialTrigger(
                if (triggerOnEntry) GeofencingRequest.INITIAL_TRIGGER_ENTER else 0
            )
            .addGeofence(geofence)
            .build()
        geofencingClient.addGeofences(request, geofencePendingIntent())
    }

    /** Stop monitoring the company region (§3, `stopGeofencing`). */
    fun stopGeofence() {
        geofencingClient.removeGeofences(listOf(REGION_ID))
    }

    companion object {
        const val REGION_ID = "company"
        const val RADIUS_M = 500f
    }
}
