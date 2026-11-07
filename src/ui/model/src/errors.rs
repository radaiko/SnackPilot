//! Maps core errors to the German user-facing strings of `AppModel.humanError`.

use snackpilot_core::error::CoreError;

/// German sentence for a core error, ported from `AppModel.humanError`
/// (AppModel.swift:209-230).
pub fn human_error(error: &CoreError) -> String {
    match error {
        CoreError::LoginFailed { detail } if !detail.is_empty() => detail.clone(),
        CoreError::LoginFailed { .. } => "Anmeldung fehlgeschlagen.".to_string(),
        CoreError::AddToCartFailed { detail } if !detail.is_empty() => detail.clone(),
        CoreError::AddToCartFailed { .. } => "Bestellung fehlgeschlagen.".to_string(),
        CoreError::SessionExpired => {
            "Die Sitzung ist abgelaufen. Bitte melde dich erneut an.".to_string()
        }
        CoreError::NotLoggedIn => "Nicht angemeldet.".to_string(),
        CoreError::Http { .. } => {
            "Verbindungsproblem. Bitte prüfe deine Internetverbindung und versuche es erneut."
                .to_string()
        }
        CoreError::Parse { .. } => {
            "Die Serverantwort konnte nicht gelesen werden. Bitte später erneut versuchen."
                .to_string()
        }
        CoreError::EditModeFailed => "Bestellung konnte nicht bearbeitet werden.".to_string(),
        CoreError::Storage { .. } => "Speicherfehler.".to_string(),
    }
}

/// Whether a loaded session belongs to the offline demo account (demo-mode §1).
pub fn is_demo_user(info: &snackpilot_core::domain::GourmetUserInfo) -> bool {
    info.username == "Demo User"
}
