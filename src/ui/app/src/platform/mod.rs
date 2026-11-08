//! Platform bridges for each host (openspec: gpui-kit-ui-rewrite, design D4).
//!
//! The model talks only to `snackpilot_model::platform::Platform`; each host
//! implements it over its native APIs. Android over JNI: [`android`], iOS over
//! a C function table registered by Swift shims: [`ios`].

pub mod android;
pub mod ios;
