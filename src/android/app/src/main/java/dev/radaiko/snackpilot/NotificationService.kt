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
import uniffi.snackpilot_core.NotificationCommand

/**
 * Executes the core's [NotificationCommand]s via the Android NotificationManager
 * (05-platform-services §3). The core decides *what/when*; this shell delivers. Channels mirror
 * the core's ids (order-reminders / menu-updates) with the exact v1 names/importance (§5.3).
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
            NotificationChannel(CHANNEL_ORDER_REMINDERS, "Bestellungs-Erinnerungen", NotificationManager.IMPORTANCE_HIGH)
                .apply { vibrationPattern = longArrayOf(0, 250, 250, 250) }
        )
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL_MENU_UPDATES, "Neue Menüs", NotificationManager.IMPORTANCE_DEFAULT)
        )
    }

    fun execute(command: NotificationCommand): Boolean = when (command) {
        is NotificationCommand.FireNow -> {
            deliver(command.id, command.title, command.body, command.channelId)
            true
        }
        is NotificationCommand.ScheduleAt -> {
            schedule(command.id, command.title, command.body, command.channelId, command.fireAtEpochMs)
            true
        }
        is NotificationCommand.CancelPending -> {
            NotificationManagerCompat.from(context).cancel(command.id.hashCode())
            alarmPendingIntent(command.id)?.let {
                (context.getSystemService(Context.ALARM_SERVICE) as AlarmManager).cancel(it)
            }
            true
        }
    }

    /** Post a notification now (also called from AlarmReceiver when a scheduled alarm fires). */
    fun deliver(id: String, title: String, body: String, channelId: String?) {
        // POST_NOTIFICATIONS (API 33+): NotificationManagerCompat silently no-ops when denied,
        // so check explicitly — the background paths (worker / alarm) have no UI to prompt.
        if (Build.VERSION.SDK_INT >= 33 &&
            context.checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS) !=
                android.content.pm.PackageManager.PERMISSION_GRANTED
        ) {
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
        val exact = Build.VERSION.SDK_INT < 31 || am.canScheduleExactAlarms()
        when {
            exact -> am.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, fireAtEpochMs, pending)
            else -> am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, fireAtEpochMs, pending)
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
    }
}
