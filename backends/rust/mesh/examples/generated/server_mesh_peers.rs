// SCE-GENERATED — DO NOT EDIT
// source-hash: 34aaab241046b30864ac859c2172c0519c3149860e3abe044095473301ada5f1

#![doc = "SCE-MAP: server.scxml:11 :: _machine"]
// SCE-MAP: server.scxml:11 :: _machine
// server's Mesh peers, as deploy.yaml binds them (SCE_MESH.md §mesh-19),
// generated from server.
//
// GENERATED — DO NOT EDIT. `sce-codegen generate --deploy <deploy.yaml> --lang rust`.
// A host builds its router with `MACHINE.router()` and opens one connection per
// `PeerLink` below; every rule about envelopes lives in `sce_rust_mesh`.

use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::peers::{Machine, Peer, PeerLink};
use sce_rust_mesh::router::PeerConfig;

/// server's side of the deployment.
pub const MACHINE: Machine = Machine {
    name: "server",
    dedup_window: 64,
    gap_timeout_ms: 100,
    peers: &[Peer {
        name: "client",
        config: PeerConfig {
            transport: "wss",
            buffer: Some(sce_rust_mesh::outbound::OutboundBuffer {
                max_pending: 64,
                max_age_ms: 0,
            }),
            retry: None,
            stamp_sequence: false,
            delivery: Delivery {
                dedup: true,
                ordered: false,
            },
            responders: &["client"],
            deadline_ms: None,
            reply_events: &[],
        },
        link: PeerLink::WssAccept { keepalive_ms: None },
    }],
};
