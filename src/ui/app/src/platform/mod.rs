//! Platform bridges for each host (openspec: gpui-kit-ui-rewrite, design D4).
//!
//! The model talks only to `snackpilot_model::platform::Platform`; each host
//! implements it over its native APIs. Android over JNI: [`android`], iOS over
//! a C function table registered by Swift shims: [`ios`], desktop over the
//! OS keychain with the rest unavailable: [`desktop`].

pub mod android;
pub mod ios;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod desktop;
