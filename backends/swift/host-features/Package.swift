// swift-tools-version:6.0
// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import PackageDescription

// The reference host of the standard host features (docs/SCE_HOST_FEATURES.md): the
// operations a generated machine's host-run `<invoke>`s ask for, as one object that
// knows no engine, and the function that registers it as the invoker of each type on
// one. The same platforms as the runtime it registers on.
let package = Package(
    name: "SceHostFeatures",
    platforms: [.macOS(.v15), .iOS(.v18)],
    products: [
        .library(name: "SceHostFeatures", targets: ["SceHostFeatures"])
    ],
    dependencies: [
        .package(path: "../runtime")
    ],
    targets: [
        .target(
            name: "SceHostFeatures",
            dependencies: [.product(name: "SceRuntime", package: "runtime")]
        ),
        .testTarget(
            name: "SceHostFeaturesTests",
            dependencies: [
                "SceHostFeatures",
                .product(name: "SceRuntime", package: "runtime"),
            ]
        ),
    ]
)
