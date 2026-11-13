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
        // A debug evidence run gets isolated data and dummy credentials, and
        // the model runs offline before its launch order (task 1.5).
        let storage = crate::evidence::storage_dir(std::path::PathBuf::from(&storage_dir));
        // Debug evidence selection, read once before bootstrap. The viewport
        // is reported to the isolated model before its launch order; the
        // synthetic scenario is applied after it (model lifecycle owns the
        // fixtures). A release build selects neither.
        let viewport = crate::evidence::viewport();
        let scenario = crate::evidence::scenario();
        match crate::evidence::model(
            storage,
            std::sync::Arc::clone(&bridge) as std::sync::Arc<dyn snackpilot_model::Platform>,
        ) {
            Ok(model) => {
                if let Some(viewport) = viewport {
                    crate::evidence::install_viewport(&model, viewport);
                }
                // The shell renders from the same AppModel the geofence
                // session pumps (task 4.2).
                crate::install_shell_model(std::sync::Arc::clone(&model));
                model.bootstrap();
                if let Some(scenario) = scenario {
                    crate::evidence::apply_scenario(&model, &scenario);
                }
                crate::platform::android::install_debug_session(model);
                log::info!("android_main: geofence session ready");
            }
            Err(e) => log::error!("android_main: model session unavailable: {e:?}"),
        }

        // Platform::run blocks driving the NDK event loop; the closure runs once
        // InitWindow delivers the native surface (gpui-mobile example pattern).
        // AllAssets (not the component-default bundle) so every §2 icon resolves
        // from the embedded catalog (4.1).
        Application::with_platform(shared.into_rc())
            .with_assets(gpui_kit::assets::AllAssets)
            .run(crate::mount);
        log::info!("android_main: run loop exited");
    }
}

#[cfg(target_os = "ios")]
mod ios_host {
    use gpui::App;

    /// Starts the GPUI run loop for the UIKit host (Host/App.swift calls this
    /// after `gpui_ios_set_embedded()`). Mirrors gpui-mobile's `run_app`
    /// (initialize → `run_embedded` → finish-launching) but adds
    /// `with_assets(AllAssets)` so every §2 icon resolves from the embedded
    /// catalog (4.1) — the library's own entry point builds the application
    /// without an asset source.
    #[no_mangle]
    pub extern "C" fn gpui_ios_run() {
        crate::platform::ios::install_stderr_logger();
        std::panic::set_hook(Box::new(|info| eprintln!("GPUI PANIC: {info}")));
        unsafe { gpui_mobile::ios::ffi::gpui_ios_initialize() };
        let platform = gpui_mobile::ios::current_platform(false);
        // The handle owns the App (and the window's PlatformWindow); dropping
        // it frees the IosWindow while the FFI window list still points at it,
        // so the next layout pass messages a freed view. Keep it alive for the
        // process lifetime, as gpui-mobile's own run_app does.
        thread_local! {
            static APP: std::cell::RefCell<Option<gpui::ApplicationHandle>> =
                const { std::cell::RefCell::new(None) };
        }
        let handle = gpui::Application::with_platform(platform)
            .with_assets(gpui_kit::assets::AllAssets)
            .run_embedded(|cx: &mut App| crate::mount(cx));
        APP.with(|slot| *slot.borrow_mut() = Some(handle));
        unsafe { gpui_mobile::ios::ffi::gpui_ios_did_finish_launching(std::ptr::null_mut()) };
    }
}
