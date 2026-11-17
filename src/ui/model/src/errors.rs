//! Maps core errors to the German user-facing strings of `AppModel.humanError`.

use snackpilot_core::error::CoreError;

/// German sentence for a core error, ported from `AppModel.humanError`
/// (AppModel.swift:209-230).
pub fn human_error(error: &CoreError) -> String {
    error.user_message()
}

/// Whether a loaded session belongs to the offline demo account (demo-mode §1).
pub fn is_demo_user(info: &snackpilot_core::domain::GourmetUserInfo) -> bool {
    info.username == "Demo User"
}
