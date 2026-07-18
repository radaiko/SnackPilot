import java.io.FileInputStream
import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

// Release signing is read from a gitignored keystore.properties at the android/ root (07-release).
// Absent that file (fresh clone, CI without secrets), the release build is simply left unsigned and
// only debug builds are usable — nothing breaks. Generate the keystore + fill the file per README.
val keystorePropertiesFile = rootProject.file("keystore.properties")
val keystoreProperties = Properties().apply {
    if (keystorePropertiesFile.exists()) FileInputStream(keystorePropertiesFile).use { load(it) }
}

android {
    namespace = "dev.radaiko.snackpilot"
    compileSdk = 35

    defaultConfig {
        // v1 identity — an in-place Play update, required for credential takeover (07-release §2).
        applicationId = "dev.radaiko.gourmetclient"
        minSdk = 29          // Android 10
        targetSdk = 35
        versionCode = 33
        versionName = "2.0.4"
        ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64") }
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }

    signingConfigs {
        if (keystorePropertiesFile.exists()) {
            create("release") {
                // rootProject.file() resolves a relative storeFile against src/android/ (as
                // keystore.properties.example documents) and passes an absolute path through as-is.
                storeFile = rootProject.file(keystoreProperties["storeFile"] as String)
                storePassword = keystoreProperties["storePassword"] as String
                keyAlias = keystoreProperties["keyAlias"] as String
                keyPassword = keystoreProperties["keyPassword"] as String
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            if (keystorePropertiesFile.exists()) {
                signingConfig = signingConfigs.getByName("release")
            }
            // NOTE: native (Rust core) debug symbols are NOT bundled — AGP's debugSymbolLevel only
            // covers libs AGP builds, not our prebuilt cargo-ndk .so, and post-build injection makes
            // Play reject the AAB signature. Play's "missing native symbols" warning is non-blocking;
            // symbols can be uploaded separately later via the Play Developer API (needs a service acct).
        }
    }
}

dependencies {
    implementation(platform("androidx.compose:compose-bom:2025.06.01"))
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    debugImplementation("androidx.compose.ui:ui-tooling")
    implementation("androidx.activity:activity-compose:1.9.3")
    // Force Fragment >=1.3.0 so lint's InvalidFragmentVersionForActivityResult (a false
    // positive here — the app is Compose/ComponentActivity, no Fragments) doesn't fail the
    // release build. A stale 1.1.0 is otherwise pulled in transitively.
    implementation("androidx.fragment:fragment:1.8.6")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")   // LocalLifecycleOwner / foreground refresh
    // UniFFI Kotlin runtime deps.
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
    implementation("net.java.dev.jna:jna:5.14.0@aar")
    // Geofencing + one-shot location fix (notifications-location).
    implementation("com.google.android.gms:play-services-location:21.3.0")
    implementation("com.github.aptabase:aptabase-kotlin:0.0.8")   // self-hosted analytics
}
