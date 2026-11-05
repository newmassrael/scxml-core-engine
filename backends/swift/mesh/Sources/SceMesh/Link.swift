// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a link (one connection to one peer) tells its host, and how the host hands that to its
// `Endpoint`. The same four events the Rust binding's links report (backends/rust/mesh/src/wss.rs)
// and the Kotlin core's `LinkEvent`; a transport adapter produces them on its own thread and the host
// delivers them on the engine's.

/// What a link tells its host.
public enum LinkEvent: Equatable, Sendable {
    /// The connection opened: `peer`'s binding is ready.
    case ready(peer: String)

    /// A binary message from `peer`: one envelope's bytes.
    case received(peer: String, bytes: [UInt8])

    /// An envelope queued for `peer` that the link closed before writing.
    case unsent(peer: String, bytes: [UInt8])

    /// The link to `peer` closed, failed, or stopped answering pings; `reason` says which.
    case lost(peer: String, reason: String)
}

/// Hand a link's event to the endpoint it concerns.
public func deliver(_ event: LinkEvent, to endpoint: Endpoint) {
    switch event {
    case let .ready(peer):
        endpoint.peerReady(peer)
    case let .received(peer, bytes):
        endpoint.receive(peer: peer, bytes: bytes)
    case let .unsent(peer, bytes):
        endpoint.transmitFailed(
            peer: peer, bytes: bytes,
            failure: TransportFailure(retryable: true, message: "the link closed before the envelope was written"))
    case let .lost(peer, _):
        endpoint.peerNotReady(peer)
    }
}

/// The path a client opens, before its own machine name (SCE_MESH.md §mesh-18.1).
public let wssPathPrefix = "/sce-mesh/1/"

/// Whether a name is a path segment as it stands: RFC 3986's unreserved characters, so no encoding can
/// make two names one.
public func isPathSegment(_ name: String) -> Bool {
    !name.isEmpty
        && name.utf8.allSatisfy { byte in
            switch byte {
            case UInt8(ascii: "a")...UInt8(ascii: "z"), UInt8(ascii: "A")...UInt8(ascii: "Z"),
                UInt8(ascii: "0")...UInt8(ascii: "9"), UInt8(ascii: "-"), UInt8(ascii: "."), UInt8(ascii: "_"),
                UInt8(ascii: "~"):
                true
            default:
                false
            }
        }
}
