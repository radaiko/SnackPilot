//! Demo mode — canned data served behind the same API surface, no network
//! (03-features/demo-mode.md). Activated by the magic credentials `demo` / `demo1234!`.
pub mod data;
pub mod gourmet;
pub mod prng;
pub mod ventopay;

/// Magic demo credentials (demo-mode §1; v1 constants.ts:23-28). Never sent to a live server.
pub const DEMO_USERNAME: &str = "demo";
pub const DEMO_PASSWORD: &str = "demo1234!";

/// True iff these credentials should activate demo mode (case-sensitive username, exact password).
pub fn is_demo_credentials(username: &str, password: &str) -> bool {
    username.eq_ignore_ascii_case(DEMO_USERNAME) && password == DEMO_PASSWORD
}
