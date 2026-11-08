import XCTest

// Task 5.4 evidence: the Einstellungen rows open the parametrised login forms
// and the text fields accept typing with the soft keyboard (§3.5/§3.6).
// GPUI has no accessibility tree, so this drives raw coordinate taps like
// SpikeTapTests and saves screenshots (SPIKE_EVIDENCE_DIR).
// @req REQ-SCR-05 @req REQ-SCR-06

final class LoginTypingTests: XCTestCase {
    private let firstRow = CGVector(dx: 0.5, dy: 0.24)
    private let usernameField = CGVector(dx: 0.5, dy: 0.22)
    private let passwordField = CGVector(dx: 0.5, dy: 0.29)

    private func saveScreenshot(_ name: String) {
        let dir = ProcessInfo.processInfo.environment["SPIKE_EVIDENCE_DIR"] ?? "/tmp/ev54"
        let data = XCUIScreen.main.screenshot().pngRepresentation
        try? data.write(to: URL(fileURLWithPath: "\(dir)/\(name).png"))
    }

    func testSettingsRowsOpenLoginAndFieldsAcceptTyping() {
        let app = XCUIApplication()
        app.launchArguments = ["--tab=3"]
        app.launch()
        sleep(4)
        saveScreenshot("a-settings")

        // Tap "Kantine-Zugangsdaten" -> pushed login form.
        app.coordinate(withNormalizedOffset: firstRow).tap()
        sleep(2)
        saveScreenshot("b-login-empty")

        // Username: tap the field and type.
        app.coordinate(withNormalizedOffset: usernameField).tap()
        sleep(1)
        app.typeText("demo")
        sleep(1)
        saveScreenshot("c-username-typed")

        // Password: tap the field and type the demo password.
        app.coordinate(withNormalizedOffset: passwordField).tap()
        sleep(1)
        app.typeText("demo1234!")
        sleep(1)
        saveScreenshot("d-password-typed")

        // Backspace once to prove deletion works.
        app.typeText(XCUIKeyboardKey.delete.rawValue)
        sleep(1)
        saveScreenshot("e-password-after-backspace")
    }
}
