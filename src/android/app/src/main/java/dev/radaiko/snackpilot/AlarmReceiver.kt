package dev.radaiko.snackpilot

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/**
 * Fires a scheduled notification when its AlarmManager alarm goes off (the delivery half of
 * NotificationCommand.ScheduleAt). Registered in the manifest.
 */
class AlarmReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val id = intent.getStringExtra(EXTRA_ID) ?: return
        NotificationService(context).deliver(
            id = id,
            title = intent.getStringExtra(EXTRA_TITLE).orEmpty(),
            body = intent.getStringExtra(EXTRA_BODY).orEmpty(),
            channelId = intent.getStringExtra(EXTRA_CHANNEL)
        )
    }

    companion object {
        const val EXTRA_ID = "id"
        const val EXTRA_TITLE = "title"
        const val EXTRA_BODY = "body"
        const val EXTRA_CHANNEL = "channel"
    }
}
