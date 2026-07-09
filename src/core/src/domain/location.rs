use serde::{Deserialize, Serialize};

/// Geofence region-monitoring event delivered by the shell (03-features/notifications-location).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeofenceEvent {
    Enter,
    Exit,
}

/// Saved office coordinates (decimal degrees). Persisted with `is_at_company` under the
/// `company-location` key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompanyLocation {
    pub latitude: f64,
    pub longitude: f64,
}
