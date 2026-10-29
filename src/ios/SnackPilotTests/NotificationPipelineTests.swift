import XCTest
@testable import SnackPilot

/// Unit tests for the notification-pipeline glue that lives in the shell (the decision logic
/// itself is core-tested). Runs on the simulator without a network or credentials: these pin
/// the command-shape contract between the generated UniFFI binding and our delivery switch,
/// plus the pure state of the background path. Real banner delivery is verified on device via
/// the "Hintergrund-Sync ausführen" debug button and the `uiTestNotify`/`uiTestSchedule` hooks.
final class NotificationPipelineTests: XCTestCase {

    func testFireNowCommandCarriesPayload() {
        let command = NotificationCommand.fireNow(
            id: "menu-1", title: "Neues Menü", body: "Das Menü ist online.",
            channelId: nil, screen: nil
        )
        guard case let .fireNow(id, title, body, channelId, screen) = command else {
            return XCTFail("expected fireNow")
        }
        XCTAssertEqual(id, "menu-1")
        XCTAssertEqual(title, "Neues Menü")
        XCTAssertNil(channelId)
        XCTAssertNil(screen)
        _ = body
    }

    func testScheduleAtCommandCarriesFireTime() {
        let command = NotificationCommand.scheduleAt(
            id: "rem-1", title: "Erinnerung", body: "Bestellen!",
            channelId: "order-reminders", fireAtEpochMs: 1_000, screen: nil
        )
        guard case let .scheduleAt(id, _, _, channelId, fireAtEpochMs, _) = command else {
            return XCTFail("expected scheduleAt")
        }
        XCTAssertEqual(id, "rem-1")
        XCTAssertEqual(channelId, "order-reminders")
        XCTAssertEqual(fireAtEpochMs, 1_000)
    }

    func testBackgroundIdentifierMatchesProjectConfig() {
        // BGTaskSchedulerPermittedIdentifiers in project.yml must stay in sync with the code,
        // otherwise register() fails silently at launch.
        XCTAssertEqual(
            BackgroundTasks.refreshIdentifier,
            "dev.radaiko.gourmetclient.menu-refresh"
        )
    }

    func testCoreExposesHeadlessSyncCheckAndSettings() throws {
        // The background path calls these without any UI state; pin their existence & shape so
        // a binding regeneration that drops them breaks the build instead of silently disabling
        // reminders.
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("snackpilot-tests-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: dir) }
        let core = try SnackPilotCore(config: CoreConfig(storageDir: dir.path), analytics: nil)
        let settings = core.dailyReminderSettings()
        XCTAssertFalse(settings.enabled) // fresh storage dir → defaults
        let commands = core.runOrderSyncCheck() // empty cache → the cancel-reminder retraction (§2 step 3)
        XCTAssertEqual(commands.count, 1)
        guard case .cancelPending(let id) = commands.first else {
            return XCTFail("expected cancelPending for the cancel reminder, got \(commands)")
        }
        XCTAssertEqual(id, "cancel-order-reminder")
    }
}
