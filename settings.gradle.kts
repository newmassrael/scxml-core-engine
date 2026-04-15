pluginManagement {
    repositories {
        google()
        gradlePluginPortal()
        mavenCentral()
    }
}

// Justification (UnstableApiUsage): dependencyResolutionManagement is the
// Gradle 8 idiom for project-wide repository configuration; the API is
// marked incubating but is the only supported way to enforce
// FAIL_ON_PROJECT_REPOS for a multi-module build.
@Suppress("UnstableApiUsage")
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "scxml-core-engine"

include(":sce-forge-runtime-kotlin")
project(":sce-forge-runtime-kotlin").projectDir = file("sce-forge-runtime/kotlin")

include(":sce-kotlin-runtime")
include(":sce-kotlin-rhino")
include(":sce-kotlin-lua")
include(":sce-kotlin-quickjs")
include(":sce-kotlin-tests")
include(":sce-kotlin-benchmark")
include(":sce-spring-boot-starter")

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
