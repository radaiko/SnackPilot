use crate::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

/// Geofence region-monitoring event delivered by the shell (03-features/notifications-location).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum GeofenceEvent {
    Enter,
    Exit,
}

/// Saved office coordinates (decimal degrees). Persisted with `is_at_company` under the
/// `company-location` key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct CompanyLocation {
    pub latitude: f64,
    pub longitude: f64,
}

/// Inclusive bounds of the configurable geofence reminder target, in Vienna
/// minutes-since-midnight: 06:00 … 08:45.
pub const LOCATION_REMINDER_MIN_MINUTES: i64 = 6 * 60;
pub const LOCATION_REMINDER_MAX_MINUTES: i64 = 8 * 60 + 45;
/// User-facing rejection message for a target outside the allowed window.
pub const LOCATION_REMINDER_RANGE_MESSAGE: &str =
    "Erinnerungszeit muss zwischen 06:00 und 08:45 liegen";

/// Target time for the geofence "no order" reminder, as **Europe/Vienna wall-clock**
/// (notifications-location §5).
///
/// v1 hard-coded 08:45; v2 makes it user-configurable within 06:00-08:45 inclusive
/// (feedback #1) — a deliberate, documented divergence from the v1-parity spec. The
/// default is 08:45, so an install that never touches the setting behaves exactly as before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct LocationReminderTime {
    pub hour: u8,
    pub minute: u8,
}

impl Default for LocationReminderTime {
    fn default() -> Self {
        Self {
            hour: 8,
            minute: 45,
        }
    }
}

impl LocationReminderTime {
    /// Construct a validated target; out-of-range values are rejected rather than clamped so
    /// the shell can surface the constraint instead of silently moving the user's choice.
    pub fn new(hour: u8, minute: u8) -> CoreResult<Self> {
        let candidate = Self { hour, minute };
        if candidate.is_valid() {
            Ok(candidate)
        } else {
            Err(CoreError::InvalidSetting {
                detail: LOCATION_REMINDER_RANGE_MESSAGE.to_string(),
            })
        }
    }

    /// Vienna minutes since midnight.
    pub fn minutes_since_midnight(&self) -> i64 {
        self.hour as i64 * 60 + self.minute as i64
    }

    /// Within 06:00-08:45 inclusive AND a real wall-clock time.
    pub fn is_valid(&self) -> bool {
        if self.hour > 23 || self.minute > 59 {
            return false;
        }
        let m = self.minutes_since_midnight();
        (LOCATION_REMINDER_MIN_MINUTES..=LOCATION_REMINDER_MAX_MINUTES).contains(&m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// v1's fixed 08:45 target (notifications-location §5) stays the default.
    #[test]
    fn default_matches_the_v1_fixed_target() {
        let time = LocationReminderTime::default();
        assert_eq!((time.hour, time.minute), (8, 45));
        assert_eq!(time.minutes_since_midnight(), 8 * 60 + 45);
    }

    /// The inclusive window is 06:00-08:45; one minute outside either end is
    /// rejected rather than clamped, so the shell can say why.
    #[test]
    fn accepts_the_bounds_and_rejects_everything_outside() {
        assert!(LocationReminderTime::new(6, 0).is_ok());
        assert!(LocationReminderTime::new(7, 30).is_ok());
        assert!(LocationReminderTime::new(8, 45).is_ok());
        for (hour, minute) in [(5, 59), (8, 46), (9, 0), (8, 60), (24, 0)] {
            let err = LocationReminderTime::new(hour, minute).unwrap_err();
            assert_eq!(err.to_string(), LOCATION_REMINDER_RANGE_MESSAGE);
        }
    }
}
