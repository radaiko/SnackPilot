//! Core error type. Variants preserve v1's exact user-facing messages where the shell
//! displays them (01-gourmet-scraping §14, 02-ventopay-scraping §3-4, orders.md §8).
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    /// LoginFailed carries the full message verbatim (Gourmet or Ventopay variant text).
    #[error("{message}")]
    LoginFailed { message: String },
    #[error("Session expired")]
    SessionExpired,
    #[error("Not logged in")]
    NotLoggedIn,
    #[error("Add to cart failed: {message}")]
    AddToCartFailed { message: String },
    #[error("Failed to enter edit mode")]
    EditModeFailed,
    /// Parser errors, incl. missing-token messages, carried verbatim.
    #[error("{message}")]
    Parse { message: String },
    /// Transport-level failure or HTTP status >= 400.
    #[error("{message}")]
    Http { message: String },
    #[error("{message}")]
    Storage { message: String },
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
        assert_eq!(CoreError::EditModeFailed.to_string(), "Failed to enter edit mode");
        assert_eq!(
            CoreError::LoginFailed {
                message: "Login failed: invalid credentials or account blocked".into()
            }
            .to_string(),
            "Login failed: invalid credentials or account blocked"
        );
        assert_eq!(
            CoreError::AddToCartFailed { message: "boom".into() }.to_string(),
            "Add to cart failed: boom"
        );
    }
}
