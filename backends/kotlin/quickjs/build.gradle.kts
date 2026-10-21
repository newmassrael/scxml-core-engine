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
// Native Library Build (QuickJS + JNI bridge)
//
// Two-step process using separate tasks to satisfy configuration cache:
//   1. cmakeConfigure: runs cmake to generate build files
//   2. buildNativeLib: runs cmake --build to compile
// ---------------------------------------------------------------------------

val quickjsSrcDir = rootProject.file("third_party/quickjs")
val nativeSrcDir = file("src/main/cpp")
val nativeBuildDir = layout.buildDirectory.dir("native")
val nativeLibDir = layout.buildDirectory.dir("native/lib")

// The JNI headers of this module's Java toolchain JDK, the one that loads
// the library; the script says why CMake is not asked to find a JDK.
apply(from = rootProject.file("gradle/sce-jni-headers.gradle.kts"))
val sceJniIncludeDirs: Provider<List<String>> by extra
val sceJniJdkVersion: Provider<String> by extra

val cmakeConfigure by tasks.registering(Exec::class) {
    group = "native"
    description = "Configure CMake for QuickJS JNI library"

    val buildDir = nativeBuildDir.get().asFile
    val libDir = nativeLibDir.get().asFile

    // Resolved here, after the script set the toolchain, and held as plain
    // strings. The JDK is an input: another toolchain JDK means other
    // headers, so the configure reruns instead of keeping a cache that
    // names the old ones.
    val jniIncludeDirs = sceJniIncludeDirs.get()
    inputs.property("jniIncludeDirs", jniIncludeDirs)
    inputs.property("jniJdkVersion", sceJniJdkVersion.get())
    inputs.dir(nativeSrcDir).withPropertyName("jniSources")
    outputs.file(nativeBuildDir.map { it.file("CMakeCache.txt") })

    // A configure starts from no cache. One left by another compiler (a
    // build machine's slot is configured by local and by remote-execution
    // compilers in turn) is not reused: CMake deletes it itself and reruns
    // without the -D options below.
    doFirst {
        buildDir.resolve("CMakeCache.txt").delete()
        buildDir.resolve("CMakeFiles").deleteRecursively()
        buildDir.mkdirs()
        libDir.mkdirs()
    }

    workingDir(buildDir)
    commandLine("cmake",
        nativeSrcDir.absolutePath,
        "-DQUICKJS_SRC_DIR=${quickjsSrcDir.absolutePath}",
        "-DCMAKE_LIBRARY_OUTPUT_DIRECTORY=${libDir.absolutePath}",
        "-DCMAKE_BUILD_TYPE=Release",
        "-DSCE_JNI_INCLUDE_DIRS=${jniIncludeDirs.joinToString(";")}")

    onlyIf {
        File("/usr/bin/cmake").exists()
                || System.getenv("PATH")?.split(File.pathSeparator)
                    ?.any { File(it, "cmake").exists() } == true
    }
}

val buildNativeLib by tasks.registering(Exec::class) {
    group = "native"
    description = "Build QuickJS JNI shared library"
    dependsOn(cmakeConfigure)

    // The configure's cache, so a reconfigure that changed it (another
    // JDK's headers) rebuilds the library rather than answering UP-TO-DATE.
    inputs.files(cmakeConfigure).withPropertyName("cmakeCache")
    inputs.dir(quickjsSrcDir).withPropertyName("quickjsSources")
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
    compilerOptions {
        allWarningsAsErrors.set(true)
    }
}
