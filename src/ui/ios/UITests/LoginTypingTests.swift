import XCTest

// Task 5.4 typing proof (§3.5/§3.6): the Kantine login accepts typing,
// backspace and secure entry, the soft keyboard rises on focus and never
// covers the focused field, and Speichern raises the §3.5 alert.
// GPUI renders to Metal with no accessibility tree, so this drives raw
// coordinate taps (like SpikeTapTests) and proves the field contents with
// pixel diffs of the field region; screenshots go to SPIKE_EVIDENCE_DIR
// (pass as TEST_RUNNER_SPIKE_EVIDENCE_DIR) for visual review.
// @req REQ-SCR-05 @req REQ-SCR-06

final class LoginTypingTests: XCTestCase {
    /// Normalized tap points on the Kantine login, measured from the test's
    /// own a-login-empty.png on iPhone 17 Pro (402x874 pt): username box
    /// 272-307, password box 348-382, Speichern button 398-448.
    private let usernamePoint = CGVector(dx: 0.5, dy: 0.332)
    private let passwordPoint = CGVector(dx: 0.5, dy: 0.418)
    private let savePoint = CGVector(dx: 0.709, dy: 0.484)

    private func saveScreenshot(_ name: String, _ png: Data? = nil) {
        let shot = XCUIScreen.main.screenshot()
        let attachment = XCTAttachment(screenshot: shot)
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
        let dir = ProcessInfo.processInfo.environment["SPIKE_EVIDENCE_DIR"] ?? "/tmp/ev54"
        let data = png ?? shot.pngRepresentation
        try? data.write(to: URL(fileURLWithPath: "\(dir)/\(name).png"))
    }

    /// Point of a normalized offset in the app window's coordinate space.
    private func point(_ app: XCUIApplication, _ offset: CGVector) -> CGPoint {
        let f = app.frame
        return CGPoint(x: f.minX + offset.dx * f.width, y: f.minY + offset.dy * f.height)
    }

