plugins {
    alias(libs.plugins.kotlin.jvm)
}

group = "com.sce"
version = "1.0.0"

dependencies {
    implementation(project(":sce-kotlin-runtime"))
    implementation(project(":sce-kotlin-lua"))
    implementation(libs.kotlinx.coroutines.core)
    implementation(libs.rhino)

    testImplementation(kotlin("test"))
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.junit.jupiter)
}

// ---------------------------------------------------------------------------
// SCXML Code Generation (CMake parity)
//
// Calls tools/generate_kotlin_w3c.py to generate:
//   - src/main/kotlin/com/sce/generated/testXXX/  (state machine code)
//   - src/test/kotlin/com/sce/w3c/TestXXX.kt      (JUnit5 test classes)
//
// Incremental: only runs when SCXML files, templates, or codegen scripts change.
// Generated files are kept in git so builds work without Python3.
// ---------------------------------------------------------------------------

val generateScxml by tasks.registering(Exec::class) {
    group = "code generation"
    description = "Generate Kotlin state machines and test classes from W3C SCXML tests"

    workingDir = rootProject.projectDir
    commandLine("python3", "tools/generate_kotlin_w3c.py")

    // Inputs: SCXML sources + codegen infrastructure + test registry
    inputs.dir(rootProject.file("resources"))
        .withPropertyName("scxmlResources")
        .withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.dir(rootProject.file("tools/codegen"))
        .withPropertyName("codegenScripts")
        .withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.file(rootProject.file("tools/generate_kotlin_w3c.py"))
        .withPropertyName("generatorScript")
        .withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.file(rootProject.file("tests/CMakeLists.txt"))
        .withPropertyName("testRegistry")
        .withPathSensitivity(PathSensitivity.RELATIVE)

    // Outputs: generated state machines + test classes
    outputs.dir("src/main/kotlin/com/sce/generated")
        .withPropertyName("generatedSources")
    outputs.dir("src/test/kotlin/com/sce/w3c")
        .withPropertyName("generatedTests")

    // Skip if Python3 is not available (use existing generated files)
    isIgnoreExitValue = false
    onlyIf {
        val python3 = File("/usr/bin/python3").exists()
                || File("/usr/local/bin/python3").exists()
                || System.getenv("PATH")?.split(File.pathSeparator)
                    ?.any { File(it, "python3").exists() } == true
        if (!python3) logger.warn("SCE: python3 not found, using existing generated files")
        python3
    }
}

tasks.named("compileKotlin") {
    dependsOn(generateScxml)
}

tasks.test {
    useJUnitPlatform()

    // C++ DataModelInitHelper pattern: resolve data src paths relative to project root
    workingDir = rootProject.projectDir

    // Lua JNI native library path (from sce-kotlin-lua module)
    val luaLibDir = project(":sce-kotlin-lua").layout.buildDirectory.dir("native/lib")
    systemProperty("java.library.path", luaLibDir.get().asFile.absolutePath)

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
}
