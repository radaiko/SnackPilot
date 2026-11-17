import BackgroundTasks
import Foundation

/// The OS background half of the notification pipeline (notifications-daily-reminder
/// §2, notifications-location §3): one app-refresh task that re-evaluates the
/// reminders from the cached orders and runs the new-menu check while the app is
/// closed. The decision is the core's; this only asks the OS for the time and
/// hands it to Rust.
///
/// Registered at launch (a BGTask identifier must be known before the app
/// finishes launching) and re-submitted after every run — iOS keeps at most one
/// pending request per identifier, so submitting again replaces the old one.
enum BackgroundSync {
    static let refreshId = "dev.radaiko.gourmetclient.sync"

    static func register() {
        BGTaskScheduler.shared.register(forTaskWithIdentifier: refreshId, using: nil) { task in
            handle(task)
        }
    }

    /// Ask for the next run, no sooner than 15 minutes from now (the interval v1
    /// used). Best effort: the OS decides when it actually runs.
    static func schedule() {
        let request = BGAppRefreshTaskRequest(identifier: refreshId)
        request.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60)
        do {
            try BGTaskScheduler.shared.submit(request)
        } catch {
            NSLog("background sync: submit failed: \(error)")
        }
    }

    private static func handle(_ task: BGTask) {
        schedule()
        let finished = NSLock()
        var didFinish = false
        func finish(_ success: Bool) {
            finished.lock()
            defer { finished.unlock() }
            if didFinish { return }
            didFinish = true
            task.setTaskCompleted(success: success)
        }
        task.expirationHandler = { finish(false) }
        DispatchQueue.global(qos: .utility).async {
            snackpilot_ios_run_background_tick()
            finish(true)
        }
    }
}