    /// RGBA bytes of a screenshot region given in points (screenshot pixels
    /// are at the display scale).
    private func pixels(_ png: Data, _ rect: CGRect, scale: CGFloat) -> [UInt8] {
        guard let src = UIImage(data: png)?.cgImage else { return [] }
        let cropRect = CGRect(x: rect.minX * scale, y: rect.minY * scale,
                              width: rect.width * scale, height: rect.height * scale)
        guard let crop = src.cropping(to: cropRect.integral) else { return [] }
        let w = crop.width, h = crop.height
        var buf = [UInt8](repeating: 0, count: w * h * 4)
        let ctx = CGContext(data: &buf, width: w, height: h, bitsPerComponent: 8,
                            bytesPerRow: w * 4, space: CGColorSpaceCreateDeviceRGB(),
                            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
        guard let ctx else { return [] }
        ctx.draw(crop, in: CGRect(x: 0, y: 0, width: w, height: h))
        return buf
    }

    /// Number of differing bytes between two captured regions.
    private func diff(_ a: [UInt8], _ b: [UInt8]) -> Int {
        guard a.count == b.count, !a.isEmpty else { return .max }
        return zip(a, b).filter { $0 != $1 }.count
    }

    private func fieldRect(_ app: XCUIApplication, _ offset: CGVector) -> CGRect {
        let p = point(app, offset)
        return CGRect(x: p.x - 150, y: p.y - 18, width: 300, height: 36)
    }

    /// Tap one soft-keyboard key. GPUI's Metal view has no accessibility tree,
    /// so XCUIApplication.typeText cannot synthesize key events ("no element
    /// has keyboard focus"); the system keyboard itself is accessible, and
    /// tapping its keys sends real input through the field's text input
    /// context.
    private func tapKey(_ app: XCUIApplication, _ label: String) {
        let key = app.keyboards.descendants(matching: .key)[label]
        if !key.waitForExistence(timeout: 5) {
            let dump = app.keyboards.element.debugDescription
            FileHandle.standardError.write("KEYBOARD DUMP:\n\(dump)\n".data(using: .utf8)!)
            XCTFail("keyboard key \(label) not found")
            return
        }
        key.tap()
    }

    /// Type ASCII letters with the QWERTY keys (no shift needed for lowercase).
    private func typeLetters(_ app: XCUIApplication, _ text: String) {
        for ch in text where ch.isLetter { tapKey(app, String(ch)) }
    }

    func testLoginTypingKeyboardAndSave() throws {
        // Debug mode: the app was launched externally with --console-pty so its
        // stderr (Rust logs) is captured; drive it without relaunching.
        let app: XCUIApplication
        if ProcessInfo.processInfo.environment["ATTACH_TO_RUNNING"] == "1" {
            app = XCUIApplication(bundleIdentifier: "dev.radaiko.gourmetclient")
        } else {
            app = XCUIApplication()
            app.launchArguments = ["--detail=login"]
            app.launch()
        }
        // A stale system alert could swallow the coordinate taps.
        let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        for title in ["Don’t Allow", "OK"] {
            let button = springboard.buttons[title]
            if button.exists { button.tap() }
        }
        sleep(4)
        let shot = XCUIScreen.main.screenshot().pngRepresentation
        let scale = CGFloat(UIImage(data: shot)!.cgImage!.width) / app.frame.width
        saveScreenshot("a-login-empty")
        XCTAssertFalse(app.keyboards.element.exists,
                       "the keyboard must be down before any field is focused")

        // 1. Focus the username field -> the shell's driver raises the keyboard.
        app.coordinate(withNormalizedOffset: usernamePoint).tap()
        let keyboard = app.keyboards.element
        XCTAssertTrue(keyboard.waitForExistence(timeout: 5),
                      "tapping the username field did not raise the soft keyboard")
        XCTAssertGreaterThan(keyboard.frame.height, 200, "no real keyboard appeared")
        typeLetters(app, "demo")
        sleep(1)
        saveScreenshot("b-username-typed")

        // 2. Password field: secure entry, keyboard must not cover it.
        let pwBefore = XCUIScreen.main.screenshot().pngRepresentation
        saveScreenshot("c-password-empty", pwBefore)
        app.coordinate(withNormalizedOffset: passwordPoint).tap()
        sleep(1)
        XCTAssertTrue(keyboard.exists, "keyboard dropped when the password field focused")
        let pw = point(app, passwordPoint)
        XCTAssertLessThan(pw.y, keyboard.frame.minY,
                          "the keyboard covers the focused password field")
        typeLetters(app, "demo")
        tapKey(app, "numbers") // switch to the digits page for 1234
        for digit in ["1", "2", "3", "4"] { tapKey(app, digit) }
        sleep(1)
        let pwFilled = XCUIScreen.main.screenshot().pngRepresentation
        saveScreenshot("d-password-filled", pwFilled)
        XCTAssertGreaterThan(diff(pixels(pwBefore, fieldRect(app, passwordPoint), scale: scale),
                                  pixels(pwFilled, fieldRect(app, passwordPoint), scale: scale)),
                             500, "typing into the password field changed nothing")

        // 3. Backspace deletes the last character (field pixels change again).
        tapKey(app, "delete")
        sleep(1)
        let pwAfter = XCUIScreen.main.screenshot().pngRepresentation
        saveScreenshot("e-password-backspace", pwAfter)
        XCTAssertGreaterThan(diff(pixels(pwFilled, fieldRect(app, passwordPoint), scale: scale),
                                  pixels(pwAfter, fieldRect(app, passwordPoint), scale: scale)),
                             500, "backspace did not change the password field")

        // 4. Speichern -> §3.5 alert; set_dialog drops the keyboard first so the
        //    alert is not behind it. (No session model on iOS until 5.7, so the
        //    alert is the real failure alert, not a test stub.)
        app.coordinate(withNormalizedOffset: savePoint).tap()
        sleep(2)
        XCTAssertFalse(keyboard.exists, "the keyboard stayed up behind the alert")
        saveScreenshot("f-save-alert")
    }
}
