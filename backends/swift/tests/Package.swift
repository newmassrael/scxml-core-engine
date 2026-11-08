// swift-tools-version:6.0
// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import PackageDescription

// Two sets of machines are written, and not committed, by the lane that judges them: the W3C suite
// by `sce-codegen generate-w3c -l swift` (`scripts/regen_swift_w3c.sh`) and the ECMA-262 artifact
// by `scripts/regen_swift_ecma262_lowered.sh`. A checkout that has not written one has no sources
// for its machines target, and SwiftPM refuses a target with none. So each pair of targets exists
// where its machines do: `scripts/test_swift_backend.sh w3c` and `... ecma262-lowered` write them
// first, and the lane that judges the packages does not need to.
func machinesWritten(_ directory: String) -> Bool {
    FileManager.default.fileExists(
        atPath: URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("Sources/\(directory)").path)
}

let ecma262LoweredTargets: [Target] =
    machinesWritten("Ecma262LoweredMachines")
    ? [
        .target(
            name: "Ecma262LoweredMachines",
            dependencies: [
                .product(name: "SceRuntime", package: "runtime"),
                .product(name: "SceLua", package: "runtime"),
            ]
        ),
        .testTarget(
            name: "Ecma262LoweredTests",
            dependencies: [
                "Ecma262LoweredMachines",
                .product(name: "SceRuntime", package: "runtime"),
                .product(name: "SceLua", package: "runtime"),
            ]
        ),
    ] : []

let w3cTargets: [Target] =
    machinesWritten("W3cMachines")
    ? [
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
    ] : []

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
        .package(path: "../mesh"),
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
        // The test profile's runtime is the module a test template's unit counts a text's
        // characters through (docs/SCE_TEST_PROFILE.md §11.9).
        .target(
            name: "StaticDatamodelMachines",
            dependencies: [
                .product(name: "SceRuntime", package: "runtime"),
                .product(name: "SceForgeRuntime", package: "forge-runtime"),
                .product(name: "SceTestProfile", package: "runtime"),
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
        // The canonical fixtures of `integration_resources/` that the backend lowers
        // (`scripts/regen_swift_integration.sh`), the Swift channel of the integration suite, and the
        // hand-written driver of each. The documents are of every datamodel, so the target links
        // the script engine and the forge runtime alike.
        .target(
            name: "IntegrationMachines",
            dependencies: [
                .product(name: "SceRuntime", package: "runtime"),
                .product(name: "SceLua", package: "runtime"),
                .product(name: "SceForgeRuntime", package: "forge-runtime"),
            ]
        ),
        .testTarget(
            name: "IntegrationTests",
            dependencies: [
                "IntegrationMachines",
                .product(name: "SceRuntime", package: "runtime"),
                .product(name: "SceLua", package: "runtime"),
                .product(name: "SceForgeRuntime", package: "forge-runtime"),
                // The host Mesh core, for the tests that run a document through it and not only
                // through the door it is registered at.
                .product(name: "SceMesh", package: "mesh"),
            ]
        ),
    ] + w3cTargets + ecma262LoweredTargets
)
