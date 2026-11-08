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

        // The model Platform over the Kotlin shims (task 3.4). Classes are
        // cached by SnackPilotActivity calling PlatformBridge.nativeCacheClass.
        let storage_dir = app
            .internal_data_path()
            .expect("internal data path")
            .join("snackpilot")
            .to_string_lossy()
            .into_owned();
        let bridge = std::sync::Arc::new(crate::platform::android::AndroidPlatform::new(
            crate::platform::android::JniBridge,
            crate::platform::android::pending(),
            storage_dir.clone(),
        ));
        crate::platform::android::install_debug_platform(std::sync::Arc::clone(&bridge));
        log::info!("android_main: platform bridge ready");

        // The model session GeofenceBroadcastReceiver pumps transitions into
        // (task 3.4): the receiver holds no decision logic, the model and the
        // core decide (design D4, no v1 core duplication in the shell).
        match snackpilot_model::AppModel::new(
            std::path::PathBuf::from(&storage_dir),
            std::sync::Arc::clone(&bridge) as std::sync::Arc<dyn snackpilot_model::Platform>,
        ) {
            Ok(model) => {
                crate::platform::android::install_debug_session(std::sync::Arc::new(model));
                log::info!("android_main: geofence session ready");
            }
            Err(e) => log::error!("android_main: model session unavailable: {e:?}"),
        }

        // Platform::run blocks driving the NDK event loop; the closure runs once
        // InitWindow delivers the native surface (gpui-mobile example pattern).
        // The kit asset bundle backs the Lucide icons of the §2 mapping (4.1).
        Application::with_platform(shared.into_rc())
            .with_assets(gpui_kit::assets::Assets)
            .run(crate::mount);
        log::info!("android_main: run loop exited");
    }
}

#[cfg(target_os = "ios")]
mod ios_host {
    use gpui::App;

    /// Called by the UIKit host (Host/App.swift) before `gpui_ios_run_demo()`,
    /// which starts the GPUI run loop (gpui-mobile example pattern). The bridge
    /// supports exactly one GPUI view for the app lifetime.
    #[no_mangle]
    pub extern "C" fn gpui_ios_register_app() {
        crate::platform::ios::install_stderr_logger();
        std::panic::set_hook(Box::new(|info| eprintln!("GPUI PANIC: {info}")));
        gpui_mobile::ios::ffi::set_app_callback(Box::new(|cx: &mut App| crate::mount(cx)));
    }
}
