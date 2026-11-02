// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Kotlin lane of the peer core of a device that holds a shared list
// (docs/SCE_SHARED_LIST_PEER_ABI.md). There is one peer core, in Rust; a Kotlin host reaches it
// through the C ABI (backends/rust/peer-capi/include/sce_peer.h) with JNA. This program is that
// host's smallest form: it loads the shared library, makes the cores of the two devices of the
// recorded scenario, gives them the machine's sends, the rounds' answers and a save, with no relay,
// and checks that every text a core gives is the recorded one.
//
// A MODULE OF ITS OWN: the Rust test that runs it builds `installDist` and nothing else.
// Built with `./gradlew :sce-kotlin-peer-replay:installDist`; the Rust test reads the distribution's
// `lib/` directory.

plugins {
    alias(libs.plugins.kotlin.jvm)
    application
}

group = "com.sce"
version = "1.0.0"

dependencies {
    // The ABI's declarations are the binding's (backends/kotlin/peer), which every Kotlin host of
    // the core shares: the replay calls the functions the header names and declares none of them.
    implementation(project(":sce-kotlin-peer"))
    implementation(libs.jna)
    implementation(libs.kotlinx.serialization.json)
}

application {
    mainClass.set("com.sce.peer.ReplayKt")
}

kotlin {
    jvmToolchain(17)
    compilerOptions {
        allWarningsAsErrors.set(true)
    }
}
