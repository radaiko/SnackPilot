import Aptabase

/// Self-hosted Aptabase analytics (see the vault doc "Snackpilot - Aptabase Self-Host").
///
/// Anonymous by design — no user IDs, device tokens, or IPs are attached (GDPR Recital 26, so no
/// consent banner). The app key is NOT a secret: it ships in the binary and is extractable; the
/// worst case is event spoofing, never data leakage. Analytics is strictly fire-and-forget — a
/// send failure must never crash or block the app (analytics.md).
///
/// Ported verbatim from v1 `src/ios/SnackPilot/Analytics.swift` (task 3.5); the uniffi
/// `AptabaseSink` is gone — the C `track` shim forwards here instead.
enum Analytics {
    private static let appKey = "A-SH-1116712922"
    // Funnel base URL — the SDK appends `/api/v0/events`, so this must NOT include `/api/v0`.
    private static let host = "https://hetzner-server-1.ibex-dory.ts.net"

    /// Initialize the SDK once, before the core is constructed.
    static func start() {
        Aptabase.shared.initialize(appKey: appKey, with: InitOptions(host: host))
    }

    static func track(_ event: String, _ props: [String: String] = [:]) {
        Aptabase.shared.trackEvent(event, with: props)
    }
}
