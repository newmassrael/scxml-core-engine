import java.io.File

plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.jmh)
}

group = "com.sce"
version = "1.0.0"

dependencies {
    // Engine interface
    implementation(project(":sce-kotlin-runtime"))

    // All three engine implementations
    implementation(project(":sce-kotlin-lua"))
    implementation(project(":sce-kotlin-quickjs"))
    implementation(project(":sce-kotlin-tests"))  // contains RhinoScriptEngine
    implementation(libs.rhino)
}

kotlin {
    jvmToolchain(17)
}

jmh {
    jmhVersion.set("1.37")

    // Default: full benchmark profile
    // Override with: ./gradlew jmh -Pjmh.wi=2 -Pjmh.i=3 -Pjmh.f=1
    warmupIterations.set(
        providers.gradleProperty("jmh.wi").map { it.toInt() }.orElse(5))
    iterations.set(
        providers.gradleProperty("jmh.i").map { it.toInt() }.orElse(10))
    fork.set(
        providers.gradleProperty("jmh.f").map { it.toInt() }.orElse(2))

    // Filter: ./gradlew jmh -Pjmh.include="SessionBenchmark"
    val includePattern = providers.gradleProperty("jmh.include")
    if (includePattern.isPresent) {
        includes.set(listOf(includePattern.get()))
    }

    // Native library paths for Lua + QuickJS JNI
    val luaLibDir = project(":sce-kotlin-lua").layout.buildDirectory.dir("native/lib")
    val quickjsLibDir = project(":sce-kotlin-quickjs").layout.buildDirectory.dir("native/lib")
    val sep = File.pathSeparator
    jvmArgs.addAll(listOf(
        "-Djava.library.path=${luaLibDir.get().asFile.absolutePath}${sep}${quickjsLibDir.get().asFile.absolutePath}"
    ))

    // JSON output for post-processing
    resultFormat.set("JSON")
    resultsFile.set(project.file("build/reports/jmh/results.json"))
}
