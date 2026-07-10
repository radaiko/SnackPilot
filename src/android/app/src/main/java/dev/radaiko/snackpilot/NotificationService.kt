package dev.radaiko.snackpilot

import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import uniffi.snackpilot_core.NotificationCommand

/**
 * Executes the core's [NotificationCommand]s via the Android NotificationManager
 * (05-platform-services §3). The core decides *what/when*; this shell delivers. Channels mirror
 * the core's ids (order-reminders / menu-updates).
 *
 * FireNow delivers immediately; ScheduleAt registers an inexact AlarmManager alarm (guideline-
 * recommended for reminders — no exact-alarm permission); CancelPending cancels both.
 */
class NotificationService(private val context: Context) {
    private val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

    init {
        createChannel(CHANNEL_ORDER_REMINDERS, "Bestell-Erinnerungen", NotificationManager.IMPORTANCE_HIGH)
        createChannel(CHANNEL_MENU_UPDATES, "Menü-Updates", NotificationManager.IMPORTANCE_DEFAULT)
    }

    private fun createChannel(id: String, name: String, importance: Int) {
        manager.createNotificationChannel(NotificationChannel(id, name, importance))
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
        // Inexact alarm: no SCHEDULE_EXACT_ALARM permission needed; a few minutes of slack is
        // acceptable for order/menu reminders (Android guidance discourages exact alarms).
        (context.getSystemService(Context.ALARM_SERVICE) as AlarmManager)
            .set(AlarmManager.RTC_WAKEUP, fireAtEpochMs, pending)
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
