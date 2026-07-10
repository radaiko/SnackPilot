import CoreLocation
import Foundation

/// CoreLocation wrapper for the single company geofence (notifications-location §2–§4). The core
/// decides *what* to notify; this shell registers the 500 m region and reports Enter/Exit back up.
///
/// A singleton created at app launch so its `CLLocationManager` delegate is in place to receive
/// region crossings even when iOS relaunches the app in the background (§3). Device-only: region
/// monitoring does not fire reliably in the Simulator.
final class LocationService: NSObject, CLLocationManagerDelegate {
    static let shared = LocationService()

    private let manager = CLLocationManager()
    private static let regionId = "company"
    private static let radiusM = 500.0

    /// Invoked on region Enter/Exit — including the initial Enter that fires when monitoring
    /// starts while the device is already inside the zone (§3.3).
    var onRegionEvent: ((GeofenceEvent) -> Void)?

    private var authContinuation: CheckedContinuation<CLAuthorizationStatus, Never>?
    private var locationContinuation: CheckedContinuation<CLLocation, Error>?
    /// Set on the setup path so the one-shot `requestState` result fires the initial Enter (§3.3);
    /// cleared after the first determination so the launch-restore path never re-fires (§3.2).
    private var pendingInitialStateRegionId: String?

    override init() {
        super.init()
        manager.delegate = self
        manager.desiredAccuracy = kCLLocationAccuracyBest
    }

    var authorizationStatus: CLAuthorizationStatus { manager.authorizationStatus }

    /// Request "Always" authorization (region monitoring requires it) and await the decision.
    /// When status is undetermined, iOS shows the When-In-Use prompt first — the resulting
    /// `.authorizedWhenInUse` is the caller's cue to send the user to Settings for "Immer" (§7.1).
    func requestAlwaysAuthorization() async -> CLAuthorizationStatus {
        let status = manager.authorizationStatus
        // iOS only shows a prompt — and only then fires `locationManagerDidChangeAuthorization` —
        // when the status is `.notDetermined`. For any already-resolved status (`.authorizedAlways`,
        // `.authorizedWhenInUse`, `.denied`, `.restricted`) the call is a silent no-op with no
        // delegate callback, so parking a continuation would suspend forever (button stuck on
        // "wird ermittelt …"). Return the current status synchronously instead.
        guard status == .notDetermined else { return status }
        return await withCheckedContinuation { cont in
            authContinuation = cont
            manager.requestAlwaysAuthorization()
        }
    }

    /// One-shot high-accuracy GPS fix (§7.3).
    func requestOneShotLocation() async throws -> CLLocation {
        try await withCheckedThrowingContinuation { cont in
            locationContinuation = cont
            manager.requestLocation()
        }
    }

    /// Whether the single company region is currently being monitored.
    var isMonitoringCompany: Bool {
        manager.monitoredRegions.contains { $0.identifier == Self.regionId }
    }

    /// Start monitoring the single 500 m region (§2–§3). No-op if it is already monitored, so it
    /// does not re-fire the initial Enter and spam notifications (§3.2). When [requestInitialState]
    /// is true (the setup path, §3.3), it asks CoreLocation for the current region state — unlike
    /// Android's INITIAL_TRIGGER_ENTER, `startMonitoring(for:)` does NOT synthesize an Enter for a
    /// device already inside, so we synthesize it via `requestState`/`didDetermineState`.
    func startMonitoring(latitude: Double, longitude: Double, requestInitialState: Bool = false) {
        guard CLLocationManager.isMonitoringAvailable(for: CLCircularRegion.self) else { return }
        if isMonitoringCompany { return }
        let region = CLCircularRegion(
            center: CLLocationCoordinate2D(latitude: latitude, longitude: longitude),
            radius: Self.radiusM, identifier: Self.regionId)
        region.notifyOnEntry = true
        region.notifyOnExit = true
        manager.startMonitoring(for: region)
        if requestInitialState {
            pendingInitialStateRegionId = Self.regionId
            manager.requestState(for: region)
        }
    }

    /// Stop monitoring the company region if active (§3, `stopGeofencing`).
    func stopMonitoring() {
        for region in manager.monitoredRegions where region.identifier == Self.regionId {
            manager.stopMonitoring(for: region)
        }
    }

    // MARK: CLLocationManagerDelegate

    func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        authContinuation?.resume(returning: manager.authorizationStatus)
        authContinuation = nil
    }

    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        guard let loc = locations.last else { return }
        locationContinuation?.resume(returning: loc)
        locationContinuation = nil
    }

    func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        locationContinuation?.resume(throwing: error)
        locationContinuation = nil
    }

    /// One-shot state check requested only on the setup path (§3.3): if the device is already
    /// inside the region when the location is set, synthesize the initial Enter.
    func locationManager(_ manager: CLLocationManager, didDetermineState state: CLRegionState,
                         for region: CLRegion) {
        guard pendingInitialStateRegionId == region.identifier else { return }
        pendingInitialStateRegionId = nil
        if state == .inside { onRegionEvent?(.enter) }
    }

    func locationManager(_ manager: CLLocationManager, didEnterRegion region: CLRegion) {
        guard region.identifier == Self.regionId else { return }
        onRegionEvent?(.enter)
    }

    func locationManager(_ manager: CLLocationManager, didExitRegion region: CLRegion) {
        guard region.identifier == Self.regionId else { return }
        onRegionEvent?(.exit)
    }
}
