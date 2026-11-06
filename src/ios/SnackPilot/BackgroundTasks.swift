import Foundation
import BackgroundTasks

/// Background execution (docs/architecture §4.4, notifications-new-menu §3): a single
/// BGAppRefreshTask that runs the new-menu scrape + the order-driven reminder checks and
/// delivers their commands. iOS decides *when* to run it; we only request one per
/// foreground session and handle launches. The identifier is v2-owned and must match
/// `BGTaskSchedulerPermittedIdentifiers` in project.yml.
enum BackgroundTasks {
    static let refreshIdentifier = "dev.radaiko.gourmetclient.menu-refresh"

    /// Shared single-flight gate (REQ-NTF-13): at most one menu scrape at a time across the
    /// background task, the manual Diagnose button and the foreground menu check. A second
    /// concurrent run returns immediately without any request — piling up scrapes is
    /// bot-like traffic against Gourmet (ban rules).
    @MainActor static var isRunning = false

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
        do {
            try BGTaskScheduler.shared.submit(request)
        } catch {
            // A silently dropped request means the task never runs again; make it visible.
            NSLog("SnackPilot BGTask: submit failed: %@", String(describing: error))
        }
    }

    /// The one background body (design D5, REQ-NTF-13/14), shared by the BGAppRefreshTask and
    /// the manual Diagnose button. Builds nothing but its arguments: no UI model (its init logs
    /// in to Gourmet/Ventopay, fetches billing, fires the analytics launch event and prompts
    /// for permissions — all forbidden in the background per 05-platform-services §4.2).
    /// Returns false without any call when another run holds the gate.
    @MainActor
    static func runSync(
        core: SnackPilotCore,
        credentials: Credentials?,
        deliver: (NotificationCommand) -> Void
    ) async -> Bool {
        guard !isRunning else { return false }
        isRunning = true
        defer { isRunning = false }
        if let result = try? await core.runMenuCheck(creds: credentials),
           let command = result.notification {
            deliver(command)
        }
        if Task.isCancelled { return false } // expired: skip the reminder pass (D5)
        for command in core.runOrderSyncCheck() {
            deliver(command)
        }
        return true
    }

    private static func handle(_ task: BGAppRefreshTask) {
        // Queue the NEXT run up front: if this process is killed before completing, iOS still
        // has a pending request for the following opportunity.
        scheduleRefresh()
        NSLog("SnackPilot BGTask: handler invoked for %@", task.identifier)
        // Never block here: the launch handler runs on the main queue (register using: nil),
        // so a semaphore wait would wedge the very queue the @MainActor work needs (validated:
        // the sync never started). Apple's pattern — async work + expirationHandler cancels it.
        let work = Task { @MainActor in
            // Headless core over the same Application Support directory the UI uses, with
            // analytics: nil — the launch event and the sink belong to the UI process (D5).
            let base = FileManager.default
                .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
                .appendingPathComponent("snackpilot", isDirectory: true)
            try? FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
            guard let core = try? SnackPilotCore(
                config: CoreConfig(storageDir: base.path), analytics: nil
            ) else {
                NSLog("SnackPilot BGTask: core init failed")
                return false
            }
            let creds = KeychainStore.savedGourmet().map {
                Credentials(username: $0.username, password: $0.password)
            }
            return await runSync(core: core, credentials: creds) {
                NotificationService.shared.execute($0)
            }
        }
        task.expirationHandler = { work.cancel() } // time's up → stop the sync
        Task {
            // Report the real outcome: success only when the sync ran to completion — a run
            // that hit its expiration must not claim success (iOS throttles tasks that lie).
            let success = (await work.value) == true && !work.isCancelled
            NSLog("SnackPilot BGTask: completing success=%d", success)
            task.setTaskCompleted(success: success)
        }
    }
}
