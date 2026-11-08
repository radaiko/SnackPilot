package dev.radaiko.snackpilot

import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat

/** Doze-safe AlarmManager mode chosen by [NotificationService.alarmMode]. */
internal enum class AlarmMode { EXACT_IDLE, INEXACT_IDLE }

/**
 * Executes the core's notification commands via the Android NotificationManager
 * (05-platform-services §3). The core decides *what/when*; this shell delivers.
 * Channels mirror the core's ids (order-reminders / menu-updates) with the exact
 * v1 names/importance (§5.3). The command arrives as the plain [CommandData] wire
 * form — no `uniffi.snackpilot_core` type crosses the JNI boundary (task 3.4).
 *
 * FireNow delivers immediately; ScheduleAt registers a doze-safe AlarmManager alarm
 * (exact-while-idle where the OS allows it, inexact fallback otherwise); CancelPending
 * cancels both. All posts are gated on POST_NOTIFICATIONS (API 33+).
 */
class NotificationService(private val context: Context) {
    private val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

    init {
        // Exact ids/names/importance/vibration per 05-platform-services §5.3 so upgraded
        // installs keep consistent channel settings.
        manager.createNotificationChannel(
            NotificationChannel(
                CHANNEL_ORDER_REMINDERS,
                CHANNEL_ORDER_REMINDERS_NAME,
                IMPORTANCE_ORDER_REMINDERS,
            ).apply { vibrationPattern = CHANNEL_ORDER_REMINDERS_VIBRATION }
        )
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL_MENU_UPDATES, CHANNEL_MENU_UPDATES_NAME, IMPORTANCE_MENU_UPDATES)
        )
    }

    fun execute(command: CommandData): Boolean = when (command.kind) {
        KIND_FIRE_NOW -> {
            deliver(command.id, command.title, command.body, command.channelId.ifEmpty { null })
            true
        }
        KIND_SCHEDULE_AT -> {
            schedule(command.id, command.title, command.body, command.channelId.ifEmpty { null }, command.fireAtEpochMs)
            true
        }
        KIND_CANCEL_PENDING -> {
            NotificationManagerCompat.from(context).cancel(command.id.hashCode())
            alarmPendingIntent(command.id)?.let {
                (context.getSystemService(Context.ALARM_SERVICE) as AlarmManager).cancel(it)
            }
            true
        }
        else -> false
    }

    /** Post a notification now (also called from AlarmReceiver when a scheduled alarm fires). */
    fun deliver(id: String, title: String, body: String, channelId: String?) {
        // POST_NOTIFICATIONS (API 33+): NotificationManagerCompat silently no-ops when denied,
        // so check explicitly — the background paths (worker / alarm) have no UI to prompt.
        val granted = context.checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS) ==
            android.content.pm.PackageManager.PERMISSION_GRANTED
        if (!notificationsAllowed(Build.VERSION.SDK_INT, granted)) {
            return
        }
        val channel = channelId ?: CHANNEL_MENU_UPDATES
        val notification = NotificationCompat.Builder(context, channel)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle(title)
            .setContentText(body)
            .setStyle(NotificationCompat.BigTextStyle().bigText(body))
            .setAutoCancel(true)
            .build()
        // POST_NOTIFICATIONS is checked by the caller; NotificationManagerCompat no-ops if denied.
        NotificationManagerCompat.from(context).notify(id.hashCode(), notification)
    }

    private fun schedule(id: String, title: String, body: String, channelId: String?, fireAtEpochMs: Long) {
        val intent = Intent(context, AlarmReceiver::class.java).apply {
            putExtra(AlarmReceiver.EXTRA_ID, id)
            putExtra(AlarmReceiver.EXTRA_TITLE, title)
            putExtra(AlarmReceiver.EXTRA_BODY, body)
            putExtra(AlarmReceiver.EXTRA_CHANNEL, channelId)
        }
        val pending = PendingIntent.getBroadcast(
            context, id.hashCode(), intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        // Doze-safe alarm. The reminder windows (daily 11:00–13:45, cancel 08:45/09:00) are
        // tight enough that plain inexact `set` can slip past the deadline while idle, so use
        // exact-while-idle where the OS allows it and fall back to inexact otherwise
        // (no SCHEDULE_EXACT_ALARM requirement on the fallback path).
        val am = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
        // canScheduleExactAlarms() only exists on API 31+; below that, exact alarms need no grant.
        val canExact = Build.VERSION.SDK_INT < 31 || am.canScheduleExactAlarms()
        when (alarmMode(Build.VERSION.SDK_INT, canExact)) {
            AlarmMode.EXACT_IDLE -> am.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, fireAtEpochMs, pending)
            AlarmMode.INEXACT_IDLE -> am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, fireAtEpochMs, pending)
        }
    }

    private fun alarmPendingIntent(id: String): PendingIntent? {
        val intent = Intent(context, AlarmReceiver::class.java)
        return PendingIntent.getBroadcast(
            context, id.hashCode(), intent,
            PendingIntent.FLAG_NO_CREATE or PendingIntent.FLAG_IMMUTABLE
        )
    }

    companion object {
        const val CHANNEL_ORDER_REMINDERS = "order-reminders"
        const val CHANNEL_MENU_UPDATES = "menu-updates"
        // Exact names/importance/vibration per 05-platform-services §5.3, as named constants so
        // the JVM tests can pin them without a device.
        const val CHANNEL_ORDER_REMINDERS_NAME = "Bestellungs-Erinnerungen"
        const val CHANNEL_MENU_UPDATES_NAME = "Neue Menüs"
        const val IMPORTANCE_ORDER_REMINDERS = NotificationManager.IMPORTANCE_HIGH
        const val IMPORTANCE_MENU_UPDATES = NotificationManager.IMPORTANCE_DEFAULT
        val CHANNEL_ORDER_REMINDERS_VIBRATION = longArrayOf(0, 250, 250, 250)

        // Wire kinds, mirroring wire_command() in the Rust bridge (android.rs).
        const val KIND_FIRE_NOW = 0
        const val KIND_SCHEDULE_AT = 1
        const val KIND_CANCEL_PENDING = 2

        /** POST_NOTIFICATIONS gate (REQ-NTF-11): below API 33 posting is always allowed;
         *  from API 33 on it requires the runtime grant. */
        internal fun notificationsAllowed(sdkInt: Int, granted: Boolean): Boolean =
            sdkInt < 33 || granted

        /** Doze-safe alarm mode (REQ-NTF-12, design D8): exact-while-idle below API 31 (no
         *  grant needed there) or when the OS allows exact alarms; inexact allow-while-idle
         *  otherwise — those may fire minutes late under Doze (SCHEDULE_EXACT_ALARM stays
         *  undeclared). */
        internal fun alarmMode(sdkInt: Int, canScheduleExact: Boolean): AlarmMode =
            if (sdkInt < 31 || canScheduleExact) AlarmMode.EXACT_IDLE else AlarmMode.INEXACT_IDLE
    }
}
