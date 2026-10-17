// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Kotlin lane of the group key agreement of a shared list (docs/SCE_SHARED_LIST_MLS.md section 6).
// There is one implementation of the group, in Rust; a Kotlin host reaches it through the C ABI
// (backends/rust/mls-capi/include/sce_mls.h) with JNA. This program is that host's smallest form: it
// loads the shared library, loads a device as it was before it joined, gives it the Welcome and the
// commits of the recorded scenario with the membership log as data, and checks that it derives the
// recorded keys, until the commit that removes it, after which it derives none.
//
// A MODULE OF ITS OWN: the Rust test that runs it builds `installDist` and nothing else.
// Built with `./gradlew :sce-kotlin-mls-replay:installDist`; the Rust test reads the distribution's
// `lib/` directory.

plugins {
    alias(libs.plugins.kotlin.jvm)
    application
}

group = "com.sce"
version = "1.0.0"

dependencies {
    implementation(libs.jna)
    implementation(libs.kotlinx.serialization.json)
}

application {
    mainClass.set("com.sce.mls.ReplayKt")
}

kotlin {
    jvmToolchain(17)
    compilerOptions {
        allWarningsAsErrors.set(true)
    }
}
