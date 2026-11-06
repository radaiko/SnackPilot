// App module for the SnackPilot gpui-kit Android host (task 2.1 spike).
// Packages the pre-built Rust cdylib from app/src/main/jniLibs/<abi>/ into an
// APK hosted by a NativeActivity, mirroring the gpui-mobile example host.

plugins {
    id("com.android.application")
}

android {
    namespace = "dev.radaiko.gourmetclient"
    compileSdk = 36

    defaultConfig {
        applicationId = "dev.radaiko.gourmetclient"
        minSdk = 26          // Vulkan 1.0 is mandatory from API 26+
        targetSdk = 36
        versionCode = 1
        versionName = "0.0.0"

        // x86_64 for the headless emulator, arm64-v8a for devices.
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }

        // Tell NativeActivity which .so to load (must match the cdylib name).
        manifestPlaceholders["nativeLibraryName"] = "snackpilot_app"
    }

    buildTypes {
        debug {
            isDebuggable = true
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    packaging {
        // Keep the Rust library unstripped — cargo strips in release itself.
        jniLibs {
            keepDebugSymbols += listOf(
                "*/arm64-v8a/libsnackpilot_app.so",
                "*/x86_64/libsnackpilot_app.so"
            )
        }
    }

    lint {
        abortOnError = false
        checkReleaseBuilds = false
    }
}

// No dependencies: the spike uses only framework classes (android.app.NativeActivity).
