pluginManagement {
    repositories {
        google()
        gradlePluginPortal()
        mavenCentral()
    }
}

@Suppress("UnstableApiUsage")
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "scxml-core-engine"

include(":sce-kotlin-runtime")
include(":sce-kotlin-lua")
include(":sce-kotlin-quickjs")
include(":sce-kotlin-tests")
include(":sce-kotlin-benchmark")

// Android module — requires ANDROID_HOME or sdk.dir in local.properties
val androidHome = System.getenv("ANDROID_HOME")
    ?: System.getenv("ANDROID_SDK_ROOT")
    ?: file("local.properties").takeIf { it.exists() }?.let { f ->
        java.util.Properties().apply { f.inputStream().use { load(it) } }
            .getProperty("sdk.dir")
    }
if (androidHome != null) {
    include(":sce-android-app")
}
