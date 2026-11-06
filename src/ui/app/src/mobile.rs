//! Android host entry point (task 2.1 spike), following the gpui-mobile
//! example's `android_main` pattern at rev 0b882efdac7f524e0bb0b1d4c886b2aa752f9f20.

use android_activity::AndroidApp;
use gpui::Application;
use gpui_mobile::android::jni;

/// Called by `android-activity` on a dedicated native thread after the
/// Gradle host's Activity loads `libsnackpilot_app.so`.
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("snackpilot-app"),
    );
    jni::install_panic_hook();
    log::info!("android_main: entered");

    let _platform = jni::init_platform(&app);
    let shared = jni::shared_platform().expect("shared Android platform");

    // Platform::run blocks driving the NDK event loop; the closure runs once
    // InitWindow delivers the native surface (gpui-mobile example pattern).
    Application::with_platform(shared.into_rc()).run(|cx| crate::mount(cx));
    log::info!("android_main: run loop exited");
}
