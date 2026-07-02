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

// Module names are kept stable; projectDir remaps them onto the
// backends/kotlin/ tree (see ARCHITECTURE.md — backend directory layout).
include(":sce-forge-runtime-kotlin")
project(":sce-forge-runtime-kotlin").projectDir = file("backends/kotlin/forge-runtime")

include(":sce-kotlin-runtime")
project(":sce-kotlin-runtime").projectDir = file("backends/kotlin/runtime")
include(":sce-kotlin-rhino")
project(":sce-kotlin-rhino").projectDir = file("backends/kotlin/rhino")
include(":sce-kotlin-lua")
project(":sce-kotlin-lua").projectDir = file("backends/kotlin/lua")
include(":sce-kotlin-quickjs")
project(":sce-kotlin-quickjs").projectDir = file("backends/kotlin/quickjs")
include(":sce-kotlin-tests")
project(":sce-kotlin-tests").projectDir = file("backends/kotlin/tests")
include(":sce-kotlin-benchmark")
project(":sce-kotlin-benchmark").projectDir = file("backends/kotlin/benchmark")
include(":sce-spring-boot-starter")
project(":sce-spring-boot-starter").projectDir = file("backends/kotlin/spring-boot-starter")

// Android module — requires ANDROID_HOME or sdk.dir in local.properties
val androidHome = System.getenv("ANDROID_HOME")
    ?: System.getenv("ANDROID_SDK_ROOT")
    ?: file("local.properties").takeIf { it.exists() }?.let { f ->
        java.util.Properties().apply { f.inputStream().use { load(it) } }
            .getProperty("sdk.dir")
    }
if (androidHome != null) {
    include(":sce-android-app")
    project(":sce-android-app").projectDir = file("backends/kotlin/android-app")
}
