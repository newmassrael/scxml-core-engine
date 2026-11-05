// swift-tools-version:6.0
// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import PackageDescription

// The WebSocket binding's client for the Swift host Mesh core (SCE_MESH.md §mesh-18): the socket
// plumbing between one server link and the router core of `SceMesh`, over SwiftNIO. The Swift row of
// the implementations table (§mesh-18.4), beside the Rust one over tokio-tungstenite and the Kotlin
// one over OkHttp.
//
// A package of its own, as the Rust core's `wss` feature is: the core `SceMesh` stays free of any
// network library, and a host that brings its own transport does not resolve SwiftNIO. SwiftNIO and
// not Foundation's `URLSessionWebSocketTask`, because the latter needs a libcurl built with
// WebSocket support on Linux, which the distributions' builds are not (measured on the build
// machines: libcurl 8.5.0 lists no `ws` or `wss` protocol), and a client that cannot be run in the
// lane that builds it is a client nothing holds. The versions are exact: a transport is what a
// peer's bytes cross, and it is bumped on purpose.
let package = Package(
    name: "SceMeshWss",
    platforms: [.macOS(.v15), .iOS(.v18)],
    products: [
        .library(name: "SceMeshWss", targets: ["SceMeshWss"])
    ],
    dependencies: [
        .package(path: "../mesh"),
        .package(path: "../runtime"),
        .package(path: "../forge-runtime"),
        .package(url: "https://github.com/apple/swift-nio.git", exact: "2.103.0"),
        .package(url: "https://github.com/apple/swift-nio-ssl.git", exact: "2.37.5"),
    ],
    targets: [
        .target(
            name: "SceMeshWss",
            dependencies: [
                .product(name: "SceMesh", package: "mesh"),
                .product(name: "NIOCore", package: "swift-nio"),
                .product(name: "NIOConcurrencyHelpers", package: "swift-nio"),
                .product(name: "NIOPosix", package: "swift-nio"),
                .product(name: "NIOHTTP1", package: "swift-nio"),
                .product(name: "NIOWebSocket", package: "swift-nio"),
                .product(name: "NIOSSL", package: "swift-nio-ssl"),
            ]
        ),
        .testTarget(
            name: "SceMeshWssTests",
            dependencies: [
                "SceMeshWss",
                .product(name: "SceMesh", package: "mesh"),
                .product(name: "SceRuntime", package: "runtime"),
                .product(name: "SceForgeRuntime", package: "forge-runtime"),
                .product(name: "NIOCore", package: "swift-nio"),
                .product(name: "NIOConcurrencyHelpers", package: "swift-nio"),
                .product(name: "NIOPosix", package: "swift-nio"),
                .product(name: "NIOHTTP1", package: "swift-nio"),
                .product(name: "NIOWebSocket", package: "swift-nio"),
            ]
        ),
    ]
)
