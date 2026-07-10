import UIKit

/// Switches the home-screen app icon to match the accent (themes §6). Orange is the primary
/// icon; emerald/berry/golden/ocean are alternates named `AppIcon-<accent>` in the asset catalog.
/// Uses the public `setAlternateIconName` (which shows iOS's standard system alert on change).
/// Failures are swallowed — the accent change always succeeds even if icon switching is
/// unavailable (§6.1). Called ONLY from `setAccent`; there is no startup reconciliation (§6.1).
enum AppIconService {
    /// Apply the icon for the given accent. No-op if alternate icons are unsupported (§6.2
    /// "NO_SUPPORT") or the icon is already correct (avoids a redundant system alert).
    @MainActor
    static func setIcon(for accent: AccentColor) {
        let app = UIApplication.shared
        guard app.supportsAlternateIcons else { return }
        let target: String? = accent == .orange ? nil : "AppIcon-\(accent.rawValue)"
        guard app.alternateIconName != target else { return }
        attempt(target, delay: 0.5, retriesLeft: 5)
    }

    /// `setAlternateIconName` with v1's retry-on-failure schedule (0.5 → 16 s, up to 5 retries;
    /// §6.2). Errors after the final retry are swallowed.
    @MainActor
    private static func attempt(_ name: String?, delay: TimeInterval, retriesLeft: Int) {
        DispatchQueue.main.asyncAfter(deadline: .now() + delay) {
            UIApplication.shared.setAlternateIconName(name) { error in
                guard error != nil, retriesLeft > 0 else { return }
                Task { @MainActor in attempt(name, delay: delay * 2, retriesLeft: retriesLeft - 1) }
            }
        }
    }
}
