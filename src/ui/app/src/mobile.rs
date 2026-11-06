//! Mobile host entry points for the spike (task 2.1 Android, task 2.2 iOS),
//! following the gpui-mobile example at rev 0b882efdac7f524e0bb0b1d4c886b2aa752f9f20.

#[cfg(target_os = "android")]
mod android_host {
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
}

#[cfg(target_os = "ios")]
mod ios_host {
    use gpui::App;

    /// Routes Rust `log` to stderr; `xcrun simctl launch --console-pty` shows it.
    struct StderrLogger;

    impl log::Log for StderrLogger {
        fn enabled(&self, _metadata: &log::Metadata) -> bool {
            true
        }
        fn log(&self, record: &log::Record) {
            eprintln!(
                "[{}] {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
        fn flush(&self) {}
    }

    /// Called by the UIKit host (Host/App.swift) before `gpui_ios_run_demo()`,
    /// which starts the GPUI run loop (gpui-mobile example pattern). The bridge
    /// supports exactly one GPUI view for the app lifetime.
    #[no_mangle]
    pub extern "C" fn gpui_ios_register_app() {
        let _ = log::set_logger(&StderrLogger).map(|()| log::set_max_level(log::LevelFilter::Info));
        std::panic::set_hook(Box::new(|info| eprintln!("GPUI PANIC: {info}")));
        gpui_mobile::ios::ffi::set_app_callback(Box::new(|cx: &mut App| crate::mount(cx)));
    }
}
