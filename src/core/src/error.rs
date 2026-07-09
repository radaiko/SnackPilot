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
    /// Transport-level failure or HTTP status >= 400.
    #[error("{detail}")]
    Http { detail: String },
    #[error("{detail}")]
    Storage { detail: String },
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
}
