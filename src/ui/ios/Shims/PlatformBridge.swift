import Aptabase
import CoreLocation
import Foundation
import UIKit

/// Bridges the C shim table (Host/PlatformShims.h) into the Rust core.
///
/// Each `c*` function below is a non-capturing top-level function so it converts
/// implicitly to the `@convention(c)` pointer the table expects. Synchronous ops
/// return 1/0 directly; asynchronous ones (permission prompts, one-shot fixes)
/// park a Rust request and call `snackpilot_ios_complete` when done.
///
/// The Swift side stays a dumb shell (design D4): no decisions, no retry policy
/// beyond v1's icon schedule, no logging of values.

private func cstr(_ pointer: UnsafePointer<CChar>?) -> String {
    guard let pointer else { return "" }
    return String(cString: pointer)
}

/// Completes a pending Rust request. The payload is read synchronously inside
/// the call, so the transient C string is safe.
private func complete(_ id: UInt64, ok: Bool, payload: String? = nil) {
    if let payload {
        payload.withCString { snackpilot_ios_complete(id, ok ? 1 : 0, $0) }
    } else {
        snackpilot_ios_complete(id, ok ? 1 : 0, nil)
    }
}

// MARK: - Shim implementations (one per Platform capability)

private func cKeychainSet(_ key: UnsafePointer<CChar>?, _ value: UnsafePointer<CChar>?) -> UInt8 {
    KeychainStore.set(key: cstr(key), value: cstr(value)) ? 1 : 0
}

