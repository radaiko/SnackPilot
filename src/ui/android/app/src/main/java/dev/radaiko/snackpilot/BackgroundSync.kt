package dev.radaiko.snackpilot

import android.content.Context
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.Worker
import androidx.work.WorkerParameters
import java.util.concurrent.TimeUnit

/**
 * The OS background half of the notification pipeline (notifications-daily-reminder §2,
 * notifications-location §3): a 15-minute task that re-evaluates the reminder commands from the
 * cached orders and runs the new-menu check. The decision is the core's — [nativeTick] runs the
 * same model code the UI uses, over the same storage directory, and delivers through
 * [NotificationService].
 *
 * Registered at every launch (the launch restore re-arms it) and again after a reboot, because
 * Android drops both alarms and periodic work when the device restarts.
 */
object BackgroundSync {
    private const val PERIODIC = "snackpilot-background-sync"
    private const val ONCE = "snackpilot-background-sync-once"

    init {
        // A worker can be the first thing in a fresh process: no Activity has
        // run, so nothing has loaded the library yet. Loading it again when it
        // already is stays a no-op.
        runCatching { System.loadLibrary("snackpilot_app") }
    }

    /**
     * Rust half: reminder re-evaluation plus the new-menu check, no UI required.
     * [storageDir] is the directory the UI model uses, so the caches and the
     * saved session are the same ones.
     */
    external fun nativeTick(storageDir: String)

    /**
     * Geofence Enter/Exit for a process the broadcast started: the receiver has
     * no pump there, so the transition runs in Rust instead of being dropped
     * (notifications-location §4 — an Exit retracts the "no order" notice and
     * schedules the cancel reminder).
     */
    external fun nativeGeofence(inside: Boolean, storageDir: String)

    fun tick(context: Context) {
        nativeTick(storageDir(context))
    }

    fun geofence(context: Context, inside: Boolean) {
        nativeGeofence(inside, storageDir(context))
    }

    private fun storageDir(context: Context): String =
        java.io.File(context.filesDir, "snackpilot").absolutePath

    /** The periodic task; a second registration replaces the first (idempotent). */
    fun schedule(context: Context) {
        val request = PeriodicWorkRequestBuilder<BackgroundSyncWorker>(15, TimeUnit.MINUTES).build()
        WorkManager.getInstance(context)
            .enqueueUniquePeriodicWork(PERIODIC, ExistingPeriodicWorkPolicy.UPDATE, request)
    }

    /** Run once right now — the boot path, where the alarms did not survive. */
    fun scheduleNow(context: Context) {
        WorkManager.getInstance(context).enqueueUniqueWork(
            ONCE, ExistingWorkPolicy.REPLACE,
            OneTimeWorkRequestBuilder<BackgroundSyncWorker>().build(),
        )
    }
}

/** WorkManager entry point; the work itself is Rust's. */
class BackgroundSyncWorker(context: Context, params: WorkerParameters) :
    Worker(context, params) {
    override fun doWork(): Result = try {
        BackgroundSync.tick(applicationContext)
        Result.success()
    } catch (e: Throwable) {
        // No tight retry loop: the next periodic run is the retry.
        android.util.Log.w("BackgroundSync", "background tick failed", e)
        Result.failure()
    }
}
