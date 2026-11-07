// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A machine's Mesh peers as its deployment binds them (SCE_MESH.md §mesh-19): the shape
// `sce-codegen generate --deploy --language swift` fills in for a machine's peer table, and the router
// it builds. The Swift twin of backends/kotlin/mesh Peers.kt and backends/rust/mesh/src/peers.rs.
//
// The generated table is data only. Every decision about a binding (whether its envelopes are
// deduplicated, ordered, buffered, retried) was taken by the build from deploy.yaml, with the same
// function that decides it for the C++ router, and lands here as a `PeerConfig`. What a host still does
// by hand is open the connections each `PeerLink` names.

/// How the link to a peer is opened. The core never reads it; the host's transport adapter does. (Not
/// `Binding`, which is what a §mesh-16.7 row names.)
public enum PeerLink: Equatable, Sendable {
    /// §mesh-18.1: this machine dials the peer at `url` (`wss://<host>[:<port>][<base>]`).
    /// `keepaliveMs` is the §mesh-18.3 ping interval, or nil for the binding's 30 s.
    case wssDial(url: String, keepaliveMs: UInt32?)

    /// §mesh-18.1: the peer dials this machine.
    case wssAccept(keepaliveMs: UInt32?)

    /// SCE_MESH.md section 20: the peer is reached through the one Zenoh session this machine opens to its
    /// router, by the peer's name. It carries no parameters: the key is derived from the two machine names (20.1) and
    /// the router endpoints are the host's own client configuration, which the host gives `ZenohInbox` (or
    /// `ZenohReconnectingInbox`) as a `ZenohClient`.
    case zenohInbox

    /// The §mesh-18.3 ping interval, or nil for the binding's 30 s. A Zenoh link has none: presence is the
    /// router's, and the binding keeps no keepalive of its own (20.3).
    public var keepaliveMs: UInt32? {
        switch self {
        case let .wssDial(_, keepaliveMs): keepaliveMs
        case let .wssAccept(keepaliveMs): keepaliveMs
        case .zenohInbox: nil
        }
    }
}

/// One peer: the name `#<name>` addresses, how the core treats envelopes to and from it, and how it is
/// reached.
public struct Peer: Equatable, Sendable {
    public var name: String
    public var config: PeerConfig
    public var link: PeerLink

    public init(name: String, config: PeerConfig, link: PeerLink) {
        self.name = name
        self.config = config
        self.link = link
    }
}

/// One machine's side of a deployment: its `name` (the `source` its envelopes carry), deploy.yaml's
/// `dedup.window_size` (`dedupWindow`, §mesh-10.5) and `ordering.gap_timeout_ms` (`gapTimeoutMs`,
/// §mesh-10.6.1), and every peer it is bound to.
public struct MeshMachine: Equatable, Sendable {
    public var name: String
    public var dedupWindow: UInt32
    public var gapTimeoutMs: Int64
    public var peers: [Peer]

    public init(name: String, dedupWindow: UInt32, gapTimeoutMs: Int64, peers: [Peer]) {
        self.name = name
        self.dedupWindow = dedupWindow
        self.gapTimeoutMs = gapTimeoutMs
        self.peers = peers
    }

    /// A router holding every peer this machine is bound to.
    public func router() -> Router {
        let router = Router(machine: name, dedupWindow: dedupWindow, gapTimeoutMs: gapTimeoutMs)
        for peer in peers {
            router.addPeer(peer.name, config: peer.config)
        }
        return router
    }

    /// The peer named `name`, if the machine is bound to one.
    public func peer(_ name: String) -> Peer? {
        peers.first { $0.name == name }
    }
}
