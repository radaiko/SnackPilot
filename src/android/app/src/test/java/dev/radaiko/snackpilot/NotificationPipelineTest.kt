package dev.radaiko.snackpilot

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.snackpilot_core.NotificationCommand

/**
 * Unit tests for the notification pipeline glue that lives in the shell. The decision logic is
 * core-tested; what the shell owns is (a) the channel-id constants the core commands route on,
 * (b) the doze-safe alarm scheduling arithmetic and (c) the background-sync schedule constants
 * and single-flight gate. These run on the JVM with a stub android.jar, so only pure functions
 * are covered here — delivery itself needs a device (verified via the `uiTestNotify` /
 * `uiTestSchedule` / `uiTestBackgroundSync` debug hooks in MainActivity).
 */
class NotificationPipelineTest {

    /** @req REQ-NTF-12 */
    @Test
    fun channelIdsMatchSpecNames() {
        // notifications-daily-reminder §/notifications-new-menu: German channel names, ids stable.
        assertEquals("order-reminders", NotificationService.CHANNEL_ORDER_REMINDERS)
        assertEquals("menu-updates", NotificationService.CHANNEL_MENU_UPDATES)
    }

    /** @req REQ-NTF-12 */
    @Test
    fun channelDefinitionsMatchSpec() {
        // 05-platform-services §5.3: exact names, importance and vibration, pinned via the
        // named constants NotificationService creates its channels from.
        assertEquals("Bestellungs-Erinnerungen", NotificationService.CHANNEL_ORDER_REMINDERS_NAME)
        assertEquals(4, NotificationService.IMPORTANCE_ORDER_REMINDERS) // IMPORTANCE_HIGH
        assertArrayEquals(
            longArrayOf(0, 250, 250, 250),
            NotificationService.CHANNEL_ORDER_REMINDERS_VIBRATION,
        )
        assertEquals("Neue Menüs", NotificationService.CHANNEL_MENU_UPDATES_NAME)
        assertEquals(3, NotificationService.IMPORTANCE_MENU_UPDATES) // IMPORTANCE_DEFAULT
    }

    /** @req REQ-NTF-12 */
    @Test
    fun alarmModeIsExactBelowApi31() {
        // Below API 31 exact alarms need no grant, so always exact-while-idle.
        assertEquals(AlarmMode.EXACT_IDLE, NotificationService.alarmMode(30, canScheduleExact = false))
    }

    /** @req REQ-NTF-12 */
    @Test
    fun alarmModeFallsBackWhenExactNotPermitted() {
        // API 31+ without the exact-alarm grant: inexact allow-while-idle, may fire minutes late
        // under Doze (design D8 — SCHEDULE_EXACT_ALARM stays undeclared).
        assertEquals(AlarmMode.INEXACT_IDLE, NotificationService.alarmMode(31, canScheduleExact = false))
    }

    /** @req REQ-NTF-12 */
    @Test
    fun alarmModeIsExactWhenPermitted() {
        assertEquals(AlarmMode.EXACT_IDLE, NotificationService.alarmMode(34, canScheduleExact = true))
    }

    /** @req REQ-NTF-11 */
    @Test
    fun notificationsAllowedBelowApi33Always() {
        // POST_NOTIFICATIONS only exists on API 33+; below it delivery is never gated.
        assertTrue(NotificationService.notificationsAllowed(32, granted = false))
    }

    /** @req REQ-NTF-11 */
    @Test
    fun notificationsDeniedBlocksOnApi33() {
        assertFalse(NotificationService.notificationsAllowed(33, granted = false))
        assertTrue(NotificationService.notificationsAllowed(33, granted = true))
    }

    /** @req REQ-NTF-08 */
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

    /** @req REQ-NTF-10 */
    @Test
    fun backgroundSyncScheduleIsPinned() {
        // One unique periodic job named "background-sync" at the WorkManager 15-minute floor,
        // with the 5-minute initial delay (the KEEP policy itself is device-verified).
        assertEquals("background-sync", BackgroundSyncWorker.UNIQUE_NAME)
        assertTrue(
            "periodic interval must stay at the 15-minute WorkManager floor",
            BackgroundSync.INTERVAL_MINUTES >= 15,
        )
        assertEquals(5L, BackgroundSync.INITIAL_DELAY_MINUTES)
    }

    /** @req REQ-NTF-13 */
    @Test
    fun syncGateRejectsOverlappingRuns() {
        // A second menu check starting while one runs returns null and performs no work.
        runBlocking {
            var innerRan = false
            var innerResult: Int? = null
            val outer = SyncGate.runExclusive {
                innerResult = SyncGate.runExclusive { innerRan = true; 1 }
                "outer"
            }
            assertEquals("outer", outer)
            assertNull("nested run must be rejected", innerResult)
            assertTrue("rejected block must not execute", !innerRan)
        }
    }

    /** @req REQ-NTF-13 */
    @Test
    fun syncGateReleasesAfterRun() {
        // The gate frees again once a run finished, so the next caller executes.
        runBlocking {
            assertEquals("first", SyncGate.runExclusive { "first" })
            assertEquals("second", SyncGate.runExclusive { "second" })
        }
    }
}
