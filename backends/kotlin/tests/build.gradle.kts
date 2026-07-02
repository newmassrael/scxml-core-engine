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
}

// ---------------------------------------------------------------------------
// SCXML Code Generation (CMake parity)
//
// Uses sce-codegen (Rust binary) to generate:
//   - src/main/kotlin/com/sce/generated/testXXX/  (state machine code)
//   - src/test/kotlin/com/sce/w3c/TestXXX.kt      (JUnit5 test classes)
//
// Build sce-codegen: cargo build --bin sce-codegen --features cli --release -p sce-build
// Incremental: only runs when SCXML files, templates, or test registry change.
// Generated files are kept in git so builds work without sce-codegen.
// ---------------------------------------------------------------------------

// Configuration-cache-safe codegen binary resolution.  All values are plain
// strings evaluated eagerly so no Gradle script objects leak into task state.
val rootDir: String = rootProject.projectDir.absolutePath
val sceCodegenBinary: String? = listOf(
    "target/release/sce-codegen", "target/debug/sce-codegen"
).firstOrNull {
    File(rootDir, it).exists()
}?.let { File(rootDir, it).absolutePath }
    ?: System.getenv("PATH")?.split(File.pathSeparator)
        ?.map { File(it, "sce-codegen") }
        ?.firstOrNull { it.exists() }?.absolutePath
val hasCodegen: Boolean = sceCodegenBinary != null

val generateScxml by tasks.registering(Exec::class) {
    group = "code generation"
    description = "Generate Kotlin state machines and test classes from W3C SCXML tests"

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

    // Outputs: generated state machines + test classes
    outputs.dir("src/main/kotlin/com/sce/generated")
        .withPropertyName("generatedSources")
    outputs.dir("src/test/kotlin/com/sce/w3c")
        .withPropertyName("generatedTests")

    // Skip if sce-codegen is not available (use existing generated files)
    isIgnoreExitValue = false
    enabled = hasCodegen
}

tasks.named("compileKotlin") {
    dependsOn(generateScxml)
}

tasks.test {
    useJUnitPlatform()

    // C++ DataModelInitHelper pattern: resolve data src paths relative to project root
    workingDir = File(rootDir)

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

    // JUnit5 parallel execution
    systemProperty("junit.jupiter.execution.parallel.enabled", "true")
    systemProperty("junit.jupiter.execution.parallel.mode.default", "concurrent")
    systemProperty("junit.jupiter.execution.parallel.config.strategy", "dynamic")
    systemProperty("junit.jupiter.execution.parallel.config.dynamic.factor", "2")

    // Gradle-level fork control
    maxParallelForks = (Runtime.getRuntime().availableProcessors() / 2).coerceAtLeast(1)

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
