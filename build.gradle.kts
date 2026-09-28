// Root build file — intentionally minimal. Kotlin allWarningsAsErrors is
// configured per-module via kotlin { compilerOptions { } } blocks.

// The one task that builds `sce-codegen` from this checkout's sources, for
// every module that generates with it. A module depends on `:buildSceCodegen`
// rather than registering its own, so two modules in one build never race two
// cargo builds of the same binary.
//
// It runs whenever cargo is on PATH (local dev and the build machine). Gradle's
// up-to-date check short-circuits it when nothing changed, and cargo's own
// incremental build handles any drift Gradle's input tracking misses. The value
// is that no module generates from a binary older than the sources beside it:
// measured 2026-09-28, a Kotlin test run on a checkout that had never built the
// generator resolved one from PATH three days old, regenerated 682 committed
// files with it, and failed to compile what it wrote.
//
// In CI the conformance jobs download a pre-built artifact into `target/debug`
// and have no Rust toolchain; `onlyIf` evaluates PATH at execution time and
// skips this task, so they generate with the downloaded binary.
//
// The cargo-on-PATH check is inlined inside `onlyIf` on purpose: a top-level
// `fun` or `val` would be a script-level reference that Gradle's configuration
// cache refuses to serialize.
apply(from = rootProject.file("gradle/sce-codegen.gradle.kts"))
val sceCodegenBuildArgs: List<String> by rootProject.extra
val sceCodegenBuiltRelative: String by rootProject.extra

tasks.register<Exec>("buildSceCodegen") {
    group = "code generation"
    description = "Build sce-codegen from this checkout's sources"
    workingDir = rootProject.projectDir
    commandLine(sceCodegenBuildArgs)
    inputs.dir(rootProject.layout.projectDirectory.dir("sce-build/src"))
        .withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.file(rootProject.layout.projectDirectory.file("sce-build/Cargo.toml"))
        .withPathSensitivity(PathSensitivity.RELATIVE)
    outputs.file(rootProject.layout.projectDirectory.file(sceCodegenBuiltRelative))
    onlyIf {
        val path = System.getenv("PATH") ?: return@onlyIf false
        path.split(File.pathSeparator).any { File(it, "cargo").let { c -> c.exists() && c.canExecute() } }
    }
}
