package dev.radaiko.snackpilot

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import java.time.Duration

/**
 * Re-establishes the periodic background sync after a reboot (docs/architecture §4.4: the
 * WorkManager schedule must survive restarts — RECEIVE_BOOT_COMPLETED). WorkManager persists
 * its own schedule, but v1 re-registered on boot and a force-stop/OTA can drop it; re-submitting
 * with KEEP is idempotent and cheap.
 */
class BootBroadcastReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        BackgroundSync.schedule(context)
    }
}

/** Single place that owns the WorkManager schedule for [BackgroundSyncWorker]. */
object BackgroundSync {
    /** Periodic menu/order-sync job. The interval floor is 15 min (WorkManager minimum). */
    fun schedule(context: Context) {
        val request = PeriodicWorkRequestBuilder<BackgroundSyncWorker>(Duration.ofMinutes(15))
            .setInitialDelay(Duration.ofMinutes(5)) // don't scrape the instant the device wakes
            .build()
        WorkManager.getInstance(context).enqueueUniquePeriodicWork(
            BackgroundSyncWorker.UNIQUE_NAME,
            ExistingPeriodicWorkPolicy.KEEP, // keep the existing cadence across launches/boots
            request,
        )
    }
}
