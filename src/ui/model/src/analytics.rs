//! Anonymous analytics events the model emits (03-features/analytics.md §4,
//! REQ-PLT-04). Fire-and-forget: the caller ignores the result, so a failing
//! bridge can never affect the flow it reports on.

use std::collections::HashMap;

/// The clock part every reminder event carries.
pub(crate) fn clock(hour: u8, minute: u8) -> HashMap<String, String> {
    HashMap::from([
        ("hour".to_string(), hour.to_string()),
        ("minute".to_string(), minute.to_string()),
    ])
}

/// A permission answer: which permission, and whether the system granted it.
pub(crate) fn permission(kind: &str, granted: bool) -> HashMap<String, String> {
    HashMap::from([
        ("kind".to_string(), kind.to_string()),
        ("granted".to_string(), granted.to_string()),
    ])
}

/// The daily reminder's switch state, with the time it is configured for.
pub(crate) fn reminder_toggled(enabled: bool, hour: u8, minute: u8) -> HashMap<String, String> {
    let mut props = clock(hour, minute);
    props.insert("enabled".to_string(), enabled.to_string());
    props
}

/// A moved reminder target: `kind` is `daily` (push reminder) or `location`
/// (the geofence's "no order" notice).
pub(crate) fn time_changed(kind: &str, hour: u8, minute: u8) -> HashMap<String, String> {
    let mut props = clock(hour, minute);
    props.insert("kind".to_string(), kind.to_string());
    props
}
