package dev.radaiko.snackpilot

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import uniffi.snackpilot_core.NotificationCommand

/**
 * Executes the core's [NotificationCommand]s via the Android NotificationManager
 * (05-platform-services §3). The core decides *what/when*; this shell delivers. Channels mirror
 * the core's ids (order-reminders / menu-updates).
 *
 * Scope: immediate delivery (FireNow) + cancellation are wired and verifiable now. ScheduleAt
 * (timed reminders) needs AlarmManager/WorkManager and is handled in the background-tasks phase.
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

    /** Deliver/cancel a command. Returns false for the not-yet-wired ScheduleAt path. */
    fun execute(command: NotificationCommand): Boolean = when (command) {
        is NotificationCommand.FireNow -> {
            fire(command.id, command.title, command.body, command.channelId)
            true
        }
        is NotificationCommand.CancelPending -> {
            NotificationManagerCompat.from(context).cancel(command.id.hashCode())
            true
        }
        is NotificationCommand.ScheduleAt -> false // TODO(background phase): AlarmManager/WorkManager
    }

    private fun fire(id: String, title: String, body: String, channelId: String?) {
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

    companion object {
        const val CHANNEL_ORDER_REMINDERS = "order-reminders"
        const val CHANNEL_MENU_UPDATES = "menu-updates"
    }
}
