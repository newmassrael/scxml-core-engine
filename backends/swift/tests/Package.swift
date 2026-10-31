// swift-tools-version:6.0
// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import PackageDescription

// The machines `sce-codegen generate -l swift` writes, compiled against the Swift
// runtime, and the tests that drive each through its `Engine`. `StaticMachines` and
// `StaticDatamodelMachines` hold only generated files
// (`scripts/regen_swift_statecharts.sh`, `scripts/regen_static_datamodel_swift.sh`);
// what drives them is written by hand beside them. The second is the machines of
// `datamodel="sce-static"` (docs/SCE_ACCEPTED_SUBSET.md §2.15), which the shared
// replay scenarios drive. `EcmaScriptMachines` (`scripts/regen_swift_ecmascript.sh`) holds the
// machines of `datamodel="ecmascript"`, whose expressions are lowered to Lua and run by the
// `SceLua` engine; it is a target of its own so that a machine that needs no engine does not
// link Lua. `W3cMachines` and its tests are written, and not committed, by
// `scripts/regen_swift_w3c.sh` from the W3C documents the backend lowers, as Go's W3C suite is.
let package = Package(
    name: "SceSwiftTests",
    platforms: [.macOS(.v15), .iOS(.v18)],
    dependencies: [
        .package(path: "../runtime"),
        .package(path: "../forge-runtime"),
        .package(path: "../host-features"),
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
        .target(
            name: "StaticDatamodelMachines",
            dependencies: [
                .product(name: "SceRuntime", package: "runtime"),
                .product(name: "SceForgeRuntime", package: "forge-runtime"),
            ]
        ),
        .testTarget(
            name: "StaticDatamodelTests",
            dependencies: [
                "StaticDatamodelMachines",
                .product(name: "SceRuntime", package: "runtime"),
                .product(name: "SceForgeRuntime", package: "forge-runtime"),
                .product(name: "SceHostFeatures", package: "host-features"),
            ]
        ),
        .target(
            name: "EcmaScriptMachines",
            dependencies: [
                .product(name: "SceRuntime", package: "runtime"),
                .product(name: "SceLua", package: "runtime"),
            ]
        ),
        .testTarget(
            name: "EcmaScriptMachinesTests",
            dependencies: [
                "EcmaScriptMachines",
                .product(name: "SceRuntime", package: "runtime"),
                .product(name: "SceLua", package: "runtime"),
            ]
        ),
        .target(
            name: "W3cMachines",
            dependencies: [
                .product(name: "SceRuntime", package: "runtime"),
                .product(name: "SceLua", package: "runtime"),
            ]
        ),
        .testTarget(
            name: "W3cMachinesTests",
            dependencies: [
                "W3cMachines",
                .product(name: "SceRuntime", package: "runtime"),
            ]
        ),
    ]
)
