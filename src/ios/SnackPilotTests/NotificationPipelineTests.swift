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

    // @req REQ-NTF-06 @req REQ-NTF-07
    func testBackgroundIdentifierMatchesProjectConfig() {
        // BGTaskSchedulerPermittedIdentifiers in project.yml must stay in sync with the code,
        // otherwise register() fails silently at launch.
        XCTAssertEqual(
            BackgroundTasks.refreshIdentifier,
            "dev.radaiko.gourmetclient.menu-refresh"
        )
    }

    // @req REQ-NTF-07
    func testPackagedInfoPlistPermitsBackgroundRefresh() throws {
        // The GENERATE_INFOPLIST_FILE pipeline silently drops plist keys that are not set as
        // INFOPLIST_KEY_* build settings (validated: the packaged app lacked both keys, so
        // BGTaskScheduler.register failed). Unit tests run in the app host, so Bundle.main is
        // the shipped Info.plist — the only place this contract can be pinned.
        let info = try XCTUnwrap(Bundle.main.infoDictionary)
        let modes = try XCTUnwrap(info["UIBackgroundModes"] as? [String],
                                  "BGAppRefreshTask needs the fetch background mode")
        XCTAssertEqual(modes, ["fetch"], "processing is only for BGProcessingTask")
        let ids = try XCTUnwrap(info["BGTaskSchedulerPermittedIdentifiers"] as? [String],
                                "BGTaskScheduler.submit needs the permitted identifier")
        XCTAssertEqual(ids, [BackgroundTasks.refreshIdentifier])
    }

    // @req REQ-NTF-07
    func testPackagedInfoPlistKeepsAppIdentity() throws {
        // The identity keys the previous GENERATE_INFOPLIST_FILE build shipped must survive the
        // move to the xcodegen info: block (spec "Nothing else regressed" + "Version keys
        // resolve"). Bundle.main is the host app's shipped Info.plist, same as above.
        let info = try XCTUnwrap(Bundle.main.infoDictionary)
        XCTAssertEqual(info["CFBundleIdentifier"] as? String, "dev.radaiko.gourmetclient")
        XCTAssertEqual(info["CFBundleDisplayName"] as? String, "SnackPilot")
        XCTAssertFalse((info["NSLocationWhenInUseUsageDescription"] as? String ?? "").isEmpty,
                       "location purpose string missing from the packaged plist")
        XCTAssertFalse((info["NSLocationAlwaysAndWhenInUseUsageDescription"] as? String ?? "").isEmpty,
                       "always-when-in-use purpose string missing from the packaged plist")
        XCTAssertEqual(info["ITSAppUsesNonExemptEncryption"] as? Bool, false,
                       "export-compliance flag must be NO to skip the upload prompt")
        XCTAssertNotNil(info["UIApplicationSceneManifest"] as? [String: Any],
                        "scene manifest missing")
        XCTAssertNotNil(info["UILaunchScreen"] as? [String: Any],
                        "launch screen missing")
        // xcodegen writes $(MARKETING_VERSION)/$(CURRENT_PROJECT_VERSION); Xcode must have
        // expanded them at build time, otherwise TestFlight sees the raw reference.
        for key in ["CFBundleShortVersionString", "CFBundleVersion"] {
            let value = try XCTUnwrap(info[key] as? String, "\(key) missing")
            XCTAssertFalse(value.contains("$("), "\(key) still holds an unresolved build-setting reference")
        }
    }

    // @req REQ-NTF-03 @req REQ-NTF-01
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

    // @req REQ-NTF-13 @req REQ-NTF-14
    @MainActor
    func testHeadlessRunDeliversOrderSyncCommandsWithoutNetwork() async throws {
        // The shared background body must run with nothing but its arguments: a temp-dir core,
        // demo credentials (the core short-circuits the scrape → no host is contacted) and a
        // recording deliver. The only command is the order-sync result from the empty cache
        // (design D5; REQ-NTF-13 "Background body is headless on iOS").
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("snackpilot-tests-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: dir) }
        let core = try SnackPilotCore(config: CoreConfig(storageDir: dir.path), analytics: nil)
        var delivered: [NotificationCommand] = []
        let ran = await BackgroundTasks.runSync(
            core: core,
            credentials: Credentials(username: "demo", password: "demo1234!"),
            deliver: { delivered.append($0) }
        )
        XCTAssertTrue(ran, "runSync must report it ran")
        XCTAssertEqual(delivered.count, 1, "only the order-sync result may be delivered, got \(delivered)")
        guard case .cancelPending(let id) = delivered.first else {
            return XCTFail("expected cancelPending for the cancel reminder, got \(delivered)")
        }
        XCTAssertEqual(id, "cancel-order-reminder")
    }

    // @req REQ-NTF-13
    @MainActor
    func testBackgroundRunIsSingleFlight() async throws {
        // A second concurrent run (task, manual button or foreground check sharing the gate)
        // must return false immediately without any call — no scrape, no delivery.
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("snackpilot-tests-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: dir) }
        let core = try SnackPilotCore(config: CoreConfig(storageDir: dir.path), analytics: nil)
        BackgroundTasks.isRunning = true
        defer { BackgroundTasks.isRunning = false }
        var delivered: [NotificationCommand] = []
        let ran = await BackgroundTasks.runSync(
            core: core,
            credentials: Credentials(username: "demo", password: "demo1234!"),
            deliver: { delivered.append($0) }
        )
        XCTAssertFalse(ran, "a run while the gate is held must report false")
        XCTAssertTrue(delivered.isEmpty, "the busy run must deliver nothing, got \(delivered)")
    }
}
