plugins {
    alias(libs.plugins.kotlin.jvm)
}

group = "com.sce"
version = "1.0.0"

dependencies {
    implementation(project(":sce-kotlin-runtime"))
    implementation(libs.kotlinx.coroutines.core)

    testImplementation(kotlin("test"))
    testImplementation(libs.junit.jupiter)
}

// ---------------------------------------------------------------------------
// Native Library Build (Lua 5.4 + JNI bridge)
//
// Two-step process using separate tasks to satisfy configuration cache:
//   1. cmakeConfigure: runs cmake to generate build files
//   2. buildNativeLib: runs cmake --build to compile
// ---------------------------------------------------------------------------

val luaSrcDir = rootProject.file("third_party/lua/src")
val nativeSrcDir = file("src/main/cpp")
val nativeBuildDir = layout.buildDirectory.dir("native")
val nativeLibDir = layout.buildDirectory.dir("native/lib")

val cmakeConfigure by tasks.registering(Exec::class) {
    group = "native"
    description = "Configure CMake for Lua JNI library"

    val buildDir = nativeBuildDir.get().asFile
    val libDir = nativeLibDir.get().asFile

    inputs.dir(nativeSrcDir).withPropertyName("jniSources")
    outputs.file(nativeBuildDir.map { it.file("CMakeCache.txt") })

    doFirst {
        buildDir.mkdirs()
        libDir.mkdirs()
    }

    workingDir(buildDir)
    commandLine("cmake",
        nativeSrcDir.absolutePath,
        "-DLUA_SRC_DIR=${luaSrcDir.absolutePath}",
        "-DCMAKE_LIBRARY_OUTPUT_DIRECTORY=${libDir.absolutePath}",
        "-DCMAKE_BUILD_TYPE=Release",
        "-DJAVA_HOME=${System.getProperty("java.home")}")

    onlyIf {
        File("/usr/bin/cmake").exists()
                || System.getenv("PATH")?.split(File.pathSeparator)
                    ?.any { File(it, "cmake").exists() } == true
    }
}

val buildNativeLib by tasks.registering(Exec::class) {
    group = "native"
    description = "Build Lua 5.4 JNI shared library"
    dependsOn(cmakeConfigure)

    inputs.dir(luaSrcDir).withPropertyName("luaSources")
    inputs.dir(nativeSrcDir).withPropertyName("jniSources")
    outputs.dir(nativeLibDir).withPropertyName("nativeLib")

    workingDir(nativeBuildDir)
    commandLine("cmake", "--build", ".", "--parallel")

    onlyIf {
        File("/usr/bin/cmake").exists()
                || System.getenv("PATH")?.split(File.pathSeparator)
                    ?.any { File(it, "cmake").exists() } == true
    }
}

tasks.named("processResources") {
    dependsOn(buildNativeLib)
}

// Add native lib directory to java.library.path for tests
tasks.test {
    useJUnitPlatform()
    val libDir = nativeLibDir.get().asFile
    systemProperty("java.library.path", libDir.absolutePath)
    dependsOn(buildNativeLib)
}

kotlin {
    jvmToolchain(17)
}
