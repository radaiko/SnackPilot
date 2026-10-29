import SwiftUI

@main
struct SnackPilotApp: App {
    @StateObject private var model = AppModel()
    @Environment(\.scenePhase) private var scenePhase

    init() {
        // Background new-menu/order-sync task (docs/architecture §4.4). The handler must be
        // registered before the app finishes launching so iOS can relaunch us for it.
        BackgroundTasks.registerLaunchHandler()
    }

    var body: some Scene {
        WindowGroup {
            RootView().environmentObject(model)
        }
        // Returning to the foreground pulls in changes made on another device (orders, billing).
        .onChange(of: scenePhase) {
            if scenePhase == .active {
                BackgroundTasks.scheduleRefresh()
                Task { await model.refreshOnForeground() }
            }
        }
    }
}
