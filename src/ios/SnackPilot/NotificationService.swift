import Foundation
import UserNotifications

/// Executes the core's `NotificationCommand`s via `UNUserNotificationCenter`
/// (05-platform-services §3). The core decides what/when; this shell delivers. iOS has no
/// channels — the `channelId` (Android-only) is ignored.
final class NotificationService: NSObject, UNUserNotificationCenterDelegate {
    static let shared = NotificationService()
    private let center = UNUserNotificationCenter.current()

    /// Install the foreground-presentation delegate. Call once at launch.
    func configure() {
        center.delegate = self
    }

    func requestPermission() async {
        _ = try? await center.requestAuthorization(options: [.alert, .sound, .badge])
    }

    func execute(_ command: NotificationCommand) {
        switch command {
        case let .fireNow(id, title, body, _, _):
            deliver(id: id, title: title, body: body, after: 0.1)
        case let .scheduleAt(id, title, body, _, fireAtEpochMs, _):
            let nowMs = Date().timeIntervalSince1970 * 1000
            let delaySec = max(1, (Double(fireAtEpochMs) - nowMs) / 1000)
            deliver(id: id, title: title, body: body, after: delaySec)
        case let .cancelPending(id):
            center.removePendingNotificationRequests(withIdentifiers: [id])
            center.removeDeliveredNotifications(withIdentifiers: [id])
        }
    }

    private func deliver(id: String, title: String, body: String, after: TimeInterval) {
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        content.sound = .default
        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: after, repeats: false)
        center.add(UNNotificationRequest(identifier: id, content: content, trigger: trigger))
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
