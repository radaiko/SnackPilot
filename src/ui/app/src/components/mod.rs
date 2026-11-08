//! Shared primitive components (04 §4), styled by the §7 recipes in
//! [`crate::theme`] (openspec: gpui-kit-ui-rewrite, task 4.1; REQ-UI-05,
//! REQ-UI-06).
//!
//! Every component takes the resolved [`crate::theme::AppTheme`] and renders
//! German copy verbatim from `docs/requirements/04-ui-ux.md` in the bundled
//! Inter font; icons come from the gpui-kit asset bundle (Lucide).

pub mod banner;
pub mod button;
pub mod card;
pub mod chip;
pub mod day_navigator;
pub mod dialog;
pub mod loading_overlay;
pub mod spinner;
pub mod switch;
pub mod tab_bar;
pub mod text_field;
pub mod toast;
