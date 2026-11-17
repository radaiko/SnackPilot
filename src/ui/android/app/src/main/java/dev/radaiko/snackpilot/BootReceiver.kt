package dev.radaiko.snackpilot

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/**
 * Re-arms the reminder pipeline after a reboot (notifications-location §3, §9.3): Android drops
 * AlarmManager alarms and the periodic work, so the daily reminder and the cancel reminder would
 * never fire again until the app was opened by hand.
 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED &&
            intent.action != "android.intent.action.QUICKBOOT_POWERON"
        ) {
            return
        }
        BackgroundSync.schedule(context)
        BackgroundSync.scheduleNow(context)
    }
}
