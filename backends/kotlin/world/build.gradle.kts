// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The step server a world simulation (backends/rust/world) hosts a generated Kotlin
// machine with: one JVM process that wraps one machine on a manual clock and speaks a
// line-delimited JSON protocol on stdin and stdout (see com/sce/world/StepServer.kt and
// the module documentation of `sce_rust_world::external`).
//
// A MODULE OF ITS OWN rather than a source set of `:sce-kotlin-tests`. That module
// compiles every generated W3C and integration machine (about 390 directories) and runs
// its code generator first; a world test needs one machine, and the JVM it starts is
// paid for once per actor per test, so what it loads is what it waits for. The machines
// compiled here are the committed generated ones, read from that module's tree and not
// copied: a machine regenerated there is the machine this module hosts, and there is no
// second tree to drift.
//
// Built with `./gradlew :sce-kotlin-world:installDist`; the world tests read the
// distribution's `lib/` directory.

plugins {
    alias(libs.plugins.kotlin.jvm)
    application
}

group = "com.sce"
version = "1.0.0"

dependencies {
    implementation(project(":sce-kotlin-runtime"))
    // A `datamodel="sce-static"` machine checks its integer operations with the forge
    // runtime's `SceChecked` (docs/SCE_ACCEPTED_SUBSET.md §2.15).
    implementation(project(":sce-forge-runtime-kotlin"))
    implementation(libs.kotlinx.coroutines.core)
    implementation(libs.kotlinx.serialization.json)
}

// The machines this server can host, by the directory their generated source has under
// `com/sce/integration`. Each needs a row in `Machines.kt`, so a name listed here and not
// there is code nothing reaches, and a row there with no name here does not compile.
val hostedMachines = listOf("todo_peer")

val generatedTree = rootProject.file("backends/kotlin/tests/src/main/kotlin/com/sce")

// A generated machine imports the algorithms it calls as `com.sce.generated.<name>.*`, and
// an algorithm imports the record schemas and algorithms it is built on the same way. The
// closure of those imports is read off the sources, so a machine that gains an algorithm
// is compiled with it the day it is regenerated, and no list here is one more place to
// remember.
fun importedAlgorithms(directory: File): Set<String> {
    require(directory.isDirectory) {
        "no generated source at $directory: regenerate it with " +
            "scripts/regen_static_datamodel_kotlin.sh, or remove it from hostedMachines"
    }
    val pattern = Regex("""^import com\.sce\.generated\.(\w+)\.\*""")
    return directory.walkTopDown()
        .filter { it.isFile && it.extension == "kt" }
        .flatMap { it.readLines().asSequence() }
        .mapNotNull { pattern.find(it)?.groupValues?.get(1) }
        .toSet()
}

val hostedAlgorithms: Set<String> = run {
    val found = sortedSetOf<String>()
    var frontier: Set<String> =
        hostedMachines.flatMap { importedAlgorithms(File(generatedTree, "integration/$it")) }.toSet()
    while (frontier.isNotEmpty()) {
        found += frontier
        frontier = frontier
            .flatMap { importedAlgorithms(File(generatedTree, "generated/$it")) }
            .filter { it !in found }
            .toSet()
    }
    found
}

// Each directory is added at its own path rather than at the shared root, so a machine
// this module does not host is not compiled with the ones it does.
sourceSets["main"].kotlin.srcDirs(
    hostedMachines.map { File(generatedTree, "integration/$it") } +
        hostedAlgorithms.map { File(generatedTree, "generated/$it") },
)

application {
    mainClass.set("com.sce.world.MainKt")
}

kotlin {
    jvmToolchain(17)
    compilerOptions {
        allWarningsAsErrors.set(true)
    }
}
