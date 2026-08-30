plugins {
    alias(libs.plugins.kotlin.jvm)
}

group = "com.sce"
version = "1.0.0"

dependencies {
    implementation(project(":sce-kotlin-runtime"))
    implementation(project(":sce-kotlin-rhino"))
    implementation(project(":sce-kotlin-lua"))
    implementation(project(":sce-kotlin-quickjs"))
    implementation(libs.kotlinx.coroutines.core)

    testImplementation(kotlin("test"))
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.junit.jupiter)
    // `EcmaScriptSemanticsTest` reads the shared ECMA-262 table that the C++
    // engine test and the Rust frontend test also read. A parser rather than
    // a hand-rolled reader, and deliberately not one of the engines under
    // test: an engine that answers the language wrong must not also be the
    // thing that reads what the language is supposed to answer.
    testImplementation(libs.kotlinx.serialization.json)
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

// ---------------------------------------------------------------------------
// The committed tree carries ONE language; this backend emits for two.
//
// `Language::Kotlin` owns both arms of the language seam: the machines can
// hand their engine the author's ECMAScript, or the Lua that `sce-build`'s
// frontend lowered at build time (`sce-codegen generate-w3c --script-engine`).
// Only one of those can be the committed tree, and the engines are not
// interchangeable about which they are handed — `ScxmlScriptEngine` refuses a
// language it does not accept rather than guessing.
//
// So the other language's machines are generated somewhere else and named
// here. The caller owns that directory: `scripts/gates/w3c-kotlin.sh` decides
// which language each engine needs, generates it, and points this at the
// result. Nothing about which language is which lives here — a second copy of
// that decision is exactly the drift this seam has already paid for twice.
//
// Only `com/sce/generated` moves. The generated JUnit classes under
// `com/sce/w3c` do not mention the engine language at all, and the gate holds
// that to a diff rather than to this comment.
val generatedOverlay: String? = providers.gradleProperty("sce.generated.overlay").orNull
val overlayMachines: String? = generatedOverlay?.let { "$it/src/main/kotlin/com/sce/generated" }

if (overlayMachines != null) {
    require(File(overlayMachines).isDirectory) {
        "sce.generated.overlay=$generatedOverlay names no generated tree at " +
            "$overlayMachines. The overlay replaces the committed machines rather " +
            "than adding to them, so an empty one would compile a suite with no " +
            "state machines in it and report that as a pass."
    }
    sourceSets["main"].kotlin.srcDir(overlayMachines)
    // The committed machines are the OTHER language's. Excluding them is what
    // makes this a replacement: kept, they would be duplicate declarations of
    // every generated class, and dropped from the exclude they would be the
    // ones that compiled.
    sourceSets["main"].kotlin.exclude("com/sce/generated/**")
}

