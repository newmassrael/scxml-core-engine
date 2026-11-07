// The Kotlin machine of a statechart too large for one JVM method, and the test
// that it compiles and runs (JvmMethodSizeTest).
//
// ── Why this is a module of its own ────────────────────────────────
//
// The machine is a stress fixture: one 1.5 MB source file, written so that its
// transition content would pass the 65,535 bytes one JVM method holds if the
// generator still wrote it as one method. Compiling it is what it is FOR, and
// compiling it is expensive. Measured 2026-11-07, on a fresh Kotlin compile
// daemon of 2 GB (the heap it then inherited from `org.gradle.jvmargs`), the
// whole of `:sce-kotlin-tests` on the committed Lua machines took 49 full
// collections and 121.6 s with this machine in it, and 6 full collections and
// 37.9 s without it. That one file was two thirds of the time of the compile it
// shared, and most of the collector's work.
//
// The live sets those runs printed (1,581 and 1,019 MB) are lower bounds,
// sampled at whatever moments the collector ran: the module without this
// machine still needed about 1.85 GB to compile, measured by bisecting the heap
// (`gradle.properties` records it, and the split of that module since). Read
// them as a comparison between two like
// runs, not as what either compile needs.
//
// It shared that compile four times per Kotlin W3C run, because
// `scripts/gates/w3c-kotlin.sh` compiles `:sce-kotlin-tests` once per
// engine/language row, and from 2026-11-05 the fourth row's compile ran out of
// heap on every run on main (`OutOfMemoryError: GC overhead limit exceeded` in
// the Kotlin compile daemon). The machine carries no script-engine language
// (`datamodel="sce-static"`), so nothing about it varies between those rows:
// compiling it four times measured nothing three times. Here it compiles once,
// in a compilation of its own, and the W3C rows compile only what varies.
//
// The bound on every generated Kotlin method is held without a JVM by
// sce-build/tests/no_generated_kotlin_method_grows_with_the_document.rs; this
// module is the half that needs kotlinc and a JVM.

plugins {
    alias(libs.plugins.kotlin.jvm)
}

group = "com.sce"
version = "1.0.0"

dependencies {
    // The machine compiles against what the runtime exports and nothing more,
    // as a project outside this repository that depends on the runtime alone.
    implementation(project(":sce-kotlin-runtime"))

    testImplementation(kotlin("test"))
    testImplementation(libs.junit.jupiter)
}

val rootDir: String = rootProject.projectDir.absolutePath

tasks.test {
    useJUnitPlatform()

    // `JvmMethodSizeTest` reads its fixture by a path relative to the
    // repository root.
    workingDir = File(rootDir)

    // It reads the document its machine was generated from at RUN time, as the
    // sums each transition must add up, so a document edited with its machine
    // left stale must make the task out of date and turn it red.
    inputs.dir(File(rootDir, "backends/kotlin/method-size/fixtures"))
        .withPropertyName("jvmMethodSizeDocument")
        .withPathSensitivity(PathSensitivity.RELATIVE)

    testLogging {
        events("passed", "skipped", "failed")
        showStandardStreams = false
        exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
    }
}

kotlin {
    jvmToolchain(17)
    compilerOptions {
        allWarningsAsErrors.set(true)
    }
}
