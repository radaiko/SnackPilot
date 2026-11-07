//! Company-location and geofence flows, ported from the iOS `AppModel`
//! (src/ios/SnackPilot/AppModel.swift:713-784, notifications-location §7-§9).

use snackpilot_core::domain::{CompanyLocation, GeofenceEvent};

use crate::state::Alert;
use crate::AppModel;

impl AppModel {
    /// Setup flow (§7, AppModel.swift:716-766): request "Always" location, then
    /// notifications, then capture a one-shot fix → persist in core → start the
    /// geofence. Each failure aborts with the exact v1 German alert and stores
    /// nothing (REQ-SCR-08, REQ-PLT-02, REQ-PLT-03).
    pub fn set_company_location_from_current_position(&self) {
        self.hub.update("location_busy", |s| s.location_busy = true);

        // 1. Region monitoring needs "Always" (§7.1).
        if !self
            .platform
            .request_location_authorisation()
            .unwrap_or(false)
        {
            self.show_location_alert(Alert::location_required());
            return;
        }

        // 2. Notifications (§7.2).
        if !self
            .platform
            .request_notification_permission()
            .unwrap_or(false)
        {
            self.show_location_alert(Alert::denied());
            return;
        }

        // 3. One-shot fix → persist → geofence (§7.3).
        let fix = match self.platform.current_location() {
            Ok(Some(f)) => f,
            _ => {
                self.show_location_alert(Alert::error());
                return;
            }
        };
        self.core.set_company_location(fix.latitude, fix.longitude);
        let saved = CompanyLocation {
            latitude: fix.latitude,
            longitude: fix.longitude,
        };
        self.hub.update("company_location", |s| {
            s.company_location = Some(saved);
        });
        let _ = self.platform.start_geofence();
        self.show_location_alert(Alert::saved());
    }

    /// "Standort entfernen" (§8, AppModel.swift:768-773): stop the geofence and
    /// clear the saved location, which also resets core's `is_at_company` (§1).
    /// No confirmation dialog, matching v1 (REQ-PLT-03).
    pub fn clear_company_location(&self) {
        let _ = self.platform.stop_geofence();
        self.core.clear_company_location();
        self.hub.update("company_location_cleared", |s| {
            s.company_location = None;
        });
    }

    /// Region Enter/Exit handler (§4, AppModel.swift:771-784): record the flag,
    /// deliver whatever core decides, refresh the diagnostic log (REQ-PLT-03).
    pub fn handle_geofence(&self, event: GeofenceEvent) {
        let inside = event == GeofenceEvent::Enter;
        self.core.set_is_at_company(inside);
        self.hub.update("is_at_company", |s| {
            s.is_at_company = inside;
        });
        for cmd in self.core.geofence_commands(event) {
            let _ = self.platform.notify(cmd);
        }
        self.refresh_diagnostic_log();
    }

    fn show_location_alert(&self, alert: Alert) {
        self.hub.update("location_alert", |s| {
            s.location_alert = Some(alert);
            s.location_busy = false;
        });
    }
}
