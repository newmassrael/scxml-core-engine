// The Kotlin integration suite: the integration machines
// (`src/main/kotlin/com/sce/integration/`), the static datamodel's algorithms
// and the test profile's machines generated under `com.sce.generated`, and the
// tests that hold them to the scenarios, tables and specifications every
// backend shares. The integration machines are a declared tree
// (tools/codegen/trees/kotlin-integration.toml, ADR 0097), the static
// datamodel's are written by its script, and both are committed.
//
// The W3C conformance machines left this module on 2026-11-07 for
// `:sce-kotlin-w3c` (backends/kotlin/w3c/build.gradle.kts says why), and the
// harness both suites run with for `:sce-kotlin-test-harness`.

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

    // `W3CTestBase` and the virtual-time drive, which the integration tests run
    // their machines with.
    testImplementation(project(":sce-kotlin-test-harness"))
    testImplementation(kotlin("test"))
    // The Mesh host core, for the tests that run a generated machine's
    // `<invoke type="sce:mesh-rpc">` through the router a host actually
    // registers rather than a stand-in (SCE_MESH.md §mesh-9.5, §mesh-19).
    testImplementation(project(":sce-kotlin-mesh"))
    // The reference host of the standard host features, which the scenarios of
    // `tests/host_features/` replay a generated machine against
    // (docs/SCE_HOST_FEATURES.md).
    testImplementation(project(":sce-kotlin-host-features"))
    // The replay `StaticScenarioTest` holds the generated machines to the shared
    // scenarios with: the one a build outside this repository uses too.
    testImplementation(project(":sce-kotlin-scenarios"))
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.junit.jupiter)
    // `EcmaScriptSemanticsTest` reads the shared ECMA-262 table that the C++
    // engine test and the Rust frontend test also read. A parser rather than
    // a hand-rolled reader, and deliberately not one of the engines under
    // test: an engine that answers the language wrong must not also be the
    // thing that reads what the language is supposed to answer.
    testImplementation(libs.kotlinx.serialization.json)
}

// How a row replaces these machines with the other language's, and how the
// suite's tests run: shared with `:sce-kotlin-w3c`.
apply(from = rootProject.file("gradle/sce-kotlin-suite.gradle.kts"))
val rootDir: String = rootProject.projectDir.absolutePath

// ⚠ After `:sce-kotlin-w3c`, never beside it. Its compile needs about 0.8 GB
// and this one about 1.3 GB (measured 2026-11-07), and `org.gradle.parallel`
// would otherwise run the two at once in the one compile daemon, needing more
// heap together than the single compile they replaced. The test tasks are
// ordered for the same kind of reason: each forks half the machine's
// processors (gradle/sce-kotlin-suite.gradle.kts states the measurement behind
// that), so two at once would schedule the wall-clock cases twice as thin.
// `mustRunAfter` orders without depending: either task still runs alone.
tasks.named("compileKotlin") {
    mustRunAfter(":sce-kotlin-w3c:compileKotlin")
}

