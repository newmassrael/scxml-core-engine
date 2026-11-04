// swift-tools-version:6.0
// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import PackageDescription

// docs/adr/0057 §10: the Swift machine generated for every build of
// `tests/product_line/session.scxml`, compiled once for each feature set with `.define`
// of each feature's compilation condition, beside the machine that set's variant
// generates alone. `scripts/regen_swift_feature_conformance.sh` writes both into
// `Sources/`, and neither is committed: the lane judges what the generator writes today.
//
// Each module is one build of one machine, so the six declare the same names and a test
// file imports the two of one feature set.
let featureSets: [(name: String, conditions: [String])] = [
    ("None", []),
    ("Unicast", ["SCE_FEATURE_SESSION_UNICAST"]),
    ("Accept", ["SCE_FEATURE_SESSION_UNICAST", "SCE_FEATURE_SESSION_UNICAST_ACCEPT"]),
]

let runtime: Target.Dependency = .product(name: "SceRuntime", package: "runtime")

let machines: [Target] = featureSets.flatMap { set in
    [
        .target(
            name: "Lifted\(set.name)",
            dependencies: [runtime],
            swiftSettings: set.conditions.map { .define($0) }
        ),
        .target(name: "Alone\(set.name)", dependencies: [runtime]),
    ]
}

let package = Package(
    name: "SceFeatureConformance",
    platforms: [.macOS(.v15), .iOS(.v18)],
    dependencies: [
        .package(path: "../runtime")
    ],
    targets: machines + [
        .testTarget(
            name: "FeatureConformanceTests",
            dependencies: [runtime]
                + featureSets.flatMap { set in
                    [
                        Target.Dependency.target(name: "Lifted\(set.name)"),
                        Target.Dependency.target(name: "Alone\(set.name)"),
                    ]
                }
        )
    ]
)
