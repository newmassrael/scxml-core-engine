// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Single source of truth for locating the sce-codegen binary from
// Gradle. Apply it, then read the extra properties:
//
//   apply(from = rootProject.file("gradle/sce-codegen.gradle.kts"))
//   val codegenRelative: String by rootProject.extra
//
// Search order is debug first, release second — the same order
// scripts/lib/sce_codegen.sh, cmake/SCEFindCodegen.cmake and the
// Python harness use. Debug leads because that is the profile every
// build path in this repository now produces: the generator's cost is
// process start-up and I/O rather than optimisation, so a release
// build only compiles the dependency tree a second time instead of
// sharing the one clippy and the test suite already produced. Release
// stays in the search path so a tree still holding an older release
// build keeps working, and it is looked at second so a stale binary
// cannot outrank a fresh one.
//
// Consumers read these properties instead of naming a profile, because
// naming one is what broke: the profile was spelled out independently
// at ~100 sites across five languages, and moving it moved only some
// of them — the conformance jobs then looked for a release binary CI
// no longer produced. `codegen_binary_resolution.rs` fails if a
// profile-specific path reappears outside the four ecosystem locators.
//
// All values are plain strings and lists evaluated eagerly, so nothing
// here leaks a Gradle script object into task state and the
// configuration cache stays usable.

import org.gradle.kotlin.dsl.extra

val sceRootDir: String = rootProject.projectDir.absolutePath
val sceProfiles = listOf("debug", "release")

// The build this repository performs, and the path it lands the binary in.
rootProject.extra["sceCodegenBuildArgs"] = listOf(
    "cargo", "build",
    "--bin", "sce-codegen",
    "--features", "cli",
    "-p", "sce-build",
)
val sceCodegenBuiltRelative = "target/${sceProfiles.first()}/sce-codegen"
rootProject.extra["sceCodegenBuiltRelative"] = sceCodegenBuiltRelative

val sceCodegenExistingRelative: String? = sceProfiles
    .map { "target/$it/sce-codegen" }
    .firstOrNull { File(sceRootDir, it).exists() }

val sceCargoOnPath: Boolean = System.getenv("PATH")
    ?.split(File.pathSeparator)
    ?.any { File(it, "cargo").let { c -> c.exists() && c.canExecute() } }
    ?: false

// Path for task wiring. When cargo is present the build task refreshes
// the debug binary, so wire to that rather than to a stale binary an
// older profile may still hold. Without cargo — the CI conformance
// jobs, which download a prebuilt artifact — wire to whichever profile
// actually holds one, and fall back to the built path so a missing
// binary is reported against the name the build would have produced.
rootProject.extra["sceCodegenRelative"] =
    if (sceCargoOnPath) sceCodegenBuiltRelative
    else sceCodegenExistingRelative ?: sceCodegenBuiltRelative

// Absolute path of an existing binary, PATH included, or null. For
// builds that degrade gracefully when no generator is available rather
// than building one.
rootProject.extra["sceCodegenAbsoluteOrNull"] =
    sceCodegenExistingRelative?.let { File(sceRootDir, it).absolutePath }
        ?: System.getenv("PATH")
            ?.split(File.pathSeparator)
            ?.map { File(it, "sce-codegen") }
            ?.firstOrNull { it.exists() }
            ?.absolutePath
