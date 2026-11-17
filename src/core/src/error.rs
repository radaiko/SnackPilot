//! Core error type. Variants preserve v1's exact user-facing messages where the shell
//! displays them (01-gourmet-scraping §14, 02-ventopay-scraping §3-4, orders.md §8).
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error, uniffi::Error)]
pub enum CoreError {
    /// LoginFailed carries the full message verbatim (Gourmet or Ventopay variant text).
    #[error("{detail}")]
    LoginFailed { detail: String },
    #[error("Session expired")]
    SessionExpired,
    #[error("Not logged in")]
    NotLoggedIn,
    #[error("Add to cart failed: {detail}")]
    AddToCartFailed { detail: String },
    #[error("Failed to enter edit mode")]
    EditModeFailed,
    /// Parser errors, incl. missing-token messages, carried verbatim.
    #[error("{detail}")]
    Parse { detail: String },
    /// Transport-level failure or HTTP status >= 400. `status` is Some(status) for a real
    /// HTTP status response, None for transport-level failures (DNS, timeout, …).
    #[error("{detail}")]
    Http { detail: String, status: Option<u16> },
    #[error("{detail}")]
    Storage { detail: String },
    /// A setting was given a value outside its allowed range; carries the
    /// user-facing constraint text instead of clamping silently.
    #[error("{detail}")]
    InvalidSetting { detail: String },
}

impl CoreError {
    /// The one sentence the app shows for this failure.
    ///
    /// The `Display` strings are v1's verbatim texts and stay that way — they
    /// are logs and test expectations. What a person reads is this: transport
    /// and parse failures are summarised, because a reqwest message quotes the
    /// URL and says nothing anyone can act on, and the store's own texts
    /// (credentials, constraints) are carried as they are.
    pub fn user_message(&self) -> String {
        match self {
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
            // The core carries the constraint text itself ("… zwischen 06:00 und 08:45 …").
            CoreError::InvalidSetting { detail } => detail.clone(),
        }
    }
}

pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_strings_are_verbatim_v1() {
        // 01-gourmet-scraping §14, §7; orders.md §8
        assert_eq!(CoreError::SessionExpired.to_string(), "Session expired");
        assert_eq!(CoreError::NotLoggedIn.to_string(), "Not logged in");
        assert_eq!(
            CoreError::EditModeFailed.to_string(),
            "Failed to enter edit mode"
        );
        assert_eq!(
            CoreError::LoginFailed {
                detail: "Login failed: invalid credentials or account blocked".into()
            }
            .to_string(),
            "Login failed: invalid credentials or account blocked"
        );
        assert_eq!(
            CoreError::AddToCartFailed {
                detail: "boom".into()
            }
            .to_string(),
            "Add to cart failed: boom"
        );
    }

    /// What the app shows never quotes a transport or parser message.
    #[test]
    fn user_messages_are_readable() {
        let http = CoreError::Http {
            detail:
                "error sending request for url (https://alaclickneu.gourmet.at/de/bestellungen/)"
                    .into(),
            status: None,
        };
        assert_eq!(
            http.user_message(),
            "Verbindungsproblem. Bitte prüfe deine Internetverbindung und versuche es erneut."
        );
        assert_eq!(
            CoreError::Parse {
                detail: "expected token".into()
            }
            .user_message(),
            "Die Serverantwort konnte nicht gelesen werden. Bitte später erneut versuchen."
        );
        // The store's own texts are the message.
        assert_eq!(
            CoreError::LoginFailed {
                detail: "Falsches Passwort".into()
            }
            .user_message(),
            "Falsches Passwort"
        );
        assert_eq!(
            CoreError::InvalidSetting {
                detail: "Uhrzeit zwischen 06:00 und 08:45".into()
            }
            .user_message(),
            "Uhrzeit zwischen 06:00 und 08:45"
        );
        assert_eq!(CoreError::NotLoggedIn.user_message(), "Nicht angemeldet.");
    }
}