tasks.test {
    mustRunAfter(":sce-kotlin-w3c:test")

    // The inbox's recorded runs write their histories to the directory this
    // variable names (docs/adr/0147, D3; WhatAnotherThreadPostsEntersThroughTheInboxTest).
    // The files are the task's output and Gradle does not know it: a gate that
    // empties the directory and runs the task again would otherwise find it
    // UP-TO-DATE, the tests unrun and the directory empty. Asking for histories
    // therefore always runs the tests, as the forge runtime's queue histories do.
    if (providers.environmentVariable("SCE_INBOX_HISTORY_DIR").isPresent) {
        outputs.upToDateWhen { false }
    }

    // Data this suite reads through `workingDir` at RUN time, declared so
    // Gradle knows the task is out of date when it changes.
    //
    // Without this the suite is skipped whenever only the data moved.
    // `EcmaScriptSemanticsTest` reads `ecma262_semantics.json` — what
    // ECMA-262 says — and `kotlin_lua_divergences.json` — which of those cases
    // the Lua rewriter answers differently, held in BOTH directions. Neither
    // is on the compile classpath, so editing either used to leave
    // `:sce-kotlin-tests:test UP-TO-DATE`: measured 2026-08-27 by declaring a
    // case as divergent that the engine answers correctly, which the suite
    // must reject and instead reported BUILD SUCCESSFUL in 390ms without
    // running a test. A list nothing re-reads is a list that cannot be wrong,
    // which is the failure mode the list was written to remove.
    inputs.dir(File(rootDir, "tests/ecmascript"))
        .withPropertyName("sharedEcmaScriptTables")
        .withPathSensitivity(PathSensitivity.RELATIVE)

    // Same reason, same failure mode. `JsonTextTest`, `RealTextTest`, `EventDataKeyOrderTest`,
    // `DurationTest`, `DocumentStemTest` and `ExternalChainIsBoundedTest` each read one table
    // every backend shares, by a path relative to `workingDir` at RUN time, so none is on the
    // compile classpath either.
    // A table edited to a case this runtime answers differently must turn the suite red;
    // with the directories undeclared it reported UP-TO-DATE and ran nothing.
    inputs.dir(File(rootDir, "tests/json_text"))
        .withPropertyName("sharedJsonTextTables")
        .withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.dir(File(rootDir, "tests/durations"))
        .withPropertyName("sharedDurationTables")
        .withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.dir(File(rootDir, "tests/document_stem"))
        .withPropertyName("sharedDocumentStemTables")
        .withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.dir(File(rootDir, "tests/external_event_budget"))
        .withPropertyName("sharedExternalEventBudgetTables")
        .withPathSensitivity(PathSensitivity.RELATIVE)

    // Same reason, same failure mode. `StaticScenarioTest` replays the scenarios every
    // engine shares and `StaticDatamodelTest` reads the saved states they share, both
    // from `sce-build/tests/fixtures/static_datamodel` at RUN time, and the saved-state
    // schema from `schemas`. None is on the compile classpath, so editing a scenario
    // left `:sce-kotlin-tests:test UP-TO-DATE`: measured 2026-10-07 by adding two steps
    // to `static_payload_bytes.json`, which reported BUILD SUCCESSFUL in 28s without
    // replaying either, so a step the engine did not pass would have been a green.
    inputs.dir(File(rootDir, "sce-build/tests/fixtures/static_datamodel"))
        .withPropertyName("sharedStaticDatamodelFixtures")
        .withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.file(File(rootDir, "schemas/sce-saved-state.v1.schema.json"))
        .withPropertyName("savedStateSchema")
        .withPathSensitivity(PathSensitivity.RELATIVE)

    // Same reason, same failure mode. `ATestSuiteRunsThroughItsRunnerTest` holds the
    // generated suite's test log to the bytes the Rust and C++ runs are held to too
    // (docs/SCE_TEST_PROFILE.md §12.5), read at RUN time from this directory.
    inputs.dir(File(rootDir, "sce-build/tests/fixtures/test_profile/runtime/expected_logs"))
        .withPropertyName("sharedExpectedTestLogs")
        .withPathSensitivity(PathSensitivity.RELATIVE)

    // Same reason, same failure mode. The same test replays the suite from its
    // committed traces, which every backend's trace player reads
    // (docs/SCE_TTCN3_LANE.md §10.5), and `TraceTest` holds the runtime's trace
    // status to the schema file's, both read at RUN time.
    inputs.dir(File(rootDir, "sce-build/tests/fixtures/test_profile/runtime/traces"))
        .withPropertyName("sharedTestTraces")
        .withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.file(File(rootDir, "schemas/sce-test-trace.v1.schema.json"))
        .withPropertyName("testTraceSchema")
        .withPathSensitivity(PathSensitivity.RELATIVE)

    // Same reason, same failure mode. `JvmLongTextTest` reads the two documents
    // its machines were generated from, at RUN time, as the texts each machine must
    // hand back; a document edited with its machine left stale must turn it red.
    inputs.dir(File(rootDir, "backends/kotlin/tests/fixtures/jvm_long_text"))
        .withPropertyName("jvmLongTextDocuments")
        .withPathSensitivity(PathSensitivity.RELATIVE)

    // Same reason, same failure mode. `AScenarioOfTheTurnstileIsItsRunTest` reads each
    // scenario of the properties judge's turnstile and the recorded run of this lane
    // beside it, at RUN time (docs/adr/0115, D15); a recorded run edited to one the
    // engine does not take must turn the suite red, not leave it UP-TO-DATE. The
    // fixture's directory, which holds the scenarios' directory once they are written.
    inputs.dir(File(rootDir, "sce-build/tests/fixtures/formal_properties"))
        .withPropertyName("formalPropertiesFixture")
        .withPathSensitivity(PathSensitivity.RELATIVE)
}

// The runner program of the generated suite `runtime_suite` (docs/SCE_TTCN3_LANE.md §4):
// the class of the suite's unit, whose `main` sce-codegen writes, run on this module's
// classpath. Its command line is every backend's runner's, given after `--args`:
//
//   ./gradlew :sce-kotlin-tests:runRuntimeSuite --args="--trace-dir <dir> --log-dir <dir>"
//
// A relative directory is read from the repository root.
// `ATestSuiteRunsThroughItsRunnerTest` runs the same class as a process of its own.
tasks.register<JavaExec>("runRuntimeSuite") {
    group = "verification"
    description = "Run the runner program of the generated suite runtime_suite over its traces"
    mainClass.set("com.sce.generated.runtime_suite.RuntimeSuiteKt")
    classpath = sourceSets["main"].runtimeClasspath
    workingDir = File(rootDir)
}

kotlin {
    jvmToolchain(17)
    compilerOptions {
        allWarningsAsErrors.set(true)
    }
}