val generateScxml by tasks.registering(Exec::class) {
    group = "code generation"
    description = "Generate Kotlin state machines and test classes from W3C SCXML tests"

    // An overlay run compiles machines the caller generated, so regenerating
    // the committed ones would be work whose output nothing reads — and, on
    // a tree whose SOURCE_DATE_EPOCH pin ever slipped, a write into the
    // committed tree that this run has no reason to make.
    enabled = hasCodegen && overlayMachines == null

    workingDir = File(rootDir)
    commandLine(sceCodegenBinary ?: "sce-codegen", "generate-w3c", "-l", "kotlin")

    // The generator stamps `generated-at` from the wall clock unless
    // SOURCE_DATE_EPOCH says otherwise, so simply running this suite
    // rewrote the header of every committed generated file and left 449
    // of them modified. That is why the Kotlin W3C lane was CI-only: on
    // a developer's machine it could not be run without dirtying the
    // tree. The shell gates pin the same variable the same way
    // (`scripts/regen_all_committed_trees.sh`, `scripts/gates/w3c-go.sh`);
    // an inherited value still wins, so a deliberate re-stamp stays
    // possible. Read through `providers` rather than `System.getenv` so
    // the configuration cache tracks it.
    environment(
        "SOURCE_DATE_EPOCH",
        providers.environmentVariable("SOURCE_DATE_EPOCH").getOrElse("0"),
    )

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

    // Outputs: generated state machines + test classes
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

tasks.test {
    useJUnitPlatform()

    // C++ DataModelInitHelper pattern: resolve data src paths relative to project root
    workingDir = File(rootDir)

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

    // Same reason, same failure mode, measured the same way.
    // `GateEnginePairsTest` reads the conformance gate and holds its
    // `KOTLIN_ENGINE_PAIRS` array to the routes this backend actually has.
    // The gate is not on the compile classpath either, so editing it left
    // `:sce-kotlin-tests:test UP-TO-DATE`: measured 2026-08-30 by deleting the
    // `lua:lua` row — the row that whole change exists to add — which the test
    // must reject and instead reported BUILD SUCCESSFUL in 500ms without
    // running a test. A gate nothing re-reads is a gate that cannot be wrong.
    inputs.file(File(rootDir, "scripts/gates/w3c-kotlin.sh"))
        .withPropertyName("kotlinConformanceGate")
        .withPathSensitivity(PathSensitivity.RELATIVE)

    // Native library paths (Lua + QuickJS JNI)
    val luaLibDir = project(":sce-kotlin-lua").layout.buildDirectory.dir("native/lib")
    val quickjsLibDir = project(":sce-kotlin-quickjs").layout.buildDirectory.dir("native/lib")
    val sep = File.pathSeparator
    systemProperty("java.library.path",
        "${luaLibDir.get().asFile.absolutePath}${sep}${quickjsLibDir.get().asFile.absolutePath}")

    // Forward script engine selection: ./gradlew test -Psce.script.engine=lua
    val engineProp = providers.gradleProperty("sce.script.engine")
    if (engineProp.isPresent) {
        systemProperty("sce.script.engine", engineProp.get())
    }

    // W3C test timeouts: 10s per test (most complete in <100ms)
    systemProperty("junit.jupiter.execution.timeout.default", "10s")

    // Two independent parallelism knobs act on this one suite, and they
    // MULTIPLY. Gradle forks `maxParallelForks` test JVMs; JUnit then sizes a
    // pool inside each one. Sized separately, each is reasonable and the
    // product is not: on a 32-processor machine the fork count was 16 and the
    // `dynamic` strategy asked each fork for `availableProcessors * 2` = 64,
    // because a forked JVM sees the whole machine and knows nothing about its
    // 15 siblings. That is 1024 concurrent tests on 32 processors.
    //
    // Almost every test here is a few microsteps and does not notice. The one
    // that does is any test whose VERDICT is wall-clock: a W3C document that
    // arms `<send event="timeout" delay="2s"/>` as its own failure timer is
    // racing real time, and the harness's `tick()` loop has to be scheduled
    // often enough to finish the document's work before that timer fires.
    // Measured 2026-08-22, test253 (an `<invoke type="scxml">` handshake behind
    // exactly such a timer) took 0.51s run alone and 5.5s run in the suite, and
    // failed every time in the suite — including on `main`, where the red had
    // been standing unread.
    //
    // So the fork count stays, and the per-fork pool is derived FROM it rather
    // than from the machine a second time. Both numbers now come from one
    // reading of `availableProcessors`, and their product is that reading.
    val cpus = Runtime.getRuntime().availableProcessors()
    val forks = (cpus / 2).coerceAtLeast(1)

    // JUnit5 parallel execution, bounded so the whole suite fits the machine.
    systemProperty("junit.jupiter.execution.parallel.enabled", "true")
    systemProperty("junit.jupiter.execution.parallel.mode.default", "concurrent")
    // `fixed` rather than `dynamic`: the dynamic strategy multiplies a factor
    // by the processor count the JVM can see, which is the whole machine no
    // matter how many forks are sharing it. A fixed number is the only way to
    // say "this fraction of the machine".
    systemProperty("junit.jupiter.execution.parallel.config.strategy", "fixed")
    systemProperty(
        "junit.jupiter.execution.parallel.config.fixed.parallelism",
        (cpus / forks).coerceAtLeast(1).toString()
    )

    // Gradle-level fork control
    maxParallelForks = forks

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
