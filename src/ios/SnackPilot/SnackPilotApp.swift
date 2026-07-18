import SwiftUI

@main
struct SnackPilotApp: App {
    @StateObject private var model = AppModel()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView().environmentObject(model)
        }
        // Returning to the foreground pulls in changes made on another device (orders, billing).
        .onChange(of: scenePhase) {
            if scenePhase == .active { Task { await model.refreshOnForeground() } }
        }
    }
}
