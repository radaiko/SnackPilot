import XCTest

// Desktop-ui-redesign task 1.5 mobile regression (REQ-UI-04/REQ-UI-07,
// REQ-SCR-04..08): the shared Root must keep the incumbent iOS shell and the
// pushed settings forms at every width; no rail, table or desktop split may
// leak into the phone app.
//
// GPUI renders to Metal without an accessibility tree, so this follows the
// established coordinate-tap evidence route: launch the debug host, tap the
// pill's Einstellungen destination and the first settings row, and store
// before/after screenshots for visual review. It never enters a real host and
// never types real credentials.
//
// Note: `SnackPilotUITests` lists its sources explicitly in project.yml; this
// file is part of that list, so `xcodebuild test` executes it. The run itself
// still needs a real simulator (see docs/evidence/desktop-ui-redesign/1.5/
// NOT-CAPTURED.md).

final class DesktopRegressionTests: XCTestCase {
    /// Rightmost destination of the iOS floating pill on the 402x874 pt
    /// reference device (4×44 buttons, pill x 77..325, centre y ≈ 770).
    private let settingsTab = CGVector(dx: 294.0 / 402.0, dy: 770.0 / 874.0)
    /// First row of the settings list, below the "Einstellungen" title.
    private let firstSettingsRow = CGVector(dx: 0.5, dy: 0.26)
    /// The pill's leftmost destination (Menüs).
    private let menusTab = CGVector(dx: 108.0 / 402.0, dy: 770.0 / 874.0)

    private func saveScreenshot(_ name: String) {
        let shot = XCUIScreen.main.screenshot()
        let attachment = XCTAttachment(screenshot: shot)
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
        let dir = ProcessInfo.processInfo.environment["SPIKE_EVIDENCE_DIR"]
            ?? "/tmp/desktop-regression-15"
        try? FileManager.default.createDirectory(
            atPath: dir, withIntermediateDirectories: true)
        try? shot.pngRepresentation.write(to: URL(fileURLWithPath: "\(dir)/\(name).png"))
    }

