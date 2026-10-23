// swift-tools-version:6.0
// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import PackageDescription

// The runtime half of what `sce-codegen` generates from Forge documents for Swift:
// the integer arithmetic contract (SCE_FORGE.md §3.4.1) and, as the Forge kinds land,
// the codecs and collections the generated code calls. Kept apart from the statechart
// runtime `SceRuntime`, as the Go and Kotlin forge runtimes are kept apart from theirs.
let package = Package(
    name: "SceForgeRuntime",
    platforms: [.macOS(.v15), .iOS(.v18)],
    products: [
        .library(name: "SceForgeRuntime", targets: ["SceForgeRuntime"])
    ],
    targets: [
        .target(name: "SceForgeRuntime"),
        .testTarget(name: "SceForgeRuntimeTests", dependencies: ["SceForgeRuntime"]),
    ]
)
