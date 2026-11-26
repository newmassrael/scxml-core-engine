#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Run `./gradlew` on a JDK at the floor the Gradle build declares, for a caller
# that is not a gate.
#
# The Kotlin gates choose their JDK with `sce_gate_require_jdk`, from the
# `jvmToolchain(N)` of the Gradle build they run. A build declared outside the gates —
# `bx.toml`'s `build` produces the Kotlin runtime jar the forge
# gates put on `kotlinc -cp` — reaches Gradle without that step, and Gradle takes
# `JAVA_HOME`, else the `java` on PATH, whatever a machine's package manager
# installed last. Gradle 8.11.1 with this repository's Kotlin plugin cannot
# configure on JDK 25 and dies in seconds with a message that names no JDK
# (`* What went wrong: 25.0.4`, measured 2026-09-30 on the build machine).
#
# So this is the one place a non-gate caller asks for the same guarantee. Every
# argument goes to Gradle unchanged:
#
#   scripts/gradlew-on-ci-jdk.sh :sce-forge-runtime-kotlin:jvmJar --console=plain
#
# The pin is the forge runtime's Java toolchain, the module whose jar this exists
# to build.

source "$(dirname "${BASH_SOURCE[0]}")/gates/lib.sh"

sce_gate_require_jdk "$SCE_REPO_ROOT/backends/kotlin/forge-runtime/build.gradle.kts"

exec ./gradlew "$@"
