//! UniFFI binding generator entry point. The iOS/Android build scripts invoke this to
//! generate Swift / Kotlin sources from the compiled core library, e.g.:
//!   cargo run --bin uniffi-bindgen -- generate --library <path/to/libsnackpilot_core.dylib> \
//!       --language swift --out-dir <out>
fn main() {
    uniffi::uniffi_bindgen_main()
}
