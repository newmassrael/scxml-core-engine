// What every Kotlin conformance suite module shares: `:sce-kotlin-w3c` (the
// W3C machines) and `:sce-kotlin-tests` (the integration machines). Applied
// after the module's `plugins {}` block, by
// `apply(from = rootProject.file("gradle/sce-kotlin-suite.gradle.kts"))`.
//
// Two things live here, and both were one module's until 2026-11-07, when the
// W3C suite left `:sce-kotlin-tests` so that no compilation unit holds both
// populations (backends/kotlin/w3c/build.gradle.kts says why). Written once, so
// the two suites cannot drift apart on how a row replaces their machines or on
// how their tests are run.
//
// Only core Gradle types are named, as in the other scripts under `gradle/`: a
// script applied with `apply(from = …)` does not see the Kotlin plugin's
// classes, so the Kotlin source directory is reached through the core
// `SourceDirectorySet` the plugin registers on each source set.

val repositoryRoot: File = rootProject.projectDir

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
// `sce.generated.overlay` names a tree shaped like this repository, and each
// suite module reads its OWN part of it: the directory at the module's own
// path relative to the repository root. One property therefore serves every
// suite module, and the generator, which writes each population under the
// module that owns it, decides where each part sits.
//
// WHICH populations move is DERIVED from what the overlay carries, and is
// never listed here.
//
// ⚠ This comment said "only `com/sce/generated` moves" and the code below
// hardcoded that one name. The day the backend's default artifact flipped to
// Lua it cost over an hour of a CI runner: `com/sce/integration` holds 41 more
// generated machine directories, they stayed in the committed language, were
// compiled beside the overlay and handed to an engine that cannot read them.
// That does not fail the suite — it HANGS it, because Rhino answers every
// `setCurrentEvent` with an `EcmaError` and the eventless drain never settles.
//
// Reading the overlay's own directories means a caller that generates a
// population this file has never heard of replaces it too, with nothing here
// to update. What this CANNOT do is notice a population the caller forgot to
// generate; that is `scripts/gates/w3c-kotlin.sh`, which refuses an overlay
// not covering every committed source that carries a script-engine language.
//
// ⚠ A suite module whose part the overlay does not carry is refused, not left
// on its committed machines: those are the other language's, and compiling
// them under an overlay run is the mis-supply above, one module over.
val overlayTree: String? = providers.gradleProperty("sce.generated.overlay").orNull
if (overlayTree != null) {
    val ownPart = File(overlayTree, projectDir.relativeTo(repositoryRoot).path)
    val overlayPackages = File(ownPart, "src/main/kotlin/com/sce")
    require(overlayPackages.isDirectory) {
        "sce.generated.overlay=$overlayTree carries no part for $path at " +
            "$overlayPackages. The overlay replaces the committed machines rather " +
            "than adding to them, and this module's committed machines are the " +
            "other language's, so compiling them under this property would hand " +
            "an engine a language it does not read."
    }
    // The populations this module HOLDS, read from its own committed tree, and
    // the overlay replaces those of them it carries. A package directory the
    // overlay carries and this module does not hold is another module's:
    // `generate-w3c --output-dir` writes a suite that stands on its own, so its
    // tree also carries the harness's `com/sce/http`, which
    // `:sce-kotlin-test-harness` provides, and adding it here would compile a
    // second copy of a class the harness already hands this module.
    val held = File(projectDir, "src/main/kotlin/com/sce").listFiles().orEmpty()
        .filter { it.isDirectory }.map { it.name }.toSet()
    val overlaid = overlayPackages.listFiles().orEmpty()
        .filter { it.isDirectory && it.name in held }.map { it.name }.sorted()
    require(overlaid.isNotEmpty()) {
        "sce.generated.overlay=$overlayTree carries none of the package directories " +
            "$path holds (${held.sorted()}) under $overlayPackages, so this build " +
            "would replace nothing while compiling the committed machines under a " +
            "property that says otherwise."
    }
    val kotlinMain =
        the<SourceSetContainer>()["main"].extensions.getByName("kotlin") as SourceDirectorySet
    for (name in overlaid) {
        // Each population is added at its OWN directory rather than at the
        // shared `src/main/kotlin` root, and that is load-bearing: a file in
        // the overlay is then reached at `<stem>/<file>.kt`, which the
        // exclusion below cannot match, while its committed copy is reached at
        // `com/sce/<name>/<stem>/<file>.kt`, which it does. Adding the root
        // instead would exclude BOTH copies and compile neither.
        kotlinMain.srcDir(File(overlayPackages, name))
        // The committed copies are the OTHER language's. Excluding them is what
        // makes this a replacement: kept, they would be duplicate declarations
        // of every generated class, and dropped from the exclude they would be
        // the ones that compiled.
        kotlinMain.exclude("com/sce/$name/**")
    }
}

// ---------------------------------------------------------------------------
// How a suite's tests run.
tasks.named<Test>("test") {
    useJUnitPlatform()

    // C++ DataModelInitHelper pattern: resolve data src paths relative to project root
    workingDir = repositoryRoot

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

    // Where the Lua engine appends its lowering census, when asked for one.
    //
    // ⚠ Forwarded as a PROPERTY, and that is the whole reason this block
    // exists. A Gradle `Test` task forks its own JVM, so `-Dsce…` given to
    // Gradle reaches Gradle and not the tests, and the test JVM's `System.err`
    // does not reach Gradle's log either. Measured 2026-08-30: two runs of a
    // stderr-based probe reported ZERO fallbacks over a run that took the
    // fallback 100 times, because the stream was swallowed. The census goes to
    // a FILE the caller names, so nothing between here and the reader can
    // silently drop it. Both suites append to the one file the caller names.
    val censusProp = providers.gradleProperty("sce.lua.loweringCensus")
    if (censusProp.isPresent) {
        systemProperty("sce.lua.loweringCensus", censusProp.get())
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
