package dev.radaiko.snackpilot

import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.snackpilot_core.NotificationCommand

/**
 * Unit tests for the notification pipeline glue that lives in the shell. The decision logic is
 * core-tested; what the shell owns is (a) the channel-id constants the core commands route on
 * and (b) the doze-safe alarm scheduling arithmetic. These run on the JVM with a stub
 * android.jar, so only pure functions are covered here — delivery itself needs a device
 * (verified via the `uiTestNotify` / `uiTestSchedule` debug hooks in MainActivity).
 */
class NotificationPipelineTest {

    @Test
    fun channelIdsMatchSpecNames() {
        // notifications-daily-reminder §/notifications-new-menu: German channel names, ids stable.
        assertEquals("order-reminders", NotificationService.CHANNEL_ORDER_REMINDERS)
        assertEquals("menu-updates", NotificationService.CHANNEL_MENU_UPDATES)
    }

    @Test
    fun commandVariantsCarryChannelRouting() {
        // Shell-side delivery routes on channelId; keep the core enum shape we depend on pinned.
        val fire = NotificationCommand.FireNow(
            id = "menu-1", title = "Neues Menü", body = "body",
            channelId = NotificationService.CHANNEL_MENU_UPDATES, screen = null
        )
        val schedule = NotificationCommand.ScheduleAt(
            id = "rem-1", title = "Erinnerung", body = "body",
            channelId = NotificationService.CHANNEL_ORDER_REMINDERS,
            fireAtEpochMs = 1_000L, screen = null
        )
        assertEquals(
            NotificationService.CHANNEL_MENU_UPDATES,
            (fire as NotificationCommand.FireNow).channelId
        )
        assertEquals(1_000L, (schedule as NotificationCommand.ScheduleAt).fireAtEpochMs)
    }
}
