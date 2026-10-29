package dev.radaiko.snackpilot

import android.content.Context
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import uniffi.snackpilot_core.CoreConfig
import uniffi.snackpilot_core.Credentials
import uniffi.snackpilot_core.SnackPilotCore
import java.io.File

/**
 * Periodic background sync (docs/architecture §4.4, notifications-new-menu §3): the Android
 * counterpart of the iOS BGAppRefreshTask. Runs on a WorkManager schedule (≥15 min), scrapes
 * for new menus with a fresh isolated session and evaluates the order-driven reminders from
 * the cache, then delivers the core's commands. Self-contained like [GeofenceBroadcastReceiver]:
 * builds its own core over the shared storage dir, so it works in a process the UI never started.
 */
class BackgroundSyncWorker(context: Context, params: WorkerParameters) :
    CoroutineWorker(context, params) {

    override suspend fun doWork(): Result {
        val dir = File(applicationContext.filesDir, "snackpilot").apply { mkdirs() }
        val core = try {
            SnackPilotCore(CoreConfig(storageDir = dir.absolutePath), analytics = AptabaseSink())
        } catch (e: Exception) {
            return Result.retry() // transient storage/init failure — WorkManager retries with backoff
        }
        val notifications = NotificationService(applicationContext)

        // New-menu check (v1 single-flight rule: one scrape at a time; WorkManager serialises us).
        val saved = SecureCredentialStore(applicationContext).savedGourmet()
        val creds = saved?.let { Credentials(username = it.first, password = it.second) }
        runCatching { core.runMenuCheck(creds) }
            .getOrNull()?.notification?.let { notifications.execute(it) }

        // Order-driven reminders from the cache (daily + cancel), no extra network.
        runCatching { core.runOrderSyncCheck() }.getOrNull()
            ?.forEach { notifications.execute(it) }

        return Result.success()
    }

    companion object {
        const val UNIQUE_NAME = "background-sync"
    }
}
