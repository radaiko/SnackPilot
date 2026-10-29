import Foundation
import BackgroundTasks

/// Background execution (docs/architecture §4.4, notifications-new-menu §3): a single
/// BGAppRefreshTask that runs the new-menu scrape + the order-driven reminder checks and
/// delivers their commands. iOS decides *when* to run it; we only request one per
/// foreground session and handle launches. The identifier is v2-owned and must match
/// `BGTaskSchedulerPermittedIdentifiers` in project.yml.
enum BackgroundTasks {
    static let refreshIdentifier = "dev.radaiko.gourmetclient.menu-refresh"

    /// Install the launch handler. Call BEFORE the app finishes launching (from the App init).
    static func registerLaunchHandler() {
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: refreshIdentifier, using: nil
        ) { task in
            guard let refresh = task as? BGAppRefreshTask else {
                task.setTaskCompleted(success: false)
                return
            }
            handle(refresh)
        }
    }

    /// Ask iOS to run the sync again after the current foreground session (call on every
    /// foreground/launch so there is always a pending request while the user is active).
    static func scheduleRefresh() {
        let request = BGAppRefreshTaskRequest(identifier: refreshIdentifier)
        // Earliest acceptable next run — a hint, not a guarantee; iOS picks the moment.
        request.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60)
        try? BGTaskScheduler.shared.submit(request)
    }

    private static func handle(_ task: BGAppRefreshTask) {
        // Queue the NEXT run up front: if this process is killed before completing, iOS still
        // has a pending request for the following opportunity.
        scheduleRefresh()
        let started = DispatchSemaphore(value: 0)
        var success = false
        // AppModel is @MainActor; hop to the main actor without blocking it (the background
        // thread waits on the semaphore instead).
        Task { @MainActor in
            // Model init wires the core (storage dir, KV, geofence handlers) exactly like a
            // normal launch; nothing here touches UI.
            let model = AppModel()
            await model.performBackgroundSync()
            success = true
            started.signal()
        }
        task.expirationHandler = { started.signal() } // time's up → complete with what we have
        started.wait()
        // Report the real outcome: success only when the sync ran to completion — a run that
        // hit its expiration must not claim success (iOS throttles tasks that lie).
        task.setTaskCompleted(success: success)
    }
}
