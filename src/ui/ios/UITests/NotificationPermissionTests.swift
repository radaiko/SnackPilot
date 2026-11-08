import XCTest

// Grants the notification permission on a clean simulator (task 3.5 evidence
// runs). Unlike SpikeTapTests.dismissSystemAlerts — which answers "Don't
// Allow" first to keep the counter tests deterministic — this test must tap
// "Allow" so the UN authorization request resolves granted and the
// -DebugPlatformExercise banner checks pass unattended.

final class NotificationPermissionTests: XCTestCase {
    func testAllowNotificationPermission() {
        let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        let app = XCUIApplication()
        // The host only calls UNUserNotificationCenter.requestAuthorization
        // from the debug exercise, so launch with that flag to raise the
        // springboard dialog.
        app.launchArguments = ["-DebugPlatformExercise", "YES"]
        app.launch()

        // The springboard hosts the UN authorization dialog; the in-app alert
        // is the fallback for older runtimes.
        let allow = springboard.buttons["Allow"]
        if !allow.waitForExistence(timeout: 15) {
            let inApp = app.alerts.buttons["Allow"]
            XCTAssertTrue(inApp.waitForExistence(timeout: 5),
                          "notification permission dialog did not appear")
            inApp.tap()
            return
        }
        allow.tap()
    }
}
