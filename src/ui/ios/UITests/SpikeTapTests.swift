import XCTest

// SnackPilot spike UI tests (task 2.2). GPUI renders to Metal and exposes no
// accessibility tree; the host mirrors the Rust counter into the GPUI view's
// accessibility label ("counter N"), so these tests assert the real
// tap → GPUI input → Rust state → redraw round trip. @req REQ-UI-03

final class SpikeTapTests: XCTestCase {
    /// Evidence capture for the spike handoff: when SPIKE_EVIDENCE_DIR is set
    /// (via TEST_RUNNER_SPIKE_EVIDENCE_DIR), write screen shots there. The
    /// simulator shares the Mac filesystem, so the path is host-visible.
    private func saveScreenshot(_ name: String) {
        guard let dir = ProcessInfo.processInfo.environment["SPIKE_EVIDENCE_DIR"] else { return }
        let data = XCUIScreen.main.screenshot().pngRepresentation
        try? data.write(to: URL(fileURLWithPath: "\(dir)/\(name).png"))
    }

    /// The shared simulator may show a stale system permission alert (e.g. from
    /// an older build with the same bundle id); dismiss it so it cannot cover
    /// the GPUI view or swallow the coordinate tap.
    private func dismissSystemAlerts(_ app: XCUIApplication) {
        let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        for title in ["Don’t Allow", "Don't Allow", "OK", "Allow"] {
            for query in [app.alerts.buttons, springboard.alerts.buttons, springboard.buttons] {
                let button = query[title]
                if button.exists { button.tap(); return }
            }
        }
    }

    private func launchApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launch()
        dismissSystemAlerts(app)
        // The tab line is drawn by GPUI; assert it via the host's screenshot-independent
        // route: the counter label appears once the run loop renders its first frame.
        XCTAssertTrue(waitForCounter(app, "counter 0", timeout: 30),
                      "spike root did not render (expected counter 0)")
        return app
    }

    private func counterElement(_ app: XCUIApplication, _ label: String) -> XCUIElement {
        app.otherElements.matching(NSPredicate(format: "label == %@", label)).firstMatch
    }

    private func waitForCounter(_ app: XCUIApplication, _ label: String, timeout: TimeInterval) -> Bool {
        counterElement(app, label).waitForExistence(timeout: timeout)
    }

    /// @req REQ-UI-03 — one Root, tappable: a coordinate tap on the counter
    /// button increments it.
    func testCoordinateTapIncrementsCounter() {
        let app = launchApp()
        // The button sits below the centred counter number, horizontally centred.
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.60)).tap()
        XCTAssertTrue(waitForCounter(app, "counter 1", timeout: 10),
                      "coordinate tap did not increment the counter")
        saveScreenshot("counter-1")
    }

    /// @req REQ-UI-03 — background/foreground keeps the same root: the counter
    /// value and the rendered root survive the round trip.
    func testBackgroundForegroundKeepsSameRoot() {
        let app = launchApp()
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.60)).tap()
        XCTAssertTrue(waitForCounter(app, "counter 1", timeout: 10))

        XCUIDevice.shared.press(.home)
        sleep(2)
        app.activate()
        XCTAssertTrue(waitForCounter(app, "counter 1", timeout: 30),
                      "root did not survive background/foreground")
    }
}