private func cKeychainGet(_ key: UnsafePointer<CChar>?, _ outValue: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> UInt8 {
    guard let value = KeychainStore.get(key: cstr(key)), let outValue else { return 0 }
    outValue.pointee = strdup(value)  // Rust frees with the C free()
    return 1
}

private func cKeychainDelete(_ key: UnsafePointer<CChar>?) -> UInt8 {
    KeychainStore.delete(key: cstr(key)) ? 1 : 0
}

private func cNotify(_ command: UnsafePointer<SnackPilotWireCommand>?) -> UInt8 {
    guard let command else { return 0 }
    let c = command.pointee
    NotificationService.shared.execute(CommandData(
        kind: c.kind, id: cstr(c.id), title: cstr(c.title), body: cstr(c.body),
        channelId: cstr(c.channel_id), fireAtEpochMs: c.fire_at_epoch_ms))
    return 1
}

private func cNotifyPermission(_ requestId: UInt64, _ ctx: UnsafeMutableRawPointer?) -> UInt8 {
    Task {
        let granted = await NotificationService.shared.requestPermissionGranted()
        complete(requestId, ok: true, payload: granted ? "true" : "false")
    }
    return 1
}

private func cLocationAuthorisation(_ requestId: UInt64, _ ctx: UnsafeMutableRawPointer?) -> UInt8 {
    Task {
        let status = await LocationService.shared.requestAlwaysAuthorization()
        let granted = status == .authorizedAlways || status == .authorizedWhenInUse
        complete(requestId, ok: true, payload: granted ? "true" : "false")
    }
    return 1
}

private func cLocationCurrent(_ requestId: UInt64, _ ctx: UnsafeMutableRawPointer?) -> UInt8 {
    Task {
        do {
            let fix = try await LocationService.shared.requestOneShotLocation()
            complete(requestId, ok: true,
                     payload: "\(fix.coordinate.latitude),\(fix.coordinate.longitude)")
        } catch {
            complete(requestId, ok: false, payload: "location unavailable")
        }
    }
    return 1
}

private func cGeofenceStart(_ latitude: Double, _ longitude: Double,
                            _ requestId: UInt64, _ ctx: UnsafeMutableRawPointer?) -> UInt8 {
    // Setup path: synthesize the initial Enter when the device is already inside (§3.3).
    // The already-monitored guard keeps relaunches silent (§3.2).
    LocationService.shared.startMonitoring(latitude: latitude, longitude: longitude,
                                            requestInitialState: true)
    complete(requestId, ok: true)
    return 1
}

private func cGeofenceStop() -> UInt8 {
    LocationService.shared.stopMonitoring()
    return 1
}

private func cGeofenceIsMonitoring() -> UInt8 {
    LocationService.shared.isMonitoringCompany ? 1 : 0
}

private struct TrackEvent: Decodable {
    let event: String
    let props: [String: String]?
}

private func cTrack(_ eventJson: UnsafePointer<CChar>?) -> UInt8 {
    guard let data = cstr(eventJson).data(using: .utf8),
          let parsed = try? JSONDecoder().decode(TrackEvent.self, from: data) else { return 0 }
    Analytics.track(parsed.event, parsed.props ?? [:])
    return 1
}

private func cOpenEmail(_ to: UnsafePointer<CChar>?, _ subject: UnsafePointer<CChar>?,
                        _ body: UnsafePointer<CChar>?) -> UInt8 {
    let recipient = cstr(to)
    guard !recipient.isEmpty else { return 0 }
    var components = URLComponents()
    components.scheme = "mailto"
    components.path = recipient
    var items: [URLQueryItem] = []
    let subjectValue = cstr(subject)
    if !subjectValue.isEmpty { items.append(URLQueryItem(name: "subject", value: subjectValue)) }
    let bodyValue = cstr(body)
    if !bodyValue.isEmpty { items.append(URLQueryItem(name: "body", value: bodyValue)) }
    if !items.isEmpty { components.queryItems = items }
    guard let url = components.url else { return 0 }
    DispatchQueue.main.async { UIApplication.shared.open(url) }
    return 1
}

private func cOpenSettings() -> UInt8 {
    guard let url = URL(string: UIApplication.openSettingsURLString) else { return 0 }
    DispatchQueue.main.async { UIApplication.shared.open(url) }
    return 1
}

private func cOpenAppSettings() -> UInt8 {
    cOpenSettings()
}

/// v1 `AppIconService` retry schedule (0.5 → 16 s, up to 5 retries; §6.2).
/// Errors after the final retry are swallowed — the accent change always
/// succeeds even if icon switching is unavailable (§6.1).
private enum AppIconBridge {
    static func attempt(_ name: String?, delay: TimeInterval, retriesLeft: Int) {
        DispatchQueue.main.asyncAfter(deadline: .now() + delay) {
            UIApplication.shared.setAlternateIconName(name) { error in
                guard error != nil, retriesLeft > 0 else { return }
                attempt(name, delay: delay * 2, retriesLeft: retriesLeft - 1)
            }
        }
    }
}

private func cSetAppIcon(_ icon: UnsafePointer<CChar>?) -> UInt8 {
    let raw = cstr(icon)
    guard !raw.isEmpty else { return 0 }
    // Orange is the primary icon; the others are alternates AppIcon-<accent>.
    let target: String? = raw == "orange" ? nil : "AppIcon-\(raw)"
    DispatchQueue.main.async {
        let app = UIApplication.shared
        guard app.supportsAlternateIcons, app.alternateIconName != target else { return }
        AppIconBridge.attempt(target, delay: 0.5, retriesLeft: 5)
    }
    return 1
}

// MARK: - Appearance

/// Live UIKit interface style for the Rust shell (REQ-UI-05). UIKit reads are
/// main-thread-only, so a call from elsewhere hops to the main queue; nothing
/// is cached because the shell repaints continuously and reads this every
/// frame. The window scene's trait collection follows the system appearance,
/// unlike the host controller, which pins its own style.
@_cdecl("snackpilot_ios_is_dark")
public func snackpilotIosIsDark() -> UInt8 {
    let read: () -> UInt8 = {
        let style = (UIApplication.shared.connectedScenes.first as? UIWindowScene)?
            .traitCollection.userInterfaceStyle ?? .light
        return style == .dark ? 1 : 0
    }
    if Thread.isMainThread { return read() }
    return DispatchQueue.main.sync(execute: read)
}

// MARK: - Bootstrap

enum PlatformBridge {
    private static var didBootstrap = false

    /// Registers the shim table with the Rust core. Call once at launch, before
    /// any Rust code runs.
    static func bootstrap() {
        guard !didBootstrap else { return }
        didBootstrap = true

        Analytics.start()
        NotificationService.shared.configure()
        // Touch the singleton so its delegate receives region crossings even
        // when iOS relaunches the app in the background (§3).
        LocationService.shared.onRegionEvent = { inside in
            snackpilot_ios_on_geofence_event(inside ? 1 : 0)
        }

        let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("snackpilot", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)

        var table = SnackPilotIosShimTable(
            size: 17,
            keychain_set: cKeychainSet,
            keychain_get: cKeychainGet,
            keychain_delete: cKeychainDelete,
            notify: cNotify,
            notify_permission: cNotifyPermission,
            location_authorisation: cLocationAuthorisation,
            location_current: cLocationCurrent,
            geofence_start: cGeofenceStart,
            geofence_stop: cGeofenceStop,
            geofence_is_monitoring: cGeofenceIsMonitoring,
            track: cTrack,
            open_email: cOpenEmail,
            open_settings: cOpenSettings,
            open_app_settings: cOpenAppSettings,
            set_app_icon: cSetAppIcon,
            is_dark: snackpilotIosIsDark)
        dir.path.withCString { _ = snackpilot_ios_register_shims(&table, $0) }
    }
}
