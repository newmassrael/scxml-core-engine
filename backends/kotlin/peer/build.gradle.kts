// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Kotlin binding of the peer core of a device that holds a shared list
// (docs/SCE_SHARED_LIST_PEER_ABI.md, ADR 0079): one peer core, in Rust, reached from a JVM through
// the hand-written C header (backends/rust/peer-capi/include/sce_peer.h) with JNA. The header is the
// contract; this module declares it once for Kotlin, function for function, and wraps the handles
// in classes a host holds (`PeerCore`, `PeerLink`, `PeerCancel`), so that a host writes no JNA
// declaration of its own and no protocol of the relay.
//
// A LIBRARY, depended on by every Kotlin host of the core: the replay of the recorded scenario
// (`:sce-kotlin-peer-replay`), the world's Kotlin device (`:sce-kotlin-world`), and an
// application's lane. It loads the shared library from the path its host gives and starts nothing.

plugins {
    alias(libs.plugins.kotlin.jvm)
}

group = "com.sce"
version = "1.0.0"

dependencies {
    // JNA's types are in the binding's own signatures (a `Pointer`, a `Structure`, a `Callback`),
    // so a host that calls the binding names them.
    api(libs.jna)
}

kotlin {
    jvmToolchain(17)
    compilerOptions {
        allWarningsAsErrors.set(true)
    }
}
