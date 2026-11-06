// Root build.gradle.kts for the SnackPilot gpui-kit Android host (task 2.1).
// The Rust cdylib is built by tools/devops/ui-android-build.sh (cargo-ndk)
// into app/src/main/jniLibs before this assembles the APK.

buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:9.1.0")
    }
}

tasks.register("clean", Delete::class) {
    delete(rootProject.layout.buildDirectory)
}
