import Foundation
import UserNotifications

/// Executes the core's wire commands via `UNUserNotificationCenter`
/// (05-platform-services §3). The core decides what/when; this shell delivers.
/// iOS has no channels — the `channelId` (Android-only) is ignored.
///
/// Ported from v1 `src/ios/SnackPilot/NotificationService.swift` (task 3.5); the
/// uniffi `NotificationCommand` is replaced by the plain `CommandData` the C
/// shim table hands over.
struct CommandData {
    let kind: Int32
    let id: String
    let title: String
    let body: String
    let channelId: String
    let fireAtEpochMs: Int64
}

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

    /// Request notification permission and report whether it was granted (location flow §7.2).
    func requestPermissionGranted() async -> Bool {
        (try? await center.requestAuthorization(options: [.alert, .sound, .badge])) ?? false
    }

    func execute(_ command: CommandData) {
        let nowMs = Date().timeIntervalSince1970 * 1000
        switch command.kind {
        case 0, 1:
            add(Self.request(id: command.id, title: command.title, body: command.body,
                             after: Self.delay(for: command, nowMs: nowMs)))
        case 2:
            remove([command.id])
        default:
            break
        }
    }

    /// Seconds until a command should fire. Kind 0 (fire-now) is immediate
    /// (0 → triggerless request); kind 1 (schedule-at) keeps a 1 s floor
    /// (a time-interval trigger needs ≥1 s).
    static func delay(for command: CommandData, nowMs: Double) -> TimeInterval {
        guard command.kind == 1 else { return 0 }
        return max(1, (Double(command.fireAtEpochMs) - nowMs) / 1000)
    }

    /// Build the request: no trigger when `after <= 0` so it shows immediately (background
    /// task / fire-now while the process is alive), otherwise a one-shot time-interval trigger.
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

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        completionHandler()
    }
}
