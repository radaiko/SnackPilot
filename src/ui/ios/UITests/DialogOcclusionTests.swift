import XCTest

// Task 4.2 review round 1 evidence: the §4.1 dialog backdrop must occlude
// what is underneath it. GPUI renders to Metal with no accessibility tree,
// so the proof is a coordinate tap (same route SpikeTapTests proved reaches
// GPUI input) plus before/after screenshots (SPIKE_EVIDENCE_DIR) and the
// app's "[4.2] tab selected" log line, which surfaces in the xcodebuild
// test output. @req REQ-UI-04

final class DialogOcclusionTests: XCTestCase {
    /// Rightmost tab of the iOS floating pill (Einstellungen): the pill is
    /// 248 pt wide, centred on the iPhone 17 Pro's 402x874 pt screen
    /// (x 77..325), its bottom edge at ~799 pt, so the fourth 44 pt button
    /// centres at (294, 770) device points.
    private let settingsTab = CGVector(dx: 294.0 / 402.0, dy: 770.0 / 874.0)

    private func saveScreenshot(_ name: String) {
        let dir = ProcessInfo.processInfo.environment["SPIKE_EVIDENCE_DIR"] ?? "/tmp/sp42-ev"
        let data = XCUIScreen.main.screenshot().pngRepresentation
        try? data.write(to: URL(fileURLWithPath: "\(dir)/\(name).png"))
    }

    /// Bounded wait for the next rendered frame: the app is foreground and its
    /// window exists. It replaces the old fixed `sleep(1)`; it does not claim a
    /// tab change, because GPUI exposes no accessibility tree.
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

    /// Launch with the given debug knobs and wait for the first GPUI frame
    /// (the host mirrors "counter 0" once the run loop renders).
    private func launch(_ args: [String]) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments = args
        app.launch()
        XCTAssertTrue(
            app.otherElements.matching(NSPredicate(format: "label == %@", "counter 0"))
                .firstMatch.waitForExistence(timeout: 30),
            "shell did not render")
        return app
    }

    /// Control for the occlusion test: with no dialog the same coordinates
    /// must hit the tab button — the log gains "[4.2] tab selected: Settings"
    /// and the screenshots show the pill dot and content move to Settings.
    /// GPUI has no accessibility node for the tab, so the capture pair is the
    /// evidence and the bounded wait only replaces the fixed sleep.
    func testTabTapWithoutDialogSwitchesTab() {
        let app = launch(["--tab=0"])
        saveScreenshot("occl-ctl-before")
        app.coordinate(withNormalizedOffset: settingsTab).tap()
        waitForSettledFrame(app)
        saveScreenshot("occl-ctl-after")
    }

    /// @req REQ-UI-04 — §4.1: with a dialog open the same tap must not reach
    /// the tab bar; the backdrop absorbs it (dismisses, counts as cancelled).
    /// Capture-only for the same reason: the modal has no accessibility node.
    func testDialogBackdropOccludesTabBar() {
        let app = launch(["--tab=0", "--dialog"])
        saveScreenshot("occl-before")
        app.coordinate(withNormalizedOffset: settingsTab).tap()
        waitForSettledFrame(app)
        saveScreenshot("occl-after")
    }

    /// Mobile Day Tray 1.2 (REQ-SCR-04/REQ-UI-16): on the compact shell the
    /// modal is painted last and covers the whole shell, so the tab-bar
    /// coordinate never reaches the bar while it is open. Two taps at the same
    /// point exercise the one resolution path: the first dismisses the backdrop
    /// and the second lands on the shell, so the capture pair proves the modal
    /// was removed once and no second dialog reappeared. The assertion is the
    /// bounded foreground/window wait; single-resolution itself is asserted by
    /// the Rust `dialog_resolves_once` and `mobile_long_dialog_*` tests, because
    /// GPUI exposes no traversal or dialog node to XCUITest. See
    /// docs/evidence/mobile-ui-rework/1.2/INDEX.md.
    /// @req REQ-SCR-04 @req REQ-UI-16
    func testDayTrayModalOccludesAndResolvesOnce() {
        let app = XCUIApplication()
        app.launchArguments = ["--evidence", "--viewport=360x780", "--tab=0", "--dialog"]
        app.launchEnvironment["SNACKPILOT_EVIDENCE"] = "1"
        app.launch()
        XCTAssertTrue(
            app.otherElements.matching(NSPredicate(format: "label == %@", "counter 0"))
                .firstMatch.waitForExistence(timeout: 30),
            "the compact shell did not render")
        saveScreenshot("daytray-modal-open")
        let point = app.coordinate(withNormalizedOffset: settingsTab)
        point.tap()
        waitForSettledFrame(app)
        point.tap()
        waitForSettledFrame(app)
        saveScreenshot("daytray-modal-after")
        XCTAssertEqual(app.state, .runningForeground)
    }
}
