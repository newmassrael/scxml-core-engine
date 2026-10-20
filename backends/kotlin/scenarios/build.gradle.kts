// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The replay of an engine-neutral scenario against a generated Kotlin machine:
// the Kotlin twin of backends/rust/scenarios (`sce-rust-scenarios`). The
// scenarios are data (docs/SCE_ACCEPTED_SUBSET.md, "A scenario of a static
// machine") every backend replays and judges by the same answer; this module is
// what a Kotlin machine is replayed WITH, both by this repository's own suite
// (`:sce-kotlin-tests`) and by a project outside it that generates its own
// machines. It depends on the runtime and the JSON reader and on nothing a JVM
// or an Android build does not already have.

plugins {
    alias(libs.plugins.kotlin.multiplatform)
    `maven-publish`
}

group = "com.sce"
version = "1.0.0"

kotlin {
    jvmToolchain(17)
    jvm()
    compilerOptions {
        allWarningsAsErrors.set(true)
    }

    sourceSets {
        val commonMain by getting {
            dependencies {
                // `api` rather than `implementation`: the replay's signature names
                // the runtime's `SavedState` and `StateMachineEngine.HostSendRequest`
                // and the JSON reader's `JsonObject` and `JsonElement`, so a caller
                // compiles against both whatever else it depends on.
                api(project(":sce-kotlin-runtime"))
                api(libs.kotlinx.serialization.json)
            }
        }
        val commonTest by getting {
            dependencies {
                implementation(kotlin("test"))
            }
        }
    }
}
