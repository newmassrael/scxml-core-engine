// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The reference host of the standard host features (docs/SCE_HOST_FEATURES.md)
// for Kotlin hosts: the four features' semantics in common code, so a host on
// another Kotlin target takes them as they are, and the JVM's clock and HTTP
// client beside them. The generated machines meet it through the runtime's
// host-invoker registry.

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
                implementation(project(":sce-kotlin-runtime"))
            }
        }
        val commonTest by getting {
            dependencies {
                implementation(kotlin("test"))
            }
        }
    }
}
