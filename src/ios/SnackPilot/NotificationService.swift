import Foundation
import UserNotifications

/// Executes the core's `NotificationCommand`s via `UNUserNotificationCenter`
/// (05-platform-services §3). The core decides what/when; this shell delivers. iOS has no
/// channels — the `channelId` (Android-only) is ignored.
final class NotificationService: NSObject, UNUserNotificationCenterDelegate {
    static let shared = NotificationService()
    private let center: UNUserNotificationCenter
    /// Delivery seams, injectable so tests can observe what `execute` produces without
    /// touching the real notification center.
    private let add: (UNNotificationRequest) -> Void
    private let remove: ([String]) -> Void

    init(add: ((UNNotificationRequest) -> Void)? = nil, remove: (([String]) -> Void)? = nil) {
        let center = UNUserNotificationCenter.current()
        self.center = center
        self.add = add ?? { center.add($0) }
        self.remove = remove ?? { ids in
            center.removePendingNotificationRequests(withIdentifiers: ids)
            center.removeDeliveredNotifications(withIdentifiers: ids)
        }
    }

    /// Install the foreground-presentation delegate. Call once at launch.
    func configure() {
        center.delegate = self
    }

    func requestPermission() async {
        _ = try? await center.requestAuthorization(options: [.alert, .sound, .badge])
    }

    /// Request notification permission and report whether it was granted (location flow §7.2).
    func requestPermissionGranted() async -> Bool {
        (try? await center.requestAuthorization(options: [.alert, .sound, .badge])) ?? false
    }

    func execute(_ command: NotificationCommand) {
        let nowMs = Date().timeIntervalSince1970 * 1000
        switch command {
        case let .fireNow(id, title, body, _, _):
            add(Self.request(id: id, title: title, body: body,
                             after: Self.delay(for: command, nowMs: nowMs)))
        case let .scheduleAt(id, title, body, _, _, _):
            add(Self.request(id: id, title: title, body: body,
                             after: Self.delay(for: command, nowMs: nowMs)))
        case let .cancelPending(id):
            remove([id])
        }
    }

    /// Seconds until a command should fire. FireNow is immediate (0 → triggerless request);
    /// ScheduleAt keeps a 1 s floor (a time-interval trigger needs ≥1 s).
    static func delay(for command: NotificationCommand, nowMs: Double) -> TimeInterval {
        switch command {
        case .fireNow, .cancelPending:
            return 0
        case let .scheduleAt(_, _, _, _, fireAtEpochMs, _):
            return max(1, (Double(fireAtEpochMs) - nowMs) / 1000)
        }
    }

    /// Build the request: no trigger when `after <= 0` so it shows immediately (background
    /// task / FireNow while the process is alive), otherwise a one-shot time-interval trigger.
    static func request(id: String, title: String, body: String, after: TimeInterval) -> UNNotificationRequest {
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        content.sound = .default
        let trigger: UNNotificationTrigger? = after > 0
            ? UNTimeIntervalNotificationTrigger(timeInterval: after, repeats: false)
            : nil
        return UNNotificationRequest(identifier: id, content: content, trigger: trigger)
    }

    /// Show new-menu / reminder alerts even while the app is foregrounded.
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .sound, .list])
    }
}
