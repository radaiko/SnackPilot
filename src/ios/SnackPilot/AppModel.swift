import Foundation
import SwiftUI

/// Thin SwiftUI shell state over the Rust core (`SnackPilotCore`). The core owns all
/// scraping, caching and domain logic; this object just injects a storage directory,
/// forwards credentials, and republishes the records the core returns.
@MainActor
final class AppModel: ObservableObject {
    let core: SnackPilotCore

    @Published var userInfo: GourmetUserInfo?
    @Published var snapshot: MenuSnapshot?
    @Published var errorText: String?
    @Published var busy = false
    /// True while showing offline demo data (no live session) — lets the UI badge the state.
    @Published var demoMode = false

    /// Crate version — a cheap end-to-end proof the FFI is wired.
    let coreVersion: String = SnackPilot.coreVersion()

    init() {
        let base = FileManager.default
            .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("snackpilot", isDirectory: true)
        try? FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        // Constructing the core cannot fail in normal operation (only a transport-build
        // error, which would mean a broken binary); surface it loudly if it ever does.
        do {
            core = try SnackPilotCore(config: CoreConfig(storageDir: base.path), analytics: nil)
        } catch {
            fatalError("SnackPilotCore init failed: \(error)")
        }
        #if DEBUG
        // UI-test / preview hook: jump straight into offline demo data. Never compiled into
        // release builds, and only ever renders canned data (no network).
        if ProcessInfo.processInfo.arguments.contains("-uiTestDemo") {
            loadDemo()
        }
        #endif
    }

    func isDemoCredentials(user: String, pass: String) -> Bool {
        SnackPilot.isDemoCredentials(username: user, password: pass)
    }

    /// Real Gourmet login + first menu fetch. Never call with demo credentials — those must
    /// never hit the live server (see `loadDemo`).
    func login(user: String, pass: String) async {
        busy = true
        errorText = nil
        do {
            userInfo = try await core.gourmetLogin(creds: Credentials(username: user, password: pass))
            snapshot = try await core.fetchMenus(force: false)
            demoMode = false
        } catch {
            errorText = String(describing: error)
        }
        busy = false
    }

    /// Render the canned demo menus offline — no network. Used both for the magic demo
    /// credentials and the on-device FFI preview.
    func loadDemo() {
        snapshot = core.demoMenuSnapshot()
        userInfo = GourmetUserInfo(
            username: "Demo", shopModelId: "", eaterId: "", staffGroupId: "")
        demoMode = true
        errorText = nil
    }

    func logout() {
        userInfo = nil
        snapshot = nil
        demoMode = false
    }
}
