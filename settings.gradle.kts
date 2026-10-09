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
include(":sce-kotlin-mesh")
project(":sce-kotlin-mesh").projectDir = file("backends/kotlin/mesh")
// ECMA-262 through a Lua-lowered Kotlin artifact. Separate from
// `:sce-kotlin-tests` because it compiles two artifacts generated from one
// document with two `--script-engine` selections, which is a different subject
// from the W3C conformance suite and must not invalidate it.
include(":sce-kotlin-lowered-ecma262")
project(":sce-kotlin-lowered-ecma262").projectDir = file("backends/kotlin/lowered-ecma262")
include(":sce-kotlin-benchmark")
project(":sce-kotlin-benchmark").projectDir = file("backends/kotlin/benchmark")
include(":sce-spring-boot-starter")
project(":sce-spring-boot-starter").projectDir = file("backends/kotlin/spring-boot-starter")

// Android module — an opt-in, and it needs an SDK: ANDROID_HOME, ANDROID_SDK_ROOT
// or sdk.dir in local.properties.
//
// The SDK being present is not a request to build the app. A GitHub-hosted runner
// always has one (ANDROID_HOME is set in its image), so including the module
// whenever an SDK is found made every Gradle invocation on a runner — the W3C
// suite, the forge runtime, the lowered ECMA-262 gate, none of which touches the
// app — configure it, and configuring it makes the Android Gradle plugin download
// the NDK (about a gigabyte) on a fresh machine each time. One truncated download
// ("Error on ZipFile unknown archive") failed a gate that had nothing to do with
// Android, and every run paid for the download whether it failed or not.
//
// Ask for the app with `-Psce.androidApp=true`, or `sce.androidApp=true` in
// ~/.gradle/gradle.properties to keep it on a machine that builds it.
val androidAppRequested = providers.gradleProperty("sce.androidApp").orNull == "true"
val androidHome = System.getenv("ANDROID_HOME")
    ?: System.getenv("ANDROID_SDK_ROOT")
    ?: file("local.properties").takeIf { it.exists() }?.let { f ->
        java.util.Properties().apply { f.inputStream().use { load(it) } }
            .getProperty("sdk.dir")
    }
if (androidAppRequested && androidHome != null) {
    include(":sce-android-app")
    project(":sce-android-app").projectDir = file("backends/kotlin/android-app")
}
