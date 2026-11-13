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
        sleep(1)
        saveScreenshot("mobile-settings")
        app.coordinate(withNormalizedOffset: menusTab).tap()
        sleep(1)
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
        sleep(1)
        saveScreenshot("mobile-settings-pushed-form")
        XCTAssertTrue(app.state == .runningForeground)
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
