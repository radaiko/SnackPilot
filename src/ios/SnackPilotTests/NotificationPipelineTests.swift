import XCTest
import UserNotifications
@testable import SnackPilot

/// Unit tests for the notification-pipeline glue that lives in the shell (the decision logic
/// itself is core-tested). Runs on the simulator without a network or credentials: these pin
/// the command-shape contract between the generated UniFFI binding and our delivery switch,
/// plus the pure state of the background path. Real banner delivery is verified on device via
/// the "Hintergrund-Sync ausführen" debug button and the `uiTestNotify`/`uiTestSchedule` hooks.
final class NotificationPipelineTests: XCTestCase {

    // @req REQ-NTF-08
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

    // @req REQ-NTF-08
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

    // @req REQ-NTF-09
    func testFireNowRequestHasNoTrigger() throws {
        // FireNow must produce a triggerless request so it shows immediately (spec "Immediate
        // request"). Goes through execute() so the wiring, not just the builder, is pinned:
        // a regression to after: 0.1 gives a time-interval trigger and fails here.
        var added: [UNNotificationRequest] = []
        let service = NotificationService(add: { added.append($0) }, remove: { _ in })
        service.execute(.fireNow(id: "menu-1", title: "Neues Menü", body: "Das Menü ist online.",
                                 channelId: nil, screen: nil))
        XCTAssertEqual(added.count, 1, "execute(.fireNow) must add exactly one request")
        let request = try XCTUnwrap(added.first)
        XCTAssertEqual(request.identifier, "menu-1")
        XCTAssertNil(request.trigger, "FireNow must have no trigger (immediate display)")
        XCTAssertEqual(request.content.title, "Neues Menü")
        XCTAssertEqual(request.content.body, "Das Menü ist online.")
        XCTAssertEqual(request.content.sound, .default)
    }

    // @req REQ-NTF-09
    func testScheduleAtRequestUsesTimeIntervalTrigger() throws {
        // Spec "Scheduled request": 600 s ahead → non-repeating time-interval trigger ~600 s.
        var added: [UNNotificationRequest] = []
        let service = NotificationService(add: { added.append($0) }, remove: { _ in })
        let nowMs = Int64(Date().timeIntervalSince1970 * 1000)
        service.execute(.scheduleAt(id: "rem-1", title: "Erinnerung", body: "Bestellen!",
                                    channelId: nil, fireAtEpochMs: nowMs + 600_000, screen: nil))
        XCTAssertEqual(added.count, 1, "execute(.scheduleAt) must add exactly one request")
        let request = try XCTUnwrap(added.first)
        let trigger = try XCTUnwrap(request.trigger as? UNTimeIntervalNotificationTrigger,
                                    "ScheduleAt must use a time-interval trigger")
        XCTAssertFalse(trigger.repeats, "the trigger must not repeat")
        XCTAssertEqual(trigger.timeInterval, 600, accuracy: 1)
    }

    // @req REQ-NTF-09
    func testDelayForCommands() {
        // Pure delay mapping: fireNow → 0 (no trigger), past scheduleAt → 1 s floor,
        // 600 s ahead → ~600.
        let fireNow = NotificationCommand.fireNow(id: "a", title: "t", body: "b",
                                                  channelId: nil, screen: nil)
        XCTAssertEqual(NotificationService.delay(for: fireNow, nowMs: 1_000), 0)
        let past = NotificationCommand.scheduleAt(id: "a", title: "t", body: "b",
                                                  channelId: nil, fireAtEpochMs: 0, screen: nil)
        XCTAssertEqual(NotificationService.delay(for: past, nowMs: 1_000_000), 1,
                       "a past fire time keeps the 1 s floor")
        let ahead = NotificationCommand.scheduleAt(id: "a", title: "t", body: "b",
                                                   channelId: nil, fireAtEpochMs: 1_600_000,
                                                   screen: nil)
        XCTAssertEqual(NotificationService.delay(for: ahead, nowMs: 1_000_000), 600, accuracy: 0.001)
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

    // @req REQ-NTF-04
    func testReminderClampUsesNearestBound() {
        // Design D10 (nearest bound): a time outside 11:00–13:45 snaps to the NEAREST bound as a
        // whole time. The old independent hour/minute clamp turned 08:30 into 11:30 — a bug; this
        // pins 08:30 → 11:00 and both upper cases → 13:45, and that an in-window time is untouched.
        var c = ReminderWindow.clamp(hour: 8, minute: 30)
        XCTAssertEqual(c.hour, 11); XCTAssertEqual(c.minute, 0)
        c = ReminderWindow.clamp(hour: 13, minute: 50)
        XCTAssertEqual(c.hour, 13); XCTAssertEqual(c.minute, 45)
        c = ReminderWindow.clamp(hour: 14, minute: 30)
        XCTAssertEqual(c.hour, 13); XCTAssertEqual(c.minute, 45)
        c = ReminderWindow.clamp(hour: 12, minute: 20)
        XCTAssertEqual(c.hour, 12); XCTAssertEqual(c.minute, 20)
    }

    // @req REQ-NTF-04
    func testReminderHydrationDefaultsAndStoredValues() {
        // No stored time (fresh install / core defaults) hydrates to the spec default 11:00 off;
        // stored values pass through unchanged (shared source of truth with the core KV).
        let fresh = ReminderWindow.hydrate(DailyReminderSettings(enabled: false, hour: nil, minute: nil))
        XCTAssertEqual(fresh.enabled, false)
        XCTAssertEqual(fresh.hour, 11)
        XCTAssertEqual(fresh.minute, 0)
        let stored = ReminderWindow.hydrate(DailyReminderSettings(enabled: true, hour: 12, minute: 20))
        XCTAssertEqual(stored.enabled, true)
        XCTAssertEqual(stored.hour, 12)
        XCTAssertEqual(stored.minute, 20)
    }

    // @req REQ-NTF-05
    func testEnableDeniedPersistsNothing() async throws {
        // Permission denied → enable returns false and the core KV keeps its previous values
        // (spec "Permission denied": switch stays off, nothing written).
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("snackpilot-tests-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: dir) }
        let core = try SnackPilotCore(config: CoreConfig(storageDir: dir.path), analytics: nil)
        let ok = await ReminderWindow.enable(core: core, hour: 12, minute: 0, requestPermission: { false })
        XCTAssertFalse(ok, "a denied permission must report false")
        let settings = core.dailyReminderSettings()
        XCTAssertFalse(settings.enabled, "denied enable must not set the enabled flag")
        XCTAssertNil(settings.hour, "denied enable must not write the time")
    }

    // @req REQ-NTF-05
    func testEnableGrantedWritesTimeThenFlag() async throws {
        // Permission granted → enable returns true and the settings are enabled at the given time
        // (the time-before-flag write order is the implementation contract, spec "Write order").
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("snackpilot-tests-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: dir) }
        let core = try SnackPilotCore(config: CoreConfig(storageDir: dir.path), analytics: nil)
        let ok = await ReminderWindow.enable(core: core, hour: 12, minute: 20, requestPermission: { true })
        XCTAssertTrue(ok, "a granted permission must report true")
        let settings = core.dailyReminderSettings()
        XCTAssertTrue(settings.enabled)
        XCTAssertEqual(settings.hour, 12)
        XCTAssertEqual(settings.minute, 20)
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
