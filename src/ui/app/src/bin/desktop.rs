//! Desktop spike entry (task 2.3): `snackpilot-desktop` opens a window through
//! `gpui_kit::application()` and mounts the same `Root` as the mobile hosts
//! (REQ-UI-01, REQ-UI-07).
//!
//! A release launch keeps its cache/preferences in the durable per-user
//! directory the desktop storage adapter resolves (Application Support on
//! macOS, LOCALAPPDATA on Windows); a missing or unwritable location stops the
//! launch with the real reason instead of falling back to a temp directory
//! (REQ-DST-04).

// Built for the Android package too (cargo-ndk builds all targets), where
// `gpui_kit::application()` does not exist — the mobile hosts enter through
// `android_main` / the iOS FFI instead, so this main is a stub there.
#[cfg(any(target_os = "ios", target_os = "android"))]
fn main() {
    unimplemented!("mobile hosts do not use the desktop binary");
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn main() {
    use snackpilot_app::platform::desktop::DesktopPlatform;
    use std::sync::Arc;

    let platform: Arc<dyn snackpilot_model::Platform> = Arc::new(DesktopPlatform::new());
    // Desktop reports no notification/geofence service; the model gates those
    // bootstrap calls on exactly this capability (REQ-PLT-07, REQ-DST-04).
    let caps = platform.capabilities();
    eprintln!(
        "desktop platform ready: notifications={} geofence={}",
        caps.notifications, caps.geofence
    );

    // Dock activation after the window was closed reopens it through the
    // installed model, so there is one session and one window (REQ-UI-07).
    snackpilot_app::desktop::application_hook(gpui_kit::application())
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            // The shell renders from this AppModel (task 4.2). A normal launch
            // resolves the durable host directory; only the debug evidence
            // harness keeps its isolated temp subdirectory.
            let storage = match snackpilot_app::desktop::storage::launch_storage() {
                Ok(dir) => dir,
                Err(error) => {
                    eprintln!("desktop: {error}");
                    std::process::exit(1);
                }
            };
            match snackpilot_app::evidence::model(storage, Arc::clone(&platform)) {
                Ok(model) => {
                    snackpilot_app::install_shell_model(model);
                    if let Some(model) = snackpilot_app::shell_model() {
                        model.bootstrap();
                    }
                }
                Err(e) => eprintln!("desktop: model unavailable: {e:?}"),
            }
            snackpilot_app::mount_desktop(cx);
            // Kept alive for the app lifetime; the UI wires it into the model in
            // later tasks.
            let _ = &platform;
        });
}
