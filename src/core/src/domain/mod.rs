//! Portable domain records and enums (docs/architecture §4.2).
pub mod billing;
pub mod location;
pub mod menu;
pub mod order;
pub mod user;

pub use billing::*;
pub use location::*;
pub use menu::*;
pub use order::*;
pub use user::*;
