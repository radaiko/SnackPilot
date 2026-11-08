//! Desktop spike entry (task 2.3): `snackpilot-desktop` opens a window through
//! `gpui_kit::application()` and mounts the same `Root` as the mobile hosts
//! (REQ-UI-01, REQ-UI-07).

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
    let caps = platform.capabilities();
    eprintln!(
        "desktop platform ready: notifications={} geofence={}",
        caps.notifications, caps.geofence
    );

    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            // The shell renders from this AppModel (task 4.2).
            match snackpilot_model::AppModel::new(
                std::env::temp_dir().join("snackpilot-desktop"),
                Arc::clone(&platform),
            ) {
                Ok(model) => snackpilot_app::install_shell_model(Arc::new(model)),
                Err(e) => eprintln!("desktop: model unavailable: {e:?}"),
            }
            snackpilot_app::mount_desktop(cx);
            // Kept alive for the app lifetime; the UI wires it into the model in
            // later tasks.
            let _ = &platform;
        });
}
