// Settings for the SnackPilot gpui-kit Android host (task 2.1 spike).
// Minimal single-module project mirroring the gpui-mobile example's
// example/android/gradle tree at rev 0b882efdac7f524e0bb0b1d4c886b2aa752f9f20.

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "SnackPilotGpui"
include(":app")
