//! One case per `CoreError` variant of [`snackpilot_model::human_error`],
//! ported from `AppModel.humanError` (AppModel.swift:209-230).
//!
//! @req REQ-PLT-06

use snackpilot_core::error::CoreError;
use snackpilot_model::human_error;

/// @req REQ-PLT-06
#[test]
fn login_failed_shows_detail_verbatim() {
    assert_eq!(
        human_error(&CoreError::LoginFailed {
            detail: "Login failed: invalid credentials".into()
        }),
        "Login failed: invalid credentials"
    );
}

/// @req REQ-PLT-06
#[test]
fn login_failed_empty_detail_uses_german_fallback() {
    assert_eq!(
        human_error(&CoreError::LoginFailed {
            detail: String::new()
        }),
        "Anmeldung fehlgeschlagen."
    );
}

/// @req REQ-PLT-06
#[test]
fn add_to_cart_failed_shows_detail_verbatim() {
    assert_eq!(
        human_error(&CoreError::AddToCartFailed {
            detail: "Bestellung nicht möglich".into()
        }),
        "Bestellung nicht möglich"
    );
}

/// @req REQ-PLT-06
#[test]
fn add_to_cart_failed_empty_detail_uses_german_fallback() {
    assert_eq!(
        human_error(&CoreError::AddToCartFailed {
            detail: String::new()
        }),
        "Bestellung fehlgeschlagen."
    );
}

/// @req REQ-PLT-06
#[test]
fn session_expired() {
    assert_eq!(
        human_error(&CoreError::SessionExpired),
        "Die Sitzung ist abgelaufen. Bitte melde dich erneut an."
    );
}

/// @req REQ-PLT-06
#[test]
fn not_logged_in() {
    assert_eq!(human_error(&CoreError::NotLoggedIn), "Nicht angemeldet.");
}

/// @req REQ-PLT-06
#[test]
fn http_maps_to_connectivity_sentence() {
    assert_eq!(
        human_error(&CoreError::Http {
            detail: "error sending request".into(),
            status: None,
        }),
        "Verbindungsproblem. Bitte prüfe deine Internetverbindung und versuche es erneut."
    );
    assert_eq!(
        human_error(&CoreError::Http {
            detail: "HTTP 503".into(),
            status: Some(503),
        }),
        "Verbindungsproblem. Bitte prüfe deine Internetverbindung und versuche es erneut."
    );
}

/// @req REQ-PLT-06
#[test]
fn parse_maps_to_server_answer_sentence() {
    assert_eq!(
        human_error(&CoreError::Parse {
            detail: "missing token".into()
        }),
        "Die Serverantwort konnte nicht gelesen werden. Bitte später erneut versuchen."
    );
}

/// @req REQ-PLT-06
#[test]
fn edit_mode_failed() {
    assert_eq!(
        human_error(&CoreError::EditModeFailed),
        "Bestellung konnte nicht bearbeitet werden."
    );
}

/// @req REQ-PLT-06
#[test]
fn storage() {
    assert_eq!(
        human_error(&CoreError::Storage {
            detail: "io".into()
        }),
        "Speicherfehler."
    );
}
