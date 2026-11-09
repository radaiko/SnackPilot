// App module for the SnackPilot gpui-kit Android host (task 2.1 spike).
// Packages the pre-built Rust cdylib from app/src/main/jniLibs/<abi>/ into an
// APK hosted by a NativeActivity, mirroring the gpui-mobile example host.

import java.util.Properties

plugins {
    // AGP 9 compiles Kotlin itself; the standalone KGP plugin is forbidden here.
    id("com.android.application")
}

android {
    namespace = "dev.radaiko.gourmetclient"
    compileSdk = 36

    defaultConfig {
        applicationId = "dev.radaiko.gourmetclient"
        minSdk = 29          // shipped v1 floor (task 6.1 checklist)
        targetSdk = 36
        versionCode = 35
        versionName = "2.1.0"

        // x86_64 for the headless emulator, arm64-v8a for devices.
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }

        // Tell NativeActivity which .so to load (must match the cdylib name).
        manifestPlaceholders["nativeLibraryName"] = "snackpilot_app"
    }

    // Release signing reads the gitignored keystore.properties (created by
    // `make android-keystore`). Without it the release build stays unsigned
    // so a plain assembleDebug keeps working.
    val keystoreProps = rootProject.file("keystore.properties")
    val keystore = if (keystoreProps.exists()) {
        Properties().apply { keystoreProps.inputStream().use { load(it) } }
    } else {
        null
    }
    signingConfigs {
        if (keystore != null) {
            create("release") {
                storeFile = file(keystore.getProperty("storeFile"))
                storePassword = keystore.getProperty("storePassword")
                keyAlias = keystore.getProperty("keyAlias")
                keyPassword = keystore.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        debug {
            isDebuggable = true
        }
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.findByName("release")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    // AGP 9 built-in Kotlin: jvmTarget defaults to compileOptions.targetCompatibility.
    // jniLibs default srcDir is already src/main/jniLibs, no sourceSets override needed.

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

// Platform bridge (task 3.4): Play services geofencing, Aptabase analytics,
// NotificationCompat (androidx.core), coroutine dispatchers for Play services callbacks.
dependencies {
    implementation("com.google.android.gms:play-services-location:21.3.0")
    implementation("com.github.aptabase:aptabase-kotlin:0.0.8")   // self-hosted analytics
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
}
