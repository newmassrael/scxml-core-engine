// swift-tools-version:6.0
// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import PackageDescription

// The machines `sce-codegen generate -l swift` writes, compiled against the Swift
// runtime, and the tests that drive each through its `Engine`. `StaticMachines` holds
// only generated files (`scripts/regen_swift_statecharts.sh`); what drives them is
// written by hand beside it.
let package = Package(
    name: "SceSwiftTests",
    platforms: [.macOS(.v15), .iOS(.v18)],
    dependencies: [
        .package(path: "../runtime"),
        .package(path: "../forge-runtime"),
    ],
    targets: [
        .target(
            name: "StaticMachines",
            dependencies: [
                .product(name: "SceRuntime", package: "runtime"),
                .product(name: "SceForgeRuntime", package: "forge-runtime"),
            ]
        ),
        .testTarget(
            name: "StaticMachinesTests",
            dependencies: [
                "StaticMachines",
                .product(name: "SceRuntime", package: "runtime"),
            ]
        ),
    ]
)