    /// Launch the shared host and wait for its first GPUI frame.
    private func launch(_ args: [String]) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments = args
        app.launch()
        XCTAssertTrue(
            app.otherElements.matching(NSPredicate(format: "label == %@", "counter 0"))
                .firstMatch.waitForExistence(timeout: 30),
            "shared shell did not render")
        return app
    }

    /// Bounded wait for the next rendered frame (foreground + existing window).
    /// It replaces the fixed `sleep(1)`; GPUI has no accessibility tree, so the
    /// screenshots remain the evidence and this helper does not invent a label
    /// assertion.
    private func waitForSettledFrame(_ app: XCUIApplication, timeout: TimeInterval = 5) {
        let foreground = expectation(
            for: NSPredicate(
                format: "state == %d",
                XCUIApplication.State.runningForeground.rawValue),
            evaluatedWith: app)
        wait(for: [foreground], timeout: timeout)
        XCTAssertTrue(
            app.windows.firstMatch.waitForExistence(timeout: timeout),
            "no rendered window after the interaction")
    }

    /// The Day Tray baseline runs on the isolated debug host and records the
    /// actual rendered window bounds for every destination at the selected
    /// logical viewport. It waits for the window rather than sleeping, and
    /// never enters a credential form or a real host.
    /// @req REQ-UI-16
    func testDayTrayBaselineIsolationAndViewport() {
        let tabs = ["menus", "orders", "billing", "settings"]
        for (index, tab) in tabs.enumerated() {
            let app = XCUIApplication()
            app.launchArguments = [
                "--evidence",
                "--viewport=390x844",
                "--tab=\(index)",
            ]
            app.launchEnvironment["SNACKPILOT_EVIDENCE"] = "1"
            app.launch()

            let window = app.windows.firstMatch
            XCTAssertTrue(
                window.waitForExistence(timeout: 30),
                "\(tab): the isolated shell did not render")
            let bounds = window.frame
            XCTAssertGreaterThan(bounds.width, 0, "\(tab): no rendered width")
            XCTAssertGreaterThan(bounds.height, 0, "\(tab): no rendered height")
            XCTAssertEqual(
                app.state, .runningForeground,
                "\(tab): baseline host is not foreground")

            // An actual frame is the readiness proof; a zero-sized capture or
            // a blank screen fails here instead of being indexed as baseline.
            let shot = XCUIScreen.main.screenshot()
            XCTAssertGreaterThan(
                shot.image.size.width, 0, "\(tab): empty baseline capture")
            XCTAssertGreaterThan(
                shot.image.size.height, 0, "\(tab): empty baseline capture")
            saveScreenshot("baseline-\(tab)-viewport-\(Int(bounds.width))x\(Int(bounds.height))")
            app.terminate()
        }
    }

    /// The phone keeps the current shell: the floating pill switches tabs and
    /// no desktop rail appears (REQ-UI-04).
    /// @req REQ-UI-04
    func testMobileKeepsTheFloatingPillShell() {
        let app = launch([])
        saveScreenshot("mobile-before-menus")
        app.coordinate(withNormalizedOffset: settingsTab).tap()
        waitForSettledFrame(app)
        saveScreenshot("mobile-settings")
        app.coordinate(withNormalizedOffset: menusTab).tap()
        waitForSettledFrame(app)
        saveScreenshot("mobile-after-menus")
        XCTAssertTrue(app.state == .runningForeground)
    }

    /// Settings still pushes the incumbent credential form with a back
    /// affordance; the desktop split/back-less containment must not appear on
    /// the phone (REQ-SCR-04/05/06).
    /// @req REQ-SCR-04
    func testMobileSettingsStillPushesTheCredentialForm() {
        let app = launch(["--tab=3"])
        saveScreenshot("mobile-settings-list")
        app.coordinate(withNormalizedOffset: firstSettingsRow).tap()
        waitForSettledFrame(app)
        saveScreenshot("mobile-settings-pushed-form")
        XCTAssertTrue(app.state == .runningForeground)
    }

    /// Mobile Day Tray 1.2 (REQ-UI-03/REQ-UI-04/REQ-UI-07/REQ-UI-15): the
    /// compact shell is one root with the platform navigation at360, 390, 768
    /// and1280. Each launch records a real frame at the requested in-app
    /// viewport; GPUI has no accessibility tree, so the only honest assertions
    /// are the rendered window/screenshot and foreground state. The content cap,
    /// single column and rail absence are visual-review facts in the 1.2 index,
    /// backed by the Rust `compact_shell_plan` tests, not coordinate claims.
    /// @req REQ-UI-03 @req REQ-UI-04 @req REQ-UI-07
    func testDayTrayCompactShellAtEveryWidth() {
        for viewport in ["360x780", "390x844", "768x1024", "1280x1024"] {
            let app = XCUIApplication()
            app.launchArguments = ["--evidence", "--viewport=\(viewport)", "--tab=0"]
            app.launchEnvironment["SNACKPILOT_EVIDENCE"] = "1"
            app.launch()
            let window = app.windows.firstMatch
            XCTAssertTrue(
                window.waitForExistence(timeout: 30),
                "\(viewport): the compact shell did not render")
            XCTAssertGreaterThan(window.frame.width, 0, "\(viewport): no frame")
            XCTAssertEqual(app.state, .runningForeground)
            saveScreenshot("daytray-shell-\(viewport)")
            app.terminate()
        }
    }

    /// Mobile Day Tray 1.2 (REQ-UI-06): the in-app new-menu toast uses the
    /// approved C1 copy without the exclamation mark, at its unchanged
    /// 300/4000/300 ms timing. The toast is a painted overlay with no
    /// accessibility node, so the capture is the evidence and no label
    /// assertion is fabricated.
    /// @req REQ-UI-06
    func testDayTrayToastCopy() {
        let app = XCUIApplication()
        app.launchArguments = ["--evidence", "--viewport=390x844", "--tab=0", "--toast"]
        app.launchEnvironment["SNACKPILOT_EVIDENCE"] = "1"
        app.launch()
        XCTAssertTrue(
            app.windows.firstMatch.waitForExistence(timeout: 30),
            "the compact shell did not render")
        saveScreenshot("daytray-toast")
        XCTAssertEqual(app.state, .runningForeground)
    }

    /// Mobile Day Tray 1.3 (REQ-SCR-01/REQ-UI-16): the compact Menüs screen
    /// selects dishes across two days, shows the mixed pending dock, a repeated
    /// activation of that dock records its action trace, and the now-ordered
    /// dish toggles to a queued cancellation in the same dock. GPUI renders
    /// without an accessibility tree, so the captures plus the bounded frame
    /// waits are the evidence; the phase strings and the single-flight guard
    /// are asserted by the Rust model/app tests, not fabricated here. The
    /// launches use the isolated synthetic scenario knobs only.
    /// @req REQ-SCR-01 @req REQ-UI-16
    func testDayTrayMenusDayNavigationAndMixedDock() {
        // The first card of the selected day's first group (below the heading
        // row, the day navigator and the shell heading).
        let firstCard = CGVector(dx: 0.5, dy: 0.30)
        // The right day arrow of the navigator (48 pt target at the column's
        // trailing edge, below the shell heading).
        let nextDay = CGVector(dx: 0.90, dy: 0.18)
        // The measured dock band: 8 pt above the pill, 48 pt tall.
        let dock = CGVector(dx: 0.5, dy: 0.855)

        let app = XCUIApplication()
        app.launchArguments = [
            "--evidence", "--viewport=390x844", "--tab=0", "--scenario=pending",
        ]
        app.launchEnvironment["SNACKPILOT_EVIDENCE"] = "1"
        app.launch()
        XCTAssertTrue(
            app.windows.firstMatch.waitForExistence(timeout: 30),
            "the compact Menüs screen did not render")
        XCTAssertEqual(app.state, .runningForeground)
        saveScreenshot("daytray-menus-pending")

        // Select a dish on the first day, then one on the next day: the dock
        // carries both, the all-days set the model owns.
        app.coordinate(withNormalizedOffset: firstCard).tap()
        waitForSettledFrame(app)
        app.coordinate(withNormalizedOffset: nextDay).tap()
        waitForSettledFrame(app)
        app.coordinate(withNormalizedOffset: firstCard).tap()
        waitForSettledFrame(app)
        saveScreenshot("daytray-menus-dock-two-days")

        // A repeated activation of the same dock is one action trace; each tap
        // is followed by a real frame, never a fixed sleep.
        for _ in 0..<2 {
            app.coordinate(withNormalizedOffset: dock).tap()
        }
        waitForSettledFrame(app)
        saveScreenshot("daytray-menus-dock-phases")

        // The submitted dish is now ordered: activating it queues the
        // cancellation and the dock switches to the `Stornieren` label; a
        // second activation unselects it again, so the trace shows the toggle
        // both ways.
        app.coordinate(withNormalizedOffset: firstCard).tap()
        waitForSettledFrame(app)
        saveScreenshot("daytray-menus-cancellation-dock")
        app.coordinate(withNormalizedOffset: firstCard).tap()
        waitForSettledFrame(app)
        saveScreenshot("daytray-menus-cancellation-cleared")
        XCTAssertEqual(app.state, .runningForeground)
        XCTAssertTrue(app.windows.firstMatch.exists, "the shell is still one window")
        app.terminate()
    }

    /// Mobile Day Tray 1.5 (REQ-SCR-03/REQ-UI-15): the compact Abrechnung
    /// screen keeps its month and source controls while data settles, switches
    /// month and then source, and paints one summary plus the read-only
    /// receipts. GPUI renders without an accessibility tree, so the captures
    /// and the bounded frame waits are the evidence here; the totals/subsidy
    /// equality, the selected-context guard and the read-only cards are
    /// asserted by the Rust model/app tests, not fabricated from coordinates.
    /// The launches use the isolated synthetic evidence knobs only.
    /// @req REQ-SCR-03 @req REQ-UI-15
    func testDayTrayBillingMonthsSourcesAndReceipts() {
        // Second of the three month choices: the row starts 16 pt below the
        // shell heading (heading box top ≈ 78 pt on a 390-wide viewport).
        let secondMonth = CGVector(dx: 0.5, dy: 0.172)
        // The Kantine/Automaten source options in the row below the months
        // (the chip band sits at ≈ 201..240 pt on the 874 pt device).
        let kantineSource = CGVector(dx: 0.388, dy: 0.253)
        let automatenSource = CGVector(dx: 0.657, dy: 0.253)

        let app = XCUIApplication()
        app.launchArguments = ["--evidence", "--viewport=390x844", "--tab=2"]
        app.launchEnvironment["SNACKPILOT_EVIDENCE"] = "1"
        app.launch()
        XCTAssertTrue(
            app.windows.firstMatch.waitForExistence(timeout: 30),
            "the compact Abrechnung screen did not render")
        XCTAssertEqual(app.state, .runningForeground)
        saveScreenshot("daytray-billing-default")

        app.coordinate(withNormalizedOffset: secondMonth).tap()
        waitForSettledFrame(app)
        saveScreenshot("daytray-billing-second-month")

        app.coordinate(withNormalizedOffset: kantineSource).tap()
        waitForSettledFrame(app)
        saveScreenshot("daytray-billing-kantine")

        app.coordinate(withNormalizedOffset: automatenSource).tap()
        waitForSettledFrame(app)
        saveScreenshot("daytray-billing-automaten")
        XCTAssertEqual(app.state, .runningForeground)
        XCTAssertTrue(app.windows.firstMatch.exists, "the shell stays one window")
        app.terminate()
    }

    /// A stale notification route on the phone keeps the mobile screen, not
    /// the desktop unavailable fallback (REQ-SCR-08).
    /// @req REQ-SCR-08
    func testMobileNotificationsKeepTheirCapableScreen() {
        let app = launch(["--tab=3", "--detail=notifications"])
        saveScreenshot("mobile-notifications")
        XCTAssertTrue(app.state == .runningForeground)
    }
}
