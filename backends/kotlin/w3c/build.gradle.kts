// The W3C SCXML conformance suite in Kotlin: one machine per registered W3C
// document (`src/main/kotlin/com/sce/generated/test<N>/`) and its JUnit class
// (`src/test/kotlin/com/sce/w3c/Test<N>.kt`), both written by
// `sce-codegen generate-w3c -l kotlin` and committed.
//
// ── Why this is a module of its own ────────────────────────────────
//
// Until 2026-11-07 these machines compiled in `:sce-kotlin-tests` beside the
// integration machines, as one 13.8 MB compilation unit, which the Kotlin
// compile daemon needed about 1.85 GB to compile (bisected on a build host;
// `gradle.properties` records it). The heap a compile needs grows with its
// unit, and the two populations grow independently: the W3C suite with the
// conformance registry, the integration machines with every fixture added. Kept
// together, every addition to either moved the other's compile closer to the
// budget. Measured the same day, each alone: these 4.7 MB compile in 768 MB,
// and `:sce-kotlin-tests` without them (9.0 MB) in 1,280 MB.
//
// The boundary is the generator's, not a naming convention: this module is
// exactly what `generate-w3c -l kotlin` writes (`KotlinBackend::RELATIVE_ROOT`
// in `sce-build/src/bin/sce_codegen.rs`), and nothing else is. `com.sce.generated`
// in `:sce-kotlin-tests` still holds forge helpers and integration fixtures
// under the same package root, so a pattern over directory names would have
// split one population across two modules.
//
// ⚠ Two compiles of about 0.8 and 1.3 GB need MORE heap at once than the one
// they replaced, so `:sce-kotlin-tests` orders its compile and its test task
// after this module's (its build file states it). The headroom probe in
// `scripts/gates/w3c-kotlin.sh` compiles both in one invocation in a heap two
// thirds of the budget, so a change that let them overlap fails there.

plugins {
    alias(libs.plugins.kotlin.jvm)
}

group = "com.sce"
version = "1.0.0"

dependencies {
    // The generated machines compile against what the runtime exports and nothing
    // this module adds: the forge runtime (`SceChecked`, `AlgorithmFailure`) and
    // kotlinx.coroutines (a machine's `snapshot`) reach them through the runtime's
    // `api`, as they reach a project outside this repository that depends on the
    // runtime alone. Declaring either here as well is what hid that the runtime did
    // not export them.
    implementation(project(":sce-kotlin-runtime"))
    implementation(project(":sce-kotlin-rhino"))
    implementation(project(":sce-kotlin-lua"))
    implementation(project(":sce-kotlin-quickjs"))

    // `W3CTestBase`, `W3CHttpTestBase` and the virtual-time drive, which every
    // generated JUnit class extends or calls.
    testImplementation(project(":sce-kotlin-test-harness"))
    testImplementation(kotlin("test"))
    testImplementation(libs.junit.jupiter)
}

// ---------------------------------------------------------------------------
// SCXML Code Generation (CMake parity)
//
// Uses sce-codegen (Rust binary) to generate:
//   - src/main/kotlin/com/sce/generated/testXXX/  (state machine code)
//   - src/test/kotlin/com/sce/w3c/TestXXX.kt      (JUnit5 test classes)
//
// Build sce-codegen: cargo build --bin sce-codegen --features cli -p sce-build
// Incremental: only runs when SCXML files, templates, or test registry change.
// Generated files are kept in git so builds work without sce-codegen.
// ---------------------------------------------------------------------------

// Configuration-cache-safe codegen binary resolution.  The search order —
// and the fact that this build names no profile — lives in
// gradle/sce-codegen.gradle.kts; the value it hands back is a plain string
// evaluated eagerly, so no Gradle script object leaks into task state.
apply(from = rootProject.file("gradle/sce-codegen.gradle.kts"))
val rootDir: String = rootProject.projectDir.absolutePath
val sceCodegenAbsoluteOrNull: String? by rootProject.extra
val sceCodegenBinary: String? = sceCodegenAbsoluteOrNull
val hasCodegen: Boolean = sceCodegenBinary != null

// How a row replaces these machines with the other language's, and how the
// suite's tests run: shared with `:sce-kotlin-tests`.
apply(from = rootProject.file("gradle/sce-kotlin-suite.gradle.kts"))
val generatedOverlay: String? = providers.gradleProperty("sce.generated.overlay").orNull

val generateScxml by tasks.registering(Exec::class) {
    group = "code generation"
    description = "Generate Kotlin state machines and test classes from W3C SCXML tests"

    // An overlay run compiles machines the caller generated, so regenerating
    // the committed ones would be work whose output nothing reads.
    enabled = hasCodegen && generatedOverlay == null
    // The committed machines are regenerated in place, so the generator must
    // be the one these sources build — never an older binary that happens to
    // be lying around (gradle/sce-codegen.gradle.kts states the measurement).
    dependsOn(":buildSceCodegen")

    workingDir = File(rootDir)
    commandLine(sceCodegenBinary ?: "sce-codegen", "generate-w3c", "-l", "kotlin")

    // Inputs: SCXML sources + codegen infrastructure + test registry
    inputs.dir(File(rootDir, "resources"))
        .withPropertyName("scxmlResources")
        .withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.dir(File(rootDir, "tools/codegen/templates"))
        .withPropertyName("codegenTemplates")
        .withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.file(File(rootDir, "tests/CMakeLists.txt"))
        .withPropertyName("testRegistry")
        .withPathSensitivity(PathSensitivity.RELATIVE)

    // Outputs: what `generate-w3c` writes, and since this module holds nothing
    // else, all of its sources.
    outputs.dir("src/main/kotlin/com/sce/generated")
        .withPropertyName("generatedSources")
    outputs.dir("src/test/kotlin/com/sce/w3c")
        .withPropertyName("generatedTests")

    // Skip if sce-codegen is not available (use existing generated files)
    isIgnoreExitValue = false
}

tasks.named("compileKotlin") {
    dependsOn(generateScxml)
}

kotlin {
    jvmToolchain(17)
    compilerOptions {
        allWarningsAsErrors.set(true)
    }
}
