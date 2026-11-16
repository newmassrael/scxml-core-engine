// The hand-written harness the Kotlin suites run their machines with:
// `W3CTestBase` (engine selection, the drive to a final state), `VirtualDrive`
// (`initializeOnVirtualTime`, `driveOnVirtualTime`), `W3CHttpTestBase` and the
// `W3CHttpTestServer` it listens through.
//
// A library rather than test sources of either suite, because both suites use
// it: every generated W3C JUnit class in `:sce-kotlin-w3c` extends
// `W3CTestBase`, and 74 integration tests in `:sce-kotlin-tests` call it. It
// lived in the W3C package of `:sce-kotlin-tests` until 2026-11-07, when the W3C
// suite became a module of its own (backends/kotlin/w3c/build.gradle.kts). The
// package names did not move with it, so neither did any import.
//
// `sce-codegen generate-w3c --output-dir` copies these files into a suite that
// stands on its own (`sce-build/src/w3c_suite.rs` reads them from here).
//
// Its own tests hold the harness to what the conformance lane says about
// itself: `ScriptEngineSelectionTest` (the suite runs the engine it was asked
// for, or refuses) and `GateEnginePairsTest` (the lane's engine rows are every
// route into every engine). Neither depends on which engine a row selects, so
// `scripts/gates/w3c-kotlin.sh` runs them once rather than in every row.

plugins {
    alias(libs.plugins.kotlin.jvm)
}

group = "com.sce"
version = "1.0.0"

dependencies {
    // A suite's machines and its JUnit classes are typed against these, so they
    // are part of what the harness hands its users.
    api(project(":sce-kotlin-runtime"))
    api(project(":sce-kotlin-rhino"))
    api(project(":sce-kotlin-lua"))
    api(project(":sce-kotlin-quickjs"))
    // `W3CTestBase` and `W3CHttpTestBase` declare the `@Test` every generated
    // class inherits, so JUnit's API is part of theirs.
    api(libs.junit.jupiter)

    testImplementation(kotlin("test"))
}

val rootDir: String = rootProject.projectDir.absolutePath

tasks.test {
    useJUnitPlatform()

    // `GateEnginePairsTest` reads the conformance gate by a path relative to
    // the repository root.
    workingDir = File(rootDir)

    // `GateEnginePairsTest` reads the conformance gate and holds its
    // `KOTLIN_ENGINE_PAIRS` array to the routes this backend actually has. The
    // gate is not on the compile classpath, so editing it left the task
    // UP-TO-DATE: measured 2026-08-30 by deleting the `lua:lua` row — the row
    // that whole change exists to add — which the test must reject and instead
    // reported BUILD SUCCESSFUL in 500ms without running a test. A gate nothing
    // re-reads is a gate that cannot be wrong.
    inputs.file(File(rootDir, "scripts/gates/w3c-kotlin.sh"))
        .withPropertyName("kotlinConformanceGate")
        .withPathSensitivity(PathSensitivity.RELATIVE)

    // `ScriptEngineSelectionTest` builds each engine, and Lua and QuickJS are
    // native libraries.
    val luaLibDir = project(":sce-kotlin-lua").layout.buildDirectory.dir("native/lib")
    val quickjsLibDir = project(":sce-kotlin-quickjs").layout.buildDirectory.dir("native/lib")
    val sep = File.pathSeparator
    systemProperty("java.library.path",
        "${luaLibDir.get().asFile.absolutePath}${sep}${quickjsLibDir.get().asFile.absolutePath}")

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
